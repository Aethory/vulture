//! Browser cache / cookie / history discovery and cleaning.
//! Detects Chrome, Edge, Brave, Opera and Firefox, plus their usual profiles.

use humansize::{format_size, BINARY};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct BrowserInfo {
    pub name: &'static str,
    /// Root the browser stores its data under (used to detect presence).
    pub data_root: PathBuf,
    pub profiles: Vec<PathBuf>,
    /// Cache-ish directories inside each profile.
    pub cache_dirs: Vec<PathBuf>,
    /// SQLite cookie stores (only touched when the user opts in).
    pub cookie_files: Vec<PathBuf>,
    /// History files (only touched when the user opts in).
    pub history_files: Vec<PathBuf>,
}

impl BrowserInfo {
    pub fn detected(&self) -> bool {
        self.data_root.exists()
    }

    pub fn cache_size(&self) -> u64 {
        dir_size(&self.cache_dirs)
    }

    pub fn size_label(&self) -> String {
        format_size(self.cache_size(), BINARY)
    }
}

fn dir_size(paths: &[PathBuf]) -> u64 {
    let mut total = 0u64;
    for p in paths {
        if !p.exists() {
            continue;
        }
        for entry in WalkDir::new(p).into_iter().filter_map(|e| e.ok()) {
            if let Ok(md) = entry.metadata() {
                if md.is_file() {
                    total += md.len();
                }
            }
        }
    }
    total
}

fn chrome_family(name: &'static str, user_data: PathBuf) -> BrowserInfo {
    let mut profiles = Vec::new();
    let mut cache_dirs = Vec::new();
    let mut cookie_files = Vec::new();
    let mut history_files = Vec::new();

    if user_data.exists() {
        if let Ok(entries) = fs::read_dir(&user_data) {
            for entry in entries.filter_map(|e| e.ok()) {
                let dir_name = entry.file_name().to_string_lossy().into_owned();
                let is_profile = dir_name == "Default"
                    || dir_name == "Guest Profile"
                    || dir_name.starts_with("Profile ");
                if !is_profile {
                    continue;
                }
                let profile = entry.path();
                cache_dirs.push(profile.join("Cache"));
                cache_dirs.push(profile.join("Code Cache"));
                cache_dirs.push(profile.join("GPUCache"));
                cache_dirs.push(profile.join("Service Worker").join("CacheStorage"));
                cache_dirs.push(profile.join("CachedData"));
                cookie_files.push(profile.join("Network").join("Cookies"));
                cookie_files.push(profile.join("Cookies"));
                history_files.push(profile.join("History"));
                profiles.push(profile);
            }
        }
    }

    BrowserInfo { name, data_root: user_data, profiles, cache_dirs, cookie_files, history_files }
}

fn firefox() -> BrowserInfo {
    let roaming = env::var("APPDATA").unwrap_or_default();
    let local = env::var("LOCALAPPDATA").unwrap_or_default();
    let roaming_root = PathBuf::from(&roaming).join("Mozilla").join("Firefox");
    let local_profiles = PathBuf::from(&local).join("Mozilla").join("Firefox").join("Profiles");

    let mut profiles = Vec::new();
    let mut cache_dirs = Vec::new();
    let mut cookie_files = Vec::new();
    let mut history_files = Vec::new();

    if let Ok(entries) = fs::read_dir(&local_profiles) {
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            cache_dirs.push(p.join("cache2"));
            cache_dirs.push(p.join("gpucache"));
            cache_dirs.push(p.join("shader-cache"));
            cache_dirs.push(p.join("startupCache"));
            profiles.push(p);
        }
    }
    let roaming_profiles = roaming_root.join("Profiles");
    if let Ok(entries) = fs::read_dir(&roaming_profiles) {
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            cache_dirs.push(p.join("cache2"));
            cookie_files.push(p.join("cookies.sqlite"));
            history_files.push(p.join("places.sqlite"));
            history_files.push(p.join("places.sqlite-wal"));
            history_files.push(p.join("places.sqlite-shm"));
        }
    }

    BrowserInfo {
        name: "Firefox",
        data_root: roaming_root.join("Profiles"),
        profiles,
        cache_dirs,
        cookie_files,
        history_files,
    }
}

