/// Resolve a local source port to (pid, process_name).
/// Returns (0, "unknown") when attribution is not possible.
///
/// All platforms resolve the process that owns the socket at the moment of
/// capture. Short-lived child processes (e.g. curl spawned from a shell) may
/// resolve to their parent process if the child exits before the OS table is
/// read. This is expected behavior and does not affect real AI tool detection
/// since tools like Cursor, Claude Code, and Python scripts own their sockets
/// directly.
pub fn pid_and_name(src_port: u16) -> (u32, String) {
    match resolve_pid(src_port) {
        Some(pid) => {
            let name = process_name(pid).unwrap_or_else(|| "unknown".to_string());
            (pid, name)
        }
        None => (0, "unknown".to_string()),
    }
}

/// Resolve a PID to a process name. Windows-only — used by the ETW DNS path
/// which provides the PID directly (no port→PID lookup needed).
#[cfg(windows)]
pub fn name_by_pid(pid: u32) -> String {
    process_name(pid).unwrap_or_else(|| "unknown".to_string())
}

/// Return the full executable path for a PID, or None if unavailable.
pub fn process_path(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    process_path_impl(pid)
}

fn resolve_pid(src_port: u16) -> Option<u32> {
    resolve_pid_impl(src_port)
}

fn process_name(pid: u32) -> Option<String> {
    process_name_impl(pid)
}

// ── Windows ──────────────────────────────────────────────────────────────────
//
// Uses GetExtendedTcpTable (iphlpapi.dll) for port→PID resolution and
// QueryFullProcessImageNameW (kernel32) for PID→name. No shell, no subprocess.

#[cfg(windows)]
fn resolve_pid_impl(src_port: u16) -> Option<u32> {
    use windows::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_ALL,
    };
    use windows::Win32::Networking::WinSock::AF_INET;

    // dwLocalPort stores the port as a big-endian u16 zero-extended to u32.
    // Example: port 443 (0x01BB) → dwLocalPort = 0x0000BB01 = 47873.
    // So we swap the u16 bytes first, then widen to u32.
    let target_port = src_port.to_be() as u32;

    // First call to determine required buffer size.
    let mut size: u32 = 0;
    unsafe {
        let _ = GetExtendedTcpTable(
            None,
            &mut size,
            false,
            AF_INET.0 as u32,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        );
    }

    let mut buf = vec![0u8; size as usize];
    let ret = unsafe {
        GetExtendedTcpTable(
            Some(buf.as_mut_ptr() as *mut _),
            &mut size,
            false,
            AF_INET.0 as u32,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        )
    };
    if ret != 0 {
        return None;
    }

    let table = unsafe { &*(buf.as_ptr() as *const MIB_TCPTABLE_OWNER_PID) };
    let rows = unsafe {
        std::slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize)
    };

    for row in rows {
        if row.dwLocalPort == target_port {
            return Some(row.dwOwningPid);
        }
    }
    None
}

#[cfg(windows)]
fn query_full_image_path(pid: u32) -> Option<String> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()? };

    let mut buf = [0u16; 260]; // MAX_PATH
    let mut len = buf.len() as u32;
    let ok = unsafe {
        QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len)
    };
    unsafe { let _ = CloseHandle(handle); }

    if ok.is_err() {
        return None;
    }

    Some(String::from_utf16_lossy(&buf[..len as usize]))
}

#[cfg(windows)]
fn process_name_impl(pid: u32) -> Option<String> {
    let full_path = query_full_image_path(pid)?;
    // Extract filename from full path (e.g. "C:\...\curl.exe" → "curl.exe")
    full_path.rsplit('\\').next().map(|s| s.to_string())
}

