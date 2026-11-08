//! Habillage « Human » d'Ubuntu 8.04 (2008) : beige, brun et orange, boutons bombés,
//! cases à cocher GNOME 2 et icônes dessinées façon Tango.

use eframe::egui::{
    self, pos2, vec2, Align2, Color32, ColorImage, Context, CornerRadius, FontData, FontDefinitions,
    FontFamily, FontId, Id, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, TextureHandle,
    TextureId, TextureOptions, Ui, Vec2,
};
use std::sync::Arc;

pub const FOND: Color32 = Color32::from_rgb(0xEF, 0xEB, 0xE7);
pub const FOND2: Color32 = Color32::from_rgb(0xE2, 0xDC, 0xD4);
pub const BORD: Color32 = Color32::from_rgb(0xA6, 0x9C, 0x91);
pub const TEXTE: Color32 = Color32::from_rgb(0x1A, 0x1A, 0x1A);
pub const GRIS: Color32 = Color32::from_rgb(0x55, 0x57, 0x53);
pub const GRIS_CLAIR: Color32 = Color32::from_rgb(0x88, 0x8A, 0x85);
pub const SEL: Color32 = Color32::from_rgb(0xE5, 0x82, 0x2E);
pub const SEL_CLAIR: Color32 = Color32::from_rgb(0xFB, 0xE6, 0xD3);
pub const BLANC: Color32 = Color32::WHITE;
pub const LIGNE_PAIRE: Color32 = Color32::from_rgb(0xF7, 0xF5, 0xF2);
pub const BULLE: Color32 = Color32::from_rgb(0xF5, 0xF5, 0xB5);
pub const VERT: Color32 = Color32::from_rgb(0x4E, 0x9A, 0x06);
pub const ROUGE: Color32 = Color32::from_rgb(0xCC, 0x00, 0x00);
pub const BRUN_FONCE: Color32 = Color32::from_rgb(0x5A, 0x43, 0x30);

pub fn gras() -> FontFamily {
    FontFamily::Name("gras".into())
}

pub fn police(t: f32) -> FontId {
    FontId::new(t, FontFamily::Proportional)
}

pub fn police_gras(t: f32) -> FontId {
    FontId::new(t, gras())
}

/// Polices DejaVu Sans (celles d'Ubuntu 8.04) et visuels Human.
pub fn installer(ctx: &Context) {
    let mut polices = FontDefinitions::default();
    let normal = ["/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"];
    let gras_f = ["/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"];
    let mono = ["/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"];
    let mut charger = |nom: &str, chemins: &[&str]| -> bool {
        for c in chemins {
            if let Ok(o) = std::fs::read(c) {
                polices.font_data.insert(nom.into(), Arc::new(FontData::from_owned(o)));
                return true;
            }
        }
        false
    };
    let a_normal = charger("dejavu", &normal);
    let a_gras = charger("dejavu-gras", &gras_f);
    let a_mono = charger("dejavu-mono", &mono);
    if a_normal {
        polices.families.entry(FontFamily::Proportional).or_default().insert(0, "dejavu".into());
    }
    if a_mono {
        polices.families.entry(FontFamily::Monospace).or_default().insert(0, "dejavu-mono".into());
    }
    let mut famille_gras = Vec::new();
    if a_gras {
        famille_gras.push("dejavu-gras".to_string());
    }
    famille_gras.extend(polices.families.get(&FontFamily::Proportional).cloned().unwrap_or_default());
    polices.families.insert(gras(), famille_gras);
    ctx.set_fonts(polices);

    let mut v = egui::Visuals::light();
    v.panel_fill = FOND;
    v.window_fill = FOND;
    v.extreme_bg_color = BLANC;
    v.faint_bg_color = LIGNE_PAIRE;
    v.override_text_color = Some(TEXTE);
    v.selection.bg_fill = SEL;
    v.selection.stroke = Stroke::new(1.0, BLANC);
    v.hyperlink_color = Color32::from_rgb(0x20, 0x4A, 0x87);
    v.window_stroke = Stroke::new(1.0, Color32::from_rgb(0x5E, 0x46, 0x30));
    v.window_corner_radius = CornerRadius::same(6);
    v.menu_corner_radius = CornerRadius::same(3);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = FOND;
    w.noninteractive.weak_bg_fill = FOND;
    w.noninteractive.bg_stroke = Stroke::new(1.0, BORD);
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXTE);
    w.inactive.bg_fill = Color32::from_rgb(0xF4, 0xF1, 0xEC);
    w.inactive.weak_bg_fill = Color32::from_rgb(0xF4, 0xF1, 0xEC);
    w.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x9C, 0x92, 0x86));
    w.inactive.fg_stroke = Stroke::new(1.0, TEXTE);
    w.inactive.corner_radius = CornerRadius::same(3);
    w.hovered.bg_fill = Color32::from_rgb(0xFB, 0xF9, 0xF6);
    w.hovered.weak_bg_fill = Color32::from_rgb(0xFB, 0xF9, 0xF6);
    w.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0xB8, 0x8A, 0x5C));
    w.hovered.fg_stroke = Stroke::new(1.0, TEXTE);
    w.hovered.corner_radius = CornerRadius::same(3);
    w.hovered.expansion = 0.0;
    w.active.bg_fill = Color32::from_rgb(0xD9, 0xD1, 0xC7);
    w.active.weak_bg_fill = Color32::from_rgb(0xD9, 0xD1, 0xC7);
    w.active.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x8B, 0x6A, 0x48));
    w.active.fg_stroke = Stroke::new(1.0, TEXTE);
    w.active.corner_radius = CornerRadius::same(3);
    w.active.expansion = 0.0;
    w.open = w.active;
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = vec2(6.0, 5.0);
        s.spacing.button_padding = vec2(9.0, 4.0);
        s.spacing.interact_size.y = 24.0;
        s.text_styles.insert(egui::TextStyle::Body, police(13.0));
        s.text_styles.insert(egui::TextStyle::Button, police(13.0));
        s.text_styles.insert(egui::TextStyle::Small, police(11.0));
        s.text_styles.insert(egui::TextStyle::Heading, police_gras(16.0));
        s.text_styles.insert(egui::TextStyle::Monospace, FontId::monospace(12.0));
    });
}

