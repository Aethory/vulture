//! Installed-program enumeration and uninstall launching.
//! Reads the three Uninstall registry hives.

use serde::Serialize;
use std::path::PathBuf;
use winreg::enums::*;
use winreg::{RegKey, HKEY};

#[derive(Debug, Clone, Serialize)]
pub struct InstalledApp {
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub install_location: Option<PathBuf>,
    /// Bytes (registry stores KB).
    pub estimated_size: u64,
    pub uninstall_string: String,
    pub quiet_uninstall_string: Option<String>,
    pub source: &'static str,
}

fn hives() -> [(&'static str, &'static str, HKEY); 3] {
    [
        (
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
            "HKLM (64-bit)",
            HKEY_LOCAL_MACHINE,
        ),
        (
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
            "HKLM (32-bit)",
            HKEY_LOCAL_MACHINE,
        ),
        (
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
            "HKCU",
            HKEY_CURRENT_USER,
        ),
    ]
}

pub fn list() -> Vec<InstalledApp> {
    let mut apps: Vec<InstalledApp> = Vec::new();

    for (subkey, source, hive) in hives() {
        let root = RegKey::predef(hive);
        let key = match root.open_subkey_with_flags(subkey, KEY_READ) {
            Ok(k) => k,
            Err(_) => continue,
        };
        for name in key.enum_keys().filter_map(|k| k.ok()) {
            let sub = match key.open_subkey_with_flags(&name, KEY_READ) {
                Ok(k) => k,
                Err(_) => continue,
            };
            // Skip system components and things that shouldn't be in the list.
            if sub
                .get_value::<u32, _>("SystemComponent")
                .map(|v| v == 1)
                .unwrap_or(false)
            {
                continue;
            }
            if sub.get_value::<String, _>("ParentKeyName").is_ok() {
                continue;
            }
            let display_name = match sub.get_value::<String, _>("DisplayName") {
                Ok(n) if !n.trim().is_empty() => n,
                _ => continue,
            };
            if display_name.starts_with("Update for ")
                || display_name.starts_with("Security Update")
            {
                continue;
            }

            let estimated_kb = sub.get_value::<u32, _>("EstimatedSize").unwrap_or(0) as u64;
            let install_location = sub
                .get_value::<String, _>("InstallLocation")
                .ok()
                .map(|s| PathBuf::from(s.trim_matches('"').trim()))
                .filter(|p| !p.as_os_str().is_empty());

            apps.push(InstalledApp {
                name: display_name,
                version: sub
                    .get_value::<String, _>("DisplayVersion")
                    .unwrap_or_default(),
                publisher: sub
                    .get_value::<String, _>("Publisher")
                    .unwrap_or_else(|_| "Unknown publisher".to_string()),
                install_location,
                estimated_size: estimated_kb.saturating_mul(1024),
                uninstall_string: sub
                    .get_value::<String, _>("UninstallString")
                    .unwrap_or_default(),
                quiet_uninstall_string: sub.get_value::<String, _>("QuietUninstallString").ok(),
                source,
            });
        }
    }

    apps.sort_by_key(|a| std::cmp::Reverse(a.estimated_size));
    apps
}

/// Split an UninstallString into (program, args). Handles a quoted program with
/// spaces (`"C:\foo bar\unins000.exe" /S`), an unquoted path followed by switches,
/// and msiexec-style `msiexec /I{guid}` invocations.
pub fn parse_uninstall_command(command: &str) -> Option<(String, String)> {
    let command = command.trim();
    if command.is_empty() {
        return None;
    }
    if let Some(rest) = command.strip_prefix('"') {
        // Quoted program: everything up to the closing quote.
        let end = rest.find('"')?;
        let program = rest[..end].trim().to_string();
        let args = rest[end + 1..].trim().to_string();
        return (!program.is_empty()).then_some((program, args));
    }
    // Unquoted: the executable usually carries an extension; find its end by
    // scanning for the first .exe / .bat / .cmd / .msi token.
    let lower = command.to_lowercase();
    for ext in [".exe", ".bat", ".cmd", ".msi"] {
        if let Some(pos) = lower.find(ext) {
            let program = command[..pos + ext.len()].trim().to_string();
            let args = command[pos + ext.len()..].trim().to_string();
            if !program.is_empty() {
                return Some((program, args));
            }
        }
    }
    // No recognizable executable (e.g. `msiexec ...`): first word is the program.
    let mut parts = command.splitn(2, char::is_whitespace);
    let program = parts.next()?.trim().to_string();
    let args = parts.next().unwrap_or("").trim().to_string();
    (!program.is_empty()).then_some((program, args))
}

