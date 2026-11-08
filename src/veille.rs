//! Veille lancée avec la session : cherche régulièrement les mises à jour et les
//! pilotes, et affiche la bulle quand il y a du nouveau.

use crate::bulle::Annonce;
use crate::commun::*;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

pub fn lancer() {
    let _ = fs::create_dir_all(dossier_cache());
    let Ok(mut verrou) = OpenOptions::new().create(true).truncate(false).read(true).write(true).open(fichier_verrou()) else {
        eprintln!("Impossible de créer {}", fichier_verrou().display());
        return;
    };
    if unsafe { libc::flock(verrou.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        // une veille tourne déjà
        return;
    }
    let _ = verrou.set_len(0);
    let _ = write!(verrou, "{}", std::process::id());
    let _ = verrou.flush();

    let delai: u64 = std::env::var("GMAJ_DELAI").ok().and_then(|v| v.parse().ok()).unwrap_or(60);
    journal(&format!("veille démarrée, première recherche dans {delai} s"));
    sleep(Duration::from_secs(delai));
    // Au démarrage de l'ordinateur, la bulle s'affiche dès qu'il y a des mises à jour,
    // même si elles avaient déjà été signalées avant.
    let mut premier = true;
    loop {
        if !demarrage_actif() && std::env::var_os("GMAJ_FORCER").is_none() {
            journal("lancement au démarrage désactivé : fin de la veille");
            return;
        }
        cycle(premier);
        premier = false;
        // attente jusqu'à la prochaine recherche, minute par minute (réglages relus)
        let debut = maintenant();
        loop {
            sleep(Duration::from_secs(60));
            if !demarrage_actif() && std::env::var_os("GMAJ_FORCER").is_none() {
                return;
            }
            let h = Reglages::charger().intervalle_h.max(1) as u64;
            if maintenant().saturating_sub(debut) >= h * 3600 {
                break;
            }
        }
    }
}

fn cycle(premier: bool) {
    let reg = Reglages::charger();
    let maintenant = maintenant();
    // la liste des logiciels est relue si elle a plus de 3 h (et si polkit le permet sans mot de passe)
    if age_listes() > (reg.intervalle_h.clamp(1, 3) as u64) * 3600 && actualiser_sans_mot_de_passe() {
        let c = commande_actualiser();
        let _ = Command::new(&c[0]).args(&c[1..]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
    let r = chercher_tout(&reg);
    let mut etat = EtatVeille::charger();
    let cles = r.cles();
    let mut annonce = Annonce::default();
    if cles.is_empty() {
        etat.signales.clear();
    } else {
        let nouvelles = cles.iter().any(|c| !etat.signales.contains(c));
        if premier || nouvelles || maintenant.saturating_sub(etat.t_signal) >= reg.rappel_h as u64 * 3600 {
            annonce.nombre = r.majs.iter().filter(|m| m.groupe.coche_defaut()).count();
            annonce.securite = r.majs.iter().filter(|m| m.groupe == Groupe::Securite).count();
            annonce.taille = r.majs.iter().filter(|m| m.groupe.coche_defaut()).map(|m| m.taille).sum::<u64>() + r.taille_nouveaux;
        }
    }
    if reg.pilotes && maintenant.saturating_sub(etat.t_recherche_pilotes) >= 12 * 3600 {
        etat.t_recherche_pilotes = maintenant;
        let absents = chercher_pilotes().recommandes_absents();
        let nouveaux = absents.iter().any(|(_, p)| !etat.pilotes_signales.contains(p));
        if !absents.is_empty() && (nouveaux || maintenant.saturating_sub(etat.t_pilotes) >= 7 * 86400) {
            annonce.pilotes = absents;
        }
    }
    if let Some(nv) = &r.nouvelle_version {
        if premier || etat.version_signalee != nv.version || maintenant.saturating_sub(etat.t_version) >= 7 * 86400 {
            annonce.version = Some(nv.titre());
            etat.version_signalee = nv.version.clone();
            etat.t_version = maintenant;
        }
    }
    journal(&format!("recherche : {} mise(s) à jour, {} à signaler, {} pilote(s)", r.majs.len(), annonce.nombre, annonce.pilotes.len()));
    if annonce.nombre > 0 || !annonce.pilotes.is_empty() || annonce.version.is_some() {
        if annonce.nombre > 0 {
            etat.signales = cles;
            etat.t_signal = maintenant;
        }
        if !annonce.pilotes.is_empty() {
            etat.pilotes_signales = annonce.pilotes.iter().map(|(_, p)| p.clone()).collect();
            etat.t_pilotes = maintenant;
        }
        etat.enregistrer();
        let f = dossier_cache().join("bulle.json");
        if fs::write(&f, serde_json::to_string(&annonce).unwrap_or_default()).is_ok() {
            // la veille attend que la bulle soit fermée
            let _ = Command::new(executable())
                .arg("--bulle")
                .arg(&f)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    } else {
        etat.enregistrer();
    }
}