// ---------------------------------------------------------------------------
//  Dégradés : une petite texture verticale, étirée sur le rectangle
// ---------------------------------------------------------------------------
fn texture_degrade(ctx: &Context, haut: Color32, bas: Color32) -> TextureId {
    let id = Id::unique(("degrade", haut.to_array(), bas.to_array()));
    if let Some(t) = ctx.data(|d| d.get_temp::<TextureHandle>(id)) {
        return t.id();
    }
    let n = 32;
    let pixels: Vec<Color32> = (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1) as f32;
            melange(haut, bas, t)
        })
        .collect();
    let img = ColorImage::new([1, n], pixels);
    let h = ctx.load_texture(format!("degrade-{id:?}"), img, TextureOptions::LINEAR);
    let tid = h.id();
    ctx.data_mut(|d| d.insert_temp(id, h));
    tid
}

pub fn melange(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()), m(a.a(), b.a()))
}

pub fn degrade(p: &Painter, rect: Rect, haut: Color32, bas: Color32, rayon: u8) {
    let tex = texture_degrade(p.ctx(), haut, bas);
    let uv = Rect::from_min_max(pos2(0.5, 0.5 / 32.0), pos2(0.5, 1.0 - 0.5 / 32.0));
    p.add(Shape::Rect(
        egui::epaint::RectShape::filled(rect, CornerRadius::same(rayon), BLANC).with_texture(tex, uv),
    ));
}

