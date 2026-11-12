//! App shell — ink sidebar with the Vulture logo and tab routing.

use crate::config::Config;
use crate::theme;
use crate::ui;
use sysinfo::System;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Dashboard,
    Cleaner,
    Startup,
    Memory,
    Disk,
    Duplicates,
    Uninstaller,
    Privacy,
    Scheduler,
    Settings,
}

const NAV: &[(Tab, &str, &str)] = &[
    (Tab::Dashboard, "📊", "Dashboard"),
    (Tab::Cleaner, "🧹", "Smart Clean"),
    (Tab::Startup, "🚀", "Startup"),
    (Tab::Memory, "⚡", "RAM Booster"),
    (Tab::Disk, "💽", "Disk Analyzer"),
    (Tab::Duplicates, "🧬", "Duplicates"),
    (Tab::Uninstaller, "🗑️", "Uninstaller"),
    (Tab::Privacy, "🍪", "Privacy"),
    (Tab::Scheduler, "⏰", "Auto Clean"),
    (Tab::Settings, "⚙️", "Settings"),
];

pub struct VultureApp {
    tab: Tab,
    sys: System,
    config: Config,
    dashboard: ui::dashboard::DashboardTab,
    cleaner: ui::cleaner::CleanerTab,
    startup: ui::startup::StartupTab,
    memory: ui::memory::MemoryTab,
    disk: ui::disk::DiskTab,
    duplicates: ui::duplicates::DuplicatesTab,
    uninstaller: ui::uninstaller::UninstallerTab,
    privacy: ui::privacy::PrivacyTab,
    scheduler: ui::scheduler::SchedulerTab,
    settings: ui::settings::SettingsTab,
}

impl VultureApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            tab: Tab::Dashboard,
            sys: System::new(),
            config: Config::load(),
            dashboard: ui::dashboard::DashboardTab::default(),
            cleaner: ui::cleaner::CleanerTab::default(),
            startup: ui::startup::StartupTab::default(),
            memory: ui::memory::MemoryTab::default(),
            disk: ui::disk::DiskTab::default(),
            duplicates: ui::duplicates::DuplicatesTab::default(),
            uninstaller: ui::uninstaller::UninstallerTab::default(),
            privacy: ui::privacy::PrivacyTab::default(),
            scheduler: ui::scheduler::SchedulerTab::default(),
            settings: ui::settings::SettingsTab::default(),
        }
    }

    fn paint_logo(ui: &mut egui::Ui, accent: egui::Color32) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(46.0, 46.0), egui::Sense::hover());
        let painter = ui.painter();
        painter.circle_filled(rect.center(), 23.0, accent);
        painter.circle_stroke(rect.center(), 23.0, egui::Stroke::new(2.5, theme::PAPER));
        // Bold V beak
        let c = rect.center();
        let left = c + egui::vec2(-11.0, -8.0);
        let right = c + egui::vec2(11.0, -8.0);
        let tip = c + egui::vec2(0.0, 10.0);
        painter.line_segment([left, tip], egui::Stroke::new(6.5, theme::INK));
        painter.line_segment([right, tip], egui::Stroke::new(6.5, theme::INK));
    }
}

impl eframe::App for VultureApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let accent = theme::accent(&self.config.accent);
        theme::apply(ctx, accent);

        egui::SidePanel::left("nav")
            .exact_width(238.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(theme::SIDEBAR).inner_margin(egui::Margin::same(14)))
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    Self::paint_logo(ui, accent);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("VULTURE")
                                .font(egui::FontId::proportional(25.0))
                                .strong()
                                .color(theme::SIDEBAR_TEXT),
                        );
                        ui.label(
                            egui::RichText::new("PC CARE ENGINE")
                                .font(egui::FontId::proportional(9.0))
                                .strong()
                                .color(theme::SIDEBAR_MUTED),
                        );
                    });
                });
                ui.add_space(18.0);

                for (tab, icon, label) in NAV {
                    let selected = self.tab == *tab;
                    let (fill, text) = if selected {
                        (accent, theme::INK)
                    } else {
                        (egui::Color32::TRANSPARENT, theme::SIDEBAR_TEXT)
                    };
                    let response = ui.add(
                        egui::Button::new(
                            egui::RichText::new(format!("{icon}  {label}"))
                                .strong()
                                .font(egui::FontId::proportional(14.0))
                                .color(text),
                        )
                        .fill(fill)
                        .stroke(egui::Stroke::new(2.0, if selected { accent } else { egui::Color32::TRANSPARENT }))
                        .corner_radius(egui::CornerRadius::same(6))
                        .min_size(egui::vec2(ui.available_width(), 38.0)),
                    );
                    if response.clicked() {
                        self.tab = *tab;
                    }
                    ui.add_space(4.0);
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.horizontal(|ui| {
                        ui.hyperlink_to(
                            egui::RichText::new("github.com/aethory/vulture").small().color(theme::SIDEBAR_MUTED),
                            "https://github.com/aethory/vulture",
                        );
                    });
                    ui.label(
                        egui::RichText::new("v1.0.0 · MIT · 100% Rust")
                            .small()
                            .color(theme::SIDEBAR_MUTED),
                    );
                    ui.add_space(6.0);
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::PAPER).inner_margin(egui::Margin::same(22)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                    match self.tab {
                        Tab::Dashboard => self.dashboard.show(ui, &mut self.sys, &mut self.config, accent),
                        Tab::Cleaner => self.cleaner.show(ui, &mut self.config, accent),
                        Tab::Startup => self.startup.show(ui, accent),
                        Tab::Memory => self.memory.show(ui, &mut self.sys, accent),
                        Tab::Disk => self.disk.show(ui, accent),
                        Tab::Duplicates => self.duplicates.show(ui, accent),
                        Tab::Uninstaller => self.uninstaller.show(ui, accent),
                        Tab::Privacy => self.privacy.show(ui, accent),
                        Tab::Scheduler => self.scheduler.show(ui, accent),
                        Tab::Settings => self.settings.show(ui, &mut self.config),
                    }
                });
            });

        ctx.request_repaint_after(std::time::Duration::from_secs(2));
    }
}
