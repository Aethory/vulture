//! Startup program management: registry Run keys (HKCU / HKLM / Wow6432Node)
//! and the per-user Startup folder. Disabling always backs up first.

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use winreg::enums::*;
use winreg::RegKey;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartupLocation {
    HkcuRun,
    HklmRun,
    HklmRunWow64,
    StartupFolder,
}

impl StartupLocation {
    pub fn label(&self) -> &'static str {
        match self {
            StartupLocation::HkcuRun => "Registry (per user)",
            StartupLocation::HklmRun => "Registry (all users)",
            StartupLocation::HklmRunWow64 => "Registry (32-bit, all users)",
            StartupLocation::StartupFolder => "Startup folder",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupEntry {
    pub name: String,
    pub command: String,
    pub location: StartupLocation,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DisabledBackup {
    entries: Vec<DisabledEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DisabledEntry {
    name: String,
    command: String,
    location: StartupLocation,
    /// Original full path for StartupFolder entries (the file was moved away).
    folder_backup: Option<PathBuf>,
    /// Registry value type name (e.g. "REG_EXPAND_SZ"), if this came from a Run key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reg_type: Option<String>,
    /// Raw registry value bytes, preserving the exact original type on restore.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reg_bytes: Option<Vec<u8>>,
}

fn backup_path() -> PathBuf {
    crate::config::data_dir().join("disabled_startup.json")
}

fn load_backup() -> DisabledBackup {
    fs::read_to_string(backup_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_backup(backup: &DisabledBackup) {
    if let Ok(json) = serde_json::to_string_pretty(backup) {
        let _ = fs::write(backup_path(), json);
    }
}

fn disabled_folder() -> PathBuf {
    let p = crate::config::data_dir().join("disabled_startup");
    let _ = fs::create_dir_all(&p);
    p
}

fn startup_folder() -> PathBuf {
    let appdata = env::var("APPDATA").unwrap_or_default();
    PathBuf::from(appdata)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Startup")
}

fn registry_root(location: &StartupLocation) -> Option<RegKey> {
    match location {
        StartupLocation::HkcuRun => Some(RegKey::predef(HKEY_CURRENT_USER)),
        StartupLocation::HklmRun | StartupLocation::HklmRunWow64 => Some(RegKey::predef(HKEY_LOCAL_MACHINE)),
        StartupLocation::StartupFolder => None,
    }
}

fn registry_subkey(location: &StartupLocation) -> &'static str {
    match location {
        StartupLocation::HkcuRun => r"Software\Microsoft\Windows\CurrentVersion\Run",
        StartupLocation::HklmRun => r"Software\Microsoft\Windows\CurrentVersion\Run",
        StartupLocation::HklmRunWow64 => r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Run",
        StartupLocation::StartupFolder => "",
    }
}

fn reg_value_to_command(value: &winreg::RegValue) -> String {
    String::from_utf16_lossy(
        &value
            .bytes
            .chunks(2)
            .map(|c| u16::from_le_bytes([c[0], c.get(1).copied().unwrap_or(0)]))
            .collect::<Vec<u16>>(),
    )
    .trim_end_matches('\0')
    .to_string()
}

fn read_registry_entries(location: StartupLocation) -> Vec<StartupEntry> {
    let mut entries = Vec::new();
    let root = match registry_root(&location) {
        Some(r) => r,
        None => return entries,
    };
    let subkey = registry_subkey(&location);
    if let Ok(key) = root.open_subkey_with_flags(subkey, KEY_READ) {
        for (name, value) in key.enum_values().filter_map(|v| v.ok()) {
            entries.push(StartupEntry {
                name,
                command: reg_value_to_command(&value),
                location: location.clone(),
                enabled: true,
            });
        }
    }
    entries
}

pub fn list() -> Vec<StartupEntry> {
    let mut entries = Vec::new();
    entries.extend(read_registry_entries(StartupLocation::HkcuRun));
    entries.extend(read_registry_entries(StartupLocation::HklmRun));
    entries.extend(read_registry_entries(StartupLocation::HklmRunWow64));

    // Enabled Startup-folder shortcuts.
    if let Ok(rd) = fs::read_dir(startup_folder()) {
        for entry in rd.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                let name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Unknown shortcut".to_string());
                entries.push(StartupEntry {
                    name,
                    command: path.display().to_string(),
                    location: StartupLocation::StartupFolder,
                    enabled: true,
                });
            }
        }
    }

    // Backed-up (disabled) entries.
    let backup = load_backup();
    for d in &backup.entries {
        entries.push(StartupEntry {
            name: d.name.clone(),
            command: d.command.clone(),
            location: d.location.clone(),
            enabled: false,
        });
    }

    entries.sort_by(|a, b| b.enabled.cmp(&a.enabled).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    entries
}

/// Enable or disable an entry. Disabled entries are backed up to disk first,
/// so nothing is ever lost.
pub fn set_enabled(entry: &StartupEntry, enable: bool) -> Result<(), String> {
    if enable {
        enable_entry(entry)
    } else {
        disable_entry(entry)
    }
}

fn disable_entry(entry: &StartupEntry) -> Result<(), String> {
    let mut backup = load_backup();

    match entry.location {
        StartupLocation::StartupFolder => {
            let src = Path::new(&entry.command);
            if !src.exists() {
                return Err("The startup shortcut no longer exists.".to_string());
            }
            let file_name = src
                .file_name()
                .ok_or_else(|| "Invalid shortcut path.".to_string())?;
            let dst = disabled_folder().join(file_name);
            fs::rename(src, &dst).map_err(|e| format!("Could not move the shortcut: {e}"))?;
            backup.entries.push(DisabledEntry {
                name: entry.name.clone(),
                command: entry.command.clone(),
                location: entry.location.clone(),
                folder_backup: Some(dst),
                reg_type: None,
                reg_bytes: None,
            });
        }
        _ => {
            let root = registry_root(&entry.location).ok_or("Unknown registry hive.")?;
            let subkey = registry_subkey(&entry.location);
            let key = root
                .open_subkey_with_flags(subkey, KEY_WRITE | KEY_READ)
                .map_err(|e| format!("No access to this registry key — run Vulture as administrator. ({e})"))?;
            let value: winreg::RegValue = key
                .get_raw_value(&entry.name)
                .map_err(|e| format!("Could not read the value before removing it: {e}"))?;
            key.delete_value(&entry.name)
                .map_err(|e| format!("Could not remove the value — run Vulture as administrator. ({e})"))?;
            backup.entries.push(DisabledEntry {
                name: entry.name.clone(),
                command: reg_value_to_command(&value),
                location: entry.location.clone(),
                folder_backup: None,
                reg_type: Some(format!("{:?}", value.vtype)),
                reg_bytes: Some(value.bytes.to_vec()),
            });
        }
    }

    save_backup(&backup);
    Ok(())
}

fn enable_entry(entry: &StartupEntry) -> Result<(), String> {
    let mut backup = load_backup();
    let pos = backup
        .entries
        .iter()
        .position(|d| d.name == entry.name && d.location == entry.location)
        .ok_or("Backup entry not found — it may have been re-enabled already.")?;
    let record = backup.entries[pos].clone();

    match record.location {
        StartupLocation::StartupFolder => {
            let src = record
                .folder_backup
                .ok_or("Missing backup path for the shortcut.")?;
            let file_name = src
                .file_name()
                .ok_or("Invalid backup path.")?
                .to_os_string();
            let dst = startup_folder().join(file_name);
            fs::rename(&src, &dst).map_err(|e| format!("Could not restore the shortcut: {e}"))?;
        }
        _ => {
            use winreg::RegValue;
            let root = registry_root(&record.location).ok_or("Unknown registry hive.")?;
            let subkey = registry_subkey(&record.location);
            let key = root
                .open_subkey_with_flags(subkey, KEY_WRITE)
                .map_err(|e| format!("No access to this registry key — run Vulture as administrator. ({e})"))?;
            match &record.reg_bytes {
                // Restore the exact original value type and payload when we have it.
                Some(bytes) => {
                    let vtype = match record.reg_type.as_deref() {
                        Some("REG_EXPAND_SZ") => winreg::enums::REG_EXPAND_SZ,
                        Some("REG_MULTI_SZ") => winreg::enums::REG_MULTI_SZ,
                        Some("REG_BINARY") => winreg::enums::REG_BINARY,
                        Some("REG_DWORD") => winreg::enums::REG_DWORD,
                        Some("REG_QWORD") => winreg::enums::REG_QWORD,
                        _ => winreg::enums::REG_SZ,
                    };
                    key.set_raw_value(
                        &record.name,
                        &RegValue { vtype, bytes: std::borrow::Cow::Borrowed(bytes.as_slice()) },
                    )
                    .map_err(|e| format!("Could not restore the value: {e}"))?;
                }
                // Legacy backup (pre-v1 backup files): fall back to a plain string.
                None => key
                    .set_value(&record.name, &record.command)
                    .map_err(|e| format!("Could not restore the value: {e}"))?,
            }
        }
    }

    backup.entries.remove(pos);
    save_backup(&backup);
    Ok(())
}
