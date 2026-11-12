//! Vulture's UI — one module per tab. Pure presentation; all system logic
//! lives in `crate::core`.

pub mod cleaner;
pub mod dashboard;
pub mod disk;
pub mod duplicates;
pub mod memory;
pub mod privacy;
pub mod scheduler;
pub mod settings;
pub mod startup;
pub mod uninstaller;

pub fn fmt_bytes(bytes: u64) -> String {
    humansize::format_size(bytes, humansize::BINARY)
}

pub fn fmt_gb(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
}

/// "3d 4h", "6h 12m", "42m" — compact uptime for tiles.
pub fn fmt_uptime(secs: u64) -> String {
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let mins = (secs % 3_600) / 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uptime_formats_compactly() {
        assert_eq!(fmt_uptime(45), "0m");
        assert_eq!(fmt_uptime(3_600 * 2 + 90), "2h 1m");
        assert_eq!(fmt_uptime(86_400 * 3 + 3600 * 4), "3d 4h");
    }

    #[test]
    fn fmt_gb_is_exact_for_gib() {
        assert_eq!(fmt_gb(1_073_741_824), "1.0 GB");
        assert_eq!(fmt_gb(0), "0.0 GB");
    }
}
