//! Windows Task Scheduler integration for the daily auto-clean.

use std::process::Command;

pub const TASK_NAME: &str = "Vulture Auto Clean";
const RUN_TIME: &str = "02:30";

pub fn is_enabled() -> bool {
    Command::new("schtasks")
        .args(["/Query", "/TN", TASK_NAME])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn enable() -> Result<(), String> {
    let exe =
        std::env::current_exe().map_err(|e| format!("Could not resolve Vulture's path: {e}"))?;
    let task_run = format!("\"{}\" --auto-clean", exe.display());

    let output = Command::new("schtasks")
        .args([
            "/Create", "/TN", TASK_NAME, "/TR", &task_run, "/SC", "DAILY", "/ST", RUN_TIME, "/F",
        ])
        .output()
        .map_err(|e| format!("Could not run schtasks: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!(
            "schtasks refused the task: {}{}",
            stderr.trim(),
            stdout.trim()
        )
        .trim()
        .to_string())
    }
}

pub fn disable() -> Result<(), String> {
    let output = Command::new("schtasks")
        .args(["/Delete", "/TN", TASK_NAME, "/F"])
        .output()
        .map_err(|e| format!("Could not run schtasks: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Could not delete the task: {}", stderr.trim()))
    }
}