/// Opera keeps a single hard-coded profile directly inside "Opera Stable".
fn opera() -> BrowserInfo {
    let roaming = env::var("APPDATA").unwrap_or_default();
    let root = PathBuf::from(&roaming).join("Opera Software").join("Opera Stable");
    BrowserInfo {
        name: "Opera",
        data_root: root.clone(),
        profiles: if root.exists() { vec![root.clone()] } else { Vec::new() },
        cache_dirs: vec![
            root.join("Cache"),
            root.join("Code Cache"),
            root.join("GPUCache"),
        ],
        cookie_files: vec![root.join("Network").join("Cookies")],
        history_files: vec![root.join("History")],
    }
}

pub fn detect_all() -> Vec<BrowserInfo> {
    let local = env::var("LOCALAPPDATA").unwrap_or_default();

    vec![
        chrome_family("Google Chrome", PathBuf::from(&local).join("Google").join("Chrome").join("User Data")),
        chrome_family("Microsoft Edge", PathBuf::from(&local).join("Microsoft").join("Edge").join("User Data")),
        chrome_family(
            "Brave",
            PathBuf::from(&local).join("BraveSoftware").join("Brave-Browser").join("User Data"),
        ),
        opera(),
        firefox(),
    ]
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PrivacyOptions {
    pub cache: bool,
    pub cookies: bool,
    pub history: bool,
}

#[derive(Default)]
pub struct PrivacyReport {
    pub items_removed: u64,
    pub bytes_freed: u64,
    pub failures: Vec<String>,
}

/// Best-effort delete: trash the directory as-is, fall back to per-file deletion.
fn trash_best_effort(path: &Path) -> Result<(u64, u64), String> {
    if !path.exists() {
        return Ok((0, 0));
    }
    let size = dir_size(&[path.to_path_buf()]);
    if trash::delete(path).is_ok() {
        return Ok((1, size));
    }
    // Fallback: browser may hold a few locked files — remove what we can.
    let mut removed = 0u64;
    let mut freed = 0u64;
    if path.is_dir() {
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            let md = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if md.is_file() && trash::delete(entry.path()).is_ok() {
                removed += 1;
                freed += md.len();
            }
        }
        // Try to remove now-empty dirs (ignores failures for locked leftovers).
        let _ = fs::remove_dir_all(path);
        Ok((removed, freed))
    } else if trash::delete(path).is_ok() {
        Ok((1, size))
    } else {
        Err(format!("Could not remove {}", path.display()))
    }
}

pub fn clean(options: PrivacyOptions) -> PrivacyReport {
    let mut report = PrivacyReport::default();
    if !options.cache && !options.cookies && !options.history {
        return report;
    }

    for browser in detect_all() {
        if !browser.detected() {
            continue;
        }

        if options.cache {
            for dir in &browser.cache_dirs {
                match trash_best_effort(dir) {
                    Ok((n, bytes)) => {
                        report.items_removed += n;
                        report.bytes_freed += bytes;
                    }
                    Err(e) => report.failures.push(format!("{}: {e}", browser.name)),
                }
            }
        }

        if options.cookies {
            for file in &browser.cookie_files {
                if file.exists() {
                    match trash::delete(file) {
                        Ok(()) => {
                            report.items_removed += 1;
                            report.bytes_freed += file.metadata().map(|m| m.len()).unwrap_or(0);
                        }
                        Err(_) => report
                            .failures
                            .push(format!("{}: cookies are locked — close the browser first.", browser.name)),
                    }
                }
            }
        }

        if options.history {
            for file in &browser.history_files {
                if file.exists() {
                    match trash::delete(file) {
                        Ok(()) => {
                            report.items_removed += 1;
                            report.bytes_freed += file.metadata().map(|m| m.len()).unwrap_or(0);
                        }
                        Err(_) => report
                            .failures
                            .push(format!("{}: history is locked — close the browser first.", browser.name)),
                    }
                }
            }
        }
    }

    report
}
