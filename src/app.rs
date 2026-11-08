//! Fenêtre principale : onglets « Mises à jour » et « Pilotes ».

use crate::commun::*;
use crate::theme::{self, bouton, Etat, Icone, BLANC, BORD, FOND, GRIS, GRIS_CLAIR, LIGNE_PAIRE, SEL, TEXTE};
use eframe::egui::{
    self, pos2, vec2, Align, Align2, Color32, Context, Frame, Id, Layout, Margin, Rect, RichText, Sense, Stroke,
    Ui,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Normal,
    /// Ouverte depuis la bulle : installe tout de suite ce qui est proposé
    Installer,
    /// Ouverte depuis la bulle sur l'onglet des pilotes
    Pilotes,
    /// Ouverte depuis la bulle : propose la mise à niveau vers la nouvelle version d'Ubuntu
    MiseANiveau,
}

/// Mise à jour du gestionnaire lui-même depuis GitHub
#[derive(Clone, PartialEq)]
enum AutoMaj {
    Rien,
    Recherche,
    AJour(String),
    Disponible(String),
    EnCours(String),
    Fait(String),
    Erreur(String),
}

enum Msg {
    Recherche(Recherche),
    Versions(Vec<VersionUbuntu>),
    AutoMaj(AutoMaj),
    Pilotes(Pilotes),
    Modifs(String, String),
    Ligne(String),
    FinEtape(i32),
}

struct Etape {
    texte: String,
    cmd: Vec<String>,
}

#[derive(PartialEq, Clone, Copy)]
enum Apres {
    Recherche,
    Pilotes,
}

struct Travail {
    titre: String,
    explication: String,
    etape: String,
    detail: String,
    progression: Option<f32>,
    journal: String,
    details: bool,
    restantes: VecDeque<Etape>,
    fini: Option<(bool, String)>,
    apres: Apres,
    redemarrage: bool,
    erreurs: Vec<String>,
}

pub struct Appli {
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    ctx: Context,
    onglet: usize,
    reglages: Reglages,
    demarrage: bool,
    message_demarrage: Option<String>,
    // mises à jour
    recherche: Option<Recherche>,
    coche: Vec<bool>,
    replies: HashSet<Groupe>,
    selection: Option<usize>,
    occupe: Option<String>,
    modifs: HashMap<String, Option<String>>,
    vue_desc: usize,
    desc_ouverte: bool,
    auto_installer: bool,
    // pilotes
    pilotes: Option<Pilotes>,
    pilotes_occupe: bool,
    sel_pilote: Option<(usize, usize)>,
    // fenêtres
    travail: Option<Travail>,
    reglages_ouverts: bool,
    a_propos: bool,
    // mise à niveau vers une nouvelle version d'Ubuntu
    confirmer_version: bool,
    demande_version: bool,
    prevenir: String,
    prevenir_initial: String,
    erreur_version: Option<String>,
    versions: Option<Vec<VersionUbuntu>>,
    versions_occupe: bool,
    auto_maj: AutoMaj,
}

impl Appli {
    pub fn nouveau(cc: &eframe::CreationContext<'_>, mode: Mode) -> Self {
        theme::installer(&cc.egui_ctx);
        let (tx, rx) = channel();
        let mut a = Appli {
            tx,
            rx,
            ctx: cc.egui_ctx.clone(),
            onglet: match mode {
                Mode::Pilotes => 1,
                Mode::MiseANiveau => 2,
                _ => 0,
            },
            reglages: Reglages::charger(),
            demarrage: demarrage_actif(),
            message_demarrage: None,
            recherche: None,
            coche: Vec::new(),
            replies: HashSet::new(),
            selection: None,
            occupe: None,
            modifs: HashMap::new(),
            vue_desc: if std::env::var_os("GMAJ_MODIFS").is_some() { 0 } else { 1 },
            desc_ouverte: false,
            auto_installer: mode == Mode::Installer,
            pilotes: None,
            pilotes_occupe: false,
            sel_pilote: None,
            travail: None,
            reglages_ouverts: false,
            a_propos: false,
            confirmer_version: false,
            demande_version: mode == Mode::MiseANiveau,
            prevenir: prevenir_versions(),
            prevenir_initial: prevenir_versions(),
            erreur_version: None,
            versions: None,
            versions_occupe: false,
            auto_maj: AutoMaj::Rien,
        };
        if mode == Mode::MiseANiveau {
            a.lancer_versions();
        }
        a.verifier_gestionnaire(false);
        if std::env::var_os("GMAJ_DESC").is_some() {
            a.desc_ouverte = true;
            a.selection = Some(0);
        }
        // Liste des logiciels vieille de plus de 6 h : on la relit d'abord (sans mot de passe si permis)
        let relire = age_listes() > 6 * 3600 && actualiser_sans_mot_de_passe();
        a.lancer_recherche(relire);
        if mode == Mode::Pilotes {
            a.lancer_pilotes();
        }
        a
    }

    // -----------------------------------------------------------------------
    //  Tâches de fond
    // -----------------------------------------------------------------------
    fn lancer_recherche(&mut self, relire: bool) {
        self.occupe = Some(if relire {
            "Lecture de la liste des logiciels disponibles…".into()
        } else {
            "Recherche des mises à jour…".into()
        });
        let (tx, ctx, reg) = (self.tx.clone(), self.ctx.clone(), self.reglages.clone());
        thread::spawn(move || {
            if relire {
                let c = commande_actualiser();
                let _ = Command::new(&c[0]).args(&c[1..]).stdin(Stdio::null()).output();
            }
            let _ = tx.send(Msg::Recherche(chercher_tout(&reg)));
            ctx.request_repaint();
        });
    }

    fn lancer_pilotes(&mut self) {
        if self.pilotes_occupe {
            return;
        }
        self.pilotes_occupe = true;
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        thread::spawn(move || {
            let _ = tx.send(Msg::Pilotes(chercher_pilotes()));
            ctx.request_repaint();
        });
    }

    fn lancer_versions(&mut self) {
        if self.versions_occupe {
            return;
        }
        self.versions_occupe = true;
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        thread::spawn(move || {
            let _ = tx.send(Msg::Versions(versions_ubuntu()));
            ctx.request_repaint();
        });
    }

    /// Cherche une nouvelle version du gestionnaire sur GitHub et l'installe toute seule
    /// si le réglage le permet (sans mot de passe, grâce à polkit).
    fn verifier_gestionnaire(&mut self, demande: bool) {
        if !installe_par_paquet() || matches!(self.auto_maj, AutoMaj::Recherche | AutoMaj::EnCours(_)) {
            if demande && !installe_par_paquet() {
                self.auto_maj = AutoMaj::Erreur("Lancé depuis les sources : pas de mise à jour automatique.".into());
            }
            return;
        }
        self.auto_maj = AutoMaj::Recherche;
        let auto = self.reglages.auto_maj_logiciel;
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        thread::spawn(move || {
            let envoyer = |e: AutoMaj| {
                let _ = tx.send(Msg::AutoMaj(e));
                ctx.request_repaint();
            };
            match derniere_version_github() {
                None => envoyer(AutoMaj::Erreur("GitHub ne répond pas.".into())),
                Some(v) if !plus_recente(&v, VERSION) => envoyer(AutoMaj::AJour(v)),
                Some(v) if auto && auto_maj_sans_mot_de_passe() => {
                    envoyer(AutoMaj::EnCours(v.clone()));
                    match auto_maj_silencieuse() {
                        Ok(_) => envoyer(AutoMaj::Fait(v)),
                        Err(e) => envoyer(AutoMaj::Erreur(e)),
                    }
                }
                Some(v) => envoyer(AutoMaj::Disponible(v)),
            }
        });
    }

    fn demander_modifs(&mut self, i: usize) {
        let Some(m) = self.recherche.as_ref().and_then(|r| r.majs.get(i)).cloned() else { return };
        let cle = format!("{}={}", m.nom, m.nouvelle);
        if self.modifs.contains_key(&cle) {
            return;
        }
        self.modifs.insert(cle.clone(), None);
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        thread::spawn(move || {
            let _ = tx.send(Msg::Modifs(cle, journal_modifications(&m)));
            ctx.request_repaint();
        });
    }

