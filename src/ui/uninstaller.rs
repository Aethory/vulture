//! Uninstaller tab — everything Windows knows about, with size and publisher.

use crate::core::uninstall::{self, InstalledApp};
use crate::theme;
use crate::ui::fmt_bytes;

#[derive(Default)]
pub struct UninstallerTab {
    apps: Option<Vec<InstalledApp>>,
    query: String,
    confirm: Option<usize>,
    message: Option<(bool, String)>,
}

impl UninstallerTab {
    pub fn show(&mut self, ui: &mut egui::Ui, accent: egui::Color32) {
        if self.apps.is_none() {
            self.apps = Some(uninstall::list());
        }

        theme::heading(ui, "Uninstaller");
        theme::subtitle(ui, "Every app Windows lists, sorted by the disk it eats. Sizes come from the installer — treat as approximate.");
        ui.add_space(14.0);

        if let Some((ok, msg)) = &self.message {
            if *ok {
                theme::success_banner(ui, msg);
            } else {
                theme::error_banner(ui, msg);
            }
            ui.add_space(8.0);
        }

        // Search + actions row
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("SEARCH")
                    .small()
                    .strong()
                    .color(theme::MUTED),
            );
            ui.add_sized(
                egui::vec2(320.0, 30.0),
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text("slack, steam, adobe…")
                    .margin(egui::vec2(8.0, 6.0)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::ghost_button(ui, "Rescan").clicked() {
                    self.apps = Some(uninstall::list());
                }
                let total = self.apps.as_ref().map(|a| a.len()).unwrap_or(0);
                theme::tag(ui, &format!("{total} apps"));
            });
        });
        ui.add_space(10.0);

        // Confirm modal
        if let Some(idx) = self.confirm {
            let app = self.apps.as_ref().and_then(|a| a.get(idx)).cloned();
            if let Some(app) = app {
                egui::Window::new("Uninstall?")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ui.ctx(), |ui| {
                        ui.label(egui::RichText::new(&app.name).strong().color(theme::INK));
                        ui.label(
                            egui::RichText::new(format!("{} · {} · {}", app.publisher, app.version, fmt_bytes(app.estimated_size)))
                                .small()
                                .color(theme::MUTED),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new("Vulture will launch the app's own uninstaller — follow its prompts.")
                                .color(theme::INK),
                        );
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if theme::danger_button(ui, "Launch uninstaller").clicked() {
                                self.message = Some(match uninstall::launch_uninstaller(&app) {
                                    Ok(()) => (true, format!("Uninstaller launched for '{}'.", app.name)),
                                    Err(e) => (false, e),
                                });
                                self.confirm = None;
                            }
                            if theme::ghost_button(ui, "Keep it").clicked() {
                                self.confirm = None;
                            }
                        });
                    });
            } else {
                self.confirm = None;
            }
        }

        // App list
        let query = self.query.to_lowercase();
        let apps = self.apps.as_ref().unwrap();
        let visible: Vec<(usize, &InstalledApp)> = apps
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                query.is_empty()
                    || a.name.to_lowercase().contains(&query)
                    || a.publisher.to_lowercase().contains(&query)
            })
            .collect();

        egui::ScrollArea::vertical().show(ui, |ui| {
            if visible.is_empty() {
                theme::info_banner(ui, "Nothing matches that search.");
                return;
            }
            // Affordance: report which rows we want the user to notice.
            for (i, app) in visible {
                theme::card(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Monogram avatar
                        let initial: String = app
                            .name
                            .chars()
                            .next()
                            .unwrap_or('?')
                            .to_uppercase()
                            .collect();
                        egui::Frame::new()
                            .fill(accent)
                            .stroke(egui::Stroke::new(2.0, theme::INK))
                            .corner_radius(egui::CornerRadius::same(6))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(initial)
                                        .font(egui::FontId::proportional(18.0))
                                        .strong()
                                        .color(theme::INK),
                                );
                            });
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&app.name).strong().color(theme::INK));
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}{}",
                                    app.publisher,
                                    if app.version.is_empty() {
                                        String::new()
                                    } else {
                                        format!(" · v{}", app.version)
                                    }
                                ))
                                .small()
                                .color(theme::MUTED),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if theme::danger_button(ui, "Uninstall").clicked() {
                                self.confirm = Some(i);
                            }
                            let has_folder = app
                                .install_location
                                .as_ref()
                                .map(|p| p.exists())
                                .unwrap_or(false);
                            ui.add_enabled_ui(has_folder, |ui| {
                                if theme::ghost_button(ui, "Folder").clicked() {
                                    let _ = uninstall::open_install_folder(app);
                                }
                            });
                            ui.label(
                                egui::RichText::new(if app.estimated_size > 0 {
                                    fmt_bytes(app.estimated_size)
                                } else {
                                    "—".to_string()
                                })
                                .font(egui::FontId::monospace(13.0))
                                .strong()
                                .color(theme::INK),
                            );
                        });
                    });
                });
                ui.add_space(6.0);
            }
        });
    }
}
