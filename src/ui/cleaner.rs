//! Smart Clean tab — full category-by-category control.

use crate::config::Config;
use crate::core::scanner::{self, CategoryId, CategoryScan};
use crate::theme;
use crate::ui::fmt_bytes;
use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver};
use std::thread;

enum CleanerMsg {
    ScanDone(Vec<CategoryScan>),
    CleanDone(scanner::CleanReport),
}

pub struct CleanerTab {
    started: bool,
    busy: bool,
    phase: &'static str,
    rx: Option<Receiver<CleanerMsg>>,
    scans: Option<Vec<CategoryScan>>,
    checked: HashSet<CategoryId>,
    empty_bin: bool,
    report: Option<scanner::CleanReport>,
}

impl Default for CleanerTab {
    fn default() -> Self {
        Self {
            started: false,
            busy: false,
            phase: "Scanning",
            rx: None,
            scans: None,
            checked: HashSet::new(),
            empty_bin: false,
            report: None,
        }
    }
}

impl CleanerTab {
    fn start_scan(&mut self) {
        self.busy = true;
        self.phase = "Scanning";
        self.scans = None;
        self.report = None;
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        thread::spawn(move || {
            let _ = tx.send(CleanerMsg::ScanDone(scanner::scan_all()));
        });
    }

    fn start_clean(&mut self) {
        // Hand the worker its own copy of the scan.
        let snapshot = self.scans.clone().unwrap_or_default();
        let selected: Vec<CategoryId> = snapshot
            .iter()
            .filter(|s| self.checked.contains(&s.id))
            .map(|s| s.id)
            .collect();
        let empty_bin = self.empty_bin;
        self.busy = true;
        self.phase = "Cleaning";
        self.report = None;
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        thread::spawn(move || {
            let mut report = scanner::clean(&snapshot, &selected);
            if empty_bin {
                if let Err(e) = scanner::empty_recycle_bin() {
                    report.failures.push(e);
                }
            }
            let _ = tx.send(CleanerMsg::CleanDone(report));
        });
    }