    fn recevoir(&mut self) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Recherche(r) => {
                    let tout = std::env::var_os("GMAJ_TOUT_COCHER").is_some();
                    self.coche = r.majs.iter().map(|m| tout || m.groupe.coche_defaut()).collect();
                    self.selection = if self.desc_ouverte && !r.majs.is_empty() { Some(0) } else { None };
                    self.occupe = None;
                    if self.demande_version && r.nouvelle_version.is_some() {
                        self.demande_version = false;
                        self.confirmer_version = true;
                    }
                    if !self.reglages_ouverts {
                        self.prevenir = prevenir_versions();
                        self.prevenir_initial = self.prevenir.clone();
                    }
                    self.recherche = Some(r);
                    if self.auto_installer {
                        self.auto_installer = false;
                        if self.coche.iter().any(|c| *c) {
                            self.installer();
                        }
                    }
                }
                Msg::Pilotes(p) => {
                    self.pilotes = Some(p);
                    self.pilotes_occupe = false;
                    self.sel_pilote = None;
                }
                Msg::Versions(v) => {
                    self.versions = Some(v);
                    self.versions_occupe = false;
                }
                Msg::AutoMaj(e) => self.auto_maj = e,
                Msg::Modifs(cle, texte) => {
                    self.modifs.insert(cle, Some(texte));
                }
                Msg::Ligne(l) => self.ligne_travail(l),
                Msg::FinEtape(code) => self.fin_etape(code),
            }
        }
    }

    // -----------------------------------------------------------------------
    //  Travaux en administrateur (fenêtre d'avancement)
    // -----------------------------------------------------------------------
    fn commencer(&mut self, titre: &str, explication: &str, etapes: Vec<Etape>, apres: Apres) {
        if etapes.is_empty() || self.travail.is_some() {
            return;
        }
        self.travail = Some(Travail {
            titre: titre.into(),
            explication: explication.into(),
            etape: "Préparation…".into(),
            detail: String::new(),
            progression: None,
            journal: String::new(),
            details: false,
            restantes: etapes.into(),
            fini: None,
            apres,
            redemarrage: false,
            erreurs: Vec::new(),
        });
        self.etape_suivante();
    }

    fn etape_suivante(&mut self) {
        let Some(t) = self.travail.as_mut() else { return };
        let Some(e) = t.restantes.pop_front() else {
            t.fini = Some((true, String::new()));
            t.progression = Some(1.0);
            t.redemarrage = redemarrage_necessaire();
            let apres = t.apres;
            match apres {
                Apres::Recherche => self.lancer_recherche(false),
                Apres::Pilotes => {
                    self.lancer_pilotes();
                    self.lancer_recherche(false);
                }
            }
            return;
        };
        t.etape = e.texte.clone();
        t.progression = None;
        t.detail.clear();
        t.journal.push_str(&format!("\n━━ {} ━━\n$ {}\n", e.texte, e.cmd.join(" ")));
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        thread::spawn(move || {
            let enfant = Command::new(&e.cmd[0])
                .args(&e.cmd[1..])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();
            let code = match enfant {
                Err(err) => {
                    let _ = tx.send(Msg::Ligne(format!("Impossible de lancer {} : {err}", e.cmd[0])));
                    127
                }
                Ok(mut ch) => {
                    let lire = |flux: Box<dyn std::io::Read + Send>, tx: Sender<Msg>, ctx: Context| {
                        let mut r = BufReader::new(flux);
                        let mut buf = Vec::new();
                        loop {
                            buf.clear();
                            match r.read_until(b'\n', &mut buf) {
                                Ok(0) | Err(_) => break,
                                Ok(_) => {
                                    let l = String::from_utf8_lossy(&buf).trim_end().to_string();
                                    let _ = tx.send(Msg::Ligne(l));
                                    ctx.request_repaint();
                                }
                            }
                        }
                    };
                    let err = ch.stderr.take().map(|f| Box::new(f) as Box<dyn std::io::Read + Send>);
                    let (tx2, ctx2) = (tx.clone(), ctx.clone());
                    let h = err.map(|f| thread::spawn(move || lire(f, tx2, ctx2)));
                    if let Some(o) = ch.stdout.take() {
                        lire(Box::new(o), tx.clone(), ctx.clone());
                    }
                    if let Some(h) = h {
                        let _ = h.join();
                    }
                    ch.wait().ok().and_then(|s| s.code()).unwrap_or(-1)
                }
            };
            let _ = tx.send(Msg::FinEtape(code));
            ctx.request_repaint();
        });
    }

    fn ligne_travail(&mut self, l: String) {
        if let Some(v) = l.strip_prefix("@@INSTALLE:") {
            if plus_recente(v, VERSION) {
                self.auto_maj = AutoMaj::Fait(v.to_string());
            }
        }
        let Some(t) = self.travail.as_mut() else { return };
        if let Some(s) = l.strip_prefix("@@") {
            if let Some(e) = s.strip_prefix("ETAPE:") {
                t.etape = e.to_string();
                t.progression = None;
                t.detail.clear();
                t.journal.push_str(&format!("── {e} ──\n"));
                return;
            }
            let champs: Vec<&str> = s.splitn(4, ':').collect();
            if champs.len() == 4 {
                let pct: f32 = champs[2].parse().unwrap_or(0.0);
                match champs[0] {
                    "dlstatus" => {
                        t.progression = Some(pct / 100.0);
                        t.detail = format!("Téléchargement : {}", champs[3]);
                    }
                    "pmstatus" => {
                        t.progression = Some(pct / 100.0);
                        t.detail = champs[3].to_string();
                    }
                    "pmerror" => {
                        t.erreurs.push(format!("{} : {}", champs[1], champs[3]));
                        t.journal.push_str(&format!("ERREUR {} : {}\n", champs[1], champs[3]));
                    }
                    "pmconffile" => t.journal.push_str(&format!("Fichier de configuration gardé : {}\n", champs[1])),
                    _ => {}
                }
            }
            return;
        }
        if !l.trim().is_empty() {
            if t.progression.is_none() || !t.detail.starts_with("Téléchargement") {
                let court: String = l.trim().chars().take(140).collect();
                if t.progression.is_none() {
                    t.detail = court;
                }
            }
            t.journal.push_str(&l);
            t.journal.push('\n');
            if t.journal.len() > 400_000 {
                let coupe = t.journal.len() - 300_000;
                let coupe = (coupe..t.journal.len()).find(|&i| t.journal.is_char_boundary(i)).unwrap_or(0);
                t.journal.drain(..coupe);
            }
        }
    }

    fn fin_etape(&mut self, code: i32) {
        let Some(t) = self.travail.as_mut() else { return };
        if code == 0 {
            self.etape_suivante();
            return;
        }
        t.restantes.clear();
        let msg = match code {
            126 => "La demande de mot de passe a été annulée : rien n'a été modifié.".to_string(),
            127 => "L'autorisation a été refusée (mot de passe incorrect ou compte non administrateur).".to_string(),
            _ => {
                let mut m = format!("L'opération a échoué (code {code}). Ouvrez « Détails » pour en savoir plus.");
                if let Some(e) = t.erreurs.first() {
                    m.push_str(&format!("\n{e}"));
                }
                m
            }
        };
        t.fini = Some((false, msg));
        t.redemarrage = redemarrage_necessaire();
        let apres = t.apres;
        if code != 126 && code != 127 {
            match apres {
                Apres::Recherche => self.lancer_recherche(false),
                Apres::Pilotes => self.lancer_pilotes(),
            }
        }
    }

    fn verifier(&mut self) {
        self.commencer(
            "Vérification des mises à jour",
            "La liste des logiciels disponibles est relue depuis Internet.",
            vec![Etape { texte: "Lecture de la liste des logiciels disponibles".into(), cmd: commande_actualiser() }],
            Apres::Recherche,
        );
    }

    fn installer(&mut self) {
        let Some(r) = self.recherche.as_ref() else { return };
        let aide = aide().display().to_string();
        let choisis = |f: &dyn Fn(&Maj) -> bool| -> Vec<&Maj> {
            r.majs.iter().zip(&self.coche).filter(|(m, c)| **c && f(m)).map(|(m, _)| m).collect()
        };
        let normales: Vec<&Maj> = r.majs.iter().filter(|m| m.groupe.est_apt() && m.groupe != Groupe::Progressives).collect();
        let normales_choisies = choisis(&|m| m.groupe.est_apt() && m.groupe != Groupe::Progressives);
        let progressives = choisis(&|m| m.groupe == Groupe::Progressives);
        let snaps_tous: Vec<&Maj> = r.majs.iter().filter(|m| m.groupe == Groupe::Snap).collect();
        let snaps = choisis(&|m| m.groupe == Groupe::Snap);
        let flatpaks = choisis(&|m| m.groupe == Groupe::Flatpak);

        let mut cmd = en_admin(&[&aide, "maj"]);
        let base = cmd.len();
        let mut paquets: Vec<String> = progressives.iter().map(|m| m.nom.clone()).collect();
        if !normales.is_empty() && normales_choisies.len() == normales.len() {
            cmd.push("--tout".into());
        } else {
            paquets.extend(normales_choisies.iter().map(|m| m.nom.clone()));
        }
        if !paquets.is_empty() {
            cmd.push("--paquets".into());
            cmd.extend(paquets);
        }
        if !snaps.is_empty() {
            cmd.push("--snap".into());
            if snaps.len() != snaps_tous.len() {
                cmd.extend(snaps.iter().map(|m| m.nom.clone()));
            }
        }
        let mut etapes = Vec::new();
        if cmd.len() > base {
            etapes.push(Etape { texte: "Installation des mises à jour".into(), cmd });
        }
        if !flatpaks.is_empty() {
            let simul = std::env::var_os("GMAJ_SIMULER").is_some();
            let base: &[&str] = if simul { &["flatpak", "remote-ls", "--updates"] } else { &["flatpak", "update", "-y", "--noninteractive"] };
            let mut c: Vec<String> = base.iter().map(|s| s.to_string()).collect();
            if !simul {
                c.extend(flatpaks.iter().map(|m| m.nom.clone()));
            }
            etapes.push(Etape { texte: "Mise à jour des applications Flatpak".into(), cmd: c });
        }
        self.commencer(
            "Installation des mises à jour",
            "Les mises à jour sont téléchargées puis installées. Votre mot de passe peut vous être demandé.",
            etapes,
            Apres::Recherche,
        );
    }

    fn piloter(&mut self, paquet: &str, activer: bool) {
        let cmd = en_admin(&[&aide().display().to_string(), if activer { "installer" } else { "retirer" }, paquet]);
        let (titre, expl) = if activer {
            ("Activation du pilote", format!("Le pilote « {paquet} » est téléchargé et installé."))
        } else {
            ("Désactivation du pilote", format!("Le pilote « {paquet} » est retiré de l'ordinateur."))
        };
        self.commencer(titre, &expl, vec![Etape { texte: titre.into(), cmd }], Apres::Pilotes);
    }

    // -----------------------------------------------------------------------
    //  Onglet « Mises à jour »
    // -----------------------------------------------------------------------
    fn entete(ui: &mut Ui, ic: Icone, titre: &str, texte: &str) {
        ui.horizontal(|ui| {
            theme::icone(ui, ic, 48.0);
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.add_space(4.0);
                ui.label(RichText::new(titre).font(theme::police_gras(15.0)));
                ui.add(egui::Label::new(RichText::new(texte).color(GRIS)).wrap());
            });
        });
        ui.add_space(8.0);
    }

    fn vue_maj(&mut self, ui: &mut Ui) {
        let (ic, titre, texte) = match (&self.recherche, &self.occupe) {
            (_, Some(o)) => (Icone::Verifier, o.clone(), "Patientez un instant.".to_string()),
            (None, None) => (Icone::Verifier, "Recherche des mises à jour…".into(), String::new()),
            (Some(r), None) => {
                if let Some(e) = &r.erreur {
                    (Icone::Erreur, "Impossible de calculer les mises à jour".into(), e.clone())
                } else if r.majs.is_empty() {
                    let age = if r.age_listes == u64::MAX {
                        String::new()
                    } else {
                        format!(" Dernière vérification il y a {}.", duree(r.age_listes))
                    };
                    (
                        Icone::AJour,
                        "Votre ordinateur est à jour".into(),
                        format!("Aucune mise à jour logicielle n'est disponible pour le moment.{age}"),
                    )
                } else {
                    let n = r.majs.len();
                    let s = r.majs.iter().filter(|m| m.groupe == Groupe::Securite).count();
                    let titre = if n == 1 {
                        "Une mise à jour logicielle est disponible".to_string()
                    } else {
                        format!("{n} mises à jour logicielles sont disponibles")
                    };
                    let texte = if s > 0 {
                        format!("Dont {s} de sécurité importante{} : installez-les dès que possible pour protéger votre ordinateur.", if s > 1 { "s" } else { "" })
                    } else {
                        "Les mises à jour corrigent des erreurs et améliorent vos logiciels.".into()
                    };
                    (if s > 0 { Icone::Securite } else { Icone::Appli }, titre, texte)
                }
            }
        };
        egui::Panel::top("maj_entete").frame(Frame::new()).show(ui, |ui| Self::entete(ui, ic, &titre, &texte));
        if let Some(nv) = self.recherche.as_ref().and_then(|r| r.nouvelle_version.clone()) {
            egui::Panel::top("maj_version").frame(Frame::new()).show(ui, |ui| {
                if bandeau_version(ui, &nv, self.travail.is_none()) {
                    self.erreur_version = None;
                    self.confirmer_version = true;
                }
            });
        }
        egui::Panel::bottom("maj_bas").frame(Frame::new()).show(ui, |ui| self.bas_maj(ui));
        egui::CentralPanel::default().frame(Frame::new()).show(ui, |ui| self.liste_maj(ui));
    }

    fn etat_groupe(&self, g: Groupe) -> Etat {
        let Some(r) = &self.recherche else { return Etat::Non };
        let v: Vec<bool> = r.majs.iter().zip(&self.coche).filter(|(m, _)| m.groupe == g).map(|(_, c)| *c).collect();
        if v.iter().all(|c| *c) {
            Etat::Oui
        } else if v.iter().any(|c| *c) {
            Etat::Partiel
        } else {
            Etat::Non
        }
    }

    fn liste_maj(&mut self, ui: &mut Ui) {
        theme::cadre_blanc().show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            let larg = ui.available_width();
            let (c_ver, c_taille) = (170.0f32.min(larg * 0.25), 90.0);
            // en-têtes de colonnes
            let (rh, _) = ui.allocate_exact_size(vec2(larg, 24.0), Sense::hover());
            {
                let p = ui.painter();
                theme::degrade(p, rh, Color32::from_rgb(0xF8, 0xF6, 0xF3), Color32::from_rgb(0xE4, 0xDE, 0xD6), 0);
                p.line_segment([rh.left_bottom(), rh.right_bottom()], Stroke::new(1.0, BORD));
                let x_ver = rh.right() - c_taille - c_ver;
                let x_t = rh.right() - c_taille;
                for x in [x_ver, x_t] {
                    p.line_segment([pos2(x, rh.top() + 4.0), pos2(x, rh.bottom() - 4.0)], Stroke::new(1.0, BORD));
                }
                let f = theme::police(12.5);
                p.text(pos2(rh.left() + 8.0, rh.center().y), Align2::LEFT_CENTER, "Mise à jour", f.clone(), TEXTE);
                p.text(pos2(x_ver + 8.0, rh.center().y), Align2::LEFT_CENTER, "Nouvelle version", f.clone(), TEXTE);
                p.text(pos2(x_t + 8.0, rh.center().y), Align2::LEFT_CENTER, "Taille", f, TEXTE);
            }
            let Some(r) = self.recherche.clone() else {
                attente(ui, self.occupe.as_deref().unwrap_or("Recherche des mises à jour…"));
                return;
            };
            if self.occupe.is_some() && r.majs.is_empty() {
                attente(ui, self.occupe.as_deref().unwrap_or(""));
                return;
            }
            if r.majs.is_empty() {
                ui.add_space(30.0);
                ui.vertical_centered(|ui| {
                    theme::icone(ui, if r.erreur.is_some() { Icone::Attention } else { Icone::AJour }, 64.0);
                    ui.add_space(8.0);
                    ui.label(RichText::new(if r.erreur.is_some() { "La liste n'a pas pu être établie." } else { "Aucune mise à jour à installer." }).color(GRIS));
                });
                return;
            }
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let mut ligne = 0usize;
                for g in Groupe::TOUS {
                    let indices: Vec<usize> = r.majs.iter().enumerate().filter(|(_, m)| m.groupe == g).map(|(i, _)| i).collect();
                    if indices.is_empty() {
                        continue;
                    }
                    // ligne du groupe
                    let (rect, rep) = ui.allocate_exact_size(vec2(larg, 26.0), Sense::click());
                    let replie = self.replies.contains(&g);
                    let p = ui.painter().clone();
                    p.rect_filled(rect, 0, Color32::from_rgb(0xF1, 0xEC, 0xE6));
                    p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0, Color32::from_rgb(0xE0, 0xD9, 0xD0)));
                    let tri = Rect::from_center_size(pos2(rect.left() + 11.0, rect.center().y), vec2(16.0, 16.0));
                    theme::dessiner_triangle(&p, tri.center(), !replie, if rep.hovered() { SEL } else { GRIS });
                    let r_case = Rect::from_center_size(pos2(rect.left() + 30.0, rect.center().y), vec2(15.0, 15.0));
                    let rep_case = ui.interact(r_case.expand(3.0), Id::unique(("case_g", g)), Sense::click());
                    theme::dessiner_case(&p, r_case, self.etat_groupe(g), rep_case.hovered());
                    let ic = match g {
                        Groupe::Securite => Icone::Securite,
                        Groupe::Snap | Groupe::Flatpak => Icone::Snap,
                        Groupe::Progressives => Icone::Info,
                        _ => Icone::Paquet,
                    };
                    theme::dessiner_icone(&p, ic, Rect::from_center_size(pos2(rect.left() + 52.0, rect.center().y), vec2(16.0, 16.0)), true);
                    let total: u64 = indices.iter().map(|&i| r.majs[i].taille).sum();
                    let titre = format!("{} ({})", g.titre(), indices.len());
                    p.text(pos2(rect.left() + 66.0, rect.center().y), Align2::LEFT_CENTER, titre, theme::police_gras(13.0), TEXTE);
                    if total > 0 {
                        p.text(pos2(rect.right() - c_taille + 8.0, rect.center().y), Align2::LEFT_CENTER, taille_lisible(total), theme::police(12.0), GRIS);
                    }
                    if rep_case.clicked() {
                        let nouveau = self.etat_groupe(g) != Etat::Oui;
                        for &i in &indices {
                            self.coche[i] = nouveau;
                        }
                    } else if rep.clicked() {
                        if replie {
                            self.replies.remove(&g);
                        } else {
                            self.replies.insert(g);
                        }
                    }
                    if replie {
                        continue;
                    }
                    for &i in &indices {
                        let m = &r.majs[i];
                        let (rect, rep) = ui.allocate_exact_size(vec2(larg, 24.0), Sense::click());
                        if !ui.is_rect_visible(rect) {
                            ligne += 1;
                            continue;
                        }
                        let choisi = self.selection == Some(i);
                        let p = ui.painter().clone();
                        let fond = if choisi {
                            SEL
                        } else if rep.hovered() {
                            theme::SEL_CLAIR
                        } else if ligne % 2 == 1 {
                            LIGNE_PAIRE
                        } else {
                            BLANC
                        };
                        p.rect_filled(rect, 0, fond);
                        let coul = if choisi { BLANC } else { TEXTE };
                        let coul2 = if choisi { Color32::from_rgb(0xFF, 0xEE, 0xDD) } else { GRIS };
                        let r_case = Rect::from_center_size(pos2(rect.left() + 46.0, rect.center().y), vec2(15.0, 15.0));
                        let rep_case = ui.interact(r_case.expand(3.0), Id::unique(("case", i)), Sense::click());
                        theme::dessiner_case(&p, r_case, if self.coche[i] { Etat::Oui } else { Etat::Non }, rep_case.hovered());
                        let x_ver = rect.right() - c_taille - c_ver;
                        let nom = if m.resume.is_empty() || m.resume == m.nom {
                            m.nom.clone()
                        } else {
                            format!("{} — {}", m.resume, m.nom)
                        };
                        let g1 = theme::texte_coupe(&p, &nom, theme::police(12.5), coul, x_ver - rect.left() - 70.0);
                        p.galley(pos2(rect.left() + 62.0, rect.center().y - g1.size().y / 2.0), g1, coul);
                        let g2 = theme::texte_coupe(&p, &m.nouvelle, theme::police(12.0), coul2, c_ver - 12.0);
                        p.galley(pos2(x_ver + 8.0, rect.center().y - g2.size().y / 2.0), g2, coul2);
                        if m.taille > 0 {
                            p.text(pos2(rect.right() - c_taille + 8.0, rect.center().y), Align2::LEFT_CENTER, taille_lisible(m.taille), theme::police(12.0), coul2);
                        }
                        if rep_case.clicked() {
                            self.coche[i] = !self.coche[i];
                        } else if rep.clicked() {
                            self.selection = Some(i);
                        } else if rep.double_clicked() {
                            self.coche[i] = !self.coche[i];
                        }
                        ligne += 1;
                    }
                }
            });
        });
    }

    fn bas_maj(&mut self, ui: &mut Ui) {
        ui.add_space(6.0);
        if let Some(r) = self.recherche.clone() {
            let n = self.coche.iter().filter(|c| **c).count();
            let apt_choisi = r.majs.iter().zip(&self.coche).any(|(m, c)| *c && m.groupe.est_apt());
            let mut taille: u64 = r.majs.iter().zip(&self.coche).filter(|(_, c)| **c).map(|(m, _)| m.taille).sum();
            if apt_choisi {
                taille += r.taille_nouveaux;
            }
            ui.horizontal_wrapped(|ui| {
                if !r.majs.is_empty() {
                    let t = match n {
                        0 => "Aucune mise à jour sélectionnée.".to_string(),
                        1 => format!("1 mise à jour sélectionnée. Taille du téléchargement : {}", taille_lisible(taille)),
                        _ => format!("{n} mises à jour sélectionnées. Taille du téléchargement : {}", taille_lisible(taille)),
                    };
                    ui.label(t);
                }
                if apt_choisi && !r.nouveaux.is_empty() {
                    ui.label(RichText::new(format!("· {} paquet(s) nouveau(x) nécessaire(s)", r.nouveaux.len())).color(GRIS))
                        .on_hover_text(r.nouveaux.join(", "));
                }
                if apt_choisi && !r.supprimes.is_empty() {
                    ui.label(RichText::new(format!("· {} paquet(s) seront retirés", r.supprimes.len())).color(theme::ROUGE))
                        .on_hover_text(r.supprimes.join(", "));
                }
            });
            if r.redemarrage {
                ui.horizontal(|ui| {
                    theme::icone(ui, Icone::Redemarrer, 18.0);
                    ui.label(RichText::new("Un redémarrage est nécessaire pour terminer des mises à jour déjà installées.").color(theme::BRUN_FONCE));
                    if bouton(ui, Some(Icone::Redemarrer), "Redémarrer…", true, false).clicked() {
                        redemarrer();
                    }
                });
            }
        }
        ui.add_space(4.0);
        let avant = self.desc_ouverte;
        theme::depliant(ui, &mut self.desc_ouverte, "Description de la mise à jour");
        if self.desc_ouverte {
            if !avant && self.selection.is_none() && self.recherche.as_ref().is_some_and(|r| !r.majs.is_empty()) {
                self.selection = Some(0);
            }
            let mut v = self.vue_desc;
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.selectable_value(&mut v, 0, "Modifications");
                ui.selectable_value(&mut v, 1, "Description");
            });
            self.vue_desc = v;
            let texte = self.texte_description();
            theme::cadre_blanc().show(ui, |ui| {
                ui.set_width(ui.available_width());
                egui::ScrollArea::vertical().max_height(150.0).min_scrolled_height(150.0).auto_shrink([false, false]).show(ui, |ui| {
                    egui::Frame::new().inner_margin(Margin::same(6)).show(ui, |ui| {
                        let mono = self.vue_desc == 0;
                        let rt = if mono { RichText::new(texte).monospace() } else { RichText::new(texte) };
                        ui.add(egui::Label::new(rt).wrap().selectable(true));
                    });
                });
            });
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let libre = self.travail.is_none();
            if bouton(ui, Some(Icone::Parametres), "Paramètres…", libre, false).clicked() {
                self.reglages_ouverts = true;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let peut = libre && self.occupe.is_none() && self.coche.iter().any(|c| *c);
                if bouton(ui, Some(Icone::Installer), "Installer les mises à jour", peut, true).clicked() {
                    self.installer();
                }
                if bouton(ui, Some(Icone::Verifier), "Vérifier", libre && self.occupe.is_none(), false).clicked() {
                    self.verifier();
                }
            });
        });
        ui.add_space(4.0);
    }

    fn texte_description(&mut self) -> String {
        let Some(i) = self.selection else { return "Choisissez une mise à jour dans la liste.".into() };
        let Some(m) = self.recherche.as_ref().and_then(|r| r.majs.get(i)).cloned() else { return String::new() };
        if self.vue_desc == 0 {
            self.demander_modifs(i);
            match self.modifs.get(&format!("{}={}", m.nom, m.nouvelle)) {
                Some(Some(t)) => t.clone(),
                _ => "Téléchargement de la liste des modifications…".into(),
            }
        } else {
            let mut t = format!("{}\n\nPaquet : {}\n", m.resume, m.nom);
            if !m.ancienne.is_empty() {
                t.push_str(&format!("Version installée : {}\n", m.ancienne));
            }
            t.push_str(&format!("Nouvelle version : {}\n", m.nouvelle));
            if m.taille > 0 {
                t.push_str(&format!("Taille du téléchargement : {}\n", taille_lisible(m.taille)));
            }
            if !m.description.is_empty() {
                t.push('\n');
                t.push_str(&m.description);
            }
            t
        }
    }

    // -----------------------------------------------------------------------
    //  Onglet « Pilotes »
    // -----------------------------------------------------------------------
    fn vue_pilotes(&mut self, ui: &mut Ui) {
        if self.pilotes.is_none() && !self.pilotes_occupe {
            self.lancer_pilotes();
        }
        let (titre, texte) = match &self.pilotes {
            _ if self.pilotes_occupe => ("Recherche du matériel et des pilotes…".to_string(), "Patientez un instant.".to_string()),
            Some(p) if !p.outil => (
                "L'outil de détection des pilotes n'est pas installé".into(),
                "Le paquet « ubuntu-drivers-common » permet de trouver les pilotes adaptés à votre matériel.".into(),
            ),
            Some(p) if p.peripheriques.is_empty() => (
                "Aucun pilote supplémentaire n'est nécessaire".into(),
                "Tout votre matériel fonctionne avec les pilotes livrés avec le système.".into(),
            ),
            Some(p) => {
                let manque = p.recommandes_absents();
                if manque.is_empty() {
                    ("Pilotes supplémentaires".into(), "Ces pilotes sont fournis par les fabricants. Ils permettent d'utiliser pleinement le matériel (carte graphique, Wi-Fi…).".into())
                } else {
                    (
                        format!("{} pilote(s) recommandé(s) disponible(s)", manque.len()),
                        "Activez le pilote recommandé pour que votre matériel fonctionne au mieux. Un redémarrage sera nécessaire.".into(),
                    )
                }
            }
            None => ("Pilotes supplémentaires".into(), String::new()),
        };
        egui::Panel::top("pil_entete").frame(Frame::new()).show(ui, |ui| Self::entete(ui, Icone::Pilote, &titre, &texte));
        egui::Panel::bottom("pil_bas").frame(Frame::new()).show(ui, |ui| self.bas_pilotes(ui));
        egui::CentralPanel::default().frame(Frame::new()).show(ui, |ui| {
            let h = ui.available_height();
            let Some(p) = self.pilotes.clone() else {
                theme::cadre_blanc().show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    attente(ui, "Recherche du matériel…");
                });
                return;
            };
            if p.outil && !p.peripheriques.is_empty() {
                theme::cadre_blanc().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    egui::ScrollArea::vertical().id_salt("pilotes").max_height(h * 0.45).auto_shrink([false, true]).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let larg = ui.available_width();
                        for (i, per) in p.peripheriques.iter().enumerate() {
                            let (rect, _) = ui.allocate_exact_size(vec2(larg, 28.0), Sense::hover());
                            let pt = ui.painter();
                            pt.rect_filled(rect, 0, Color32::from_rgb(0xF1, 0xEC, 0xE6));
                            theme::dessiner_icone(pt, Icone::Materiel, Rect::from_center_size(pos2(rect.left() + 16.0, rect.center().y), vec2(18.0, 18.0)), true);
                            let g = theme::texte_coupe(pt, &per.nom(), theme::police_gras(13.0), TEXTE, larg - 40.0);
                            pt.galley(pos2(rect.left() + 32.0, rect.center().y - g.size().y / 2.0), g, TEXTE);
                            for (j, d) in per.pilotes.iter().enumerate() {
                                let (rect, rep) = ui.allocate_exact_size(vec2(larg, 26.0), Sense::click());
                                let choisi = self.sel_pilote == Some((i, j));
                                let pt = ui.painter();
                                let fond = if choisi { SEL } else if rep.hovered() { theme::SEL_CLAIR } else { BLANC };
                                pt.rect_filled(rect, 0, fond);
                                let coul = if choisi { BLANC } else { TEXTE };
                                let coul2 = if choisi { Color32::from_rgb(0xFF, 0xEE, 0xDD) } else { GRIS };
                                theme::dessiner_radio(pt, pos2(rect.left() + 40.0, rect.center().y), d.installe, rep.hovered());
                                pt.text(pos2(rect.left() + 56.0, rect.center().y), Align2::LEFT_CENTER, &d.paquet, theme::police(13.0), coul);
                                let mut etiquettes = vec![if d.libre { "libre" } else { "propriétaire" }];
                                if d.integre {
                                    etiquettes.push("fourni avec le système");
                                }
                                let x = rect.left() + 56.0 + 330.0f32.min(larg * 0.42);
                                pt.text(pos2(x, rect.center().y), Align2::LEFT_CENTER, etiquettes.join(", "), theme::police(12.0), coul2);
                                let (etat, c_etat) = if d.installe {
                                    ("En service", if choisi { BLANC } else { theme::VERT })
                                } else if d.recommande {
                                    ("Recommandé", if choisi { BLANC } else { Color32::from_rgb(0xCE, 0x5C, 0x00) })
                                } else {
                                    ("Non installé", coul2)
                                };
                                pt.text(pos2(rect.right() - 10.0, rect.center().y), Align2::RIGHT_CENTER, etat, theme::police_gras(12.0), c_etat);
                                if rep.clicked() {
                                    self.sel_pilote = Some((i, j));
                                }
                            }
                        }
                    });
                });
                ui.add_space(8.0);
            }
            ui.horizontal(|ui| {
                theme::icone(ui, Icone::Materiel, 18.0);
                ui.label(RichText::new("Matériel de l'ordinateur et pilote utilisé").font(theme::police_gras(13.0)));
            });
            theme::cadre_blanc().show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                let larg = ui.available_width();
                let (c1, c3) = (170.0, 150.0);
                let (rh, _) = ui.allocate_exact_size(vec2(larg, 24.0), Sense::hover());
                let pt = ui.painter();
                theme::degrade(pt, rh, Color32::from_rgb(0xF8, 0xF6, 0xF3), Color32::from_rgb(0xE4, 0xDE, 0xD6), 0);
                pt.line_segment([rh.left_bottom(), rh.right_bottom()], Stroke::new(1.0, BORD));
                for (x, t) in [(rh.left() + 8.0, "Type"), (rh.left() + c1 + 8.0, "Composant"), (rh.right() - c3 + 8.0, "Pilote utilisé")] {
                    pt.text(pos2(x, rh.center().y), Align2::LEFT_CENTER, t, theme::police(12.5), TEXTE);
                }
                egui::ScrollArea::vertical().id_salt("materiel").auto_shrink([false, false]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for (k, m) in p.materiel.iter().enumerate() {
                        let (rect, rep) = ui.allocate_exact_size(vec2(larg, 22.0), Sense::hover());
                        let pt = ui.painter();
                        pt.rect_filled(rect, 0, if k % 2 == 1 { LIGNE_PAIRE } else { BLANC });
                        let f = theme::police(12.0);
                        let g = theme::texte_coupe(pt, &m.classe, f.clone(), GRIS, c1 - 12.0);
                        pt.galley(pos2(rect.left() + 8.0, rect.center().y - g.size().y / 2.0), g, GRIS);
                        let g = theme::texte_coupe(pt, &m.nom, f.clone(), TEXTE, larg - c1 - c3 - 16.0);
                        pt.galley(pos2(rect.left() + c1 + 8.0, rect.center().y - g.size().y / 2.0), g, TEXTE);
                        let c = if m.pilote == "Aucun" { GRIS_CLAIR } else { TEXTE };
                        let g = theme::texte_coupe(pt, &m.pilote, f, c, c3 - 12.0);
                        pt.galley(pos2(rect.right() - c3 + 8.0, rect.center().y - g.size().y / 2.0), g, c);
                        rep.on_hover_text(format!("{}\n{}\nPilote : {}", m.classe, m.nom, m.pilote));
                    }
                    if p.materiel.is_empty() {
                        ui.add_space(10.0);
                        ui.label(RichText::new("  La liste du matériel n'est pas disponible (commande lspci absente).").color(GRIS));
                    }
                });
            });
        });
    }

    fn bas_pilotes(&mut self, ui: &mut Ui) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let libre = self.travail.is_none() && !self.pilotes_occupe;
            if bouton(ui, Some(Icone::Verifier), "Actualiser", libre, false).clicked() {
                self.pilotes = None;
                self.lancer_pilotes();
            }
            let outil_absent = self.pilotes.as_ref().is_some_and(|p| !p.outil);
            if outil_absent && bouton(ui, Some(Icone::Installer), "Installer l'outil de détection", libre, true).clicked() {
                self.piloter("ubuntu-drivers-common", true);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let choisi = self
                    .sel_pilote
                    .and_then(|(i, j)| self.pilotes.as_ref()?.peripheriques.get(i)?.pilotes.get(j).cloned());
                match choisi {
                    Some(d) if d.installe => {
                        if bouton(ui, Some(Icone::Fermer), "Désactiver", libre && !d.integre, false)
                            .on_hover_text(if d.integre { "Ce pilote fait partie du système." } else { "Retire ce pilote de l'ordinateur." })
                            .clicked()
                        {
                            self.piloter(&d.paquet, false);
                        }
                    }
                    Some(d) => {
                        if bouton(ui, Some(Icone::Installer), "Activer", libre, true).clicked() {
                            self.piloter(&d.paquet, true);
                        }
                    }
                    None => {
                        let reco = self.pilotes.as_ref().and_then(|p| p.recommandes_absents().into_iter().next());
                        if let Some((_, paquet)) = reco {
                            if bouton(ui, Some(Icone::Installer), "Activer le pilote recommandé", libre, true).clicked() {
                                self.piloter(&paquet, true);
                            }
                        } else {
                            bouton(ui, Some(Icone::Installer), "Activer", false, false);
                        }
                    }
                }
            });
        });
        ui.add_space(4.0);
    }

    // -----------------------------------------------------------------------
    //  Fenêtres secondaires
    // -----------------------------------------------------------------------
    fn fenetre_travail(&mut self, ctx: &Context) {
        let mut fermer = false;
        let Some(t) = self.travail.as_mut() else { return };
        let cadre = Frame::window(&ctx.global_style()).inner_margin(Margin::same(16)).fill(theme::FOND);
        egui::Modal::new(Id::unique("travail")).frame(cadre).show(ctx, |ui| {
            ui.set_width(560.0);
            let ic = match &t.fini {
                None => Icone::Installer,
                Some((true, _)) => Icone::AJour,
                Some((false, _)) => Icone::Erreur,
            };
            ui.horizontal(|ui| {
                theme::icone(ui, ic, 48.0);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.add_space(2.0);
                    ui.label(RichText::new(&t.titre).font(theme::police_gras(16.0)));
                    ui.add(egui::Label::new(RichText::new(&t.explication).color(GRIS)).wrap());
                });
            });
            ui.add_space(12.0);
            match &t.fini {
                None => {
                    ui.label(RichText::new(&t.etape).font(theme::police_gras(13.0)));
                    ui.add_space(4.0);
                    theme::barre_progression(ui, t.progression);
                    ui.add_space(2.0);
                    let g = theme::texte_coupe(ui.painter(), &t.detail, theme::police(12.0), GRIS, ui.available_width());
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::hover());
                    ui.painter().galley(r.left_top(), g, GRIS);
                }
                Some((ok, msg)) => {
                    let texte = if *ok {
                        "L'opération est terminée avec succès.".to_string()
                    } else {
                        msg.clone()
                    };
                    ui.label(RichText::new(texte).font(theme::police_gras(13.0)));
                    if t.redemarrage {
                        ui.add_space(6.0);
                        egui::Frame::new()
                            .fill(theme::BULLE)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(0xC4, 0xA0, 0x00)))
                            .corner_radius(3)
                            .inner_margin(Margin::same(8))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    theme::icone(ui, Icone::Redemarrer, 24.0);
                                    ui.add(egui::Label::new("Redémarrez l'ordinateur pour terminer l'installation.").wrap());
                                });
                            });
                    }
                }
            }
            ui.add_space(8.0);
            theme::depliant(ui, &mut t.details, "Détails");
            if t.details {
                theme::cadre_blanc().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    egui::ScrollArea::vertical()
                        .max_height(220.0)
                        .min_scrolled_height(220.0)
                        .stick_to_bottom(true)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            egui::Frame::new().inner_margin(Margin::same(4)).show(ui, |ui| {
                                ui.add(egui::Label::new(RichText::new(t.journal.trim_start()).monospace().size(11.5)).selectable(true));
                            });
                        });
                });
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let fini = t.fini.is_some();
                    if bouton(ui, Some(Icone::Fermer), "Fermer", fini, !t.redemarrage).clicked() {
                        fermer = true;
                    }
                    if fini && t.redemarrage && bouton(ui, Some(Icone::Redemarrer), "Redémarrer maintenant", true, true).clicked() {
                        redemarrer();
                    }
                    if !fini {
                        ui.label(RichText::new("Ne pas éteindre l'ordinateur pendant l'installation.").color(GRIS));
                    }
                });
            });
        });
        if fermer {
            self.travail = None;
        }
    }

    fn fenetre_reglages(&mut self, ctx: &Context) {
        let cadre = Frame::window(&ctx.global_style()).inner_margin(Margin::same(16)).fill(theme::FOND);
        let mut fermer = false;
        egui::Modal::new(Id::unique("reglages")).frame(cadre).show(ctx, |ui| {
            ui.set_width(470.0);
            ui.horizontal(|ui| {
                theme::icone(ui, Icone::Parametres, 40.0);
                ui.label(RichText::new("Paramètres").font(theme::police_gras(16.0)));
            });
            ui.add_space(10.0);
            ui.label(RichText::new("Recherche automatique").font(theme::police_gras(13.0)));
            egui::Grid::new("grille_reglages").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                ui.label("    Rechercher les mises à jour :");
                choix(ui, "intervalle", &mut self.reglages.intervalle_h, &[(1, "toutes les heures"), (3, "toutes les 3 heures"), (6, "toutes les 6 heures"), (24, "tous les jours"), (168, "chaque semaine")]);
                ui.end_row();
                ui.label("    Me rappeler celles non installées :");
                choix(ui, "rappel", &mut self.reglages.rappel_h, &[(4, "toutes les 4 heures"), (24, "tous les jours"), (72, "tous les 3 jours"), (168, "chaque semaine")]);
                ui.end_row();
            });
            ui.add_space(10.0);
            ui.label(RichText::new("Ce qu'il faut surveiller").font(theme::police_gras(13.0)));
            ui.indent("r1", |ui| {
                theme::case(ui, &mut self.reglages.snap, "Les applications Snap");
                theme::case(ui, &mut self.reglages.flatpak, "Les applications Flatpak");
                theme::case(ui, &mut self.reglages.pilotes, "Les pilotes recommandés pour mon matériel");
            });
            ui.add_space(10.0);
            ui.label(RichText::new("Ce logiciel").font(theme::police_gras(13.0)));
            ui.indent("r3", |ui| {
                theme::case(ui, &mut self.reglages.auto_maj_logiciel, "Le mettre à jour automatiquement depuis GitHub");
                ui.label(RichText::new("Le choix des nouvelles versions d'Ubuntu est dans l'onglet « Mise à niveau ».").color(GRIS).size(11.5));
            });
            ui.add_space(10.0);
            ui.label(RichText::new("Démarrage").font(theme::police_gras(13.0)));
            ui.indent("r2", |ui| {
                let mut d = self.demarrage;
                if theme::case(ui, &mut d, "Lancer au démarrage de l'ordinateur et me prévenir par une bulle").changed() {
                    self.changer_demarrage(d);
                }
            });
            ui.add_space(14.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if bouton(ui, Some(Icone::Fermer), "Fermer", true, true).clicked() {
                    fermer = true;
                }
            });
        });
        if fermer {
            self.reglages.enregistrer();
            self.reglages_ouverts = false;
            self.lancer_recherche(false);
            if self.reglages.auto_maj_logiciel && matches!(self.auto_maj, AutoMaj::Disponible(_)) {
                self.verifier_gestionnaire(true);
            }
        }
    }

    fn fenetre_version(&mut self, ctx: &Context) {
        let Some(r) = self.recherche.clone() else { return };
        let Some(nv) = r.nouvelle_version.clone() else {
            self.confirmer_version = false;
            return;
        };
        let attente = r.majs.iter().filter(|m| m.groupe.est_apt() && m.groupe != Groupe::Progressives).count();
        let cadre = Frame::window(&ctx.global_style()).inner_margin(Margin::same(16)).fill(theme::FOND);
        let (mut fermer, mut lancer_maj, mut lancer_niveau) = (false, false, false);
        egui::Modal::new(Id::unique("version")).frame(cadre).show(ctx, |ui| {
            ui.set_width(540.0);
            ui.horizontal(|ui| {
                theme::icone(ui, Icone::Appli, 48.0);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.add_space(2.0);
                    ui.label(RichText::new(format!("Mise à niveau vers {}", nv.titre())).font(theme::police_gras(16.0)));
                    ui.add(egui::Label::new(RichText::new(format!(
                        "Vous utilisez {}. L'assistant d'Ubuntu va télécharger et installer la nouvelle version. Vos fichiers, vos réglages et vos logiciels sont conservés.",
                        systeme_actuel()
                    )).color(GRIS)).wrap());
                });
            });
            ui.add_space(10.0);
            ui.label(RichText::new("Avant de commencer").font(theme::police_gras(13.0)));
            for t in [
                "Enregistrez votre travail et fermez vos applications.",
                "Faites une sauvegarde de vos fichiers importants.",
                "Branchez l'ordinateur sur le secteur : comptez une heure ou plus.",
                "L'ordinateur redémarrera à la fin.",
            ] {
                ui.label(format!("    •  {t}"));
            }
            if attente > 0 {
                ui.add_space(8.0);
                egui::Frame::new()
                    .fill(theme::BULLE)
                    .stroke(Stroke::new(1.0, Color32::from_rgb(0xC4, 0xA0, 0x00)))
                    .corner_radius(3)
                    .inner_margin(Margin::same(8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            theme::icone(ui, Icone::Attention, 24.0);
                            ui.add(egui::Label::new(format!(
                                "Il reste {attente} mise(s) à jour à installer. Ubuntu demande de les installer avant de passer à la nouvelle version."
                            )).wrap());
                        });
                    });
            }
            if nv.developpement {
                ui.add_space(6.0);
                ui.label(RichText::new("Attention : il s'agit d'une version encore en développement (essai).").color(theme::ROUGE));
            }
            if let Some(e) = &self.erreur_version {
                ui.add_space(6.0);
                ui.label(RichText::new(e).color(theme::ROUGE));
            }
            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if attente > 0 {
                    if bouton(ui, Some(Icone::Installer), "Installer d'abord les mises à jour", true, true).clicked() {
                        lancer_maj = true;
                    }
                    if bouton(ui, None, "Mettre à niveau quand même", true, false).clicked() {
                        lancer_niveau = true;
                    }
                } else if bouton(ui, Some(Icone::Installer), "Mettre à niveau", true, true).clicked() {
                    lancer_niveau = true;
                }
                if bouton(ui, Some(Icone::Fermer), "Annuler", true, false).clicked() {
                    fermer = true;
                }
            });
        });
        if lancer_maj {
            self.confirmer_version = false;
            self.installer();
        } else if lancer_niveau {
            match lancer_mise_a_niveau(self.reglages.devel) {
                Ok(()) => {
                    self.confirmer_version = false;
                    self.message_demarrage = Some("L'assistant de mise à niveau d'Ubuntu s'ouvre (votre mot de passe va être demandé).".into());
                }
                Err(e) => self.erreur_version = Some(e),
            }
        } else if fermer {
            self.confirmer_version = false;
        }
    }

    /// Barre « nouvelle version du gestionnaire » (en haut de chaque onglet).
    fn barre_gestionnaire(&mut self, ui: &mut Ui) {
        let (texte, bouton_txt, ic) = match &self.auto_maj {
            AutoMaj::Disponible(v) => (format!("Une nouvelle version de ce gestionnaire ({v}) est disponible sur GitHub."), Some("Mettre à jour le gestionnaire"), Icone::Info),
            AutoMaj::EnCours(v) => (format!("Mise à jour de ce gestionnaire en version {v}…"), None, Icone::Verifier),
            AutoMaj::Fait(v) => (format!("Ce gestionnaire a été mis à jour en version {v}."), Some("Relancer"), Icone::AJour),
            _ => return,
        };
        let mut clic = false;
        egui::Panel::top("barre_gestionnaire").frame(Frame::new()).show(ui, |ui| {
            egui::Frame::new()
                .fill(Color32::from_rgb(0xE6, 0xF0, 0xFA))
                .stroke(Stroke::new(1.0, Color32::from_rgb(0x72, 0x9F, 0xCF)))
                .corner_radius(3)
                .inner_margin(Margin::symmetric(10, 4))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        theme::icone(ui, ic, 22.0);
                        ui.label(texte);
                        if let Some(b) = bouton_txt {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                clic = bouton(ui, None, b, self.travail.is_none(), true).clicked();
                            });
                        }
                    });
                });
            ui.add_space(8.0);
        });
        if clic {
            match &self.auto_maj {
                AutoMaj::Fait(_) => {
                    lancer_detache(&[]);
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
                AutoMaj::Disponible(v) => {
                    let v = v.clone();
                    self.commencer(
                        "Mise à jour du gestionnaire",
                        &format!("La version {v} est téléchargée depuis GitHub puis installée."),
                        vec![Etape { texte: "Mise à jour du gestionnaire".into(), cmd: commande_auto_maj() }],
                        Apres::Recherche,
                    );
                }
                _ => {}
            }
        }
    }

    // -----------------------------------------------------------------------
    //  Onglet « Mise à niveau » (nouvelles versions d'Ubuntu)
    // -----------------------------------------------------------------------
    fn vue_niveau(&mut self, ui: &mut Ui) {
        let (actuelle, lts) = version_installee();
        let texte = format!(
            "Vous utilisez {} ({}, prise en charge jusqu'en {}).",
            systeme_actuel(),
            if lts { "version LTS" } else { "version normale" },
            fin_support(&actuelle, lts)
        );
        egui::Panel::top("niv_entete").frame(Frame::new()).show(ui, |ui| Self::entete(ui, Icone::Appli, "Version d'Ubuntu", &texte));
        let nv = self.recherche.as_ref().and_then(|r| r.nouvelle_version.clone());
        egui::Panel::bottom("niv_bas").frame(Frame::new()).show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let libre = self.travail.is_none();
                if bouton(ui, Some(Icone::Verifier), "Rechercher une nouvelle version", libre && self.occupe.is_none(), false).clicked() {
                    self.versions = None;
                    self.lancer_versions();
                    self.lancer_recherche(false);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let t = match &nv {
                        Some(v) => format!("Mettre à niveau vers {}…", v.version),
                        None => "Mettre à niveau…".into(),
                    };
                    if bouton(ui, Some(Icone::Installer), &t, libre && nv.is_some(), true).clicked() {
                        self.erreur_version = None;
                        self.confirmer_version = true;
                    }
                });
            });
            ui.add_space(4.0);
        });
        egui::CentralPanel::default().frame(Frame::new()).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                match (&nv, &self.occupe) {
                    (Some(v), _) => {
                        if bandeau_version(ui, v, self.travail.is_none()) {
                            self.erreur_version = None;
                            self.confirmer_version = true;
                        }
                    }
                    (None, Some(_)) => {
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new().size(16.0).color(SEL));
                            ui.label(RichText::new("Recherche d'une nouvelle version…").color(GRIS));
                        });
                        ui.add_space(8.0);
                    }
                    (None, None) => {
                        ui.horizontal(|ui| {
                            theme::icone(ui, Icone::AJour, 22.0);
                            let t = match self.prevenir.as_str() {
                                "never" => "Les nouvelles versions ne sont pas proposées (réglage « Jamais »).",
                                "lts" => "Aucune nouvelle version LTS à installer pour le moment.",
                                _ => "Aucune nouvelle version à installer pour le moment.",
                            };
                            ui.label(t);
                        });
                        ui.add_space(8.0);
                    }
                }

                // ---- choix des versions proposées ----
                ui.label(RichText::new("Quelles nouvelles versions me proposer ?").font(theme::police_gras(13.0)));
                ui.add_space(2.0);
                let mut choisi = None;
                for (val, titre, aide_txt) in [
                    ("lts", "Seulement les versions LTS (recommandé)", "Support longue durée : une version tous les 2 ans, prise en charge 5 ans. Le choix le plus tranquille."),
                    ("normal", "Toutes les nouvelles versions", "Une version tous les 6 mois, prise en charge 9 mois : les nouveautés arrivent plus vite."),
                    ("never", "Jamais", "L'ordinateur reste sur la version actuelle."),
                ] {
                    if radio(ui, self.prevenir == val, titre, aide_txt, self.travail.is_none()) && self.prevenir != val {
                        choisi = Some(val);
                    }
                }
                if !lts && self.prevenir == "lts" {
                    ui.horizontal(|ui| {
                        ui.add_space(26.0);
                        theme::icone(ui, Icone::Attention, 16.0);
                        ui.add(egui::Label::new(RichText::new(format!(
                            "Votre version n'est pas une LTS : elle est prise en charge jusqu'en {}. Avant cette date, passez à la version suivante (choix « Toutes les nouvelles versions ») si aucune LTS n'est encore sortie.",
                            fin_support(&actuelle, false)
                        )).color(theme::BRUN_FONCE).size(12.0)).wrap());
                    });
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    let mut d = self.reglages.devel;
                    if theme::case(ui, &mut d, "Proposer aussi les versions en développement (pour les essais, déconseillé)").changed() {
                        self.reglages.devel = d;
                        self.reglages.enregistrer();
                        self.lancer_recherche(false);
                    }
                });
                if let Some(val) = choisi {
                    self.prevenir = val.to_string();
                    self.prevenir_initial = self.prevenir.clone();
                    let cmd = en_admin(&[&aide().display().to_string(), "version", val]);
                    self.commencer(
                        "Réglage des nouvelles versions d'Ubuntu",
                        "Le choix est enregistré dans le système. Votre mot de passe peut être demandé.",
                        vec![Etape { texte: "Enregistrement du réglage".into(), cmd }],
                        Apres::Recherche,
                    );
                }

                // ---- tableau des versions ----
                ui.add_space(12.0);
                ui.label(RichText::new("Versions d'Ubuntu").font(theme::police_gras(13.0)));
                ui.add_space(2.0);
                theme::cadre_blanc().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let larg = ui.available_width();
                    let cols = [0.0, 0.19, 0.43, 0.60, 0.80];
                    let titres = ["Version", "Nom", "Type", "Prise en charge jusqu'en", "État"];
                    let (rh, _) = ui.allocate_exact_size(vec2(larg, 24.0), Sense::hover());
                    let p = ui.painter();
                    theme::degrade(p, rh, Color32::from_rgb(0xF8, 0xF6, 0xF3), Color32::from_rgb(0xE4, 0xDE, 0xD6), 0);
                    p.line_segment([rh.left_bottom(), rh.right_bottom()], Stroke::new(1.0, BORD));
                    for (c, t) in cols.iter().zip(titres) {
                        p.text(pos2(rh.left() + larg * c + 8.0, rh.center().y), Align2::LEFT_CENTER, t, theme::police(12.5), TEXTE);
                    }
                    match &self.versions {
                        None => attente(ui, "Lecture de la liste des versions d'Ubuntu…"),
                        Some(v) if v.is_empty() => {
                            ui.add_space(8.0);
                            ui.label(RichText::new("  La liste n'a pas pu être téléchargée.").color(GRIS));
                            ui.add_space(8.0);
                        }
                        Some(v) => {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            for (k, x) in v.iter().enumerate() {
                                let (rect, _) = ui.allocate_exact_size(vec2(larg, 24.0), Sense::hover());
                                let p = ui.painter();
                                let fond = if x.installee { theme::SEL_CLAIR } else if k % 2 == 1 { LIGNE_PAIRE } else { BLANC };
                                p.rect_filled(rect, 0, fond);
                                let (etat, c_etat) = if x.installee {
                                    ("Installée", theme::VERT)
                                } else if x.developpement {
                                    ("En développement", Color32::from_rgb(0xCE, 0x5C, 0x00))
                                } else if plus_recente(&x.version, &actuelle) {
                                    if x.prise_en_charge { ("Disponible", Color32::from_rgb(0x20, 0x4A, 0x87)) } else { ("Plus prise en charge", GRIS_CLAIR) }
                                } else {
                                    ("Dernière LTS", GRIS)
                                };
                                let valeurs = [
                                    format!("Ubuntu {}", x.version),
                                    x.nom.clone(),
                                    if x.lts { "LTS (5 ans)".into() } else { "normale (9 mois)".into() },
                                    x.fin_support(),
                                ];
                                for (i, val) in valeurs.iter().enumerate() {
                                    let f = if i == 0 { theme::police_gras(12.5) } else { theme::police(12.5) };
                                    let w = larg * (cols[i + 1] - cols[i]) - 12.0;
                                    let g = theme::texte_coupe(p, val, f, TEXTE, w);
                                    p.galley(pos2(rect.left() + larg * cols[i] + 8.0, rect.center().y - g.size().y / 2.0), g, TEXTE);
                                }
                                p.text(pos2(rect.left() + larg * cols[4] + 8.0, rect.center().y), Align2::LEFT_CENTER, etat, theme::police_gras(12.0), c_etat);
                            }
                        }
                    }
                });
            });
        });
    }

    fn fenetre_a_propos(&mut self, ctx: &Context) {
        let cadre = Frame::window(&ctx.global_style()).inner_margin(Margin::same(18)).fill(theme::FOND);
        let mut fermer = false;
        egui::Modal::new(Id::unique("a_propos")).frame(cadre).show(ctx, |ui| {
            ui.set_width(380.0);
            ui.vertical_centered(|ui| {
                theme::icone(ui, Icone::Appli, 64.0);
                ui.label(RichText::new(format!("{NOM} {VERSION}")).font(theme::police_gras(16.0)));
                ui.label("Mises à jour et pilotes, au look Ubuntu « Human » de 2008.");
                ui.add_space(6.0);
                ui.hyperlink_to(format!("github.com/{DEPOT}"), format!("https://github.com/{DEPOT}"));
                ui.add_space(4.0);
                let etat = match &self.auto_maj {
                    AutoMaj::Rien => String::new(),
                    AutoMaj::Recherche => "Recherche d'une nouvelle version sur GitHub…".into(),
                    AutoMaj::AJour(v) => format!("Ce logiciel est à jour (dernière version : {v})."),
                    AutoMaj::Disponible(v) => format!("La version {v} est disponible sur GitHub."),
                    AutoMaj::EnCours(v) => format!("Installation de la version {v}…"),
                    AutoMaj::Fait(v) => format!("La version {v} est installée : relancez le logiciel."),
                    AutoMaj::Erreur(e) => format!("Vérification impossible : {e}"),
                };
                if !etat.is_empty() {
                    ui.label(RichText::new(etat).color(GRIS));
                }
                let libre = !matches!(self.auto_maj, AutoMaj::Recherche | AutoMaj::EnCours(_));
                if bouton(ui, Some(Icone::Verifier), "Rechercher une mise à jour de ce logiciel", libre, false).clicked() {
                    self.verifier_gestionnaire(true);
                }
                ui.add_space(6.0);
                ui.label(RichText::new("© 2026 Jean-Baptiste Andrivet — licence GPL 3 ou ultérieure").color(GRIS));
                ui.add_space(10.0);
                if bouton(ui, Some(Icone::Fermer), "Fermer", true, true).clicked() {
                    fermer = true;
                }
            });
        });
        if fermer {
            self.a_propos = false;
        }
    }

    fn changer_demarrage(&mut self, actif: bool) {
        match regler_demarrage(actif) {
            Ok(()) => {
                self.demarrage = actif;
                self.message_demarrage = Some(if actif {
                    "Le gestionnaire se lancera à chaque démarrage et vous préviendra des mises à jour.".into()
                } else {
                    "Le gestionnaire ne se lancera plus au démarrage.".into()
                });
            }
            Err(e) => self.message_demarrage = Some(format!("Réglage impossible : {e}")),
        }
    }
}