/// Launch the program's uninstaller. Prefers the interactive uninstall string
/// — a one-click quiet uninstall would run with zero prompts, so the silent
/// switch is only a fallback for entries that ship no interactive command.
pub fn launch_uninstaller(app: &InstalledApp) -> Result<(), String> {
    let interactive = app.uninstall_string.trim();
    let command = if !interactive.is_empty() {
        interactive
    } else {
        app.quiet_uninstall_string.as_deref().unwrap_or("").trim()
    };

    if command.is_empty() {
        return Err(
            "This entry has no uninstall command — it may be a system component.".to_string(),
        );
    }

    // Pre-flight: if the uninstaller is a clear absolute path that is gone, fail
    // NOW instead of reporting a phantom success from a fire-and-forget spawn.
    if let Some((program, _)) = parse_uninstall_command(command) {
        let looks_like_absolute = program.len() >= 3
            && program.as_bytes()[1] == b':'
            && [".exe", ".bat", ".cmd", ".msi"]
                .iter()
                .any(|ext| program.to_lowercase().ends_with(ext));
        if looks_like_absolute && !std::path::Path::new(&program).exists() {
            return Err(format!(
                "Uninstaller not found at {program} — it may have been removed already."
            ));
        }
    }

    // /S tells cmd to strip the outer quotes being added — otherwise the
    // registry's quoted, space-laden paths break cmd's /C parsing ("not recognized").
    std::process::Command::new("cmd")
        .args(["/S", "/C", command])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not start the uninstaller: {e}"))
}

pub fn open_install_folder(app: &InstalledApp) -> Result<(), String> {
    match &app.install_location {
        Some(dir) if dir.exists() => std::process::Command::new("explorer")
            .arg(dir)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Could not open Explorer: {e}")),
        _ => Err("No install location recorded for this program.".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_path_with_spaces() {
        let (program, args) =
            parse_uninstall_command(r#""C:\Games\Grand Theft Auto V Enhanced\unins000.exe""#)
                .unwrap();
        assert_eq!(
            program,
            r"C:\Games\Grand Theft Auto V Enhanced\unins000.exe"
        );
        assert_eq!(args, "");
    }

    #[test]
    fn parses_quoted_path_with_args() {
        let (program, args) =
            parse_uninstall_command(r#""C:\Foo Bar\unins.exe" /S /NORESTART"#).unwrap();
        assert_eq!(program, r"C:\Foo Bar\unins.exe");
        assert_eq!(args, "/S /NORESTART");
    }

    #[test]
    fn parses_unquoted_path_with_spaces() {
        let (program, args) =
            parse_uninstall_command(r"C:\Program Files\Foo\unins.exe /S").unwrap();
        assert_eq!(program, r"C:\Program Files\Foo\unins.exe");
        assert_eq!(args, "/S");
    }

    #[test]
    fn parses_msiexec_invocation() {
        let (program, args) =
            parse_uninstall_command(r"msiexec /I{83D41AC9-7E8C-11ED-A812-0242AC120003}").unwrap();
        assert_eq!(program, "msiexec");
        assert!(args.starts_with("/I"));
    }

    #[test]
    fn rejects_empty_command() {
        assert!(parse_uninstall_command("   ").is_none());
    }
}
