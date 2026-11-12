//! Junk-file discovery and safe cleaning (Recycle Bin only, never permanent).

use std::env;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CategoryId {
    WindowsTemp,
    WindowsUpdate,
    Prefetch,
    Thumbnails,
    CrashDumps,
    ShaderCache,
    DeliveryOpt,
    LogFiles,
}

pub struct CategoryDef {
    pub id: CategoryId,
    pub label: &'static str,
    pub description: &'static str,
    pub needs_admin: bool,
    paths: Vec<PathBuf>,
    /// When set, only files whose lower-cased name starts with this prefix are counted.
    file_prefix: Option<&'static str>,
    /// When set, only files with this extension (lower-cased, with dot) are counted.
    extension: Option<&'static str>,
}

impl CategoryDef {
    fn new(
        id: CategoryId,
        label: &'static str,
        description: &'static str,
        needs_admin: bool,
        paths: Vec<PathBuf>,
    ) -> Self {
        Self {
            id,
            label,
            description,
            needs_admin,
            paths,
            file_prefix: None,
            extension: None,
        }
    }
    fn prefix(mut self, p: &'static str) -> Self {
        self.file_prefix = Some(p);
        self
    }
    fn extension(mut self, ext: &'static str) -> Self {
        self.extension = Some(ext);
        self
    }
}

pub fn categories() -> Vec<CategoryDef> {
    let windir = env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".to_string());
    let local_appdata = env::var("LOCALAPPDATA").unwrap_or_default();
    let temp = env::temp_dir();

    vec![
        CategoryDef::new(
            CategoryId::WindowsTemp,
            "Windows & user temp",
            "Scratch files apps and Windows leave behind (%TEMP%, C:\\Windows\\Temp).",
            false,
            vec![temp, PathBuf::from(format!(r"{windir}\Temp"))],
        ),
        CategoryDef::new(
            CategoryId::WindowsUpdate,
            "Windows Update cache",
            "Downloaded update payloads that are no longer needed.",
            true,
            vec![PathBuf::from(format!(
                r"{windir}\SoftwareDistribution\Download"
            ))],
        ),
        CategoryDef::new(
            CategoryId::Prefetch,
            "Prefetch data",
            "Boot/app launch traces. Windows rebuilds them automatically.",
            true,
            vec![PathBuf::from(format!(r"{windir}\Prefetch"))],
        ),
        CategoryDef::new(
            CategoryId::Thumbnails,
            "Thumbnail & icon cache",
            "Cached image thumbnails — safe to discard, they rebuild on demand.",
            false,
            vec![PathBuf::from(format!(
                r"{local_appdata}\Microsoft\Windows\Explorer"
            ))],
        )
        .prefix("thumbcache_"),
        CategoryDef::new(
            CategoryId::CrashDumps,
            "Crash dumps",
            "Memory dumps Windows writes when a program crashes.",
            false,
            vec![
                PathBuf::from(format!(r"{local_appdata}\CrashDumps")),
                PathBuf::from(format!(r"{windir}\Minidump")),
            ],
        )
        .extension(".dmp"),
        CategoryDef::new(
            CategoryId::ShaderCache,
            "DirectX shader cache",
            "Compiled shaders games/GPU drivers cache. Rebuilt when needed.",
            false,
            vec![
                PathBuf::from(format!(r"{local_appdata}\D3DSCache")),
                PathBuf::from(format!(r"{local_appdata}\NVIDIA\DXCache")),
            ],
        ),
        CategoryDef::new(
            CategoryId::DeliveryOpt,
            "Delivery Optimization",
            "Cached update payloads shared with other PCs on your network.",
            true,
            vec![PathBuf::from(format!(
                r"{windir}\SoftwareDistribution\DeliveryOptimization"
            ))],
        ),
        CategoryDef::new(
            CategoryId::LogFiles,
            "Windows log files",
            "Setup and diagnostic logs older installers left behind.",
            true,
            vec![PathBuf::from(format!(r"{windir}\Logs"))],
        )
        .extension(".log"),
    ]
}

#[derive(Default, Clone)]
pub struct ScanItem {
    pub path: PathBuf,
    pub size: u64,
}

#[derive(Clone)]
pub struct CategoryScan {
    pub id: CategoryId,
    pub label: String,
    pub description: String,
    pub needs_admin: bool,
    pub items: Vec<ScanItem>,
    pub bytes: u64,
    pub error: Option<String>,
}