/// Bouton radio avec un titre et une ligne d'explication. Renvoie vrai au clic.
fn radio(ui: &mut Ui, actif: bool, titre: &str, explication: &str, permis: bool) -> bool {
    let larg = ui.available_width();
    let (rect, rep) = ui.allocate_exact_size(vec2(larg, 40.0), if permis { Sense::click() } else { Sense::hover() });
    let p = ui.painter();
    if rep.hovered() && permis {
        p.rect_filled(rect, 3, Color32::from_rgb(0xF6, 0xF2, 0xED));
    }
    theme::dessiner_radio(p, pos2(rect.left() + 14.0, rect.top() + 12.0), actif, rep.hovered() && permis);
    p.text(pos2(rect.left() + 30.0, rect.top() + 12.0), Align2::LEFT_CENTER, titre, if actif { theme::police_gras(13.0) } else { theme::police(13.0) }, TEXTE);
    p.text(pos2(rect.left() + 30.0, rect.top() + 29.0), Align2::LEFT_CENTER, explication, theme::police(11.5), GRIS);
    rep.clicked()
}

/// Bandeau jaune « nouvelle version disponible ». Renvoie vrai au clic sur « Mettre à niveau… ».
fn bandeau_version(ui: &mut Ui, nv: &NouvelleVersion, actif: bool) -> bool {
    let mut clic = false;
    egui::Frame::new()
        .fill(theme::BULLE)
        .stroke(Stroke::new(1.0, Color32::from_rgb(0xC4, 0xA0, 0x00)))
        .corner_radius(3)
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                theme::icone(ui, Icone::Appli, 32.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(format!("{} est disponible", nv.titre())).font(theme::police_gras(13.0)));
                    ui.label(RichText::new(format!("Vous utilisez {}. Vos fichiers et réglages sont conservés.", systeme_actuel())).color(GRIS).size(12.0));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if bouton(ui, Some(Icone::Installer), "Mettre à niveau…", actif, true).clicked() {
                        clic = true;
                    }
                });
            });
        });
    ui.add_space(8.0);
    clic
}