    pub fn show(&mut self, ui: &mut egui::Ui, config: &mut Config, accent: egui::Color32) {
        if !self.started {
            self.started = true;
            self.start_scan();
        }

        // Poll worker messages.
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(CleanerMsg::ScanDone(scans)) => {
                    self.checked = scans.iter().map(|s| s.id).collect();
                    self.scans = Some(scans);
                    self.busy = false;
                    self.rx = None;
                }
                Ok(CleanerMsg::CleanDone(report)) => {
                    config.record_clean(report.bytes_freed, report.files_removed);
                    self.report = Some(report);
                    self.busy = false;
                    self.rx = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.busy = false;
                    self.rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.busy {
            ui.ctx().request_repaint();
        }

        theme::heading(ui, "Smart Clean");
        theme::subtitle(
            ui,
            "Hand-pick the categories — everything is recoverable from the Recycle Bin.",
        );
        ui.add_space(14.0);

        if let Some(report) = &self.report {
            theme::success_banner(
                ui,
                &format!(
                    "Freed {} across {} files.",
                    fmt_bytes(report.bytes_freed),
                    report.files_removed
                ),
            );
            if !report.failures.is_empty() {
                ui.add_space(6.0);
                egui::CollapsingHeader::new(format!("{} items were locked", report.failures.len()))
                    .id_salt("cleaner-failures")
                    .show(ui, |ui| {
                        for f in &report.failures {
                            ui.label(egui::RichText::new(f).small().color(theme::MUTED));
                        }
                    });
            }
            ui.add_space(8.0);
            if theme::ghost_button(ui, "↻ Scan again").clicked() {
                self.start_scan();
            }
            ui.add_space(12.0);
        }

        let available = ui.available_width();
        let left_w = available * 0.58;
        ui.horizontal(|ui| {
            ui.allocate_ui(egui::vec2(left_w, ui.available_height()), |ui| {
                theme::card(ui, |ui| {
                    ui.horizontal(|ui| {
                        theme::section_label(ui, "Categories");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("NONE").clicked() {
                                self.checked.clear();
                            }
                            if ui.small_button("ALL").clicked() {
                                if let Some(scans) = &self.scans {
                                    self.checked = scans.iter().map(|s| s.id).collect();
                                }
                            }
                        });
                    });
                    ui.add_space(8.0);

                    if self.scans.is_none() {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(egui::RichText::new("Scanning your drive…").strong().color(theme::INK));
                        });
                        return;
                    }

                    egui::ScrollArea::vertical().max_height(ui.available_height() - 40.0).show(ui, |ui| {
                        let scans = self.scans.as_mut().unwrap();
                        for scan in scans.iter_mut() {
                            let mut checked = self.checked.contains(&scan.id);
                            let response = ui.checkbox(
                                &mut checked,
                                egui::RichText::new(&scan.label).strong().color(theme::INK),
                            );
                            if response.changed() {
                                if checked {
                                    self.checked.insert(scan.id);
                                } else {
                                    self.checked.remove(&scan.id);
                                }
                            }
                            ui.horizontal(|ui| {
                                ui.add_space(26.0);
                                ui.label(egui::RichText::new(&scan.description).small().color(theme::MUTED));
                            });
                            ui.horizontal(|ui| {
                                ui.add_space(26.0);
                                ui.label(
                                    egui::RichText::new(fmt_bytes(scan.bytes))
                                        .font(egui::FontId::proportional(13.0))
                                        .strong()
                                        .color(theme::INK),
                                );
                                ui.label(
                                    egui::RichText::new(format!("· {} items", scan.items.len()))
                                        .small()
                                        .color(theme::MUTED),
                                );
                                if scan.needs_admin {
                                    theme::tag(ui, "admin");
                                }
                            });
                            if let Some(err) = &scan.error {
                                ui.horizontal(|ui| {
                                    ui.add_space(26.0);
                                    ui.label(egui::RichText::new(format!("⚠ {err}")).small().color(theme::RED));
                                });
                            }
                            ui.add_space(6.0);
                        }
                    });
                });
            });

            ui.add_space(6.0);

            ui.allocate_ui(egui::vec2(ui.available_width(), ui.available_height()), |ui| {
                // Plan card
                theme::card(ui, |ui| {
                    theme::section_label(ui, "Cleaning plan");
                    ui.add_space(6.0);

                    let (items, bytes) = self
                        .scans
                        .as_ref()
                        .map(|scans| {
                            scans
                                .iter()
                                .filter(|s| self.checked.contains(&s.id))
                                .fold((0u64, 0u64), |(n, b), s| (n + s.items.len() as u64, b + s.bytes))
                        })
                        .unwrap_or((0, 0));

                    ui.label(
                        egui::RichText::new(fmt_bytes(bytes))
                            .font(egui::FontId::proportional(34.0))
                            .strong()
                            .color(theme::INK),
                    );
                    ui.label(
                        egui::RichText::new(format!("across {items} files · {} categories", self.checked.len()))
                            .color(theme::MUTED),
                    );
                    ui.add_space(12.0);

                    if self.busy {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(egui::RichText::new(format!("{}…", self.phase)).strong().color(theme::INK));
                        });
                    } else if items > 0 {
                        if theme::primary_button(ui, "Clean selected", accent).clicked() {
                            self.start_clean();
                        }
                    } else {
                        ui.label(egui::RichText::new("Select at least one category.").color(theme::MUTED));
                    }

                    ui.add_space(8.0);
                    ui.checkbox(
                        &mut self.empty_bin,
                        egui::RichText::new("Also empty the Recycle Bin").color(theme::MUTED),
                    );
                });

                ui.add_space(12.0);

                theme::card(ui, |ui| {
                    theme::section_label(ui, "Field manual");
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(
                            "• Vulture never permanently deletes — files land in the Recycle Bin.\n• Close your browsers first; their caches are locked while open.\n• Categories marked ADMIN need Vulture to run as administrator.",
                        )
                        .small()
                        .color(theme::MUTED),
                    );
                });
            });
        });
    }
}
