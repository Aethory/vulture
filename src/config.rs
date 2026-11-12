//! Persisted state: last-clean stats, totals and user preferences.
//! Stored as JSON in %APPDATA%\Vulture\vulture.json.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub last_clean_bytes: u64,
    pub last_clean_files: u64,
    pub last_clean_unix: u64,
    pub total_bytes_cleaned: u64,
    pub clean_runs: u64,
    /// Accent palette: "amber" | "teal" | "violet"
    pub accent: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            last_clean_bytes: 0,
            last_clean_files: 0,
            last_clean_unix: 0,
            total_bytes_cleaned: 0,
            clean_runs: 0,
            accent: "amber".to_string(),
        }
    }
}

pub fn data_dir() -> PathBuf {
    let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("Vulture");
    let _ = fs::create_dir_all(&p);
    p
}

pub fn logs_dir() -> PathBuf {
    let mut p = data_dir();
    p.push("logs");
    let _ = fs::create_dir_all(&p);
    p
}

fn config_path() -> PathBuf {
    data_dir().join("vulture.json")
}

impl Config {
    pub fn load() -> Self {
        fs::read_to_string(config_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(config_path(), json);
        }
    }

    pub fn record_clean(&mut self, bytes: u64, files: u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.last_clean_bytes = bytes;
        self.last_clean_files = files;
        self.last_clean_unix = now;
        self.total_bytes_cleaned = self.total_bytes_cleaned.saturating_add(bytes);
        self.clean_runs += 1;
        self.save();
    }

    pub fn reset_stats(&mut self) {
        self.last_clean_bytes = 0;
        self.last_clean_files = 0;
        self.last_clean_unix = 0;
        self.total_bytes_cleaned = 0;
        self.clean_runs = 0;
        self.save();
    }

    /// Human readable "2 days ago" for the last clean.
    pub fn last_clean_label(&self) -> String {
        if self.last_clean_unix == 0 {
            return "Never".to_string();
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let elapsed = now.saturating_sub(self.last_clean_unix);
        if elapsed < 60 {
            "Just now".to_string()
        } else if elapsed < 3_600 {
            format!("{} min ago", elapsed / 60)
        } else if elapsed < 86_400 {
            format!("{} h ago", elapsed / 3_600)
        } else {
            format!("{} days ago", elapsed / 86_400)
        }
    }
}
