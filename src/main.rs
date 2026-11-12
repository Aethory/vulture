#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod core;
mod icon;
mod theme;
mod ui;

fn main() -> eframe::Result<()> {
    // Headless mode used by the Windows scheduled task.
    if std::env::args().any(|a| a == "--auto-clean") {
        let report = core::auto_clean::run();
        std::process::exit(if report.failures.is_empty() { 0 } else { 1 });
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Vulture — PC Care")
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([980.0, 640.0])
            .with_icon(icon::app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Vulture",
        options,
        Box::new(|cc| Ok(Box::new(app::VultureApp::new(cc)))),
    )
}
