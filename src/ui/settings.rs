//! Settings tab — appearance, data folder, stats, about.

pub struct SettingsTab {
    confirm_reset: bool,
    note: Option<String>,
}

impl Default for SettingsTab {
    fn default() -> Self {
        Self {
            confirm_reset: false,
            note: None,
        }
    }
}

const ACCENTS: &[(&str, &str)] = &[("amber", "Amber"), ("teal", "Teal"), ("violet", "Violet")];

impl SettingsTab {
    pub fn show(&mut self, ui: &mut egui::Ui, config: &mut crate::config::Config) {
        use crate::theme;

        theme::heading(ui, "Settings");
        theme::subtitle(ui, "Make Vulture yours. Nothing here phones home — there is no home.");
        ui.add_space(14.0);

        ui.columns(2, |cols| {
            // Left column — appearance + data
            theme::card(&mut cols[0], |ui| {
                theme::section_label(ui, "Accent");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    for (key, label) in ACCENTS {
                        let swatch = theme::accent(key);
                        let selected = config.accent == *key;
                        egui::Frame::new()
                            .fill(swatch)
                            .stroke(egui::Stroke::new(if selected { 3.0 } else { 2.0 }, theme::INK))
                            .corner_radius(egui::CornerRadius::same(6))
                            .inner_margin(egui::Margin::symmetric(18, 14))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(label.to_uppercase())
                                        .strong()
                                        .color(theme::INK),
                                );
                            })
                            .response
                            .interact(egui::Sense::click())
                            .clicked()
                            .then(|| {
                                config.accent = key.to_string();
                                config.save();
                            });
                    }
                });
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Used for buttons, highlights and the V.")
                        .small()
                        .color(theme::MUTED),
                );

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(10.0);

                theme::section_label(ui, "Your data");
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(crate::config::data_dir().display().to_string())
                        .font(egui::FontId::monospace(11.0))
                        .color(theme::INK),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if theme::ghost_button(ui, "Open data folder").clicked() {
                        let _ = std::process::Command::new("explorer")
                            .arg(crate::config::data_dir())
                            .spawn();
                    }
                    if !self.confirm_reset {
                        if theme::ghost_button(ui, "Reset stats").clicked() {
                            self.confirm_reset = true;
                        }
                    } else {
                        if theme::danger_button(ui, "Confirm reset").clicked() {
                            config.reset_stats();
                            self.confirm_reset = false;
                            self.note = Some("Lifetime stats wiped. Fresh start.".to_string());
                        }
                        if theme::ghost_button(ui, "Keep").clicked() {
                            self.confirm_reset = false;
                        }
                    }
                });
                if let Some(note) = &self.note {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(note).small().color(theme::GREEN));
                }
            });

            // Right column — about
            theme::card(&mut cols[1], |ui| {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("VULTURE")
                        .font(egui::FontId::proportional(46.0))
                        .strong()
                        .color(theme::INK),
                );
                ui.label(
                    egui::RichText::new("PC CARE ENGINE")
                        .font(egui::FontId::proportional(13.0))
                        .strong()
                        .color(theme::MUTED),
                );
                ui.add_space(10.0);
                ui.label(egui::RichText::new("v1.0.0 · MIT License").strong().color(theme::INK));
                ui.label(
                    egui::RichText::new("Built 100% in Rust. No telemetry, no accounts, no network calls, no installer upsells.")
                        .small()
                        .color(theme::MUTED),
                );
                ui.add_space(10.0);
                ui.hyperlink_to("github.com/aethory/vulture", "https://github.com/aethory/vulture");
                ui.hyperlink_to("Report a bug", "https://github.com/aethory/vulture/issues");
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                theme::section_label(ui, "Guarantees");
                ui.add_space(6.0);
                for line in [
                    "Files are moved to the Recycle Bin — never destroyed",
                    "Startup entries are backed up before any change",
                    "Critical Windows processes are never touched",
                    "Free forever — MIT licensed, always",
                ] {
                    ui.label(egui::RichText::new(format!("•  {line}")).small().color(theme::INK));
                }
            });
        });
    }
}
