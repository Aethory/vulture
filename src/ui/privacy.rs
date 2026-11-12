//! Privacy tab — browser caches (and, opt-in, cookies & history).

use crate::core::browsers::{self, BrowserInfo, PrivacyOptions, PrivacyReport};
use crate::theme;
use crate::ui::fmt_bytes;
use std::sync::mpsc::{self, Receiver};
use std::thread;

enum PrivacyMsg {
    Sizes(Vec<u64>),
    Cleaned(PrivacyReport),
}

pub struct PrivacyTab {
    browsers: Option<Vec<BrowserInfo>>,
    sizes: Option<Vec<u64>>,
    calculating: bool,
    cleaning: bool,
    cache: bool,
    cookies: bool,
    history: bool,
    rx: Option<Receiver<PrivacyMsg>>,
    report: Option<PrivacyReport>,
}

impl Default for PrivacyTab {
    fn default() -> Self {
        Self {
            browsers: None,
            sizes: None,
            calculating: false,
            cleaning: false,
            cache: true,
            cookies: false,
            history: false,
            rx: None,
            report: None,
        }
    }
}

impl PrivacyTab {
    fn start_measure(&mut self) {
        self.calculating = true;
        let browsers = self.browsers.as_ref().unwrap().clone();
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        thread::spawn(move || {
            let sizes = browsers.iter().map(|b| b.cache_size()).collect();
            let _ = tx.send(PrivacyMsg::Sizes(sizes));
        });
    }

    fn start_clean(&mut self) {
        self.cleaning = true;
        self.report = None;
        let options = PrivacyOptions {
            cache: self.cache,
            cookies: self.cookies,
            history: self.history,
        };
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        thread::spawn(move || {
            let report = browsers::clean(options);
            let _ = tx.send(PrivacyMsg::Cleaned(report));
        });
    }

    pub fn show(&mut self, ui: &mut egui::Ui, accent: egui::Color32) {
        if self.browsers.is_none() {
            let detected = browsers::detect_all();
            self.browsers = Some(detected);
            self.start_measure();
        }

        // Poll worker.
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(PrivacyMsg::Sizes(sizes)) => {
                    self.sizes = Some(sizes);
                    self.calculating = false;
                    self.rx = None;
                }
                Ok(PrivacyMsg::Cleaned(report)) => {
                    self.report = Some(report);
                    self.cleaning = false;
                    self.rx = None;
                    // Re-measure after cleaning.
                    self.start_measure();
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.calculating = false;
                    self.cleaning = false;
                    self.rx = None;
                }
            }
        }
        if self.calculating || self.cleaning {
            ui.ctx().request_repaint();
        }

        theme::heading(ui, "Privacy");
        theme::subtitle(
            ui,
            "Caches regrow in days. Cookies and history are optional — read before ticking.",
        );
        ui.add_space(14.0);

        if let Some(report) = &self.report {
            theme::success_banner(
                ui,
                &format!(
                    "Removed {} items — freed {}.",
                    report.items_removed,
                    fmt_bytes(report.bytes_freed)
                ),
            );
            if !report.failures.is_empty() {
                ui.add_space(6.0);
                egui::CollapsingHeader::new(format!(
                    "{} locked — close the browser first",
                    report.failures.len()
                ))
                .id_salt("privacy-failures")
                .show(ui, |ui| {
                    for f in &report.failures {
                        ui.label(egui::RichText::new(f).small().color(theme::MUTED));
                    }
                });
            }
            ui.add_space(10.0);
        }

        // Browser cards
        let browsers = self.browsers.as_ref().unwrap();
        for (i, browser) in browsers.iter().enumerate() {
            let detected = browser.detected();
            theme::card(ui, |ui| {
                ui.horizontal(|ui| {
                    egui::Frame::new()
                        .fill(if detected {
                            theme::GREEN
                        } else {
                            theme::CARD_HI
                        })
                        .stroke(egui::Stroke::new(1.5, theme::INK))
                        .corner_radius(egui::CornerRadius::same(20))
                        .inner_margin(egui::Margin::same(9))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(if detected { "●" } else { "○" })
                                    .color(theme::INK),
                            );
                        });
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(browser.name).strong().color(theme::INK));
                        ui.label(
                            egui::RichText::new(if detected {
                                format!(
                                    "{} profile(s) · {}",
                                    browser.profiles.len(),
                                    browser.data_root.display()
                                )
                            } else {
                                "Not installed".to_string()
                            })
                            .small()
                            .color(theme::MUTED),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if detected {
                            if let Some(sizes) = &self.sizes {
                                ui.label(
                                    egui::RichText::new(fmt_bytes(sizes[i]))
                                        .font(egui::FontId::proportional(19.0))
                                        .strong()
                                        .color(theme::INK),
                                );
                            } else {
                                ui.spinner();
                            }
                        }
                    });
                });
            });
            ui.add_space(8.0);
        }

        ui.add_space(4.0);

        // What to clean
        theme::card(ui, |ui| {
            theme::section_label(ui, "What to wipe");
            ui.add_space(8.0);
            ui.checkbox(
                &mut self.cache,
                egui::RichText::new("Browser caches — code, GPU shaders, site data (recommended)")
                    .color(theme::INK),
            );
            ui.checkbox(
                &mut self.cookies,
                egui::RichText::new("Cookies — you will be signed out of every website")
                    .color(theme::RED),
            );
            ui.checkbox(
                &mut self.history,
                egui::RichText::new(
                    "History — empties your browsing history and address bar suggestions",
                )
                .color(theme::RED),
            );
            ui.add_space(12.0);

            if self.cleaning {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(egui::RichText::new("Wiping…").strong().color(theme::INK));
                });
            } else if !(self.cache || self.cookies || self.history) {
                ui.label(
                    egui::RichText::new("Select at least one thing to wipe.").color(theme::MUTED),
                );
            } else if theme::primary_button(ui, "Clean selected data", accent).clicked() {
                self.start_clean();
            }

            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Close all browsers first. Locked files are skipped and reported — never forced.")
                    .small()
                    .color(theme::MUTED),
            );
        });
    }
}
