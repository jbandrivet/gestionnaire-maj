//! Bulle d'annonce en haut à droite de l'écran, façon notification GNOME 2.

use crate::commun::*;
use crate::theme::{self, bouton, Icone, BLANC, GRIS, TEXTE};
use eframe::egui::{self, pos2, vec2, Align, Align2, Color32, Frame, Layout, Margin, Rect, RichText, Sense, Stroke, Ui};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Annonce {
    pub nombre: usize,
    pub securite: usize,
    pub taille: u64,
    /// (composant, paquet du pilote)
    pub pilotes: Vec<(String, String)>,
    /// Nouvelle version d'Ubuntu (« Ubuntu 27.04 « Tenacious T-Rex » »)
    #[serde(default)]
    pub version: Option<String>,
}

impl Annonce {
    fn depuis_recherche() -> Self {
        let reg = Reglages::charger();
        let r = chercher_tout(&reg);
        let mut a = Annonce {
            nombre: r.majs.iter().filter(|m| m.groupe.coche_defaut()).count(),
            securite: r.majs.iter().filter(|m| m.groupe == Groupe::Securite).count(),
            taille: r.majs.iter().filter(|m| m.groupe.coche_defaut()).map(|m| m.taille).sum(),
            pilotes: Vec::new(),
            version: r.nouvelle_version.as_ref().map(|v| v.titre()),
        };
        if reg.pilotes {
            a.pilotes = chercher_pilotes().recommandes_absents();
        }
        a
    }
}

struct Bulle {
    annonce: Annonce,
    place: bool,
    images: u32,
}

const LARGEUR: f32 = 400.0;

impl Bulle {
    fn hauteur(&self) -> f32 {
        let mut h = 44.0 + 16.0 + 46.0;
        if self.annonce.nombre > 0 {
            h += 58.0;
        }
        if self.annonce.version.is_some() {
            h += 58.0;
        }
        if !self.annonce.pilotes.is_empty() {
            h += 40.0 + 18.0 * self.annonce.pilotes.len().min(3) as f32;
        }
        h
    }

