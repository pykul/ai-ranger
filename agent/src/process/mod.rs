/// Resolve a local source port to (pid, process_name).
/// Returns (0, "unknown") when attribution is not possible.
pub fn pid_and_name(src_port: u16) -> (u32, String) {
    match resolve_pid(src_port) {
        Some(pid) => {
            let name = process_name(pid).unwrap_or_else(|| format!("pid:{pid}"));
            (pid, name)
        }
        None => (0, "unknown".to_string()),
    }
}

fn resolve_pid(src_port: u16) -> Option<u32> {
    resolve_pid_impl(src_port)
}

fn process_name(pid: u32) -> Option<String> {
    process_name_impl(pid)
}

// ── Windows ──────────────────────────────────────────────────────────────────

// TODO Phase 1: replace netstat/tasklist shelling with GetExtendedTcpTable
// (iphlpapi.dll) for better performance under high connection rates.

#[cfg(windows)]
fn resolve_pid_impl(src_port: u16) -> Option<u32> {
    use std::process::Command;
    let out = Command::new("netstat")
        .args(["-ano", "-p", "TCP"])
        .output()
        .ok()?;
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 5 {
            continue;
        }
        if cols
            .get(1)
            .map_or(false, |a| a.ends_with(&format!(":{src_port}")))
        {
            return cols.last().and_then(|s| s.parse().ok());
        }
    }
    None
}

#[cfg(windows)]
fn process_name_impl(pid: u32) -> Option<String> {
    use std::process::Command;
    let out = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    // CSV: "process.exe","PID","Session Name","Session#","Mem Usage"
    text.lines()
        .next()
        .and_then(|l| l.split(',').next())
        .map(|s| s.trim_matches('"').to_string())
}

// ── Linux ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn resolve_pid_impl(src_port: u16) -> Option<u32> {
    use std::fs;
    let tcp = fs::read_to_string("/proc/net/tcp").ok()?;
    let port_hex = format!("{src_port:04X}");

    // Each row: sl local_address rem_address st tx_queue:rx_queue tr:tm->when retrnsmt uid timeout inode
    let inode: u64 = tcp.lines().skip(1).find_map(|line| {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if !cols
            .get(1)
            .is_some_and(|a| a.ends_with(&format!(":{port_hex}")))
        {
            return None;
        }
        cols.get(9)?.parse().ok()
    })?;

    // Walk /proc/<pid>/fd/* to find which process owns the socket inode
    let proc_dir = fs::read_dir("/proc").ok()?;
    for entry in proc_dir.flatten() {
        let pid: u32 = match entry.file_name().to_string_lossy().parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let Ok(fds) = fs::read_dir(format!("/proc/{pid}/fd")) else {
            continue;
        };
        for fd in fds.flatten() {
            if let Ok(target) = fs::read_link(fd.path()) {
                if target
                    .to_string_lossy()
                    .contains(&format!("socket:[{inode}]"))
                {
                    return Some(pid);
                }
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn process_name_impl(pid: u32) -> Option<String> {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|s| s.trim().to_string())
}

// ── macOS ─────────────────────────────────────────────────────────────────────

// TODO Phase 1: replace lsof/ps shelling with proc_pidinfo(PROC_PIDLISTFDS)
// for better performance under high connection rates.

#[cfg(target_os = "macos")]
fn resolve_pid_impl(src_port: u16) -> Option<u32> {
    use std::process::Command;
    // lsof -F p outputs lines like "p<pid>" for matching processes
    let out = Command::new("lsof")
        .args([
            "-i",
            &format!(":{src_port}"),
            "-sTCP:ESTABLISHED",
            "-n",
            "-P",
            "-F",
            "p",
        ])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| l.starts_with('p'))
        .and_then(|l| l[1..].parse().ok())
}

#[cfg(target_os = "macos")]
fn process_name_impl(pid: u32) -> Option<String> {
    use std::process::Command;
    let out = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .ok()?;
    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

// ── Unsupported platform fallback ─────────────────────────────────────────────

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn resolve_pid_impl(_src_port: u16) -> Option<u32> {
    None
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn process_name_impl(_pid: u32) -> Option<String> {
    None
}
