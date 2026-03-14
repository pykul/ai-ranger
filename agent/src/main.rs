mod capture;
mod classifier;

use chrono::Utc;
use serde::Serialize;

#[derive(Serialize)]
struct AiConnectionEvent {
    agent_id: String,
    machine_hostname: String,
    os_username: String,
    timestamp_ms: i64,
    provider: String,
    provider_host: String,
    detection_method: &'static str,
    process_name: String,
    process_pid: u32,
    src_ip: String,
    capture_mode: &'static str,
}

fn main() {
    eprintln!("[ai-ranger] Phase 0 spike — SNI-based AI provider detection");
    eprintln!("[ai-ranger] Monitoring outbound port 443 for connections to known AI providers.");
    eprintln!("[ai-ranger] Requires administrator (Windows) or sudo (Linux/macOS).");
    eprintln!("[ai-ranger] Press Ctrl+C to stop.\n");

    if let Err(e) = capture::pcap::capture(|packet| {
        let Some(provider) = classifier::classify(&packet.sni_hostname) else {
            return;
        };

        let (process_pid, process_name) = pid_and_name(packet.src_port);

        let event = AiConnectionEvent {
            agent_id: String::new(),
            machine_hostname: String::new(),
            os_username: String::new(),
            timestamp_ms: Utc::now().timestamp_millis(),
            provider: provider.to_string(),
            provider_host: packet.sni_hostname,
            detection_method: "sni",
            process_name,
            process_pid,
            src_ip: packet.src_ip,
            capture_mode: "DNS_SNI",
        };

        println!("{}", serde_json::to_string(&event).unwrap());
    }) {
        eprintln!("[ai-ranger] Capture error: {e}");
        eprintln!("[ai-ranger] Windows: run as Administrator.");
        eprintln!("[ai-ranger] Linux/macOS: run with sudo.");
        std::process::exit(1);
    }
}

/// Resolve a local source port to (pid, process_name).
/// Returns (0, "unknown") when attribution is not possible.
fn pid_and_name(src_port: u16) -> (u32, String) {
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
