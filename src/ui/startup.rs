//! Startup Manager tab — registry Run keys + the Startup folder.

use crate::core::startup::{self, StartupEntry, StartupLocation};
use crate::theme;

pub struct StartupTab {
    entries: Option<Vec<StartupEntry>>,
    error: Option<String>,
    note: Option<String>,
}

impl Default for StartupTab {
    fn default() -> Self {
        Self {
            entries: None,
            error: None,
            note: None,
        }
    }
}

impl StartupTab {
    pub fn show(&mut self, ui: &mut egui::Ui, accent: egui::Color32) {
        if self.entries.is_none() {
            self.entries = Some(startup::list());
        }

        theme::heading(ui, "Startup Manager");
        theme::subtitle(ui, "Everything that runs when you sign in. Disabling is fully reversible — Vulture keeps backups.");
        ui.add_space(14.0);

        if let Some(err) = &self.error {
            theme::error_banner(ui, err);
            ui.add_space(8.0);
        }
        if let Some(note) = &self.note {
            theme::success_banner(ui, note);
            ui.add_space(8.0);
        }

        let entries = self.entries.as_ref().unwrap();
        let enabled = entries.iter().filter(|e| e.enabled).count();
        let disabled = entries.len() - enabled;

        ui.horizontal(|ui| {
            theme::tag(ui, &format!("{enabled} enabled"));
            theme::tag(ui, &format!("{disabled} disabled"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::ghost_button(ui, "↻ Refresh").clicked() {
                    self.entries = Some(startup::list());
                }
            });
        });
        ui.add_space(10.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            let entries = self.entries.as_mut().unwrap();
            if entries.is_empty() {
                theme::info_banner(ui, "No startup entries found. Your boot is already lean.");
                return;
            }
            let mut toggle: Option<(usize, bool)> = None;
            for (i, entry) in entries.iter().enumerate() {
                theme::card(ui, |ui| {
                    ui.horizontal(|ui| {
                        let mut on = entry.enabled;
                        if toggle_switch(ui, &mut on, accent).clicked() {
                            toggle = Some((i, on));
                        }
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&entry.name).strong().color(theme::INK));
                            ui.label(
                                egui::RichText::new(location_badge(&entry.location))
                                    .small()
                                    .color(theme::MUTED),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(truncate(&entry.command, 72))
                                    .font(egui::FontId::monospace(10.5))
                                    .color(theme::MUTED),
                            );
                        });
                    });
                });
                ui.add_space(8.0);
            }
            if let Some((i, enable)) = toggle {
                let entry = entries[i].clone();
                match startup::set_enabled(&entry, enable) {
                    Ok(()) => {
                        self.note = Some(if enable {
                            format!("'{}' will run at next sign-in.", entry.name)
                        } else {
                            format!("'{}' disabled — backed up, never lost.", entry.name)
                        });
                        self.error = None;
                        self.entries = Some(startup::list());
                    }
                    Err(e) => {
                        self.error = Some(e);
                        self.note = None;
                    }
                }
            }
        });
    }
}

fn location_badge(loc: &StartupLocation) -> String {
    loc.label().to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        format!("{}…", s.chars().take(max - 1).collect::<String>())
    } else {
        s.to_string()
    }
}

/// Chunky ON/OFF pill — very neo-brutalism.
pub fn toggle_switch(ui: &mut egui::Ui, on: &mut bool, accent: egui::Color32) -> egui::Response {
    let text = if *on { "ON" } else { "OFF" };
    let fill = if *on { accent } else { theme::CARD_HI };
    let response = ui.add(
        egui::Button::new(
            egui::RichText::new(text).font(egui::FontId::proportional(12.0)).strong().color(theme::INK),
        )
        .fill(fill)
        .stroke(egui::Stroke::new(2.0, theme::INK))
        .corner_radius(egui::CornerRadius::same(16))
        .min_size(egui::vec2(58.0, 28.0)),
    );
    if response.clicked() {
        *on = !*on;
    }
    response
}
