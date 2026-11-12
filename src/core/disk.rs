//! Disk usage analysis: per-folder breakdown and a "largest files" table.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct FolderSize {
    pub path: PathBuf,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
}

pub struct DiskReport {
    pub folders: Vec<FolderSize>,
    pub largest_files: Vec<FileEntry>,
    pub cancelled: bool,
}

fn dir_size(path: &Path, cancel: &Arc<AtomicBool>) -> u64 {
    let mut total = 0u64;
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if cancel.load(Ordering::Relaxed) {
            return total;
        }
        if let Ok(md) = entry.metadata() {
            if md.is_file() {
                total += md.len();
            }
        }
    }
    total
}

/// Analyze `root`: sizes of its immediate child folders plus the 40 largest
/// files anywhere under it. Cancellable at any point.
pub fn analyze(root: &Path, cancel: Arc<AtomicBool>) -> DiskReport {
    let mut folders = Vec::new();
    let mut heap: BinaryHeap<Reverse<(u64, PathBuf)>> = BinaryHeap::new();

    if let Ok(children) = std::fs::read_dir(root) {
        for child in children.filter_map(|c| c.ok()) {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let child_path = child.path();
            let size = if child_path.is_dir() {
                dir_size(&child_path, &cancel)
            } else {
                child.metadata().map(|m| m.len()).unwrap_or(0)
            };
            folders.push(FolderSize {
                path: child_path,
                size,
            });
        }
    }

    // Single pass for the largest files (skip hidden reparse-ish trees by size cap).
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let md = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !md.is_file() {
            continue;
        }
        let size = md.len();
        // Only spend time tracking files bigger than what's already in the heap.
        let min_in_heap = heap.peek().map(|Reverse((s, _))| *s).unwrap_or(0);
        if heap.len() < 40 || size > min_in_heap {
            heap.push(Reverse((size, entry.path().to_path_buf())));
            if heap.len() > 40 {
                heap.pop();
            }
        }
    }

    folders.sort_by_key(|f| Reverse(f.size));
    let mut largest_files: Vec<FileEntry> = heap
        .into_iter()
        .map(|Reverse((size, path))| FileEntry { path, size })
        .collect();
    largest_files.sort_by_key(|f| Reverse(f.size));

    DiskReport {
        folders,
        largest_files,
        cancelled: cancel.load(Ordering::Relaxed),
    }
}
