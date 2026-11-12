//! Content-based duplicate detection: bucket by size first, then hash with BLAKE3.

use anyhow::Result;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use walkdir::WalkDir;

#[derive(Debug)]
pub enum DupMsg {
    /// Status line for the activity log (e.g. "Hashing project-final-FINAL.zip…").
    Progress(String),
    Done(DuplicateReport),
}

#[derive(Debug)]
pub struct DuplicateGroup {
    pub size: u64,
    pub paths: Vec<PathBuf>,
}

impl DuplicateGroup {
    /// Bytes wasted by this group (keeping one copy).
    pub fn wasted_bytes(&self) -> u64 {
        self.size * (self.paths.len().saturating_sub(1)) as u64
    }
}

#[derive(Default, Debug)]
pub struct DuplicateReport {
    pub scanned_files: u64,
    pub groups: Vec<DuplicateGroup>,
    pub cancelled: bool,
    /// Set when the scan itself failed — UI should show this, not "no duplicates".
    pub error: Option<String>,
}

impl DuplicateReport {
    pub fn wasted_bytes(&self) -> u64 {
        self.groups.iter().map(|g| g.wasted_bytes()).sum()
    }
    pub fn extra_copies(&self) -> u64 {
        self.groups
            .iter()
            .map(|g| g.paths.len().saturating_sub(1) as u64)
            .sum()
    }
}

pub fn scan(root: &Path, cancel: Arc<AtomicBool>, tx: Sender<DupMsg>) {
    let report = scan_inner(root, &cancel, &tx).unwrap_or_else(|e| DuplicateReport {
        error: Some(e.to_string()),
        ..Default::default()
    });
    let _ = tx.send(DupMsg::Done(report));
}

fn scan_inner(
    root: &Path,
    cancel: &Arc<AtomicBool>,
    tx: &Sender<DupMsg>,
) -> Result<DuplicateReport> {
    // Phase 1 — bucket files by size.
    let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    let mut scanned = 0u64;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if cancel.load(Ordering::Relaxed) {
            return Ok(DuplicateReport {
                scanned_files: scanned,
                groups: Vec::new(),
                cancelled: true,
                error: None,
            });
        }
        let md = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !md.is_file() || md.len() < 1 {
            continue;
        }
        scanned += 1;
        if scanned.is_multiple_of(500) {
            let _ = tx.send(DupMsg::Progress(format!("Indexed {scanned} files…")));
        }
        by_size
            .entry(md.len())
            .or_default()
            .push(entry.path().to_path_buf());
    }

    let mut groups: Vec<DuplicateGroup> = Vec::new();

    // Phase 2 — hash only size-colliding buckets.
    let candidates: Vec<(u64, Vec<PathBuf>)> =
        by_size.into_iter().filter(|(_, v)| v.len() >= 2).collect();

    for (size, paths) in candidates {
        if cancel.load(Ordering::Relaxed) {
            return Ok(DuplicateReport {
                scanned_files: scanned,
                groups,
                cancelled: true,
                error: None,
            });
        }
        let mut by_hash: HashMap<String, Vec<PathBuf>> = HashMap::new();
        for path in paths {
            if cancel.load(Ordering::Relaxed) {
                return Ok(DuplicateReport {
                    scanned_files: scanned,
                    groups,
                    cancelled: true,
                    error: None,
                });
            }
            let _ = tx.send(DupMsg::Progress(format!(
                "Hashing {}",
                path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            )));
            match hash_file(&path) {
                Ok(hash) => by_hash.entry(hash).or_default().push(path),
                Err(_) => continue, // unreadable file — skip, don't abort the scan
            }
        }
        for (_, mut dup_paths) in by_hash {
            if dup_paths.len() >= 2 {
                dup_paths.sort_by_key(|p| p.metadata().and_then(|m| m.modified()).ok());
                groups.push(DuplicateGroup {
                    size,
                    paths: dup_paths,
                });
            }
        }
    }

    groups.sort_by_key(|g| Reverse(g.wasted_bytes()));
    Ok(DuplicateReport {
        scanned_files: scanned,
        groups,
        cancelled: false,
        error: None,
    })
}

fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 1024 * 256];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

#[derive(Default)]
pub struct DupDeleteReport {
    pub files_removed: u64,
    pub bytes_freed: u64,
    pub failures: Vec<String>,
}

/// Move the given paths to the Recycle Bin.
pub fn delete(paths: &[PathBuf]) -> DupDeleteReport {
    let mut report = DupDeleteReport::default();
    for path in paths {
        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
        match trash::delete(path) {
            Ok(()) => {
                report.files_removed += 1;
                report.bytes_freed += size;
            }
            Err(e) => report.failures.push(format!(
                "{} — {e}",
                path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            )),
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasted_bytes_counts_extra_copies() {
        let group = DuplicateGroup {
            size: 100,
            paths: vec![PathBuf::from("a"), PathBuf::from("b"), PathBuf::from("c")],
        };
        assert_eq!(group.wasted_bytes(), 200);
    }

    #[test]
    fn report_totals() {
        let report = DuplicateReport {
            scanned_files: 10,
            groups: vec![
                DuplicateGroup { size: 10, paths: vec![PathBuf::from("a"), PathBuf::from("b")] },
                DuplicateGroup { size: 50, paths: vec![PathBuf::from("c"), PathBuf::from("d"), PathBuf::from("e")] },
            ],
            cancelled: false,
            error: None,
        };
        assert_eq!(report.wasted_bytes(), 10 + 100);
        assert_eq!(report.extra_copies(), 3);
    }

    #[test]
    fn scan_finds_identical_files() {
        let root = std::env::temp_dir().join(format!("vulture-test-dup-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("x.bin"), b"identical-content").unwrap();
        fs::write(root.join("y.bin"), b"identical-content").unwrap();
        fs::write(root.join("z.bin"), b"different-content").unwrap();

        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        scan(&root, cancel, tx);

        let mut result = None;
        for msg in rx.iter() {
            if let DupMsg::Done(report) = msg {
                result = Some(report);
                break;
            }
        }
        let report = result.expect("scan should finish");
        assert!(report.error.is_none());
        assert_eq!(report.groups.len(), 1);
        assert_eq!(report.groups[0].paths.len(), 2);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn scan_honours_cancel_flag() {
        let root = std::env::temp_dir().join(format!("vulture-test-dupcancel-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        for i in 0..10 {
            fs::write(root.join(format!("f{i}.bin")), b"same-same-same-same").unwrap();
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(true)); // pre-cancelled
        scan(&root, cancel, tx);
        let mut result = None;
        for msg in rx.iter() {
            if let DupMsg::Done(report) = msg {
                result = Some(report);
                break;
            }
        }
        let report = result.expect("scan should finish");
        assert!(report.cancelled);
        assert!(report.groups.is_empty());
        let _ = fs::remove_dir_all(&root);
    }
}