// ---------------------------------------------------------------------------
//  Bouton bombé GNOME 2
// ---------------------------------------------------------------------------
pub fn bouton(ui: &mut Ui, icone: Option<Icone>, texte: &str, actif: bool, principal: bool) -> Response {
    let f = if principal { police_gras(13.0) } else { police(13.0) };
    let galet = ui.painter().layout_no_wrap(texte.to_string(), f.clone(), TEXTE);
    let ic = if icone.is_some() { 20.0 } else { 0.0 };
    let taille = vec2((galet.size().x + ic + 26.0).max(84.0), 30.0);
    let (rect, rep) = ui.allocate_exact_size(taille, if actif { Sense::click() } else { Sense::hover() });
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let (haut, bas, bord) = if !actif {
            (Color32::from_rgb(0xF4, 0xF1, 0xEC), Color32::from_rgb(0xE9, 0xE4, 0xDE), Color32::from_rgb(0xBF, 0xB7, 0xAD))
        } else if rep.is_pointer_button_down_on() {
            (Color32::from_rgb(0xCF, 0xC6, 0xBA), Color32::from_rgb(0xE4, 0xDE, 0xD6), Color32::from_rgb(0x8B, 0x6A, 0x48))
        } else if rep.hovered() {
            (Color32::from_rgb(0xFF, 0xFF, 0xFF), Color32::from_rgb(0xEE, 0xE8, 0xE0), Color32::from_rgb(0xB8, 0x8A, 0x5C))
        } else {
            (Color32::from_rgb(0xFD, 0xFC, 0xFA), Color32::from_rgb(0xDF, 0xD8, 0xCF), Color32::from_rgb(0x9C, 0x92, 0x86))
        };
        degrade(p, rect.shrink(0.5), haut, bas, 4);
        p.rect_stroke(rect.shrink(0.5), 4, Stroke::new(1.0, bord), StrokeKind::Inside);
        if principal && actif {
            p.rect_stroke(rect.shrink(2.0), 3, Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xE5, 0x82, 0x2E, 150)), StrokeKind::Inside);
        }
        let couleur = if actif { TEXTE } else { Color32::from_rgb(0x9A, 0x93, 0x8B) };
        let largeur = galet.size().x + ic;
        let x0 = rect.center().x - largeur / 2.0;
        if let Some(i) = icone {
            let r = Rect::from_center_size(pos2(x0 + 8.0, rect.center().y), vec2(16.0, 16.0));
            dessiner_icone(p, i, r, actif);
        }
        p.text(pos2(x0 + ic, rect.center().y), Align2::LEFT_CENTER, texte, f, couleur);
    }
    if actif {
        rep.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        rep
    }
}

// ---------------------------------------------------------------------------
//  Cases à cocher
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Etat {
    Non,
    Oui,
    Partiel,
}

pub fn dessiner_case(p: &Painter, r: Rect, etat: Etat, survol: bool) {
    let bord = if survol { SEL } else { Color32::from_rgb(0x8B, 0x81, 0x78) };
    degrade(p, r, Color32::from_rgb(0xEE, 0xEA, 0xE4), BLANC, 2);
    p.rect_stroke(r, 2, Stroke::new(1.0, bord), StrokeKind::Inside);
    let u = r.width() / 16.0;
    let q = |x: f32, y: f32| pos2(r.left() + x * u, r.top() + y * u);
    match etat {
        Etat::Oui => {
            p.add(Shape::line(
                vec![q(3.8, 8.3), q(6.6, 11.4), q(12.3, 4.4)],
                Stroke::new(2.3 * u, Color32::from_rgb(0x3B, 0x7A, 0x06)),
            ));
        }
        Etat::Partiel => {
            p.line_segment([q(4.0, 8.0), q(12.0, 8.0)], Stroke::new(2.3 * u, Color32::from_rgb(0x3B, 0x7A, 0x06)));
        }
        Etat::Non => {}
    }
}

/// Case avec son libellé ; inverse la valeur au clic.
pub fn case(ui: &mut Ui, valeur: &mut bool, texte: &str) -> Response {
    let galet = ui.painter().layout(texte.to_string(), police(13.0), TEXTE, ui.available_width() - 24.0);
    let taille = vec2(galet.size().x + 24.0, galet.size().y.max(18.0));
    let (rect, mut rep) = ui.allocate_exact_size(taille, Sense::click());
    if rep.clicked() {
        *valeur = !*valeur;
        rep.mark_changed();
    }
    let c = Rect::from_min_size(pos2(rect.left(), rect.top() + (taille.y - 15.0) / 2.0 - 0.0), vec2(15.0, 15.0));
    let c = if galet.rows.len() > 1 { Rect::from_min_size(pos2(rect.left(), rect.top() + 1.0), vec2(15.0, 15.0)) } else { c };
    dessiner_case(ui.painter(), c, if *valeur { Etat::Oui } else { Etat::Non }, rep.hovered());
    ui.painter().galley(pos2(rect.left() + 22.0, rect.top()), galet, TEXTE);
    rep
}

/// Bouton radio GNOME 2.
pub fn dessiner_radio(p: &Painter, c: Pos2, actif: bool, survol: bool) {
    let bord = if survol { SEL } else { Color32::from_rgb(0x8B, 0x81, 0x78) };
    p.circle_filled(c, 7.0, BLANC);
    p.circle_stroke(c, 7.0, Stroke::new(1.0, bord));
    if actif {
        p.circle_filled(c, 3.5, Color32::from_rgb(0x3B, 0x7A, 0x06));
    }
}