fn choix(ui: &mut Ui, id: &str, valeur: &mut u32, options: &[(u32, &str)]) {
    let actuel = options.iter().find(|(v, _)| v == valeur).map(|(_, t)| *t).unwrap_or("personnalisé");
    egui::ComboBox::from_id_salt(id).selected_text(actuel).width(200.0).show_ui(ui, |ui| {
        for (v, t) in options {
            ui.selectable_value(valeur, *v, *t);
        }
    });
}

fn attente(ui: &mut Ui, texte: &str) {
    ui.add_space(30.0);
    ui.vertical_centered(|ui| {
        ui.add(egui::Spinner::new().size(28.0).color(SEL));
        ui.add_space(6.0);
        ui.label(RichText::new(texte).color(GRIS));
    });
}

fn duree(s: u64) -> String {
    if s < 3600 {
        format!("{} min", (s / 60).max(1))
    } else if s < 48 * 3600 {
        format!("{} h", s / 3600)
    } else {
        format!("{} jours", s / 86400)
    }
}

pub fn redemarrer() {
    // fenêtre de confirmation du bureau si possible, sinon redémarrage direct
    for (prog, args) in [
        ("gnome-session-quit", vec!["--reboot"]),
        ("mate-session-save", vec!["--shutdown-dialog"]),
        ("systemctl", vec!["reboot"]),
    ] {
        if existe(prog) && Command::new(prog).args(&args).spawn().is_ok() {
            return;
        }
    }
}

