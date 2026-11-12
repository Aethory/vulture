//! Neo-brutalist theme: paper background, ink borders, hard offset shadows,
//! bold type. Everything visual routes through here so the app re-themes in one place.

use egui::{Color32, CornerRadius, FontId, Margin, Stroke, Visuals};

// Ink & paper
pub const INK: Color32 = Color32::from_rgb(26, 22, 18);
pub const PAPER: Color32 = Color32::from_rgb(243, 237, 226);
pub const CARD: Color32 = Color32::from_rgb(255, 251, 242);
pub const CARD_HI: Color32 = Color32::from_rgb(247, 241, 230);
pub const MUTED: Color32 = Color32::from_rgb(110, 101, 88);
pub const SIDEBAR: Color32 = Color32::from_rgb(26, 22, 18);
pub const SIDEBAR_TEXT: Color32 = Color32::from_rgb(243, 237, 226);
pub const SIDEBAR_MUTED: Color32 = Color32::from_rgb(168, 158, 144);

// Accent palette
pub const AMBER: Color32 = Color32::from_rgb(255, 176, 0);
pub const TEAL: Color32 = Color32::from_rgb(15, 139, 141);
pub const VIOLET: Color32 = Color32::from_rgb(123, 44, 191);
pub const RED: Color32 = Color32::from_rgb(228, 87, 46);
pub const GREEN: Color32 = Color32::from_rgb(42, 157, 143);

/// Resolve the persisted accent choice to a colour.
pub fn accent(name: &str) -> Color32 {
    match name {
        "teal" => TEAL,
        "violet" => VIOLET,
        _ => AMBER,
    }
}

pub fn apply(ctx: &egui::Context, accent: Color32) {
    let mut visuals = Visuals::light();
    visuals.panel_fill = PAPER;
    visuals.window_fill = CARD;
    visuals.extreme_bg_color = Color32::from_rgb(250, 248, 240);
    visuals.faint_bg_color = CARD_HI;
    visuals.widgets.noninteractive.bg_fill = CARD;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.5, INK);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(2.0, INK);
    visuals.widgets.inactive.bg_fill = CARD_HI;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.5, INK);
    visuals.widgets.inactive.bg_stroke = Stroke::new(2.0, INK);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(255, 244, 214);
    visuals.widgets.hovered.fg_stroke = Stroke::new(2.0, INK);
    visuals.widgets.hovered.bg_stroke = Stroke::new(2.0, INK);
    visuals.widgets.active.bg_fill = accent;
    visuals.widgets.active.fg_stroke = Stroke::new(2.0, INK);
    visuals.widgets.active.bg_stroke = Stroke::new(2.0, INK);
    visuals.selection.bg_fill = accent.linear_multiply(0.45);
    visuals.selection.stroke = Stroke::new(2.0, INK);
    visuals.override_text_color = Some(INK);
    ctx.set_visuals(visuals);
}

/// The signature brutalist card: ink-bordered cream panel with a hard
/// offset shadow (a solid black slab peeking out bottom-right).
pub fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(INK)
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin { left: 0, top: 0, right: 6, bottom: 6 })
        .show(ui, |ui| {
            egui::Frame::new()
                .fill(CARD)
                .stroke(Stroke::new(2.0, INK))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::same(16))
                .show(ui, add);
        });
}

pub fn heading(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .font(FontId::proportional(30.0))
            .strong()
            .color(INK),
    );
}

pub fn subtitle(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).font(FontId::proportional(13.0)).color(MUTED));
}

pub fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .font(FontId::proportional(11.0))
            .strong()
            .color(MUTED),
    );
}

pub fn primary_button(ui: &mut egui::Ui, text: &str, accent: Color32) -> egui::Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(text.to_uppercase())
                .strong()
                .font(FontId::proportional(15.0))
                .color(INK),
        )
        .fill(accent)
        .stroke(Stroke::new(2.5, INK))
        .corner_radius(CornerRadius::same(6))
        .min_size(egui::vec2(0.0, 42.0)),
    )
}

pub fn danger_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(text.to_uppercase())
                .strong()
                .font(FontId::proportional(13.0))
                .color(Color32::from_rgb(255, 251, 242)),
        )
        .fill(RED)
        .stroke(Stroke::new(2.5, INK))
        .corner_radius(CornerRadius::same(6)),
    )
}

pub fn ghost_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(text.to_uppercase()).strong().color(INK))
            .fill(CARD_HI)
            .stroke(Stroke::new(2.0, INK))
            .corner_radius(CornerRadius::same(6)),
    )
}

pub fn stat_tile(ui: &mut egui::Ui, label: &str, value: &str, sub: &str, value_color: Color32) {
    card(ui, |ui| {
        ui.set_min_size(egui::vec2(158.0, 92.0));
        ui.label(
            egui::RichText::new(label.to_uppercase())
                .font(FontId::proportional(10.0))
                .strong()
                .color(MUTED),
        );
        ui.label(egui::RichText::new(value).font(FontId::proportional(24.0)).strong().color(value_color));
        ui.label(egui::RichText::new(sub).font(FontId::proportional(11.0)).color(MUTED));
    });
}

pub fn success_banner(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgb(226, 245, 234))
        .stroke(Stroke::new(2.0, GREEN))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(format!("DONE — {text}")).strong().color(Color32::from_rgb(23, 71, 46)));
        });
}

pub fn error_banner(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgb(252, 228, 220))
        .stroke(Stroke::new(2.0, RED))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(format!("ERROR — {text}")).strong().color(Color32::from_rgb(95, 31, 15)));
        });
}

pub fn info_banner(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgb(236, 230, 216))
        .stroke(Stroke::new(2.0, INK))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(format!("NOTE — {text}")).color(INK));
        });
}

/// Small bordered tag, e.g. for "ADMIN" or counts.
pub fn tag(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(INK)
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text.to_uppercase()).font(FontId::proportional(9.0)).strong().color(PAPER));
        });
}