// ---------------------------------------------------------------------------
//  Onglets et cadres
// ---------------------------------------------------------------------------
pub fn onglets(ui: &mut Ui, choix: &mut usize, titres: &[(&str, Icone)]) {
    // filet sous les onglets (le contenu de l'onglet actif « sort » de ce filet)
    let y = ui.cursor().top() + 30.0;
    let (gauche, droite) = (ui.max_rect().left(), ui.max_rect().right());
    let mut actif_x = (0.0, 0.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.add_space(8.0);
        for (i, (t, ic)) in titres.iter().enumerate() {
            let galet = ui.painter().layout_no_wrap(t.to_string(), police(13.0), TEXTE);
            let actif = *choix == i;
            let h = if actif { 30.0 } else { 27.0 };
            let (rect, rep) = ui.allocate_exact_size(vec2(galet.size().x + 46.0, 30.0), Sense::click());
            let r = Rect::from_min_max(pos2(rect.left(), rect.bottom() - h), rect.max);
            let p = ui.painter();
            let (haut, bas) = if actif {
                (BLANC, FOND)
            } else if rep.hovered() {
                (Color32::from_rgb(0xF6, 0xF2, 0xED), Color32::from_rgb(0xE6, 0xE0, 0xD8))
            } else {
                (Color32::from_rgb(0xE8, 0xE2, 0xDA), Color32::from_rgb(0xD6, 0xCE, 0xC4))
            };
            degrade(p, r, haut, bas, 0);
            let bord = Stroke::new(1.0, BORD);
            p.line_segment([r.left_bottom(), r.left_top()], bord);
            p.line_segment([r.left_top(), r.right_top()], bord);
            p.line_segment([r.right_top(), r.right_bottom()], bord);
            if actif {
                p.line_segment([r.left_top() + vec2(1.0, 0.5), r.right_top() + vec2(-1.0, 0.5)], Stroke::new(2.0, SEL));
                actif_x = (r.left(), r.right());
            }
            dessiner_icone(p, *ic, Rect::from_center_size(pos2(r.left() + 16.0, r.center().y + 1.0), vec2(16.0, 16.0)), true);
            p.galley(pos2(r.left() + 30.0, r.center().y - galet.size().y / 2.0 + 1.0), galet, TEXTE);
            if rep.clicked() {
                *choix = i;
            }
        }
    });
    let s = Stroke::new(1.0, BORD);
    let p = ui.painter();
    p.line_segment([pos2(gauche, y - 0.5), pos2(actif_x.0, y - 0.5)], s);
    p.line_segment([pos2(actif_x.1, y - 0.5), pos2(droite, y - 0.5)], s);
}

/// Barre de progression Human (orange dégradé) ; `None` = attente animée.
pub fn barre_progression(ui: &mut Ui, valeur: Option<f32>) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
    let p = ui.painter();
    degrade(p, rect, Color32::from_rgb(0xD6, 0xCF, 0xC6), Color32::from_rgb(0xEC, 0xE8, 0xE2), 3);
    p.rect_stroke(rect, 3, Stroke::new(1.0, BORD), StrokeKind::Inside);
    let int = rect.shrink(2.0);
    let plein = match valeur {
        Some(v) => Rect::from_min_size(int.min, vec2(int.width() * v.clamp(0.0, 1.0), int.height())),
        None => {
            let t = ui.input(|i| i.time) as f32;
            let l = int.width() * 0.22;
            let x = (t * 0.8).fract();
            let x = if (t * 0.8) as i64 % 2 == 0 { x } else { 1.0 - x };
            ui.ctx().request_repaint();
            Rect::from_min_size(pos2(int.left() + (int.width() - l) * x, int.top()), vec2(l, int.height()))
        }
    };
    if plein.width() > 1.0 {
        degrade(p, plein, Color32::from_rgb(0xF6, 0xA5, 0x5C), Color32::from_rgb(0xC9, 0x5F, 0x12), 2);
        p.rect_stroke(plein, 2, Stroke::new(1.0, Color32::from_rgb(0xA3, 0x4A, 0x0B)), StrokeKind::Inside);
    }
    if let Some(v) = valeur {
        let t = format!("{:.0} %", v * 100.0);
        p.text(rect.center() + vec2(1.0, 1.0), Align2::CENTER_CENTER, &t, police_gras(11.5), Color32::from_rgba_unmultiplied(255, 255, 255, 120));
        p.text(rect.center(), Align2::CENTER_CENTER, t, police_gras(11.5), TEXTE);
    }
}

