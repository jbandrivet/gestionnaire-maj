//! Assistant administrateur du gestionnaire de mises à jour.
//!
//! Lancé par pkexec : il n'accepte que quelques commandes précises et vérifie
//! chaque nom de paquet. Sa sortie est relue par la fenêtre :
//!   « @@ETAPE:texte »      nouvelle étape
//!   « @@pmstatus:… »       avancement d'apt (APT::Status-Fd)
//!   autre ligne            journal détaillé
//!
//! Commandes :
//!   aide actualiser
//!   aide maj [--tout] [--paquets p1 p2 …] [--snap [s1 s2 …]]
//!   aide installer p1 …      (pilotes)
//!   aide retirer p1 …        (pilotes)
//!   aide version normal|lts|never   (prévenir des nouvelles versions d'Ubuntu)

use std::fs;
use std::io::Write;
use std::os::fd::{FromRawFd, OwnedFd};
use std::path::Path;
use std::process::{exit, Command, Stdio};

fn dire(t: &str) {
    let mut s = std::io::stdout().lock();
    let _ = writeln!(s, "{t}");
    let _ = s.flush();
}

fn etape(t: &str) {
    dire(&format!("@@ETAPE:{t}"));
}

fn nom_paquet_valide(n: &str) -> bool {
    let (base, arch) = n.split_once(':').unwrap_or((n, ""));
    let mut c = base.chars();
    matches!(c.next(), Some(x) if x.is_ascii_lowercase() || x.is_ascii_digit())
        && base.len() >= 2
        && base.len() <= 120
        && base.chars().all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || "+-.".contains(x))
        && arch.chars().all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || x == '-')
}

fn nom_snap_valide(n: &str) -> bool {
    !n.is_empty() && n.len() <= 60 && n.chars().all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || x == '-')
}

/// Langue du système (pkexec efface l'environnement).
fn langue() -> String {
    for f in ["/etc/default/locale", "/etc/locale.conf"] {
        if let Ok(t) = fs::read_to_string(f) {
            for l in t.lines() {
                if let Some(v) = l.trim().strip_prefix("LANG=") {
                    let v = v.trim_matches('"');
                    if v.chars().all(|c| c.is_ascii_alphanumeric() || "_.@-".contains(c)) {
                        return v.to_string();
                    }
                }
            }
        }
    }
    "C.UTF-8".into()
}