impl eframe::App for Appli {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        theme::capture_essai(&ctx);
        self.recevoir();
        // Pas de fermeture en pleine installation
        if ctx.input(|i| i.viewport().close_requested()) && self.travail.as_ref().is_some_and(|t| t.fini.is_none()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }

        egui::Panel::top("bandeau").frame(Frame::new().fill(FOND)).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            theme::bandeau(ui, NOM, "Garde votre ordinateur à jour et choisit les bons pilotes", Icone::Appli);
            ui.add_space(10.0);
            let avant = self.onglet;
            theme::onglets(ui, &mut self.onglet, &[("Mises à jour", Icone::Installer), ("Pilotes", Icone::Pilote), ("Mise à niveau", Icone::Appli)]);
            if self.onglet == 1 && avant != 1 && self.pilotes.is_none() {
                self.lancer_pilotes();
            }
            if self.onglet == 2 && avant != 2 && self.versions.is_none() {
                self.lancer_versions();
            }
        });
        egui::Panel::bottom("pied")
            .frame(Frame::new().fill(theme::FOND2).inner_margin(Margin::symmetric(12, 6)).stroke(Stroke::new(1.0, BORD)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let mut d = self.demarrage;
                    if theme::case(ui, &mut d, "Lancer au démarrage de l'ordinateur et me prévenir des mises à jour").changed() {
                        self.changer_demarrage(d);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if bouton(ui, Some(Icone::Fermer), "Fermer", self.travail.is_none(), false).clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if bouton(ui, Some(Icone::Info), "À propos", true, false).clicked() {
                            self.a_propos = true;
                        }
                    });
                });
                if let Some(m) = &self.message_demarrage {
                    ui.label(RichText::new(m).color(GRIS).size(11.5));
                }
            });
        egui::CentralPanel::default()
            .frame(Frame::new().fill(FOND).inner_margin(Margin { left: 14, right: 14, top: 12, bottom: 6 }))
            .show(ui, |ui| {
                self.barre_gestionnaire(ui);
                match self.onglet {
                    1 => self.vue_pilotes(ui),
                    2 => self.vue_niveau(ui),
                    _ => self.vue_maj(ui),
                }
            });

        if self.travail.is_some() {
            self.fenetre_travail(&ctx);
        }
        if self.reglages_ouverts {
            self.fenetre_reglages(&ctx);
        }
        if self.a_propos {
            self.fenetre_a_propos(&ctx);
        }
        if self.confirmer_version && self.travail.is_none() {
            self.fenetre_version(&ctx);
        }
    }
}

pub fn lancer(mode: Mode) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(NOM)
            .with_inner_size([860.0, 660.0])
            .with_min_inner_size([680.0, 520.0])
            .with_app_id(APP),
        ..Default::default()
    };
    eframe::run_native(NOM, options, Box::new(move |cc| Ok(Box::new(Appli::nouveau(cc, mode)))))
}