/// Petit triangle GNOME 2 pour déplier / replier.
pub fn dessiner_triangle(p: &Painter, c: Pos2, ouvert: bool, couleur: Color32) {
    let pts = if ouvert {
        vec![c + vec2(-4.5, -2.5), c + vec2(4.5, -2.5), c + vec2(0.0, 3.5)]
    } else {
        vec![c + vec2(-2.5, -4.5), c + vec2(3.5, 0.0), c + vec2(-2.5, 4.5)]
    };
    p.add(Shape::convex_polygon(pts, couleur, Stroke::NONE));
}

/// « ▸ Titre » cliquable ; inverse `ouvert`.
pub fn depliant(ui: &mut Ui, ouvert: &mut bool, titre: &str) {
    let galet = ui.painter().layout_no_wrap(titre.to_string(), police_gras(13.0), TEXTE);
    let (rect, rep) = ui.allocate_exact_size(vec2(galet.size().x + 20.0, 20.0), Sense::click());
    if rep.clicked() {
        *ouvert = !*ouvert;
    }
    let coul = if rep.hovered() { SEL } else { GRIS };
    dessiner_triangle(ui.painter(), pos2(rect.left() + 6.0, rect.center().y), *ouvert, coul);
    ui.painter().galley(pos2(rect.left() + 16.0, rect.center().y - galet.size().y / 2.0), galet, TEXTE);
}

/// Texte sur une ligne, coupé avec « … » s'il dépasse.
pub fn texte_coupe(p: &Painter, texte: &str, f: FontId, couleur: Color32, largeur: f32) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::single_section(texte.to_string(), egui::TextFormat::simple(f, couleur));
    job.wrap = egui::text::TextWrapping {
        max_width: largeur.max(10.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    p.layout_job(job)
}

/// Bandeau brun dégradé d'Ubuntu 2008 avec l'icône et le titre.
pub fn bandeau(ui: &mut Ui, titre: &str, sous_titre: &str, icone: Icone) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 70.0), Sense::hover());
    let p = ui.painter();
    let haut = Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() - 5.0));
    degrade(p, haut, Color32::from_rgb(0xAC, 0x8C, 0x67), Color32::from_rgb(0x5A, 0x43, 0x30), 0);
    p.line_segment([haut.left_top() + vec2(0.0, 0.5), haut.right_top() + vec2(0.0, 0.5)], Stroke::new(1.0, Color32::from_rgb(0xC9, 0xAE, 0x8C)));
    let bande = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 5.0), rect.max);
    degrade(p, bande, Color32::from_rgb(0xF0, 0x93, 0x40), Color32::from_rgb(0xC0, 0x5E, 0x10), 0);
    dessiner_icone(p, icone, Rect::from_min_size(pos2(rect.left() + 14.0, rect.top() + 9.0), vec2(48.0, 48.0)), true);
    let x = rect.left() + 76.0;
    let ombre = Color32::from_rgb(0x3B, 0x2B, 0x1D);
    for (dy, c) in [(1.5, ombre), (0.0, BLANC)] {
        p.text(pos2(x + dy, rect.top() + 14.0 + dy), Align2::LEFT_TOP, titre, police_gras(19.0), c);
    }
    for (dy, c) in [(1.0, ombre), (0.0, Color32::from_rgb(0xF2, 0xE6, 0xD6))] {
        p.text(pos2(x + dy, rect.top() + 41.0 + dy), Align2::LEFT_TOP, sous_titre, police(12.5), c);
    }
}

/// Cadre blanc à bord fin (fond des listes GTK).
pub fn cadre_blanc() -> egui::Frame {
    egui::Frame::new()
        .fill(BLANC)
        .stroke(Stroke::new(1.0, BORD))
        .corner_radius(CornerRadius::same(2))
        .inner_margin(egui::Margin::same(1))
}

// ---------------------------------------------------------------------------
//  Icônes façon Tango, dessinées en vectoriel
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icone {
    Appli,
    Verifier,
    Installer,
    Parametres,
    Fermer,
    Pilote,
    AJour,
    Attention,
    Erreur,
    Securite,
    Paquet,
    Snap,
    Redemarrer,
    Materiel,
    Info,
}

