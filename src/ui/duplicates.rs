//! Duplicate Finder tab — BLAKE3 content hashing across a folder you choose.

use crate::core::duplicates::{self, DupMsg, DuplicateReport};
use crate::theme;
use crate::ui::fmt_bytes;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::sync::atomic::AtomicBool;
use std::thread;

pub struct DuplicatesTab {
    root: PathBuf,
    scanning: bool,
    cancel: Arc<AtomicBool>,
    rx: Option<mpsc::Receiver<DupMsg>>,
    log: Vec<String>,
    report: Option<DuplicateReport>,
    selected: HashSet<PathBuf>,
    confirm_delete: bool,
    message: Option<(bool, String)>,
}

impl Default for DuplicatesTab {
    fn default() -> Self {
        let root = env_downloads();
        Self {
            root,
            scanning: false,
            cancel: Arc::new(AtomicBool::new(false)),
            rx: None,
            log: Vec::new(),
            report: None,
            selected: HashSet::new(),
            confirm_delete: false,
            message: None,
        }
    }
}

impl DuplicatesTab {
    fn start(&mut self) {
        self.scanning = true;
        self.report = None;
        self.message = None;
        self.log.clear();
        self.selected.clear();
        self.cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        let root = self.root.clone();
        let cancel = self.cancel.clone();
        thread::spawn(move || {
            duplicates::scan(&root, cancel, tx);
        });
    }