    fn corps(&mut self, ui: &mut Ui) {
        let a = &self.annonce;
        let rect = ui.max_rect();
        // bandeau du titre
        let haut = Rect::from_min_size(rect.min, vec2(rect.width(), 40.0));
        let p = ui.painter();
        theme::degrade(p, haut, Color32::from_rgb(0xAC, 0x8C, 0x67), Color32::from_rgb(0x5A, 0x43, 0x30), 0);
        p.rect_filled(Rect::from_min_size(haut.left_bottom(), vec2(rect.width(), 3.0)), 0, theme::SEL);
        theme::dessiner_icone(p, Icone::Appli, Rect::from_min_size(haut.min + vec2(10.0, 5.0), vec2(30.0, 30.0)), true);
        let titre = if a.nombre > 0 {
            "Mises à jour disponibles"
        } else if a.version.is_some() {
            "Nouvelle version d'Ubuntu"
        } else {
            "Pilote disponible"
        };
        p.text(haut.left_center() + vec2(49.0, 1.0), Align2::LEFT_CENTER, titre, theme::police_gras(14.5), Color32::from_rgb(0x3B, 0x2B, 0x1D));
        p.text(haut.left_center() + vec2(48.0, 0.0), Align2::LEFT_CENTER, titre, theme::police_gras(14.5), BLANC);
        // croix de fermeture
        let croix = Rect::from_center_size(haut.right_center() - vec2(18.0, 0.0), vec2(20.0, 20.0));
        let rep = ui.interact(croix, egui::Id::unique("croix"), Sense::click());
        let c = if rep.hovered() { BLANC } else { Color32::from_rgb(0xE8, 0xDC, 0xCC) };
        let s = Stroke::new(2.0, c);
        ui.painter().line_segment([croix.center() + vec2(-5.0, -5.0), croix.center() + vec2(5.0, 5.0)], s);
        ui.painter().line_segment([croix.center() + vec2(5.0, -5.0), croix.center() + vec2(-5.0, 5.0)], s);
        if rep.on_hover_text("Fermer").clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        ui.add_space(52.0);
        egui::Frame::new().inner_margin(Margin { left: 14, right: 14, top: 0, bottom: 10 }).show(ui, |ui| {
            if a.nombre > 0 {
                ui.horizontal(|ui| {
                    theme::icone(ui, if a.securite > 0 { Icone::Securite } else { Icone::Paquet }, 32.0);
                    ui.vertical(|ui| {
                        let t = if a.nombre == 1 {
                            "Une mise à jour est disponible pour votre ordinateur.".to_string()
                        } else {
                            format!("{} mises à jour sont disponibles pour votre ordinateur.", a.nombre)
                        };
                        ui.add(egui::Label::new(RichText::new(t).font(theme::police_gras(13.0))).wrap());
                        let mut d = String::new();
                        if a.securite > 0 {
                            d.push_str(&format!("Dont {} de sécurité. ", a.securite));
                        }
                        if a.taille > 0 {
                            d.push_str(&format!("Téléchargement : {}", taille_lisible(a.taille)));
                        }
                        if !d.is_empty() {
                            ui.label(RichText::new(d).color(GRIS));
                        }
                    });
                });
                ui.add_space(6.0);
            }
            if let Some(v) = &a.version {
                ui.horizontal(|ui| {
                    theme::icone(ui, Icone::Appli, 32.0);
                    ui.vertical(|ui| {
                        ui.add(egui::Label::new(RichText::new(format!("{v} est disponible.")).font(theme::police_gras(13.0))).wrap());
                        if a.nombre > 0 {
                            if ui.link("Mettre à niveau vers cette version…").clicked() {
                                lancer_detache(&["--mise-a-niveau"]);
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        } else {
                            ui.label(RichText::new("Vos fichiers et réglages sont conservés.").color(GRIS));
                        }
                    });
                });
                ui.add_space(6.0);
            }
            if !a.pilotes.is_empty() {
                ui.horizontal(|ui| {
                    theme::icone(ui, Icone::Pilote, 32.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Un pilote recommandé peut être activé :").font(theme::police_gras(13.0)));
                        for (nom, paquet) in a.pilotes.iter().take(3) {
                            let g = theme::texte_coupe(ui.painter(), &format!("{nom} ({paquet})"), theme::police(12.0), GRIS, LARGEUR - 90.0);
                            let (r, _) = ui.allocate_exact_size(vec2(LARGEUR - 90.0, 16.0), Sense::hover());
                            ui.painter().galley(r.left_top(), g, GRIS);
                        }
                    });
                });
                ui.add_space(6.0);
            }
            ui.add_space(4.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let ctx = ui.ctx().clone();
                if a.nombre > 0 {
                    if bouton(ui, Some(Icone::Installer), "Installer", true, true).clicked() {
                        lancer_detache(&["--installer"]);
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if bouton(ui, None, "Voir les détails", true, false).clicked() {
                        lancer_detache(&[]);
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                } else if a.version.is_some() {
                    if bouton(ui, Some(Icone::Installer), "Mettre à niveau…", true, true).clicked() {
                        lancer_detache(&["--mise-a-niveau"]);
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                } else if bouton(ui, Some(Icone::Pilote), "Voir les pilotes", true, true).clicked() {
                    lancer_detache(&["--pilotes"]);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if bouton(ui, None, "Plus tard", true, false).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }
}

impl eframe::App for Bulle {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] {
        [0.37, 0.27, 0.19, 1.0]
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        theme::capture_essai(&ctx);
        self.images += 1;
        if !self.place {
            if let Some(ecran) = ctx.input(|i| i.viewport().monitor_size) {
                let x = (ecran.x - LARGEUR - 14.0).max(0.0);
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos2(x, 40.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                self.place = true;
            } else if self.images > 5 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                self.place = true;
            } else {
                ctx.request_repaint();
            }
        }
        egui::CentralPanel::default()
            .frame(Frame::new().fill(theme::FOND).stroke(Stroke::new(1.0, Color32::from_rgb(0x5E, 0x46, 0x30))))
            .show(ui, |ui| self.corps(ui));
        let _ = TEXTE;
    }
}

pub fn lancer(fichier: Option<&String>) -> eframe::Result {
    let annonce = match fichier {
        Some(f) => std::fs::read_to_string(f).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default(),
        None => Annonce::depuis_recherche(),
    };
    if annonce.nombre == 0 && annonce.pilotes.is_empty() && annonce.version.is_none() {
        println!("Rien à signaler : l'ordinateur est à jour.");
        return Ok(());
    }
    // La bulle se place elle-même : sous Wayland seul X11 (XWayland) le permet.
    if std::env::var_os("DISPLAY").is_some() {
        std::env::remove_var("WAYLAND_DISPLAY");
    }
    let b = Bulle { annonce, place: false, images: 0 };
    let h = b.hauteur();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Mises à jour disponibles")
            .with_app_id(APP)
            .with_inner_size([LARGEUR, h])
            .with_resizable(false)
            .with_decorations(false)
            .with_always_on_top()
            .with_taskbar(false)
            .with_visible(false),
        ..Default::default()
    };
    eframe::run_native(
        "Mises à jour disponibles",
        options,
        Box::new(move |cc| {
            theme::installer(&cc.egui_ctx);
            Ok(Box::new(b))
        }),
    )
}
