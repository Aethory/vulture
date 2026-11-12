//! Dashboard — the landing tab. Live vitals + the one-click Smart Clean.

use crate::config::Config;
use crate::core::{browsers, scanner};
use crate::theme;
use crate::ui::{fmt_bytes, fmt_gb, fmt_uptime};
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use sysinfo::{Disks, System};

pub struct DashboardTab {
    working: bool,
    empty_bin: bool,
    rx: Option<Receiver<scanner::CleanReport>>,
    done: Option<scanner::CleanReport>,
    note: Option<String>,
}

impl Default for DashboardTab {
    fn default() -> Self {
        Self {
            working: false,
            empty_bin: false,
            rx: None,
            done: None,
            note: None,
        }
    }
}

impl DashboardTab {
    fn start_smart_clean(&mut self, include_bin: bool) {
        self.working = true;
        self.done = None;
        self.note = None;
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        thread::spawn(move || {
            let scans = scanner::scan_all();
            let wanted = [
                scanner::CategoryId::WindowsTemp,
                scanner::CategoryId::Thumbnails,
                scanner::CategoryId::CrashDumps,
                scanner::CategoryId::ShaderCache,
            ];
            let selected: Vec<_> = scans
                .iter()
                .filter(|s| wanted.contains(&s.id))
                .map(|s| s.id)
                .collect();
            let mut report = scanner::clean(&scans, &selected);

            let privacy = browsers::clean(browsers::PrivacyOptions {
                cache: true,
                cookies: false,
                history: false,
            });
            report.files_removed += privacy.items_removed;
            report.bytes_freed += privacy.bytes_freed;
            report.failures.extend(privacy.failures);

            if include_bin {
                if let Err(e) = scanner::empty_recycle_bin() {
                    report.failures.push(e);
                }
            }

            let _ = tx.send(report);
        });
    }

    pub fn show(&mut self, ui: &mut egui::Ui, sys: &mut System, config: &mut Config, accent: egui::Color32) {
        // Poll the worker.
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(report) => {
                    config.record_clean(report.bytes_freed, report.files_removed);
                    self.done = Some(report);
                    self.working = false;
                    self.rx = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.working = false;
                    self.rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.working {
            ui.ctx().request_repaint();
        }

        // Header
        theme::heading(ui, "Mission Control");
        theme::subtitle(ui, "Your machine at a glance — and the button that feeds Vulture.");
        ui.add_space(14.0);

        // Live vitals
        sys.refresh_memory();
        sys.refresh_cpu_usage();
        let cpu = sys.global_cpu_usage();
        let mem_used = sys.used_memory();
        let mem_total = sys.total_memory();
        let uptime = System::uptime().unwrap_or(0);
        let (free_gb, total_gb) = disk_free();

        ui.horizontal(|ui| {
            theme::stat_tile(ui, "CPU", &format!("{cpu:.0}%"), "global load", theme::INK);
            theme::stat_tile(
                ui,
                "Memory",
                &format!("{:.1} GB", mem_used as f64 / 1_073_741_824.0),
                &format!("of {:.1} GB used", mem_total as f64 / 1_073_741_824.0),
                theme::INK,
            );
            theme::stat_tile(
                ui,
                "Disk free",
                &format!("{free_gb:.0} GB"),
                &format!("of {total_gb:.0} GB on C:"),
                theme::INK,
            );
            theme::stat_tile(ui, "Uptime", &fmt_uptime(uptime), "since last boot", theme::INK);
        });
        ui.add_space(14.0);

        // Smart clean — the hero
        theme::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("ONE-CLICK SMART CLEAN")
                            .font(egui::FontId::proportional(22.0))
                            .strong()
                            .color(theme::INK),
                    );
                    ui.label(
                        egui::RichText::new(
                            "Temp files · thumbnail cache · crash dumps · shader cache · browser caches",
                        )
                        .color(theme::MUTED),
                    );
                    ui.add_space(10.0);

                    if self.working {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(egui::RichText::new("Hunting junk…").strong().color(theme::INK));
                        });
                    } else if theme::primary_button(ui, "⚡ Smart Clean — Feed the Vulture", accent).clicked() {
                        self.start_smart_clean(self.empty_bin);
                    }

                    ui.add_space(8.0);
                    ui.checkbox(
                        &mut self.empty_bin,
                        egui::RichText::new("Also empty the Recycle Bin").color(theme::MUTED),
                    );
                });

                // Right side: lifetime counter plate
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::Frame::new()
                        .fill(theme::INK)
                        .corner_radius(egui::CornerRadius::same(8))
                        .inner_margin(egui::Margin::symmetric(18, 16))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    egui::RichText::new(fmt_gb(config.total_bytes_cleaned))
                                        .font(egui::FontId::proportional(26.0))
                                        .strong()
                                        .color(accent),
                                );
                                ui.label(
                                    egui::RichText::new("LIFETIME FREED")
                                        .font(egui::FontId::proportional(9.0))
                                        .strong()
                                        .color(theme::SIDEBAR_MUTED),
                                );
                            });
                        });
                });
            });
        });
        ui.add_space(10.0);

        // Report / notes
        if let Some(report) = &self.done {
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
                egui::CollapsingHeader::new(format!(
                    "{} items were locked (close the owning app and retry)",
                    report.failures.len()
                ))
                .id_salt("smart-clean-failures")
                .show(ui, |ui| {
                    for f in &report.failures {
                        ui.label(egui::RichText::new(f).small().color(theme::MUTED));
                    }
                });
            }
        }
        if let Some(note) = &self.note {
            theme::info_banner(ui, note);
        }
        ui.add_space(14.0);

        // Bottom row: lifetime stats + tips
        ui.columns(2, |cols| {
            theme::card(&mut cols[0], |ui| {
                theme::section_label(ui, "Lifetime");
                ui.add_space(6.0);
                ui.label(egui::RichText::new(format!("{} cleans run", config.clean_runs)).strong().color(theme::INK));
                ui.label(
                    egui::RichText::new(format!(
                        "Last clean: {} — freed {}",
                        config.last_clean_label(),
                        fmt_bytes(config.last_clean_bytes)
                    ))
                    .color(theme::MUTED),
                );
            });
            theme::card(&mut cols[1], |ui| {
                theme::section_label(ui, "Field Manual");
                ui.add_space(6.0);
                ui.label(egui::RichText::new("• Run Vulture weekly — junk regrows.").color(theme::INK));
                ui.label(egui::RichText::new("• Smart Clean never touches cookies or history.").color(theme::INK));
                ui.label(egui::RichText::new("• Everything goes to the Recycle Bin first.").color(theme::INK));
            });
        });
    }
}

fn disk_free() -> (f64, f64) {
    let disks = match Disks::new_with_refreshed_list() {
        Ok(d) => d,
        Err(_) => return (0.0, 0.0),
    };
    let list: &[sysinfo::Disk] = &disks;
    let system_drive = Path::new("C:\\");
    let disk = list
        .iter()
        .find(|d| d.mount_point() == system_drive)
        .or_else(|| list.first());
    match disk {
        Some(d) => {
            (d.available_space() as f64 / 1_073_741_824.0, d.total_space() as f64 / 1_073_741_824.0)
        }
        None => (0.0, 0.0),
    }
}