    pub fn show(&mut self, ui: &mut egui::Ui, accent: egui::Color32) {
        // Poll worker.
        if let Some(rx) = &self.rx {
            loop {
                match rx.try_recv() {
                    Ok(DupMsg::Progress(line)) => {
                        self.log.push(line);
                        if self.log.len() > 7 {
                            self.log.remove(0);
                        }
                    }
                    Ok(DupMsg::Done(report)) => {
                        // Default selection: keep the OLDEST file in each group, mark the rest.
                        self.selected = report
                            .groups
                            .iter()
                            .flat_map(|g| g.paths.iter().skip(1).cloned())
                            .collect();
                        self.report = Some(report);
                        self.scanning = false;
                        self.rx = None;
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) | Err(mpsc::TryRecvError::Disconnected) => break,
                }
            }
        }
        if self.scanning {
            ui.ctx().request_repaint();
        }

        theme::heading(ui, "Duplicate Finder");
        theme::subtitle(ui, "Content hashing finds bit-for-bit twins — names don't matter.");
        ui.add_space(14.0);

        // Delete confirmation modal.
        if self.confirm_delete {
            let bytes: u64 = self
                .report
                .as_ref()
                .map(|r| {
                    r.groups
                        .iter()
                        .flat_map(|g| g.paths.iter())
                        .filter(|p| self.selected.contains(*p))
                        .filter_map(|p| p.metadata().ok().map(|m| m.len()))
                        .sum()
                })
                .unwrap_or(0);
            egui::Window::new("REALLY delete?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} files ({}) will be moved to the Recycle Bin. You can restore them from there.",
                            self.selected.len(),
                            fmt_bytes(bytes)
                        ))
                        .color(theme::INK),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if theme::danger_button(ui, "Delete duplicates").clicked() {
                            let paths: Vec<PathBuf> = self.selected.iter().cloned().collect();
                            let result = duplicates::delete(&paths);
                            let ok = result.failures.is_empty();
                            self.message = Some((
                                ok,
                                format!(
                                    "Moved {} files to the Recycle Bin — freed {}.{}",
                                    result.files_removed,
                                    fmt_bytes(result.bytes_freed),
                                    if result.failures.is_empty() {
                                        String::new()
                                    } else {
                                        format!(" {} locked.", result.failures.len())
                                    }
                                ),
                            ));
                            self.selected.clear();
                            self.report = None;
                            self.confirm_delete = false;
                        }
                        if theme::ghost_button(ui, "Keep everything").clicked() {
                            self.confirm_delete = false;
                        }
                    });
                });
        }

        if let Some((ok, msg)) = &self.message {
            if *ok {
                theme::success_banner(ui, msg);
            } else {
                theme::error_banner(ui, msg);
            }
            ui.add_space(8.0);
        }

        // Controls
        theme::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("SCAN TARGET").small().strong().color(theme::MUTED));
                ui.label(
                    egui::RichText::new(self.root.display().to_string())
                        .font(egui::FontId::monospace(12.0))
                        .strong()
                        .color(theme::INK),
                );
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if theme::ghost_button(ui, "Change folder…").clicked() && !self.scanning {
                    if let Some(dir) = rfd::FileDialog::new().set_directory(&self.root).pick_folder() {
                        self.root = dir;
                        self.report = None;
                    }
                }
                if self.scanning {
                    if theme::danger_button(ui, "✕ Cancel").clicked() {
                        self.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    ui.spinner();
                    ui.label(egui::RichText::new("Hashing…").strong().color(theme::INK));
                } else if theme::primary_button(ui, "Find duplicates", accent).clicked() {
                    self.start();
                }
            });
            if self.scanning && !self.log.is_empty() {
                ui.add_space(8.0);
                for line in &self.log {
                    ui.label(egui::RichText::new(line).font(egui::FontId::monospace(10.5)).color(theme::MUTED));
                }
            }
        });
        ui.add_space(12.0);

        let Some(report) = &self.report else { return; };
        if report.cancelled {
            theme::info_banner(ui, "Scan cancelled.");
            return;
        }
        if report.groups.is_empty() {
            theme::success_banner(ui, &format!("No duplicates found across {} files. Squeaky clean.", report.scanned_files));
            return;
        }

        // Summary bar
        theme::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} duplicate groups · {} extra copies · {} wasted",
                        report.groups.len(),
                        report.extra_copies(),
                        fmt_bytes(report.wasted_bytes())
                    ))
                    .strong()
                    .color(theme::INK),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.selected.is_empty() {
                        let sel_bytes: u64 = report
                            .groups
                            .iter()
                            .flat_map(|g| g.paths.iter())
                            .filter(|p| self.selected.contains(*p))
                            .filter_map(|p| p.metadata().ok().map(|m| m.len()))
                            .sum();
                        if theme::danger_button(
                            ui,
                            &format!("Delete {} ({})", self.selected.len(), fmt_bytes(sel_bytes)),
                        )
                        .clicked()
                        {
                            self.confirm_delete = true;
                        }
                    }
                    if ui.small_button("SELECT ALL EXTRAS").clicked() {
                        self.selected =
                            report.groups.iter().flat_map(|g| g.paths.iter().skip(1).cloned()).collect();
                    }
                    if ui.small_button("CLEAR").clicked() {
                        self.selected.clear();
                    }
                });
            });
        });
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("Checked copies will be deleted; the oldest copy in each group stays by default.")
                .small()
                .color(theme::MUTED),
        );
        ui.add_space(6.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            let selected = &mut self.selected;
            for (i, group) in report.groups.iter().enumerate() {
                egui::CollapsingHeader::new(format!(
                    "{} × {} — wasting {}",
                    group.paths.len(),
                    fmt_bytes(group.size),
                    fmt_bytes(group.wasted_bytes())
                ))
                .id_salt(("dup-group", i))
                .default_open(i < 3)
                .show(ui, |ui| {
                    for path in &group.paths {
                        let mut marked = selected.contains(path);
                        ui.checkbox(
                            &mut marked,
                            egui::RichText::new(path.display().to_string())
                                .font(egui::FontId::monospace(11.0))
                                .color(theme::INK),
                        );
                        if marked {
                            selected.insert(path.clone());
                        } else {
                            selected.remove(path);
                        }
                    }
                });
                ui.add_space(4.0);
            }
        });
    }
}

fn env_downloads() -> PathBuf {
    std::env::var("USERPROFILE")
        .map(PathBuf::from)
        .map(|p| p.join("Downloads"))
        .unwrap_or_else(|_| PathBuf::from("C:\\"))
}
