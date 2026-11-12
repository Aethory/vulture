//! Headless auto-clean — the code path behind `vulture.exe --auto-clean`,
//! which the scheduled task runs daily at 02:30.

use crate::core::{browsers, scanner};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct AutoCleanReport {
    pub bytes_freed: u64,
    pub files_removed: u64,
    pub failures: Vec<String>,
}

impl AutoCleanReport {
    /// One-line human summary, printed in headless mode and handy for the log.
    pub fn summary(&self) -> String {
        format!(
            "removed {} files, freed {}",
            self.files_removed,
            humansize::format_size(self.bytes_freed, humansize::BINARY)
        )
    }
}

pub fn run() -> AutoCleanReport {
    // System junk worth auto-clearing (skip Prefetch/Update — too admin-y, too aggressive).
    let wanted = [
        scanner::CategoryId::WindowsTemp,
        scanner::CategoryId::Thumbnails,
        scanner::CategoryId::CrashDumps,
        scanner::CategoryId::ShaderCache,
    ];

    let scans = scanner::scan_all();
    let selected: Vec<scanner::CategoryId> = scans
        .iter()
        .filter(|s| wanted.contains(&s.id))
        .map(|s| s.id)
        .collect();
    let mut report = scanner::clean(&scans, &selected);

    // Browser caches only — cookies and history are never touched automatically.
    let privacy = browsers::clean(browsers::PrivacyOptions {
        cache: true,
        cookies: false,
        history: false,
    });
    report.files_removed += privacy.items_removed;
    report.bytes_freed += privacy.bytes_freed;
    report.failures.extend(privacy.failures);

    write_log(&report);

    AutoCleanReport {
        bytes_freed: report.bytes_freed,
        files_removed: report.files_removed,
        failures: report.failures,
    }
}

fn write_log(report: &scanner::CleanReport) {
    let dir = crate::config::logs_dir();
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = dir.join(format!("auto-clean-{secs}.log"));
    let body = format!(
        "Vulture auto-clean run\nfiles removed : {}\nbytes freed   : {}\nfailures      : {}\n",
        report.files_removed,
        report.bytes_freed,
        if report.failures.is_empty() {
            "none".to_string()
        } else {
            report.failures.join("\n                ")
        }
    );
    let _ = fs::write(path, body);
}