pub fn dessiner_icone(p: &Painter, ic: Icone, r: Rect, actif: bool) {
    let u = r.width() / 24.0;
    let q = |x: f32, y: f32| pos2(r.left() + x * u, r.top() + y * u);
    let rr = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(q(x0, y0), q(x1, y1));
    let gris = |c: Color32| if actif { c } else { melange(c, Color32::from_rgb(0xB0, 0xAA, 0xA2), 0.7) };
    let fin = (u * 1.0).max(1.0);
    let boite = |p: &Painter| {
        // carton brun (paquet logiciel)
        p.add(Shape::convex_polygon(
            vec![q(2.5, 9.0), q(12.0, 5.5), q(21.5, 9.0), q(21.5, 19.0), q(12.0, 22.5), q(2.5, 19.0)],
            gris(Color32::from_rgb(0xE9, 0xB9, 0x6E)),
            Stroke::new(fin, gris(Color32::from_rgb(0x8F, 0x59, 0x02))),
        ));
        p.add(Shape::convex_polygon(
            vec![q(2.5, 9.0), q(12.0, 12.5), q(12.0, 22.5), q(2.5, 19.0)],
            gris(Color32::from_rgb(0xC1, 0x7D, 0x11)),
            Stroke::new(fin, gris(Color32::from_rgb(0x8F, 0x59, 0x02))),
        ));
        p.line_segment([q(12.0, 12.5), q(21.5, 9.0)], Stroke::new(fin, gris(Color32::from_rgb(0x8F, 0x59, 0x02))));
        p.line_segment([q(7.2, 7.3), q(16.8, 10.8)], Stroke::new(1.6 * u, gris(Color32::from_rgb(0xFC, 0xE9, 0x4F))));
    };
    let fleche_bas = |p: &Painter, cx: f32, cy: f32, k: f32, c: Color32, b: Color32| {
        let pts = vec![
            q(cx - 2.0 * k, cy - 5.0 * k),
            q(cx + 2.0 * k, cy - 5.0 * k),
            q(cx + 2.0 * k, cy),
            q(cx + 4.5 * k, cy),
            q(cx, cy + 5.0 * k),
            q(cx - 4.5 * k, cy),
            q(cx - 2.0 * k, cy),
        ];
        p.add(Shape::closed_line(pts.clone(), Stroke::new(fin * 1.6, b)));
        // remplissage en deux morceaux convexes (tige + pointe)
        p.add(Shape::convex_polygon(vec![pts[0], pts[1], pts[2], pts[6]], c, Stroke::NONE));
        p.add(Shape::convex_polygon(vec![pts[3], pts[4], pts[5]], c, Stroke::NONE));
    };
    let pastille = |p: &Painter, cx: f32, cy: f32, ray: f32, c1: Color32, c2: Color32| {
        p.circle_filled(q(cx, cy), ray * u, gris(c2));
        p.circle_filled(q(cx - ray * 0.12, cy - ray * 0.12), ray * 0.86 * u, gris(c1));
        p.circle_filled(q(cx - ray * 0.3, cy - ray * 0.38), ray * 0.42 * u, gris(melange(c1, BLANC, 0.45)));
        p.circle_stroke(q(cx, cy), ray * u, Stroke::new(fin, gris(melange(c2, Color32::BLACK, 0.25))));
    };
    match ic {
        Icone::Appli | Icone::Paquet => {
            boite(p);
            if ic == Icone::Appli {
                pastille(p, 17.0, 7.0, 6.3, Color32::from_rgb(0xFC, 0xAF, 0x3E), Color32::from_rgb(0xCE, 0x5C, 0x00));
                fleche_bas(p, 17.0, 6.6, 0.82, BLANC, Color32::from_rgb(0xA4, 0x45, 0x00));
            }
        }
        Icone::Installer => {
            p.add(Shape::convex_polygon(
                vec![q(3.0, 15.0), q(21.0, 15.0), q(21.0, 21.0), q(3.0, 21.0)],
                gris(Color32::from_rgb(0xD3, 0xD7, 0xCF)),
                Stroke::new(fin, gris(Color32::from_rgb(0x55, 0x57, 0x53))),
            ));
            p.line_segment([q(5.0, 18.0), q(19.0, 18.0)], Stroke::new(fin, gris(Color32::from_rgb(0x88, 0x8A, 0x85))));
            fleche_bas(p, 12.0, 9.0, 1.15, gris(Color32::from_rgb(0x8A, 0xE2, 0x34)), gris(Color32::from_rgb(0x4E, 0x9A, 0x06)));
        }
        Icone::Verifier | Icone::Redemarrer => {
            let (c, b) = if ic == Icone::Verifier {
                (Color32::from_rgb(0x72, 0x9F, 0xCF), Color32::from_rgb(0x20, 0x4A, 0x87))
            } else {
                (Color32::from_rgb(0xFC, 0xAF, 0x3E), Color32::from_rgb(0xCE, 0x5C, 0x00))
            };
            let centre = q(12.0, 12.0);
            let arc = |a0: f32, a1: f32| -> Vec<Pos2> {
                (0..=16)
                    .map(|i| {
                        let a = a0 + (a1 - a0) * i as f32 / 16.0;
                        centre + vec2(a.cos(), a.sin()) * 7.5 * u
                    })
                    .collect()
            };
            for (a0, a1) in [(3.5f32, 5.9f32), (0.35, 2.75)] {
                let pts = arc(a0, a1);
                p.add(Shape::line(pts.clone(), Stroke::new(3.6 * u, gris(b))));
                p.add(Shape::line(pts.clone(), Stroke::new(2.0 * u, gris(c))));
                let fin_pt = *pts.last().unwrap();
                let dir = (fin_pt - pts[pts.len() - 2]).normalized();
                let n = vec2(-dir.y, dir.x);
                p.add(Shape::convex_polygon(
                    vec![fin_pt + dir * 3.8 * u, fin_pt + n * 3.6 * u, fin_pt - n * 3.6 * u],
                    gris(c),
                    Stroke::new(fin, gris(b)),
                ));
            }
        }
        Icone::Parametres => {
            let c = q(12.0, 12.0);
            let teinte = gris(Color32::from_rgb(0x88, 0x8A, 0x85));
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::TAU / 8.0;
                let d = vec2(a.cos(), a.sin());
                p.line_segment([c + d * 5.0 * u, c + d * 9.6 * u], Stroke::new(3.4 * u, teinte));
            }
            p.circle_filled(c, 7.4 * u, gris(Color32::from_rgb(0xBA, 0xBD, 0xB6)));
            p.circle_stroke(c, 7.4 * u, Stroke::new(fin, gris(Color32::from_rgb(0x55, 0x57, 0x53))));
            p.circle_filled(c, 3.0 * u, gris(FOND));
            p.circle_stroke(c, 3.0 * u, Stroke::new(fin, gris(Color32::from_rgb(0x55, 0x57, 0x53))));
        }
        Icone::Fermer | Icone::Erreur => {
            pastille(p, 12.0, 12.0, 10.0, Color32::from_rgb(0xEF, 0x29, 0x29), Color32::from_rgb(0xA4, 0x00, 0x00));
            let s = Stroke::new(2.6 * u, BLANC);
            p.line_segment([q(8.0, 8.0), q(16.0, 16.0)], s);
            p.line_segment([q(16.0, 8.0), q(8.0, 16.0)], s);
        }
        Icone::AJour => {
            pastille(p, 12.0, 12.0, 10.0, Color32::from_rgb(0x8A, 0xE2, 0x34), Color32::from_rgb(0x4E, 0x9A, 0x06));
            p.add(Shape::line(vec![q(7.0, 12.5), q(10.5, 16.0), q(17.0, 8.0)], Stroke::new(2.8 * u, BLANC)));
        }
        Icone::Info => {
            pastille(p, 12.0, 12.0, 10.0, Color32::from_rgb(0x72, 0x9F, 0xCF), Color32::from_rgb(0x20, 0x4A, 0x87));
            p.circle_filled(q(12.0, 7.3), 1.5 * u, BLANC);
            p.line_segment([q(12.0, 10.5), q(12.0, 17.5)], Stroke::new(2.8 * u, BLANC));
        }
        Icone::Attention => {
            let pts = vec![q(12.0, 2.0), q(22.5, 21.0), q(1.5, 21.0)];
            p.add(Shape::convex_polygon(pts, gris(Color32::from_rgb(0xFC, 0xE9, 0x4F)), Stroke::new(1.4 * u, gris(Color32::from_rgb(0xC4, 0xA0, 0x00)))));
            p.line_segment([q(12.0, 8.0), q(12.0, 14.5)], Stroke::new(2.6 * u, Color32::from_rgb(0x2E, 0x34, 0x36)));
            p.circle_filled(q(12.0, 17.8), 1.4 * u, Color32::from_rgb(0x2E, 0x34, 0x36));
        }
        Icone::Securite => {
            let pts = vec![q(12.0, 2.0), q(20.5, 5.0), q(20.0, 12.0), q(12.0, 22.0), q(4.0, 12.0), q(3.5, 5.0)];
            p.add(Shape::convex_polygon(pts, gris(Color32::from_rgb(0xEF, 0x29, 0x29)), Stroke::new(fin, gris(Color32::from_rgb(0xA4, 0x00, 0x00)))));
            p.add(Shape::convex_polygon(
                vec![q(12.0, 4.2), q(18.6, 6.4), q(18.2, 12.0), q(12.0, 19.5)],
                gris(Color32::from_rgb(0xF5, 0x79, 0x79)),
                Stroke::NONE,
            ));
        }
        Icone::Snap => {
            p.add(Shape::convex_polygon(
                vec![q(4.0, 3.0), q(20.0, 3.0), q(20.0, 21.0), q(4.0, 21.0)],
                gris(Color32::from_rgb(0xFC, 0xAF, 0x3E)),
                Stroke::new(fin, gris(Color32::from_rgb(0xCE, 0x5C, 0x00))),
            ));
            for (x, y) in [(8.0, 8.0), (16.0, 8.0), (8.0, 16.0), (16.0, 16.0)] {
                p.rect_filled(rr(x - 2.3, y - 2.3, x + 2.3, y + 2.3), 1, BLANC);
            }
        }
        Icone::Pilote | Icone::Materiel => {
            // puce électronique sur circuit vert
            let carte = if ic == Icone::Pilote { Color32::from_rgb(0x73, 0xD2, 0x16) } else { Color32::from_rgb(0x72, 0x9F, 0xCF) };
            let bord = if ic == Icone::Pilote { Color32::from_rgb(0x4E, 0x9A, 0x06) } else { Color32::from_rgb(0x20, 0x4A, 0x87) };
            p.rect_filled(rr(1.5, 3.5, 22.5, 20.5), 2, gris(carte));
            p.rect_stroke(rr(1.5, 3.5, 22.5, 20.5), 2, Stroke::new(fin, gris(bord)), StrokeKind::Inside);
            for k in 0..4 {
                let x = 8.0 + k as f32 * 2.7;
                p.line_segment([q(x, 4.5), q(x, 19.5)], Stroke::new(1.1 * u, gris(Color32::from_rgb(0xBA, 0xBD, 0xB6))));
                let y = 7.5 + k as f32 * 2.7;
                p.line_segment([q(3.0, y), q(21.0, y)], Stroke::new(1.1 * u, gris(Color32::from_rgb(0xBA, 0xBD, 0xB6))));
            }
            p.rect_filled(rr(6.5, 6.5, 17.5, 17.5), 1, gris(Color32::from_rgb(0x2E, 0x34, 0x36)));
            p.rect_filled(rr(7.5, 7.5, 12.0, 10.0), 0, gris(Color32::from_rgb(0x55, 0x57, 0x53)));
        }
    }
}

