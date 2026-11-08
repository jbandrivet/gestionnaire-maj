//! Ce qui est partagé par la fenêtre, la veille et la bulle : chemins, réglages,
//! lancement au démarrage, recherche des mises à jour et des pilotes.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub const APP: &str = "gestionnaire-maj";
pub const NOM: &str = "Gestionnaire de mises à jour";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const ACTION_ACTUALISER: &str = "org.gestionnairemaj.actualiser";

// ---------------------------------------------------------------------------
//  Chemins
// ---------------------------------------------------------------------------
fn maison() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"))
}

fn dossier_conf() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| maison().join(".config"))
}

pub fn dossier_cache() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| maison().join(".cache"))
        .join(APP)
}

fn fichier_reglages() -> PathBuf {
    dossier_conf().join(APP).join("reglages.json")
}

fn fichier_demarrage() -> PathBuf {
    dossier_conf().join("autostart").join(format!("{APP}.desktop"))
}

pub fn fichier_verrou() -> PathBuf {
    dossier_cache().join("veille.pid")
}

fn fichier_etat() -> PathBuf {
    dossier_cache().join("etat.json")
}

/// Le programme lui-même (installé dans /usr/bin, ou celui en cours de développement).
pub fn executable() -> PathBuf {
    let installe = PathBuf::from(format!("/usr/bin/{APP}"));
    if installe.exists() {
        return installe;
    }
    std::env::current_exe().unwrap_or(installe)
}

/// L'assistant qui travaille en administrateur (lancé par pkexec).
pub fn aide() -> PathBuf {
    let installe = PathBuf::from(format!("/usr/libexec/{APP}/aide"));
    if installe.exists() {
        return installe;
    }
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join("gestionnaire-maj-aide")))
        .unwrap_or(installe)
}

/// Commande lancée en administrateur par pkexec.
/// (Essais : GMAJ_SIMULER=1 lance l'assistant sans pkexec, en simulation, sans rien modifier.)
pub fn en_admin(args: &[&str]) -> Vec<String> {
    let mut v = Vec::new();
    if std::env::var_os("GMAJ_SIMULER").is_none() {
        v.push("pkexec".to_string());
    }
    v.extend(args.iter().map(|s| s.to_string()));
    v
}

/// Commande qui relit la liste des logiciels (autorisée sans mot de passe une fois installé).
pub fn commande_actualiser() -> Vec<String> {
    let installe = format!("/usr/libexec/{APP}/actualiser");
    if Path::new(&installe).exists() && std::env::var_os("GMAJ_SIMULER").is_none() {
        en_admin(&[&installe])
    } else {
        en_admin(&[&aide().display().to_string(), "actualiser"])
    }
}

pub fn maintenant() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// ---------------------------------------------------------------------------
//  Réglages
// ---------------------------------------------------------------------------
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct Reglages {
    /// Recherche des mises à jour toutes les N heures
    pub intervalle_h: u32,
    /// Rappel des mises à jour déjà signalées mais pas installées
    pub rappel_h: u32,
    pub snap: bool,
    pub flatpak: bool,
    pub pilotes: bool,
    /// Le gestionnaire se met à jour tout seul depuis GitHub
    pub auto_maj_logiciel: bool,
    /// Proposer aussi les versions d'Ubuntu en développement
    pub devel: bool,
}

impl Default for Reglages {
    fn default() -> Self {
        Reglages {
            intervalle_h: 6,
            rappel_h: 24,
            snap: true,
            flatpak: true,
            pilotes: true,
            auto_maj_logiciel: true,
            devel: false,
        }
    }
}

