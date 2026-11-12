//! Auto Clean tab — Windows Task Scheduler integration + run logs.

use crate::core::schedule;
use crate::theme;
use crate::ui::startup::toggle_switch;
use std::path::PathBuf;

pub struct SchedulerTab {
    enabled: Option<bool>,
    message: Option<(bool, String)>,
    logs: Vec<PathBuf>,
    open_log: Option<(String, String)>,
}

impl Default for SchedulerTab {
    fn default() -> Self {
        Self {
            enabled: None,
            message: None,
            logs: Vec::new(),
            open_log: None,
        }
    }
}

impl SchedulerTab {
    fn refresh(&mut self) {
        self.enabled = Some(schedule::is_enabled());
        self.logs = crate::config::logs_dir()
            .read_dir()
            .map(|rd| {
                let mut logs: Vec<PathBuf> = rd
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| p.extension().map(|e| e == "log").unwrap_or(false))
                    .collect();
                logs.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
                logs.truncate(10);
                logs
            })
            .unwrap_or_default();
    }

    pub fn show(&mut self, ui: &mut egui::Ui, accent: egui::Color32) {
        if self.enabled.is_none() {
            self.refresh();
        }

        theme::heading(ui, "Auto Clean");
        theme::subtitle(ui, "Vulture, working while you sleep. A daily 02:30 sweep — no taps required.");
        ui.add_space(14.0);

        if let Some((ok, msg)) = &self.message {
            if *ok {
                theme::success_banner(ui, msg);
            } else {
                theme::error_banner(ui, msg);
            }
            ui.add_space(10.0);
        }

        // Status card
        theme::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    theme::section_label(ui, "Daily sweep · 02:30");
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("AUTO-CLEAN")
                            .font(egui::FontId::proportional(40.0))
                            .strong()
                            .color(theme::INK),
                    );
                    ui.label(
                        egui::RichText::new("Windows Task Scheduler · task name “Vulture Auto Clean”")
                            .small()
                            .color(theme::MUTED),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let enabled = self.enabled.unwrap_or(false);
                    let mut on = enabled;
                    if toggle_switch(ui, &mut on, accent, enabled).clicked() {
                        let result = if on { schedule::enable() } else { schedule::disable() };
                        self.message = Some(match result {
                            Ok(()) => (
                                true,
                                if on {
                                    "Task registered — Vulture will sweep daily at 02:30.".to_string()
                                } else {
                                    "Task removed. Vulture will only clean when you ask.".to_string()
                                },
                            ),
                            Err(e) => (false, e),
                        });
                        self.refresh();
                    }
                    ui.add_space(10.0);
                    theme::tag(ui, if enabled { "Scheduled" } else { "Off" });
                });
            });
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(
                    "The nightly sweep clears: user & Windows temp · thumbnails · crash dumps · shader cache · browser caches.\nIt never touches cookies, history or anything in your Documents.",
                )
                .small()
                .color(theme::MUTED),
            );
        });
        ui.add_space(12.0);

        ui.columns(2, |cols| {
            // What gets swept
            theme::card(&mut cols[0], |ui| {
                theme::section_label(ui, "Checklist");
                ui.add_space(6.0);
                for line in [
                    "Windows & user temp files",
                    "Thumbnail & icon cache",
                    "Crash dumps (*.dmp)",
                    "DirectX / NVIDIA shader cache",
                    "Chrome · Edge · Brave · Opera · Firefox caches",
                ] {
                    ui.label(egui::RichText::new(format!("•  {line}")).color(theme::INK));
                }
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("Everything else stays exactly where it is.")
                        .small()
                        .color(theme::MUTED),
                );
            });

            // Logs
            theme::card(&mut cols[1], |ui| {
                ui.horizontal(|ui| {
                    theme::section_label(ui, "Run logs");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::ghost_button(ui, "↻ Refresh").clicked() {
                            self.refresh();
                        }
                    });
                });
                ui.add_space(6.0);
                if self.logs.is_empty() {
                    ui.label(
                        egui::RichText::new("No runs yet — the first nightly sweep will log itself here.")
                            .small()
                            .color(theme::MUTED),
                    );
                } else {
                    for log in &self.logs.clone() {
                        let name = log.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                        if ui.selectable_label(false, egui::RichText::new(&name).font(egui::FontId::monospace(10.5))).clicked() {
                            let content = std::fs::read_to_string(log).unwrap_or_else(|e| format!("Could not read log: {e}"));
                            self.open_log = Some((name.clone(), content));
                        }
                    }
                }
            });
        });

        // Open log viewer (take first so the closure can freely use the UI)
        if let Some((name, content)) = self.open_log.take() {
            let mut close = false;
            theme::card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&name).strong().color(theme::INK));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("CLOSE").clicked() {
                            close = true;
                        }
                    });
                });
                ui.add_space(6.0);
                egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                    ui.label(egui::RichText::new(content.as_str()).font(egui::FontId::monospace(11.0)).color(theme::INK));
                });
            });
            if !close {
                self.open_log = Some((name, content));
            }
        }
    }
}
