//! RAM Booster tab — working-set trim + process triage.

use crate::core::memory::{self, MemoryStats, ProcInfo};
use crate::theme;
use crate::ui::{fmt_bytes, fmt_gb, fmt_uptime};
use std::time::Instant;
use sysinfo::System;

pub struct MemoryTab {
    stats: Option<MemoryStats>,
    sys_side: Instant,
    boost_note: Option<String>,
    kill_error: Option<String>,
    confirm_kill: Option<(u32, Instant)>,
}

impl Default for MemoryTab {
    fn default() -> Self {
        Self {
            stats: None,
            sys_side: Instant::now() - std::time::Duration::from_secs(10),
            boost_note: None,
            kill_error: None,
            confirm_kill: None,
        }
    }
}

impl MemoryTab {
    pub fn show(&mut self, ui: &mut egui::Ui, sys: &mut System, accent: egui::Color32) {
        if self.sys_side.elapsed() >= std::time::Duration::from_secs(2) {
            self.stats = Some(memory::collect(sys));
            self.sys_side = Instant::now();
            ui.ctx().request_repaint_after(std::time::Duration::from_secs(2));
        }

        // Clear a stale confirm after 3 seconds.
        if let Some((_, at)) = self.confirm_kill {
            if at.elapsed() > std::time::Duration::from_secs(3) {
                self.confirm_kill = None;
            }
        }

        theme::heading(ui, "RAM Booster");
        theme::subtitle(ui, "Flush what's cached for speed — ending tasks is on you.");
        ui.add_space(14.0);

        let stats = match &self.stats {
            Some(s) => s.clone(),
            None => {
                ui.spinner();
                return;
            }
        };

        let fraction = stats.used_bytes as f32 / stats.total_bytes.max(1) as f32;

        ui.horizontal(|ui| {
            // Left column — big usage card + boost
            ui.allocate_ui(egui::vec2(ui.available_width() * 0.42, ui.available_height()), |ui| {
                theme::card(ui, |ui| {
                    theme::section_label(ui, "Memory in use");
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "{} / {}",
                            fmt_bytes(stats.used_bytes).replace('i', ""),
                            fmt_gb(stats.total_bytes)
                        ))
                        .font(egui::FontId::proportional(30.0))
                        .strong()
                        .color(theme::INK),
                    );
                    ui.add_space(8.0);
                    brutal_bar(ui, fraction, accent);
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        theme::stat_tile(ui, "CPU", &format!("{:.0}%", stats.cpu_percent), "global", theme::INK);
                        theme::stat_tile(ui, "Uptime", &fmt_uptime(stats.uptime_secs), "since boot", theme::INK);
                    });
                });

                ui.add_space(12.0);

                theme::card(ui, |ui| {
                    theme::section_label(ui, "Quick boost");
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "Trims the working set of every background process. Windows re-pages what's needed on demand — your apps won't crash, they'll just breathe.",
                        )
                        .small()
                        .color(theme::MUTED),
                    );
                    ui.add_space(10.0);
                    if theme::primary_button(ui, "⚡ Boost now", accent).clicked() {
                        let trimmed = memory::boost(sys);
                        self.boost_note = Some(format!("Working sets trimmed on {trimmed} processes."));
                        self.kill_error = None;
                    }
                    if let Some(note) = &self.boost_note {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(format!("✓ {note}")).strong().color(theme::GREEN));
                    }
                    if let Some(err) = &self.kill_error {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(format!("! {err}")).strong().color(theme::RED));
                    }
                });
            });

            ui.add_space(6.0);

            // Right column — process table
            theme::card(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::section_label(ui, "Memory hogs — top 40");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::ghost_button(ui, "↻ Refresh").clicked() {
                            self.stats = Some(memory::collect(sys));
                            self.sys_side = Instant::now();
                        }
                    });
                });
                ui.add_space(6.0);

                egui::Grid::new("proc-grid")
                    .num_columns(4)
                    .spacing([18.0, 6.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("PROCESS").small().strong().color(theme::MUTED));
                        ui.label(egui::RichText::new("MEMORY").small().strong().color(theme::MUTED));
                        ui.label(egui::RichText::new("CPU").small().strong().color(theme::MUTED));
                        ui.label(egui::RichText::new("").small());
                        ui.end_row();

                        for proc in &stats.processes {
                            ui.horizontal(|ui| {
                                theme::tag(ui, &format!("#{}", proc.pid));
                                ui.label(
                                    egui::RichText::new(truncate(&proc.name, 34))
                                        .font(egui::FontId::monospace(12.0))
                                        .strong()
                                        .color(theme::INK),
                                );
                            });
                            ui.label(
                                egui::RichText::new(fmt_bytes(proc.memory_bytes).replace('i', ""))
                                    .font(egui::FontId::monospace(12.0))
                                    .color(theme::INK),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.1}%", proc.cpu_percent))
                                    .font(egui::FontId::monospace(12.0))
                                    .color(theme::MUTED),
                            );
                            let waiting = matches!(&self.confirm_kill, Some((pid, _)) if *pid == proc.pid);
                            let label = if waiting { "CONFIRM END?" } else { "END TASK" };
                            let clicked = if waiting {
                                theme::danger_button(ui, label).clicked()
                            } else {
                                ui.add(
                                    egui::Button::new(
                                        egui::RichText::new(label).small().strong().color(theme::RED),
                                    )
                                    .fill(egui::Color32::TRANSPARENT)
                                    .stroke(egui::Stroke::new(1.5, theme::RED))
                                    .corner_radius(egui::CornerRadius::same(4)),
                                )
                                .clicked()
                            };
                            if clicked {
                                if waiting {
                                    self.confirm_kill = None;
                                    match memory::kill(sys, proc) {
                                        Ok(()) => {
                                            self.boost_note = Some(format!("Ended '{}'.", proc.name));
                                            self.stats = Some(memory::collect(sys));
                                        }
                                        Err(e) => self.kill_error = Some(e),
                                    }
                                } else {
                                    self.confirm_kill = Some((proc.pid, Instant::now()));
                                    self.kill_error = None;
                                }
                            }
                            ui.end_row();
                        }
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

/// Chunky ink-bordered progress bar — pure egui painting.
pub fn brutal_bar(ui: &mut egui::Ui, fraction: f32, accent: egui::Color32) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 22.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, egui::CornerRadius::same(4), theme::CARD_HI);
    painter.rect_stroke(rect, egui::CornerRadius::same(4), egui::Stroke::new(2.0, theme::INK));
    let inner_w = (rect.width() - 4.0) * fraction.clamp(0.0, 1.0);
    if inner_w > 2.0 {
        let fill = egui::Rect::from_min_size(
            rect.min + egui::vec2(2.0, 2.0),
            egui::vec2(inner_w, rect.height() - 4.0),
        );
        painter.rect_filled(fill, egui::CornerRadius::same(2), accent);
    }
}