impl Reglages {
    pub fn charger() -> Self {
        fs::read_to_string(fichier_reglages())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn enregistrer(&self) {
        let f = fichier_reglages();
        if let Some(d) = f.parent() {
            let _ = fs::create_dir_all(d);
        }
        if let Ok(t) = serde_json::to_string_pretty(self) {
            let _ = fs::write(f, t);
        }
    }
}

/// Ce que la veille a déjà signalé (pour ne pas répéter la bulle sans arrêt).
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct EtatVeille {
    pub signales: Vec<String>,
    pub t_signal: u64,
    pub pilotes_signales: Vec<String>,
    pub t_pilotes: u64,
    pub t_recherche_pilotes: u64,
    pub version_signalee: String,
    pub t_version: u64,
    pub t_auto_maj: u64,
}

impl EtatVeille {
    pub fn charger() -> Self {
        fs::read_to_string(fichier_etat())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn enregistrer(&self) {
        let _ = fs::create_dir_all(dossier_cache());
        if let Ok(t) = serde_json::to_string(self) {
            let _ = fs::write(fichier_etat(), t);
        }
    }
}

// ---------------------------------------------------------------------------
//  Lancement au démarrage de l'ordinateur
// ---------------------------------------------------------------------------
pub fn demarrage_actif() -> bool {
    match fs::read_to_string(fichier_demarrage()) {
        Ok(t) => !t.lines().any(|l| l.trim() == "Hidden=true" || l.trim() == "X-GNOME-Autostart-enabled=false"),
        Err(_) => false,
    }
}

pub fn regler_demarrage(actif: bool) -> std::io::Result<()> {
    let f = fichier_demarrage();
    if actif {
        if let Some(d) = f.parent() {
            fs::create_dir_all(d)?;
        }
        let contenu = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name={NOM} (veille)\n\
             Comment=Prévient quand des mises à jour ou des pilotes sont disponibles\n\
             Exec={} --veille\n\
             Icon={APP}\n\
             Terminal=false\n\
             NoDisplay=true\n\
             X-GNOME-Autostart-enabled=true\n\
             X-GNOME-Autostart-Delay=20\n\
             X-MATE-Autostart-Delay=20\n",
            executable().display()
        );
        fs::write(&f, contenu)?;
        if veille_en_cours().is_none() {
            lancer_detache(&["--veille"]);
        }
    } else {
        if f.exists() {
            fs::remove_file(&f)?;
        }
        arreter_veille();
    }
    Ok(())
}

/// Numéro du processus de veille s'il tourne.
pub fn veille_en_cours() -> Option<i32> {
    let pid: i32 = fs::read_to_string(fichier_verrou()).ok()?.trim().parse().ok()?;
    let cmd = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let cmd = String::from_utf8_lossy(&cmd);
    (cmd.contains(APP) && cmd.contains("--veille")).then_some(pid)
}

pub fn arreter_veille() {
    if let Some(pid) = veille_en_cours() {
        unsafe {
            libc::kill(pid, libc::SIGTERM);
        }
    }
}

/// Lance une autre copie du programme, détachée de celle-ci.
pub fn lancer_detache(args: &[&str]) {
    let _ = Command::new(executable())
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn();
}

// ---------------------------------------------------------------------------
//  Petits outils
// ---------------------------------------------------------------------------
pub fn taille_lisible(o: u64) -> String {
    let f = o as f64;
    let t = if o < 1000 {
        format!("{o} octets")
    } else if f < 1e6 {
        format!("{:.0} ko", f / 1e3)
    } else if f < 1e9 {
        format!("{:.1} Mo", f / 1e6)
    } else {
        format!("{:.2} Go", f / 1e9)
    };
    t.replace('.', ",")
}

/// « 299MB », « 34.1 MB », « 118,6 Mo »… → octets
fn lire_taille(t: &str) -> u64 {
    let t = t.trim().replace(',', ".");
    let n: String = t.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    let unite = t[n.len()..].trim().to_lowercase();
    let v: f64 = n.parse().unwrap_or(0.0);
    let k = match unite.chars().next() {
        Some('k') => 1e3,
        Some('m') => 1e6,
        Some('g') => 1e9,
        Some('t') => 1e12,
        _ => 1.0,
    };
    (v * k) as u64
}

fn sortie(cmd: &str, args: &[&str], c_locale: bool) -> Option<(bool, String, String)> {
    let mut c = Command::new(cmd);
    c.args(args).stdin(Stdio::null());
    if c_locale {
        c.env("LC_ALL", "C").env("LANGUAGE", "C");
    }
    let o = c.output().ok()?;
    Some((
        o.status.success(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    ))
}

pub fn existe(cmd: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(cmd).is_file()))
        .unwrap_or(false)
}

/// Âge (secondes) de la dernière lecture réussie de la liste des logiciels.
pub fn age_listes() -> u64 {
    let candidats = [
        "/var/lib/apt/periodic/update-success-stamp",
        "/var/lib/apt/lists/partial",
        "/var/lib/apt/lists",
    ];
    for c in candidats {
        if let Ok(m) = fs::metadata(c).and_then(|m| m.modified()) {
            if let Ok(d) = SystemTime::now().duration_since(m) {
                return d.as_secs();
            }
        }
    }
    u64::MAX
}

