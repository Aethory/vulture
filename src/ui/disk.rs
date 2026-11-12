//! Disk Analyzer tab — find out where the space went.

use crate::core::disk::{self, DiskReport};
use crate::theme;
use crate::ui::memory::brutal_bar;
use crate::ui::fmt_bytes;
use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicBool, mpsc};
use std::thread;

pub struct DiskTab {
    root: PathBuf,
    analyzing: bool,
    cancel: Option<Arc<AtomicBool>>,
    rx: Option<mpsc::Receiver<DiskReport>>,
    report: Option<DiskReport>,
}

impl Default for DiskTab {
    fn default() -> Self {
        Self {
            root: dirs::home_dir().unwrap_or_else(|| PathBuf::from("C:\\")),
            analyzing: false,
            cancel: None,
            rx: None,
            report: None,
        }
    }
}

impl DiskTab {
    fn start(&mut self) {
        self.analyzing = true;
        self.report = None;
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel = Some(cancel.clone());
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        let root = self.root.clone();
        thread::spawn(move || {
            let _ = tx.send(disk::analyze(&root, cancel));
        });
    }

    pub fn show(&mut self, ui: &mut egui::Ui, accent: egui::Color32) {
        // Poll worker.
        if let Some(rx) = &self.rx {
            if let Ok(report) = rx.try_recv() {
                self.report = Some(report);
                self.analyzing = false;
                self.rx = None;
                self.cancel = None;
            }
        }
        if self.analyzing {
            ui.ctx().request_repaint();
        }

        theme::heading(ui, "Disk Analyzer");
        theme::subtitle(ui, "Every gigabyte, accounted for. Pick a folder — results are instant-ish.");
        ui.add_space(14.0);

        // Controls
        theme::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("TARGET").small().strong().color(theme::MUTED));
                ui.label(
                    egui::RichText::new(self.root.display().to_string())
                        .font(egui::FontId::monospace(13.0))
                        .strong()
                        .color(theme::INK),
                );
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if theme::ghost_button(ui, "Change folder…").clicked() && !self.analyzing {
                    if let Some(dir) = rfd::FileDialog::new().set_directory(&self.root).pick_folder() {
                        self.root = dir;
                        self.report = None;
                    }
                }
                if self.analyzing {
                    if theme::danger_button(ui, "✕ Cancel").clicked() {
                        if let Some(cancel) = &self.cancel {
                            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                    ui.spinner();
                    ui.label(egui::RichText::new("Measuring…").strong().color(theme::INK));
                } else if theme::primary_button(ui, "Analyze", accent).clicked() {
                    self.start();
                }
            });
        });
        ui.add_space(12.0);

        let Some(report) = &self.report else {
            if !self.analyzing {
                theme::info_banner(ui, "Tip: analysing your whole user profile (C:\\Users\\you) shows everything you own. It takes a minute.");
            }
            return;
        };

        if report.cancelled {
            theme::info_banner(ui, "Analysis cancelled — partial results below.");
            ui.add_space(8.0);
        }

        ui.columns(2, |cols| {
            // Left: folder breakdown
            theme::card(&mut cols[0], |ui| {
                theme::section_label(ui, "Folder breakdown");
                ui.add_space(8.0);
                let max = report.folders.iter().map(|f| f.size).max().unwrap_or(1).max(1);
                egui::ScrollArea::vertical().max_height(ui.available_height()).show(ui, |ui| {
                    for folder in report.folders.iter().take(14) {
                        let name = folder
                            .path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| folder.path.display().to_string());
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(truncate(&name, 30)).strong().color(theme::INK));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    egui::RichText::new(fmt_bytes(folder.size))
                                        .font(egui::FontId::monospace(12.0))
                                        .strong()
                                        .color(theme::INK),
                                );
                            });
                        });
                        brutal_bar(ui, folder.size as f32 / max as f32, accent);
                        ui.add_space(6.0);
                    }
                    if report.folders.len() > 14 {
                        ui.label(
                            egui::RichText::new(format!("+ {} more folders…", report.folders.len() - 14))
                                .small()
                                .color(theme::MUTED),
                        );
                    }
                });
            });

            // Right: largest files
            theme::card(&mut cols[1], |ui| {
                theme::section_label(ui, "Your 40 biggest files");
                ui.add_space(8.0);
                egui::ScrollArea::vertical().max_height(ui.available_height()).show(ui, |ui| {
                    egui::Grid::new("largest-files").num_columns(3).spacing([10.0, 5.0]).striped(true).show(ui, |ui| {
                        for file in report.largest_files.iter().take(18) {
                            let name = file
                                .path
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(truncate(&name, 34))
                                        .font(egui::FontId::monospace(11.0))
                                        .strong()
                                        .color(theme::INK),
                                );
                                ui.label(
                                    egui::RichText::new(file.path.display().to_string())
                                        .font(egui::FontId::proportional(9.0))
                                        .color(theme::MUTED),
                                );
                            });
                            ui.label(
                                egui::RichText::new(fmt_bytes(file.size))
                                    .font(egui::FontId::monospace(11.0))
                                    .strong()
                                    .color(theme::INK),
                            );
                            if theme::ghost_button(ui, "Open").clicked() {
                                let _ = std::process::Command::new("explorer")
                                    .arg("/select,")
                                    .arg(&file.path)
                                    .spawn();
                            }
                            ui.end_row();
                        }
                    });
                });
            });
        });
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        format!("{}…", s.chars().take(max - 1).collect::<String>())
    } else {
        s.to_string()
    }
}
