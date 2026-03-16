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

/// Resolve a PID to a process name. Windows-only - used by the ETW DNS path
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
    let rows =
        unsafe { std::slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize) };

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

    /// Windows MAX_PATH - maximum length of a file path in the Win32 API.
    const WIN_MAX_PATH: usize = 260;

    let mut buf = [0u16; WIN_MAX_PATH];
    let mut len = buf.len() as u32;
    let ok = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
    };
    unsafe {
        let _ = CloseHandle(handle);
    }

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
// Uses proc_pidinfo(PROC_PIDLISTFDS) to find which process owns a socket on
// a given port, and proc_name() for PID→name. No shell, no subprocess.
//
// For PROC_PIDFDSOCKETINFO, we use a raw byte buffer instead of repr(C)
// structs because the XNU socket_fdinfo struct hierarchy is deeply nested
// and the struct sizes/offsets are critical - a wrong padding byte breaks
// the proc_pidinfo call entirely (it validates exact buffer size).
//
// Offsets computed from XNU bsd/sys/proc_info.h (see comments in code).

#[cfg(target_os = "macos")]
mod macos_proc {
    use libc::{c_char, c_int, c_void};

    // libproc constants
    const PROC_PIDLISTFDS: c_int = 1;
    const PROC_FDTYPE_SOCKET: u32 = 2;
    const PROC_PIDFDSOCKETINFO: c_int = 3;

    // sizeof(struct socket_fdinfo) on 64-bit macOS.
    //
    // socket_fdinfo = proc_fileinfo(24) + socket_info(376) = 400 bytes
    //
    // struct proc_fileinfo:
    //   fi_openflags(u32) + fi_status(u32) + fi_offset(i64)
    //   + fi_type(i32) + fi_guardflags(u32) = 24 bytes
    //
    // struct socket_info:
    //   vinfo_stat(136) + soi_so(u64) + soi_pcb(u64) + soi_type(i32)
    //   + soi_protocol(i32) + soi_proto(union, 120) + soi_family(i32)
    //   + soi_options(i32) + soi_linger(i32) + soi_state(i32)
    //   + soi_qlen(i32) + soi_incqlen(i32) + soi_qlimit(i32)
    //   + soi_timeo(i32) + soi_error(u16) + pad(2) + soi_oobmark(u32)
    //   + soi_rcv(sockbuf_info, 24) + soi_snd(sockbuf_info, 24)
    //   + soi_kind(i32) + soi_reservedspace(u32) = 376 bytes
    const SOCKET_FDINFO_SIZE: c_int = 400;

    // Offsets within socket_fdinfo for the fields we need:
    //
    // socket_info starts at byte 24 (after proc_fileinfo).
    //   soi_type    = 24 + 136(vinfo_stat) + 8(soi_so) + 8(soi_pcb) = 176
    //   soi_proto   = 24 + 136 + 8 + 8 + 4(soi_type) + 4(soi_protocol) = 184
    //   insi_lport  = soi_proto + 4(insi_fport) = 188
    //   soi_family  = soi_proto + 120(union size) = 304
    const OFF_SOI_TYPE: usize = 176;
    const OFF_INSI_LPORT: usize = 188;
    const OFF_SOI_FAMILY: usize = 304;

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct ProcFdInfo {
        pub proc_fd: i32,
        pub proc_fdtype: u32,
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
        pub fn proc_pidpath(pid: c_int, buffer: *mut c_char, bufsize: u32) -> c_int;
    }

    /// Read a little-endian i32 from a byte slice at the given offset.
    fn read_i32(buf: &[u8], off: usize) -> i32 {
        i32::from_ne_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]])
    }

    pub fn find_pid_for_port(src_port: u16) -> Option<u32> {
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
            let fd_size = unsafe { proc_pidinfo(pid, PROC_PIDLISTFDS, 0, std::ptr::null_mut(), 0) };
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
                if fd_info.proc_fdtype != PROC_FDTYPE_SOCKET {
                    continue;
                }

                // Use a raw byte buffer - proc_pidinfo validates exact size.
                let mut buf = [0u8; SOCKET_FDINFO_SIZE as usize];
                let ret = unsafe {
                    proc_pidinfo(
                        pid,
                        PROC_PIDFDSOCKETINFO,
                        fd_info.proc_fd as u64,
                        buf.as_mut_ptr() as *mut c_void,
                        SOCKET_FDINFO_SIZE,
                    )
                };
                if ret != SOCKET_FDINFO_SIZE {
                    continue;
                }

                const SOCK_STREAM: i32 = 1;
                const AF_INET_VAL: i32 = 2;
                const AF_INET6_VAL: i32 = 30;

                let soi_type = read_i32(&buf, OFF_SOI_TYPE);
                let soi_family = read_i32(&buf, OFF_SOI_FAMILY);
                if soi_type != SOCK_STREAM
                    || (soi_family != AF_INET_VAL && soi_family != AF_INET6_VAL)
                {
                    continue;
                }

                // insi_lport is in network byte order (big-endian u16 in an i32).
                let raw_port = read_i32(&buf, OFF_INSI_LPORT);
                let local_port = u16::from_be((raw_port & 0xFFFF) as u16);

                if local_port == src_port {
                    return Some(pid as u32);
                }
            }
        }

        None
    }

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

    /// Maximum buffer size for proc_name() results on macOS.
    const PROC_NAME_BUFFER_SIZE: usize = 256;

    pub fn get_process_name(pid: u32) -> Option<String> {
        let mut buf = [0u8; PROC_NAME_BUFFER_SIZE];
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