/// Polkit permet-il de relire la liste sans demander de mot de passe ?
pub fn actualiser_sans_mot_de_passe() -> bool {
    if !Path::new(&format!("/usr/libexec/{APP}/actualiser")).exists() {
        return false;
    }
    let pid = std::process::id().to_string();
    Command::new("pkcheck")
        .args(["--action-id", ACTION_ACTUALISER, "--process", &pid])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn redemarrage_necessaire() -> bool {
    Path::new("/var/run/reboot-required").exists() || Path::new("/run/reboot-required").exists()
}

// ---------------------------------------------------------------------------
//  Mises à jour
// ---------------------------------------------------------------------------
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Groupe {
    Securite,
    Recommandees,
    Autres,
    Progressives,
    Snap,
    Flatpak,
}

impl Groupe {
    pub const TOUS: [Groupe; 6] = [
        Groupe::Securite,
        Groupe::Recommandees,
        Groupe::Autres,
        Groupe::Progressives,
        Groupe::Snap,
        Groupe::Flatpak,
    ];

    pub fn titre(self) -> &'static str {
        match self {
            Groupe::Securite => "Mises à jour de sécurité importantes",
            Groupe::Recommandees => "Mises à jour recommandées",
            Groupe::Autres => "Mises à jour d'autres sources",
            Groupe::Progressives => "Mises à jour progressives (pas encore proposées à tous)",
            Groupe::Snap => "Applications Snap",
            Groupe::Flatpak => "Applications Flatpak",
        }
    }

    pub fn coche_defaut(self) -> bool {
        self != Groupe::Progressives
    }

    /// Le système (apt) ou une application à part (Snap, Flatpak) ?
    pub fn est_apt(self) -> bool {
        !matches!(self, Groupe::Snap | Groupe::Flatpak)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Maj {
    pub nom: String,
    pub resume: String,
    pub description: String,
    pub ancienne: String,
    pub nouvelle: String,
    pub taille: u64,
    pub groupe: Groupe,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Recherche {
    pub majs: Vec<Maj>,
    /// Paquets nouveaux qu'il faudra installer (nouveau noyau…)
    pub nouveaux: Vec<String>,
    /// Paquets qui seront retirés
    pub supprimes: Vec<String>,
    pub taille_nouveaux: u64,
    pub erreur: Option<String>,
    pub redemarrage: bool,
    pub age_listes: u64,
    /// Nouvelle version d'Ubuntu proposée (mise à niveau)
    pub nouvelle_version: Option<NouvelleVersion>,
}

impl Recherche {
    /// Clés « nom=version » des mises à jour proposées d'office (pour la veille).
    pub fn cles(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .majs
            .iter()
            .filter(|m| m.groupe.coche_defaut())
            .map(|m| format!("{}={}", m.nom, m.nouvelle))
            .collect();
        v.sort();
        v
    }
}

/// « Inst nom [ancienne] (nouvelle Origine:version/suite, … [arch]) »
fn lire_inst(l: &str) -> Option<(String, Option<String>, String, String)> {
    let reste = l.strip_prefix("Inst ")?;
    let (nom, reste) = reste.split_once(' ')?;
    let (ancienne, reste) = if let Some(r) = reste.strip_prefix('[') {
        let (a, r) = r.split_once(']')?;
        (Some(a.to_string()), r.trim_start())
    } else {
        (None, reste)
    };
    let dedans = reste.strip_prefix('(')?;
    let dedans = dedans.rsplit_once(')').map(|(a, _)| a).unwrap_or(dedans);
    let (version, origines) = dedans.split_once(' ').unwrap_or((dedans, ""));
    Some((nom.to_string(), ancienne, version.to_string(), origines.to_string()))
}

fn groupe_origine(origines: &str) -> Groupe {
    if origines.contains("-security") || origines.contains("Security") {
        Groupe::Securite
    } else if origines.contains("Ubuntu:") || origines.contains("Debian:") {
        Groupe::Recommandees
    } else {
        Groupe::Autres
    }
}

struct Fiche {
    taille: u64,
    resume: String,
    description: String,
}

/// Taille et description (traduite si possible) de chaque paquet « nom=version ».
fn fiches(demandes: &[String]) -> HashMap<(String, String), Fiche> {
    let mut res = HashMap::new();
    for paquet in demandes.chunks(200) {
        let mut args = vec!["show", "--no-all-versions"];
        args.extend(paquet.iter().map(|s| s.as_str()));
        let Some((_, texte, _)) = sortie("apt-cache", &args, false) else { continue };
        for bloc in texte.split("\n\n") {
            let (mut nom, mut version, mut taille) = (String::new(), String::new(), 0u64);
            let (mut desc, mut desc_trad): (Option<String>, Option<String>) = (None, None);
            let mut tampon_desc = String::new();
            let mut tampon_trad = String::new();
            let mut dans = 0u8; // 1 = Description, 2 = Description-xx
            for ligne in bloc.lines() {
                if let Some(suite) = ligne.strip_prefix(' ') {
                    let t = if suite.trim() == "." { "" } else { suite };
                    match dans {
                        1 => {
                            tampon_desc.push('\n');
                            tampon_desc.push_str(t);
                        }
                        2 => {
                            tampon_trad.push('\n');
                            tampon_trad.push_str(t);
                        }
                        _ => {}
                    }
                    continue;
                }
                dans = 0;
                let Some((cle, val)) = ligne.split_once(':') else { continue };
                let val = val.trim();
                match cle {
                    "Package" => nom = val.to_string(),
                    "Version" => version = val.to_string(),
                    "Size" => taille = val.parse().unwrap_or(0),
                    "Description" => {
                        dans = 1;
                        tampon_desc = val.to_string();
                    }
                    c if c.starts_with("Description-") && c != "Description-md5" => {
                        dans = 2;
                        tampon_trad = val.to_string();
                    }
                    _ => {}
                }
            }
            if !tampon_desc.is_empty() {
                desc = Some(tampon_desc);
            }
            if !tampon_trad.is_empty() {
                desc_trad = Some(tampon_trad);
            }
            if nom.is_empty() {
                continue;
            }
            let d = desc_trad.or(desc).unwrap_or_default();
            let (resume, corps) = d.split_once('\n').unwrap_or((&d, ""));
            res.insert(
                (nom, version),
                Fiche { taille, resume: resume.trim().to_string(), description: corps.trim().to_string() },
            );
        }
    }
    res
}

fn sans_arch(nom: &str) -> &str {
    nom.split_once(':').map(|(n, _)| n).unwrap_or(nom)
}

pub fn chercher_apt(r: &mut Recherche) {
    let Some((ok, texte, err)) =
        sortie("apt-get", &["-s", "-q", "-o", "Debug::NoLocking=1", "-o", "APT::Get::Show-Upgraded=1", "full-upgrade"], true)
    else {
        r.erreur = Some("Impossible de lancer apt-get.".into());
        return;
    };
    if !ok {
        let msg: Vec<&str> = err.lines().filter(|l| l.starts_with("E:")).collect();
        r.erreur = Some(if msg.is_empty() { err.trim().to_string() } else { msg.join("\n") });
        return;
    }
    let mut vues = HashSet::new();
    let mut brutes = Vec::new(); // (nom, ancienne, nouvelle, groupe)
    let mut nouveaux = Vec::new();
    for l in texte.lines() {
        if let Some((nom, ancienne, version, origines)) = lire_inst(l) {
            vues.insert(nom.clone());
            match ancienne {
                Some(a) => brutes.push((nom, a, version, groupe_origine(&origines))),
                None => nouveaux.push((nom, version)),
            }
        } else if let Some(reste) = l.strip_prefix("Remv ") {
            if let Some(nom) = reste.split_whitespace().next() {
                r.supprimes.push(nom.to_string());
            }
        }
    }
    // Ce qu'apt garde de côté (déploiement progressif, conflit…)
    if let Some((_, liste, _)) = sortie("apt", &["list", "--upgradable"], true) {
        for l in liste.lines() {
            let Some((nom, reste)) = l.split_once('/') else { continue };
            let mut champs = reste.split_whitespace();
            let _suite = champs.next();
            let Some(version) = champs.next() else { continue };
            let arch = champs.next().unwrap_or("");
            let ancienne = l
                .split_once("from: ")
                .map(|(_, a)| a.trim_end_matches(']').to_string())
                .unwrap_or_default();
            let natif = !l.contains(":i386") && arch != "i386";
            let nom_complet = if natif || nom.contains(':') { nom.to_string() } else { format!("{nom}:{arch}") };
            if vues.contains(nom) || vues.contains(&nom_complet) {
                continue;
            }
            brutes.push((nom_complet, ancienne, version.to_string(), Groupe::Progressives));
        }
    }
    let demandes: Vec<String> = brutes
        .iter()
        .map(|(n, _, v, _)| format!("{n}={v}"))
        .chain(nouveaux.iter().map(|(n, v)| format!("{n}={v}")))
        .collect();
    let f = fiches(&demandes);
    for (nom, ancienne, nouvelle, groupe) in brutes {
        let fiche = f.get(&(sans_arch(&nom).to_string(), nouvelle.clone()));
        r.majs.push(Maj {
            resume: fiche.map(|x| x.resume.clone()).filter(|s| !s.is_empty()).unwrap_or_else(|| nom.clone()),
            description: fiche.map(|x| x.description.clone()).unwrap_or_default(),
            taille: fiche.map(|x| x.taille).unwrap_or(0),
            nom,
            ancienne,
            nouvelle,
            groupe,
        });
    }
    for (nom, version) in nouveaux {
        r.taille_nouveaux += f.get(&(sans_arch(&nom).to_string(), version)).map(|x| x.taille).unwrap_or(0);
        r.nouveaux.push(nom);
    }
}

pub fn chercher_snap(r: &mut Recherche) {
    if !existe("snap") {
        return;
    }
    let Some((_, texte, _)) = sortie("snap", &["refresh", "--list"], true) else { return };
    let mut installes = HashMap::new();
    if let Some((_, liste, _)) = sortie("snap", &["list"], true) {
        for l in liste.lines().skip(1) {
            let c: Vec<&str> = l.split_whitespace().collect();
            if c.len() >= 2 {
                installes.insert(c[0].to_string(), c[1].to_string());
            }
        }
    }
    for l in texte.lines() {
        let c: Vec<&str> = l.split_whitespace().collect();
        if c.len() < 4 || c[0] == "Name" {
            continue;
        }
        let editeur = c.get(4).map(|e| e.trim_end_matches('*').trim_end_matches('✓')).unwrap_or("");
        r.majs.push(Maj {
            nom: c[0].to_string(),
            resume: format!("Application « {} »", c[0]),
            description: format!(
                "Application installée par Snap.\nÉditeur : {editeur}\nRévision proposée : {}",
                c[2]
            ),
            ancienne: installes.get(c[0]).cloned().unwrap_or_default(),
            nouvelle: c[1].to_string(),
            taille: lire_taille(c[3]),
            groupe: Groupe::Snap,
        });
    }
}

pub fn chercher_flatpak(r: &mut Recherche) {
    if !existe("flatpak") {
        return;
    }
    let Some((_, texte, _)) = sortie(
        "flatpak",
        &["remote-ls", "--updates", "--columns=application,name,version,download-size"],
        true,
    ) else {
        return;
    };
    for l in texte.lines() {
        let c: Vec<&str> = l.split('\t').collect();
        if c.len() < 2 || c[0].is_empty() || c[0] == "Application ID" {
            continue;
        }
        let nom_lisible = c.get(1).copied().unwrap_or(c[0]);
        r.majs.push(Maj {
            nom: c[0].to_string(),
            resume: format!("Application « {nom_lisible} »"),
            description: format!("Application installée par Flatpak.\nIdentifiant : {}", c[0]),
            ancienne: String::new(),
            nouvelle: c.get(2).copied().unwrap_or("").to_string(),
            taille: c.get(3).map(|t| lire_taille(t)).unwrap_or(0),
            groupe: Groupe::Flatpak,
        });
    }
}

pub fn chercher_tout(reglages: &Reglages) -> Recherche {
    let mut r = Recherche::default();
    chercher_apt(&mut r);
    if reglages.snap {
        chercher_snap(&mut r);
    }
    if reglages.flatpak {
        chercher_flatpak(&mut r);
    }
    r.majs.sort_by(|a, b| (a.groupe, &a.nom).cmp(&(b.groupe, &b.nom)));
    r.redemarrage = redemarrage_necessaire();
    r.age_listes = age_listes();
    r.nouvelle_version = chercher_nouvelle_version(reglages.devel);
    r
}

// ---------------------------------------------------------------------------
//  Mise à niveau vers une nouvelle version d'Ubuntu
// ---------------------------------------------------------------------------
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct NouvelleVersion {
    /// « 27.04 »
    pub version: String,
    /// « Tenacious T-Rex »
    pub nom: String,
    pub lts: bool,
    /// version de développement (essais seulement)
    pub developpement: bool,
}

impl NouvelleVersion {
    pub fn titre(&self) -> String {
        let mut t = format!("Ubuntu {}", self.version);
        if self.lts && !self.version.contains("LTS") {
            t.push_str(" LTS");
        }
        if !self.nom.is_empty() {
            t.push_str(&format!(" « {} »", self.nom));
        }
        t
    }
}

/// Version d'Ubuntu installée (« Ubuntu 26.10 »).
pub fn systeme_actuel() -> String {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|t| {
            t.lines()
                .find_map(|l| l.strip_prefix("PRETTY_NAME=").map(|v| v.trim_matches('"').to_string()))
        })
        .unwrap_or_else(|| "votre système".into())
}

/// Essais : GMAJ_DEVEL=1 propose aussi la version en développement.
pub fn essai_devel() -> bool {
    std::env::var_os("GMAJ_DEVEL").is_some()
}

/// Demande à l'outil d'Ubuntu s'il existe une nouvelle version (il suit le réglage
/// « Prompt » de /etc/update-manager/release-upgrades).
pub fn chercher_nouvelle_version(devel: bool) -> Option<NouvelleVersion> {
    let devel = devel || essai_devel();
    if !existe("do-release-upgrade") {
        return None;
    }
    let mut args = vec!["-c"];
    if devel {
        args.push("-d");
    }
    let (ok, texte, err) = sortie("do-release-upgrade", &args, true)?;
    let tout = format!("{texte}\n{err}");
    let version = tout.lines().find_map(|l| {
        let r = l.trim().strip_prefix("New release '")?;
        r.split_once('\'').map(|(v, _)| v.to_string())
    })?;
    if !ok && !devel {
        return None;
    }
    let mut nv = NouvelleVersion { version: version.clone(), developpement: false, ..Default::default() };
    // nom de code dans le fichier téléchargé par l'outil
    let cache = maison().join(".cache/update-manager-core");
    for f in ["meta-release", "meta-release-lts", "meta-release-development"] {
        let Ok(t) = fs::read_to_string(cache.join(f)) else { continue };
        for bloc in t.split("\n\n") {
            let champ = |k: &str| bloc.lines().find_map(|l| l.strip_prefix(k).map(|v| v.trim().to_string()));
            if champ("Version:").is_some_and(|v| v.trim_end_matches(" LTS") == version.trim_end_matches(" LTS")) {
                nv.nom = champ("Name:").unwrap_or_default();
                nv.lts = champ("Version:").is_some_and(|v| v.contains("LTS"));
                nv.developpement = f == "meta-release-development";
            }
        }
    }
    Some(nv)
}

/// Réglage d'Ubuntu : « normal » (toute version), « lts » ou « never ».
pub fn prevenir_versions() -> String {
    fs::read_to_string("/etc/update-manager/release-upgrades")
        .ok()
        .and_then(|t| {
            t.lines()
                .find_map(|l| l.trim().strip_prefix("Prompt=").map(|v| v.trim().to_lowercase()))
        })
        .unwrap_or_else(|| "normal".into())
}

/// Lance l'assistant de mise à niveau d'Ubuntu (sa propre fenêtre, mot de passe demandé).
pub fn lancer_mise_a_niveau(devel: bool) -> Result<(), String> {
    let devel = devel || essai_devel();
    if Path::new("/usr/lib/python3/dist-packages/DistUpgrade/DistUpgradeViewGtk3.py").exists() {
        let mut c = Command::new("pkexec");
        c.args(["do-release-upgrade", "--frontend=DistUpgradeViewGtk3"]);
        if devel {
            c.arg("-d");
        }
        return c.stdin(Stdio::null()).process_group(0).spawn().map(|_| ()).map_err(|e| e.to_string());
    }
    // sans l'assistant graphique : dans un terminal
    let cmd = if devel { "sudo do-release-upgrade -d" } else { "sudo do-release-upgrade" };
    for (t, args) in [
        ("gnome-terminal", vec!["--", "sh", "-c", cmd]),
        ("mate-terminal", vec!["-e", cmd]),
        ("x-terminal-emulator", vec!["-e", cmd]),
    ] {
        if existe(t) && Command::new(t).args(&args).process_group(0).spawn().is_ok() {
            return Ok(());
        }
    }
    Err("Aucun terminal ni assistant de mise à niveau n'a été trouvé (installez ubuntu-release-upgrader-gtk).".into())
}

/// Liste des modifications d'un paquet (téléchargée depuis le dépôt).
pub fn journal_modifications(m: &Maj) -> String {
    match m.groupe {
        Groupe::Snap => return "La liste des modifications n'est pas fournie pour les applications Snap.".into(),
        Groupe::Flatpak => return "La liste des modifications n'est pas fournie pour les applications Flatpak.".into(),
        _ => {}
    }
    let cible = format!("{}={}", sans_arch(&m.nom), m.nouvelle);
    match sortie("apt-get", &["changelog", "-q", &cible], false) {
        Some((true, t, _)) if !t.trim().is_empty() => {
            // seulement les entrées plus récentes que la version installée
            let mut garde = Vec::new();
            // saute la ligne de téléchargement (« Réception de : 1 https://… »)
            let debut = t.lines().position(|l| l.contains("urgency=")).unwrap_or(0);
            for (i, l) in t.lines().skip(debut).enumerate() {
                if i > 0 && !l.starts_with(' ') && !m.ancienne.is_empty() && l.contains(&format!("({})", m.ancienne)) {
                    break;
                }
                garde.push(l);
                if garde.len() > 400 {
                    break;
                }
            }
            garde.join("\n")
        }
        Some((_, _, e)) => format!(
            "La liste des modifications n'est pas disponible pour le moment.\n\n{}",
            e.trim()
        ),
        None => "La liste des modifications n'est pas disponible.".into(),
    }
}

// ---------------------------------------------------------------------------
//  Pilotes
// ---------------------------------------------------------------------------
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Pilote {
    pub paquet: String,
    pub libre: bool,
    pub recommande: bool,
    pub integre: bool,
    pub installe: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Peripherique {
    pub vendeur: String,
    pub modele: String,
    pub pilotes: Vec<Pilote>,
}

impl Peripherique {
    pub fn nom(&self) -> String {
        match (self.vendeur.is_empty(), self.modele.is_empty()) {
            (false, false) => format!("{} — {}", self.vendeur, self.modele),
            (false, true) => self.vendeur.clone(),
            (true, false) => self.modele.clone(),
            _ => "Composant inconnu".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Materiel {
    pub classe: String,
    pub nom: String,
    pub pilote: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Pilotes {
    pub outil: bool,
    pub peripheriques: Vec<Peripherique>,
    pub materiel: Vec<Materiel>,
}

impl Pilotes {
    /// Pilotes recommandés qui ne sont pas encore en service.
    pub fn recommandes_absents(&self) -> Vec<(String, String)> {
        let mut v = Vec::new();
        for p in &self.peripheriques {
            if p.pilotes.iter().any(|d| d.installe && !d.integre) {
                continue;
            }
            for d in &p.pilotes {
                if d.recommande && !d.installe && !d.integre {
                    v.push((p.nom(), d.paquet.clone()));
                }
            }
        }
        v
    }
}

fn paquets_installes(noms: &[String]) -> HashSet<String> {
    let mut res = HashSet::new();
    if noms.is_empty() {
        return res;
    }
    let mut args = vec!["-W", "-f=${Package}\t${db:Status-Abbrev}\n"];
    args.extend(noms.iter().map(|s| s.as_str()));
    if let Some((_, t, _)) = sortie("dpkg-query", &args, true) {
        for l in t.lines() {
            if let Some((n, s)) = l.split_once('\t') {
                if s.starts_with("ii") || s.starts_with("hi") {
                    res.insert(n.to_string());
                }
            }
        }
    }
    res
}

fn traduire_classe(c: &str) -> String {
    let t = match c {
        "VGA compatible controller" | "3D controller" | "Display controller" => "Carte graphique",
        "Network controller" => "Carte réseau sans fil",
        "Ethernet controller" => "Carte réseau",
        "Audio device" | "Multimedia audio controller" => "Carte son",
        "Multimedia controller" | "Multimedia video controller" => "Multimédia",
        "USB controller" => "Contrôleur USB",
        "SATA controller" | "RAID bus controller" | "IDE interface" => "Contrôleur de disques",
        "Non-Volatile memory controller" => "Disque NVMe",
        "Host bridge" => "Pont du processeur",
        "PCI bridge" => "Pont PCI",
        "ISA bridge" => "Pont ISA",
        "SMBus" => "Bus SMBus",
        "Communication controller" | "Serial controller" => "Communication",
        "Signal processing controller" => "Traitement du signal",
        "RAM memory" | "Memory controller" => "Mémoire",
        "Serial bus controller" => "Bus série",
        "Encryption controller" => "Chiffrement",
        "System peripheral" => "Périphérique système",
        "Unassigned class [ff00]" => "Non classé",
        "Processing accelerators" => "Accélérateur",
        "Bluetooth" => "Bluetooth",
        _ => return c.to_string(),
    };
    t.to_string()
}

pub fn chercher_pilotes() -> Pilotes {
    let mut res = Pilotes { outil: existe("ubuntu-drivers"), ..Default::default() };
    if res.outil {
        if let Some((_, t, _)) = sortie("ubuntu-drivers", &["devices"], true) {
            for l in t.lines() {
                if l.starts_with("== ") {
                    res.peripheriques.push(Peripherique { vendeur: String::new(), modele: String::new(), pilotes: vec![] });
                    continue;
                }
                let Some(p) = res.peripheriques.last_mut() else { continue };
                let Some((cle, val)) = l.split_once(':') else { continue };
                let val = val.trim();
                match cle.trim() {
                    "vendor" => p.vendeur = val.to_string(),
                    "model" => p.modele = val.to_string(),
                    "driver" => {
                        let (paquet, drapeaux) = val.split_once(" - ").unwrap_or((val, ""));
                        let d: Vec<&str> = drapeaux.split_whitespace().collect();
                        p.pilotes.push(Pilote {
                            paquet: paquet.trim().to_string(),
                            libre: d.contains(&"free"),
                            recommande: d.contains(&"recommended"),
                            integre: d.contains(&"builtin"),
                            installe: false,
                        });
                    }
                    _ => {}
                }
            }
        }
        res.peripheriques.retain(|p| !p.pilotes.is_empty());
        let noms: Vec<String> =
            res.peripheriques.iter().flat_map(|p| p.pilotes.iter().map(|d| d.paquet.clone())).collect();
        let installes = paquets_installes(&noms);
        for p in &mut res.peripheriques {
            for d in &mut p.pilotes {
                d.installe = installes.contains(&d.paquet);
            }
        }
    }
    if let Some((_, t, _)) = sortie("lspci", &["-vmmk"], true) {
        for bloc in t.split("\n\n") {
            let mut champs = HashMap::new();
            for l in bloc.lines() {
                if let Some((k, v)) = l.split_once(':') {
                    champs.entry(k.trim().to_string()).or_insert_with(|| v.trim().to_string());
                }
            }
            let Some(classe) = champs.get("Class") else { continue };
            let vendeur = champs.get("Vendor").cloned().unwrap_or_default();
            let modele = champs.get("Device").cloned().unwrap_or_default();
            let pilote = champs
                .get("Driver")
                .cloned()
                .or_else(|| champs.get("Module").map(|m| format!("{m} (non chargé)")))
                .unwrap_or_else(|| "Aucun".into());
            res.materiel.push(Materiel {
                classe: traduire_classe(classe),
                nom: format!("{vendeur} {modele}").trim().to_string(),
                pilote,
            });
        }
    }
    res
}

/// Écrit une ligne de journal (utile pour la veille).
pub fn journal(msg: &str) {
    let _ = fs::create_dir_all(dossier_cache());
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(dossier_cache().join("veille.log")) {
        let _ = writeln!(f, "{} {msg}", maintenant());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ligne_inst() {
        let (n, a, v, o) = lire_inst(
            "Inst base-files [14.2ubuntu2] (14.2ubuntu2.1 Ubuntu:26.10/stonking-updates, Ubuntu:26.10/stonking-security [amd64])",
        )
        .unwrap();
        assert_eq!(n, "base-files");
        assert_eq!(a.as_deref(), Some("14.2ubuntu2"));
        assert_eq!(v, "14.2ubuntu2.1");
        assert_eq!(groupe_origine(&o), Groupe::Securite);
        let (_, a, _, o) = lire_inst("Inst linux-image-7.3 (7.3.0-11.11 Ubuntu:26.10/stonking-updates [amd64])").unwrap();
        assert!(a.is_none());
        assert_eq!(groupe_origine(&o), Groupe::Recommandees);
    }

    #[test]
    fn versions() {
        assert!(plus_recente("1.10.0", "1.9.2"));
        assert!(!plus_recente("1.1.0", "1.1.0"));
        assert!(plus_recente("27.04", "26.10"));
        assert_eq!(fin_support("26.10", false), "juillet 2027");
        assert_eq!(fin_support("26.04", true), "avril 2031");
    }

    #[test]
    fn tailles() {
        assert_eq!(lire_taille("299MB"), 299_000_000);
        assert_eq!(lire_taille("118,6 Mo"), 118_600_000);
        assert_eq!(lire_taille("34.1 kB"), 34_100);
        assert_eq!(taille_lisible(12_345_678), "12,3 Mo");
    }
}

// ---------------------------------------------------------------------------
//  Liste des versions d'Ubuntu (pour l'onglet « Mise à niveau »)
// ---------------------------------------------------------------------------
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct VersionUbuntu {
    pub version: String,
    pub nom: String,
    pub lts: bool,
    pub prise_en_charge: bool,
    pub developpement: bool,
    pub installee: bool,
}

impl VersionUbuntu {
    /// Fin de prise en charge estimée : 9 mois (version normale) ou 5 ans (LTS).
    pub fn fin_support(&self) -> String {
        fin_support(&self.version, self.lts)
    }
}

fn nombres(v: &str) -> Vec<u32> {
    v.split(|c: char| !c.is_ascii_digit()).filter(|x| !x.is_empty()).filter_map(|x| x.parse().ok()).collect()
}

/// a est-elle plus récente que b ? (« 1.10.0 » > « 1.9.2 », « 27.04 » > « 26.10 »)
pub fn plus_recente(a: &str, b: &str) -> bool {
    nombres(a) > nombres(b)
}

const MOIS: [&str; 12] = [
    "janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre",
];

pub fn fin_support(version: &str, lts: bool) -> String {
    let n = nombres(version);
    if n.len() < 2 || n[1] == 0 || n[1] > 12 {
        return String::new();
    }
    let (mut annee, mut mois) = (2000 + n[0], n[1]);
    if lts {
        annee += 5;
    } else {
        mois += 9;
        if mois > 12 {
            mois -= 12;
            annee += 1;
        }
    }
    format!("{} {annee}", MOIS[(mois - 1) as usize])
}

/// Version installée : (« 26.10 », LTS ?)
pub fn version_installee() -> (String, bool) {
    let t = fs::read_to_string("/etc/os-release").unwrap_or_default();
    let champ = |k: &str| t.lines().find_map(|l| l.strip_prefix(k).map(|v| v.trim_matches('"').to_string())).unwrap_or_default();
    let v = champ("VERSION_ID=");
    (v, champ("VERSION=").contains("LTS"))
}

fn telecharger(url: &str) -> Option<String> {
    let o = Command::new("curl").args(["-fsSL", "-m", "25", url]).stdin(Stdio::null()).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).into_owned())
}

/// Versions récentes publiées par Ubuntu (la dernière LTS et tout ce qui est plus récent).
pub fn versions_ubuntu() -> Vec<VersionUbuntu> {
    let (actuelle, actuelle_lts) = version_installee();
    let mut liste: Vec<VersionUbuntu> = Vec::new();
    for (url, devel) in [
        ("https://changelogs.ubuntu.com/meta-release", false),
        ("https://changelogs.ubuntu.com/meta-release-development", true),
    ] {
        let Some(t) = telecharger(url) else { continue };
        for bloc in t.split("\n\n") {
            let champ = |k: &str| bloc.lines().find_map(|l| l.strip_prefix(k).map(|v| v.trim().to_string()));
            let Some(v) = champ("Version:") else { continue };
            let num = v.trim_end_matches(" LTS").to_string();
            let item = VersionUbuntu {
                lts: v.contains("LTS"),
                nom: champ("Name:").unwrap_or_default(),
                prise_en_charge: champ("Supported:").as_deref() == Some("1"),
                developpement: devel,
                installee: nombres(&num)[..2.min(nombres(&num).len())] == nombres(&actuelle)[..2.min(nombres(&actuelle).len())],
                version: num,
            };
            liste.retain(|x| x.version != item.version);
            liste.push(item);
        }
    }
    if !actuelle.is_empty() && !liste.iter().any(|x| x.installee) {
        liste.push(VersionUbuntu {
            version: actuelle.clone(),
            lts: actuelle_lts,
            prise_en_charge: true,
            installee: true,
            ..Default::default()
        });
    }
    liste.sort_by(|a, b| nombres(&a.version).cmp(&nombres(&b.version)));
    let derniere_lts = liste.iter().filter(|x| x.lts && !x.developpement).map(|x| x.version.clone()).next_back();
    liste.retain(|x| !plus_recente(&actuelle, &x.version) || Some(&x.version) == derniere_lts.as_ref());
    liste
}

// ---------------------------------------------------------------------------
//  Mise à jour du gestionnaire lui-même depuis GitHub
// ---------------------------------------------------------------------------
pub const DEPOT: &str = "jbandrivet/gestionnaire-maj";
pub const ACTION_AUTO_MAJ: &str = "org.gestionnairemaj.automaj";

/// Dernière version publiée sur GitHub (« 1.2.0 »).
pub fn derniere_version_github() -> Option<String> {
    let t = telecharger(&format!("https://api.github.com/repos/{DEPOT}/releases/latest"))?;
    let j: serde_json::Value = serde_json::from_str(&t).ok()?;
    let tag = j.get("tag_name")?.as_str()?;
    Some(tag.trim_start_matches('v').to_string())
}

/// Le gestionnaire est-il installé par son paquet (et non lancé depuis les sources) ?
pub fn installe_par_paquet() -> bool {
    Path::new(&format!("/usr/libexec/{APP}/auto-maj")).exists()
}

pub fn commande_auto_maj() -> Vec<String> {
    en_admin(&[&format!("/usr/libexec/{APP}/auto-maj")])
}

/// Polkit autorise-t-il la mise à jour du gestionnaire sans mot de passe ?
pub fn auto_maj_sans_mot_de_passe() -> bool {
    if !installe_par_paquet() {
        return false;
    }
    let pid = std::process::id().to_string();
    Command::new("pkcheck")
        .args(["--action-id", ACTION_AUTO_MAJ, "--process", &pid])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Met le gestionnaire à jour en silence ; renvoie la version installée si tout s'est bien passé.
pub fn auto_maj_silencieuse() -> Result<String, String> {
    let c = commande_auto_maj();
    let o = Command::new(&c[0]).args(&c[1..]).stdin(Stdio::null()).output().map_err(|e| e.to_string())?;
    let texte = String::from_utf8_lossy(&o.stdout).into_owned();
    if o.status.success() {
        let v = texte.lines().find_map(|l| l.strip_prefix("@@INSTALLE:")).unwrap_or("").to_string();
        Ok(v)
    } else {
        Err(texte.lines().rev().find(|l| !l.trim().is_empty() && !l.starts_with("@@")).unwrap_or("échec").to_string())
    }
}
