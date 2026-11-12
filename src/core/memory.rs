//! Memory stats, process listing and the RAM boost (working-set trim).

use sysinfo::{Pid, ProcessesToUpdate, System};

#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub pid: u32,
    pub name: String,
    pub memory_bytes: u64,
    pub cpu_percent: f32,
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub cpu_percent: f32,
    pub uptime_secs: u64,
    pub processes: Vec<ProcInfo>,
}

pub fn collect(sys: &mut System) -> MemoryStats {
    sys.refresh_memory();
    sys.refresh_cpu_usage();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let mut processes: Vec<ProcInfo> = sys
        .processes()
        .iter()
        .map(|(pid, p)| ProcInfo {
            pid: pid.as_u32(),
            name: p.name().to_string_lossy().into_owned(),
            memory_bytes: p.memory(),
            cpu_percent: p.cpu_usage(),
        })
        .collect();
    processes.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
    processes.truncate(40);

    MemoryStats {
        total_bytes: sys.total_memory(),
        used_bytes: sys.used_memory(),
        cpu_percent: sys.global_cpu_usage(),
        uptime_secs: System::uptime(),
        processes,
    }
}

/// Names Windows either forbids terminating or that would take the desktop with them.
const BLOCKED: &[&str] = &[
    "system", "idle", "registry", "memory compression", "smss.exe", "csrss.exe",
    "wininit.exe", "winlogon.exe", "services.exe", "lsass.exe", "lsaiso.exe",
    "svchost.exe", "dwm.exe", "fontdrvhost.exe", "explorer.exe", "vulture.exe",
    "sihost.exe", "ctfmon.exe",
];

fn is_blocked(name: &str) -> bool {
    let lower = name.to_lowercase();
    BLOCKED.iter().any(|b| lower == *b)
}

/// Trim the working set of every accessible, non-critical process.
/// Pages get released back to the OS and reloaded on demand — classic "RAM boost".
/// Returns how many processes were actually trimmed.
pub fn boost(sys: &System) -> usize {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA};

    let self_pid = std::process::id();
    let mut trimmed = 0usize;

    for (pid, process) in sys.processes() {
        let pid_u32 = pid.as_u32();
        if pid_u32 <= 4 || pid_u32 == self_pid {
            continue;
        }
        if is_blocked(&process.name().to_string_lossy()) {
            continue;
        }
        unsafe {
            if let Ok(handle) = OpenProcess(PROCESS_SET_QUOTA | PROCESS_QUERY_INFORMATION, false, pid_u32) {
                if EmptyWorkingSet(handle).as_bool() {
                    trimmed += 1;
                }
                let _ = CloseHandle(handle);
            }
        }
    }
    trimmed
}

/// Terminate a process by pid. Refuses to touch critical Windows processes.
pub fn kill(sys: &mut System, target: &ProcInfo) -> Result<(), String> {
    if is_blocked(&target.name) {
        return Err(format!(
            "Windows protects '{}' — ending it could crash the machine.",
            target.name
        ));
    }
    if target.pid == std::process::id() {
        return Err("That's Vulture itself — it declines to be killed.".to_string());
    }
    match sys.process(Pid::from_u32(target.pid)) {
        Some(p) => {
            if p.kill() {
                Ok(())
            } else {
                Err("Windows refused the request (the process may need administrator rights).".to_string())
            }
        }
        None => Err("The process already exited.".to_string()),
    }
}