fn nettoyer(t: &str) -> String {
    // retire les séquences d'échappement du terminal
    let mut s = String::with_capacity(t.len());
    let mut it = t.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\x1b' {
            if it.peek() == Some(&'[') {
                it.next();
                while let Some(&d) = it.peek() {
                    it.next();
                    if d.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        if c.is_control() && c != '\t' {
            continue;
        }
        s.push(c);
    }
    s.trim_end().to_string()
}

/// Lance une commande dans un pseudo-terminal (pour avoir son texte au fil de l'eau)
/// et, pour apt, récupère l'avancement par un tuyau séparé.
fn lancer(prog: &str, args: &[String], statut_apt: bool) -> i32 {
    unsafe {
        let mut maitre: libc::c_int = -1;
        let mut esclave: libc::c_int = -1;
        let ws = libc::winsize { ws_row: 40, ws_col: 132, ws_xpixel: 0, ws_ypixel: 0 };
        if libc::openpty(&mut maitre, &mut esclave, std::ptr::null_mut(), std::ptr::null(), &ws) != 0 {
            dire("Impossible d'ouvrir un pseudo-terminal.");
            return 1;
        }
        libc::fcntl(maitre, libc::F_SETFD, libc::FD_CLOEXEC);
        let mut tuyau = [-1 as libc::c_int; 2];
        if statut_apt {
            if libc::pipe(tuyau.as_mut_ptr()) != 0 {
                tuyau = [-1, -1];
            } else {
                libc::fcntl(tuyau[0], libc::F_SETFD, libc::FD_CLOEXEC);
            }
        }
        let mut tous: Vec<String> = Vec::new();
        if statut_apt && tuyau[1] >= 0 {
            tous.push("-o".into());
            tous.push(format!("APT::Status-Fd={}", tuyau[1]));
        }
        tous.extend(args.iter().cloned());

        let lang = langue();
        let sortie_fd = OwnedFd::from_raw_fd(libc::dup(esclave));
        let erreur_fd = OwnedFd::from_raw_fd(libc::dup(esclave));
        let enfant = Command::new(prog)
            .args(&tous)
            .env_clear()
            .env("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin")
            .env("HOME", "/root")
            .env("LANG", &lang)
            .env("TERM", "dumb")
            .env("COLUMNS", "132")
            .env("DEBIAN_FRONTEND", "noninteractive")
            .env("APT_LISTCHANGES_FRONTEND", "none")
            .env("NEEDRESTART_MODE", "l")
            .env("NEEDRESTART_SUSPEND", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::from(sortie_fd))
            .stderr(Stdio::from(erreur_fd))
            .spawn();
        libc::close(esclave);
        if tuyau[1] >= 0 {
            libc::close(tuyau[1]);
        }
        let mut enfant = match enfant {
            Ok(e) => e,
            Err(e) => {
                dire(&format!("Impossible de lancer {prog} : {e}"));
                libc::close(maitre);
                if tuyau[0] >= 0 {
                    libc::close(tuyau[0]);
                }
                return 1;
            }
        };

        let mut fds = vec![maitre];
        if tuyau[0] >= 0 {
            fds.push(tuyau[0]);
        }
        let mut tampons: Vec<Vec<u8>> = vec![Vec::new(); fds.len()];
        let mut ouverts = vec![true; fds.len()];
        let mut buf = [0u8; 8192];
        while ouverts.iter().any(|o| *o) {
            let mut pfd: Vec<libc::pollfd> = fds
                .iter()
                .zip(&ouverts)
                .map(|(&fd, &o)| libc::pollfd { fd: if o { fd } else { -1 }, events: libc::POLLIN, revents: 0 })
                .collect();
            if libc::poll(pfd.as_mut_ptr(), pfd.len() as libc::nfds_t, 500) < 0 {
                if *libc::__errno_location() == libc::EINTR {
                    continue;
                }
                break;
            }
            for (i, p) in pfd.iter().enumerate() {
                if !ouverts[i] || p.revents == 0 {
                    continue;
                }
                let n = libc::read(fds[i], buf.as_mut_ptr() as *mut libc::c_void, buf.len());
                if n <= 0 {
                    ouverts[i] = false;
                    let reste = std::mem::take(&mut tampons[i]);
                    emettre(&reste, i == 1);
                    continue;
                }
                tampons[i].extend_from_slice(&buf[..n as usize]);
                while let Some(pos) = tampons[i].iter().position(|&b| b == b'\n' || b == b'\r') {
                    let ligne: Vec<u8> = tampons[i].drain(..=pos).collect();
                    emettre(&ligne, i == 1);
                }
            }
        }
        libc::close(maitre);
        if tuyau[0] >= 0 {
            libc::close(tuyau[0]);
        }
        enfant.wait().ok().and_then(|s| s.code()).unwrap_or(1)
    }
}

fn emettre(octets: &[u8], statut: bool) {
    let t = nettoyer(&String::from_utf8_lossy(octets));
    if t.trim().is_empty() {
        return;
    }
    if statut {
        dire(&format!("@@{t}"));
    } else {
        dire(&t);
    }
}

fn simulation() -> bool {
    (unsafe { libc::geteuid() }) != 0
}

fn options_apt() -> Vec<String> {
    let mut v: Vec<String> = if simulation() { vec!["-s".into(), "-o".into(), "Debug::NoLocking=1".into()] } else { Vec::new() };
    v.extend([
        "-y",
        "-q",
        "-o",
        "Dpkg::Options::=--force-confdef",
        "-o",
        "Dpkg::Options::=--force-confold",
        "-o",
        "DPkg::Lock::Timeout=300",
        "-o",
        "Dpkg::Progress-Fancy=0",
        "-o",
        "APT::Color=0",
    ]
    .iter()
    .map(|s| s.to_string()));
    v
}

fn apt(commande: &[&str], paquets: &[String]) -> i32 {
    let mut a = options_apt();
    a.extend(commande.iter().map(|s| s.to_string()));
    a.extend(paquets.iter().cloned());
    lancer("apt-get", &a, true)
}

/// Termine une installation interrompue (coupure de courant…) avant toute chose.
fn reparer_si_besoin() -> i32 {
    let interrompu = fs::read_dir("/var/lib/dpkg/updates").map(|mut d| d.next().is_some()).unwrap_or(false);
    if interrompu && !simulation() {
        etape("Reprise d'une installation interrompue");
        return lancer("dpkg", &["--configure".into(), "-a".into(), "--force-confdef".into(), "--force-confold".into()], false);
    }
    0
}

fn actualiser() -> i32 {
    etape("Lecture de la liste des logiciels disponibles");
    if simulation() {
        return lancer("apt-get", &["-s".into(), "check".into()], false);
    }
    let mut a = options_apt();
    a.push("update".into());
    lancer("apt-get", &a, true)
}

fn regler_prompt(v: &str) -> i32 {
    let f = "/etc/update-manager/release-upgrades";
    let texte = fs::read_to_string(f).unwrap_or_else(|_| "[DEFAULT]\n".into());
    let mut fait = false;
    let mut lignes: Vec<String> = texte
        .lines()
        .map(|l| {
            if !fait && l.trim_start().starts_with("Prompt=") {
                fait = true;
                format!("Prompt={v}")
            } else {
                l.to_string()
            }
        })
        .collect();
    if !fait {
        match lignes.iter().position(|l| l.trim() == "[DEFAULT]") {
            Some(i) => lignes.insert(i + 1, format!("Prompt={v}")),
            None => {
                lignes.push("[DEFAULT]".into());
                lignes.push(format!("Prompt={v}"));
            }
        }
    }
    if simulation() {
        dire(&format!("(simulation) {f} : Prompt={v}"));
        return 0;
    }
    let _ = fs::create_dir_all("/etc/update-manager");
    match fs::write(f, lignes.join("\n") + "\n") {
        Ok(()) => {
            dire(&format!("{f} : Prompt={v}"));
            0
        }
        Err(e) => {
            dire(&format!("Écriture impossible de {f} : {e}"));
            1
        }
    }
}

fn verifier(noms: &[String], f: fn(&str) -> bool) {
    for n in noms {
        if !f(n) {
            dire(&format!("Nom refusé : {n}"));
            exit(2);
        }
    }
}

fn main() {
    if simulation() {
        if std::env::var_os("GMAJ_SIMULER").is_none() {
            eprintln!("Cet assistant doit être lancé par pkexec (administrateur).");
            exit(1);
        }
        dire("(simulation : rien ne sera modifié)");
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(|s| s.as_str()) {
        Some("actualiser") => actualiser(),
        Some("maj") => {
            let mut tout = false;
            let mut paquets = Vec::new();
            let mut snaps = Vec::new();
            let mut snap = false;
            let mut mode = 0;
            for a in &args[1..] {
                match a.as_str() {
                    "--tout" => tout = true,
                    "--paquets" => mode = 1,
                    "--snap" => {
                        snap = true;
                        mode = 2;
                    }
                    n if mode == 1 => paquets.push(n.to_string()),
                    n if mode == 2 => snaps.push(n.to_string()),
                    n => {
                        dire(&format!("Argument inconnu : {n}"));
                        exit(2);
                    }
                }
            }
            verifier(&paquets, nom_paquet_valide);
            verifier(&snaps, nom_snap_valide);
            let mut code = reparer_si_besoin();
            if code == 0 && tout {
                etape("Installation des mises à jour");
                code = apt(&["full-upgrade"], &[]);
            }
            if code == 0 && !paquets.is_empty() {
                etape("Installation des mises à jour choisies");
                code = apt(&["install", "--only-upgrade"], &paquets);
            }
            if code == 0 && snap && Path::new("/usr/bin/snap").exists() && !simulation() {
                etape("Mise à jour des applications Snap");
                let mut a = vec!["refresh".to_string()];
                a.extend(snaps);
                code = lancer("snap", &a, false);
            }
            code
        }
        Some("installer") => {
            let paquets = args[1..].to_vec();
            verifier(&paquets, nom_paquet_valide);
            let mut code = reparer_si_besoin();
            for p in &paquets {
                if code != 0 {
                    break;
                }
                etape(&format!("Installation du pilote {p}"));
                // Les pilotes NVIDIA passent par ubuntu-drivers, qui choisit les bons modules du noyau
                let nv = p.strip_prefix("nvidia-driver-").filter(|_| Path::new("/usr/bin/ubuntu-drivers").exists() && !simulation());
                code = match nv {
                    Some(v) => {
                        dire(&format!("ubuntu-drivers install nvidia:{v}"));
                        let c = lancer("ubuntu-drivers", &["install".into(), format!("nvidia:{v}")], false);
                        if c == 0 { c } else { apt(&["install"], std::slice::from_ref(p)) }
                    }
                    None => apt(&["install"], std::slice::from_ref(p)),
                };
            }
            code
        }
        Some("version") => {
            // Réglage « me prévenir d'une nouvelle version d'Ubuntu »
            let v = args.get(1).map(|s| s.as_str()).unwrap_or("");
            if !matches!(v, "normal" | "lts" | "never") {
                dire("Valeur refusée (normal, lts ou never).");
                exit(2);
            }
            etape("Réglage des nouvelles versions d'Ubuntu");
            regler_prompt(v)
        }
        Some("retirer") => {
            let paquets = args[1..].to_vec();
            verifier(&paquets, nom_paquet_valide);
            let mut code = reparer_si_besoin();
            if code == 0 {
                etape("Désactivation du pilote");
                code = apt(&["purge"], &paquets);
            }
            if code == 0 {
                etape("Nettoyage des paquets devenus inutiles");
                code = apt(&["autoremove", "--purge"], &[]);
            }
            code
        }
        _ => {
            eprintln!("Usage : aide actualiser | maj [--tout] [--paquets …] [--snap …] | installer … | retirer …");
            2
        }
    };
    exit(code);
}