pub fn scan_all() -> Vec<CategoryScan> {
    categories().into_iter().map(scan_one).collect()
}

fn scan_one(def: CategoryDef) -> CategoryScan {
    let mut items = Vec::new();
    let mut bytes = 0u64;
    let mut error = None;

    for root in &def.paths {
        if !root.exists() {
            continue;
        }
        if fs::read_dir(root).is_err() {
            error = Some("Access denied — run Vulture as administrator to clear this.".to_string());
            continue;
        }

        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            let md = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if !md.is_file() {
                continue;
            }
            if let Some(prefix) = def.file_prefix {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if !name.starts_with(prefix) {
                    continue;
                }
            }
            if let Some(ext) = def.extension {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if !name.ends_with(ext) {
                    continue;
                }
            }
            bytes += md.len();
            items.push(ScanItem {
                path: entry.path().to_path_buf(),
                size: md.len(),
            });
        }
    }

    CategoryScan {
        id: def.id,
        label: def.label.to_string(),
        description: def.description.to_string(),
        needs_admin: def.needs_admin,
        items,
        bytes,
        error,
    }
}

#[derive(Default)]
pub struct CleanReport {
    pub files_removed: u64,
    pub bytes_freed: u64,
    pub failures: Vec<String>,
}

pub fn clean(scans: &[CategoryScan], selected: &[CategoryId]) -> CleanReport {
    let mut report = CleanReport::default();
    for scan in scans {
        if !selected.contains(&scan.id) {
            continue;
        }
        for item in &scan.items {
            match trash::delete(&item.path) {
                Ok(()) => {
                    report.files_removed += 1;
                    report.bytes_freed += item.size;
                }
                Err(err) => {
                    let name = item
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| item.path.display().to_string());
                    report.failures.push(format!("{name} — {err}"));
                }
            }
        }
    }
    report
}

/// Empty the Windows Recycle Bin. Returns a human-readable error on failure.
pub fn empty_recycle_bin() -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        SHEmptyRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND,
    };

    let result = unsafe {
        SHEmptyRecycleBinW(
            None,
            PCWSTR::null(),
            SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
        )
    };
    result.map_err(|e| format!("Could not empty the Recycle Bin: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vulture-test-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn categories_have_unique_ids_and_labels() {
        let cats = categories();
        assert!(cats.len() >= 8);
        let mut ids = std::collections::HashSet::new();
        for c in &cats {
            assert!(!c.label.is_empty());
            assert!(!c.paths.is_empty());
            assert!(ids.insert(c.id), "duplicate CategoryId in registry");
        }
    }

    #[test]
    fn scan_one_filters_by_extension() {
        let root = temp_root("ext-filter");
        fs::write(root.join("a.dmp"), b"1234").unwrap();
        fs::write(root.join("b.txt"), b"1234").unwrap();
        fs::write(root.join("c.DMP"), b"12").unwrap(); // case-insensitive match
        let def = CategoryDef::new(
            CategoryId::CrashDumps,
            "test",
            "test",
            false,
            vec![root.clone()],
        )
        .extension(".dmp");
        let scan = scan_one(def);
        assert_eq!(scan.items.len(), 2);
        assert_eq!(scan.bytes, 6);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn scan_one_filters_by_prefix() {
        let root = temp_root("prefix-filter");
        fs::write(root.join("thumbcache_32.db"), b"1234").unwrap();
        fs::write(root.join("iconcache_32.db"), b"12").unwrap();
        let def = CategoryDef::new(
            CategoryId::Thumbnails,
            "test",
            "test",
            false,
            vec![root.clone()],
        )
        .prefix("thumbcache_");
        let scan = scan_one(def);
        assert_eq!(scan.items.len(), 1);
        assert_eq!(scan.bytes, 4);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn scan_one_reports_missing_paths_as_clean() {
        let root = temp_root("missing").join("does-not-exist");
        let def = CategoryDef::new(CategoryId::WindowsTemp, "test", "test", false, vec![root]);
        let scan = scan_one(def);
        assert_eq!(scan.items.len(), 0);
        assert_eq!(scan.bytes, 0);
        assert!(scan.error.is_none());
    }

    #[test]
    fn clean_report_defaults_are_zero() {
        let report = CleanReport::default();
        assert_eq!(report.files_removed, 0);
        assert_eq!(report.bytes_freed, 0);
        assert!(report.failures.is_empty());
    }
}