/// Zone réservée à une icône dans la mise en page.
pub fn icone(ui: &mut Ui, ic: Icone, taille: f32) -> Response {
    let (r, rep) = ui.allocate_exact_size(Vec2::splat(taille), Sense::hover());
    dessiner_icone(ui.painter(), ic, r, true);
    rep
}

/// Essais : GMAJ_CAPTURE=fichier.ppm enregistre la fenêtre après GMAJ_CAPTURE_T secondes puis quitte.
pub fn capture_essai(ctx: &Context) {
    let Some(chemin) = std::env::var_os("GMAJ_CAPTURE") else { return };
    let t: f64 = std::env::var("GMAJ_CAPTURE_T").ok().and_then(|v| v.parse().ok()).unwrap_or(6.0);
    let id = Id::unique("capture_faite");
    if ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(false) {
        return;
    }
    ctx.request_repaint();
    if ctx.input(|i| i.time) < t {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(id, true));
    ctx.request_screenshot(move |img| {
        let [w, h] = img.size;
        let mut o = format!("P6\n{w} {h}\n255\n").into_bytes();
        for p in &img.pixels {
            o.extend_from_slice(&[p.r(), p.g(), p.b()]);
        }
        let _ = std::fs::write(&chemin, o);
        std::process::exit(0);
    });
}