#[cfg(windows)]
fn process_path_impl(pid: u32) -> Option<String> {
    query_full_image_path(pid)
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

#[cfg(target_os = "linux")]
fn process_path_impl(pid: u32) -> Option<String> {
    std::fs::read_link(format!("/proc/{pid}/exe"))
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

// ── macOS ─────────────────────────────────────────────────────────────────────
//
// MACOS-UNVERIFIED: This entire block requires compile-test on Apple hardware.
// Specifically:
//   - proc_pidinfo / proc_listallpids / proc_name FFI declarations
//   - SocketFdInfo, InSockInfo, and related struct layouts and sizes
//   - Field offsets (insi_lport, insi_fport) in the in_sockinfo struct
//
// Uses proc_pidinfo(PROC_PIDLISTFDS) to find which process owns a socket on
// a given port, and proc_name() for PID→name. No shell, no subprocess.

#[cfg(target_os = "macos")]
mod macos_proc {
    use libc::{c_char, c_int, c_void};

    // libproc constants
    const PROC_PIDLISTFDS: c_int = 1;
    const PROX_FDTYPE_SOCKET: u32 = 2;
    const PROC_PIDFDSOCKETINFO: c_int = 3;

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct ProcFdInfo {
        pub proc_fd: i32,
        pub proc_fdtype: u32,
    }

    // socket_fdinfo contains the socket info we need
    // We only care about the local port, which is at a known offset.
    // Full struct is large (~700 bytes) — we define only what we need.
    #[repr(C)]
    pub struct SocketFdInfo {
        pub pfi: ProcFdInfo,
        pub psi: SocketInfo,
    }

    #[repr(C)]
    pub struct SocketInfo {
        pub soi_stat: SoiStat,
        pub soi_family: c_int,
        pub soi_type: c_int,
        pub soi_protocol: c_int,
        pub soi_proto: SoiProto,
    }

    #[repr(C)]
    pub struct SoiStat {
        _pad: [u8; 168], // stat fields we don't need
    }

    #[repr(C)]
    pub struct SoiProto {
        pub pri_tcp: PriTcp,
    }

    #[repr(C)]
    pub struct PriTcp {
        pub tcpsi_ini: InSockInfo,
        _rest: [u8; 64], // remaining TCP-specific fields
    }

    #[repr(C)]
    pub struct InSockInfo {
        pub insi_fport: c_int,  // foreign port
        pub insi_lport: c_int,  // local port
        _rest: [u8; 376],       // remaining in_sockinfo fields
    }

    extern "C" {
        pub fn proc_listallpids(buffer: *mut c_void, bufsize: c_int) -> c_int;
        pub fn proc_pidinfo(
            pid: c_int,
            flavor: c_int,
            arg: u64,
            buffer: *mut c_void,
            bufsize: c_int,
        ) -> c_int;
        pub fn proc_name(pid: c_int, buffer: *mut c_char, bufsize: u32) -> c_int;
        // MACOS-UNVERIFIED: proc_pidpath FFI declaration. Requires compile-test
        // on Apple hardware to verify buffer size and return value semantics.
        pub fn proc_pidpath(pid: c_int, buffer: *mut c_char, bufsize: u32) -> c_int;
    }

    pub fn find_pid_for_port(src_port: u16) -> Option<u32> {
        let target_port = src_port as c_int;

        // Get all PIDs
        let count = unsafe { proc_listallpids(std::ptr::null_mut(), 0) };
        if count <= 0 {
            return None;
        }
        let mut pids = vec![0i32; count as usize];
        let ret = unsafe {
            proc_listallpids(
                pids.as_mut_ptr() as *mut c_void,
                (pids.len() * std::mem::size_of::<i32>()) as c_int,
            )
        };
        if ret <= 0 {
            return None;
        }
        let pid_count = ret as usize;

        for &pid in &pids[..pid_count] {
            if pid <= 0 {
                continue;
            }

            // Get FD list for this process
            let fd_size = unsafe {
                proc_pidinfo(pid, PROC_PIDLISTFDS, 0, std::ptr::null_mut(), 0)
            };
            if fd_size <= 0 {
                continue;
            }
            let fd_count = fd_size as usize / std::mem::size_of::<ProcFdInfo>();
            let mut fds = vec![
                ProcFdInfo {
                    proc_fd: 0,
                    proc_fdtype: 0,
                };
                fd_count
            ];
            let ret = unsafe {
                proc_pidinfo(
                    pid,
                    PROC_PIDLISTFDS,
                    0,
                    fds.as_mut_ptr() as *mut c_void,
                    fd_size,
                )
            };
            if ret <= 0 {
                continue;
            }

            for fd_info in &fds {
                if fd_info.proc_fdtype != PROX_FDTYPE_SOCKET {
                    continue;
                }
                let mut si: SocketFdInfo = unsafe { std::mem::zeroed() };
                let ret = unsafe {
                    proc_pidinfo(
                        pid,
                        PROC_PIDFDSOCKETINFO,
                        fd_info.proc_fd as u64,
                        &mut si as *mut _ as *mut c_void,
                        std::mem::size_of::<SocketFdInfo>() as c_int,
                    )
                };
                if ret <= 0 {
                    continue;
                }
                // AF_INET = 2, SOCK_STREAM = 1
                if si.psi.soi_family == 2 && si.psi.soi_type == 1 {
                    let local_port = si.psi.soi_proto.pri_tcp.tcpsi_ini.insi_lport;
                    if local_port == target_port {
                        return Some(pid as u32);
                    }
                }
            }
        }

        None
    }

    // MACOS-UNVERIFIED: proc_pidpath returns the full executable path for a PID.
    // Requires compile-test on Apple hardware.
    pub fn get_process_path(pid: u32) -> Option<String> {
        const PROC_PIDPATHINFO_MAXSIZE: u32 = 4096;
        let mut buf = [0u8; PROC_PIDPATHINFO_MAXSIZE as usize];
        let ret = unsafe {
            proc_pidpath(
                pid as c_int,
                buf.as_mut_ptr() as *mut c_char,
                PROC_PIDPATHINFO_MAXSIZE,
            )
        };
        if ret <= 0 {
            return None;
        }
        let path = String::from_utf8_lossy(&buf[..ret as usize])
            .trim_end_matches('\0')
            .to_string();
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    }

    pub fn get_process_name(pid: u32) -> Option<String> {
        let mut buf = [0u8; 256];
        let ret = unsafe {
            proc_name(
                pid as c_int,
                buf.as_mut_ptr() as *mut c_char,
                buf.len() as u32,
            )
        };
        if ret <= 0 {
            return None;
        }
        let name = String::from_utf8_lossy(&buf[..ret as usize])
            .trim_end_matches('\0')
            .to_string();
        if name.is_empty() {
            None
        } else {
            Some(name)
        }
    }
}

#[cfg(target_os = "macos")]
fn resolve_pid_impl(src_port: u16) -> Option<u32> {
    macos_proc::find_pid_for_port(src_port)
}

#[cfg(target_os = "macos")]
fn process_name_impl(pid: u32) -> Option<String> {
    macos_proc::get_process_name(pid)
}

#[cfg(target_os = "macos")]
fn process_path_impl(pid: u32) -> Option<String> {
    macos_proc::get_process_path(pid)
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

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn process_path_impl(_pid: u32) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unresolvable_port_returns_unknown() {
        // Port 0 should never resolve to a real process.
        let (pid, name) = pid_and_name(0);
        assert_eq!(pid, 0);
        assert_eq!(name, "unknown");
    }

    #[test]
    fn nonexistent_pid_returns_unknown() {
        // PID u32::MAX should not exist on any system.
        let name = process_name(u32::MAX);
        assert!(name.is_none());
    }

    #[test]
    fn fallback_format_is_unknown_not_pid() {
        // When process_name returns None, the fallback should be "unknown",
        // not "pid:N" or any other format.
        let fallback = process_name(u32::MAX).unwrap_or_else(|| "unknown".to_string());
        assert_eq!(fallback, "unknown");
        assert!(!fallback.starts_with("pid:"));
    }
}
