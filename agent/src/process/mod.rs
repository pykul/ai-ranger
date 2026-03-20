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

/// Resolve the OS username that owns the process with the given PID.
///
/// This is a best-effort synchronous lookup. It will never panic or block
/// the event pipeline. All failures are silent with graceful fallback.
/// If this proves slow under load it should be moved to a background resolver.
///
/// Returns:
/// - The username of the process owner on success (e.g. "omria", "john")
/// - "unknown" if the PID exists but resolution fails for any reason
/// - Should not be called with pid == 0 (caller handles that case with a fallback)
pub fn resolve_process_owner(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    resolve_owner_impl(pid)
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

/// Resolve the owner of a process on Windows using OpenProcessToken +
/// GetTokenInformation(TokenUser) + LookupAccountSidW.
#[cfg(windows)]
fn resolve_owner_impl(pid: u32) -> Option<String> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        GetTokenInformation, LookupAccountSidW, TokenUser, SID_NAME_USE, TOKEN_QUERY, TOKEN_USER,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    // Open process handle.
    let proc_handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()? };

    // Open process token.
    let mut token_handle = HANDLE::default();
    let ok = unsafe { OpenProcessToken(proc_handle, TOKEN_QUERY, &mut token_handle) };
    unsafe {
        let _ = CloseHandle(proc_handle);
    }
    if ok.is_err() {
        return None;
    }

    // Query token user info - first call to get required buffer size.
    let mut needed: u32 = 0;
    let _ = unsafe { GetTokenInformation(token_handle, TokenUser, None, 0, &mut needed) };
    if needed == 0 {
        unsafe {
            let _ = CloseHandle(token_handle);
        }
        return None;
    }

    let mut buf = vec![0u8; needed as usize];
    let ok = unsafe {
        GetTokenInformation(
            token_handle,
            TokenUser,
            Some(buf.as_mut_ptr() as *mut _),
            needed,
            &mut needed,
        )
    };
    unsafe {
        let _ = CloseHandle(token_handle);
    }
    if ok.is_err() {
        return None;
    }

    let token_user = unsafe { &*(buf.as_ptr() as *const TOKEN_USER) };
    let sid = token_user.User.Sid;

    // Lookup account name from SID.
    let mut name_len: u32 = 256;
    let mut domain_len: u32 = 256;
    let mut name_buf = vec![0u16; name_len as usize];
    let mut domain_buf = vec![0u16; domain_len as usize];
    let mut sid_type = SID_NAME_USE::default();

    let ok = unsafe {
        LookupAccountSidW(
            None,
            sid,
            Some(PWSTR(name_buf.as_mut_ptr())),
            &mut name_len,
            Some(PWSTR(domain_buf.as_mut_ptr())),
            &mut domain_len,
            &mut sid_type,
        )
    };
    if ok.is_err() {
        return None;
    }

    Some(String::from_utf16_lossy(&name_buf[..name_len as usize]))
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

/// Resolve the owner of a process on Linux by reading /proc/<pid>/status
/// and looking up the real UID via nix::unistd::User::from_uid.
#[cfg(target_os = "linux")]
fn resolve_owner_impl(pid: u32) -> Option<String> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    // The Uid line format is: Uid:\t<real>\t<effective>\t<saved>\t<fs>
    let uid_line = status.lines().find(|l| l.starts_with("Uid:"))?;
    let real_uid_str = uid_line.split_whitespace().nth(1)?;
    let uid: u32 = real_uid_str.parse().ok()?;
    let user = nix::unistd::User::from_uid(nix::unistd::Uid::from_raw(uid)).ok()??;
    Some(user.name)
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

/// Resolve the owner of a process on macOS using proc_pidinfo with
/// PROC_PIDTBSDINFO flavor to get the process UID, then getpwuid_r
/// to resolve it to a username.
#[cfg(target_os = "macos")]
fn resolve_owner_impl(pid: u32) -> Option<String> {
    use libc::{c_int, c_void};

    // PROC_PIDTBSDINFO returns a proc_bsdinfo struct.
    const PROC_PIDTBSDINFO: c_int = 3;
    // sizeof(struct proc_bsdinfo) on 64-bit macOS = 636 bytes.
    const PROC_BSDINFO_SIZE: usize = 636;
    // Offset of pbi_uid (uint32_t) within proc_bsdinfo.
    // From bsd/sys/proc_info.h: pbi_flags(4) + pbi_status(4) + pbi_pid(4) +
    // pbi_ppid(4) + pbi_pgid(4) + pbi_pjobc(4) + pbi_e_tdev(4) +
    // pbi_e_tpgid(4) = 32 bytes, then pbi_nfiles(4) + pbi_uid(4) at offset 36... no.
    // Actually: pbi_flags(u32=4) + pbi_status(u32=4) + pbi_xstatus(u32=4) +
    // pbi_pid(u32=4) + pbi_ppid(u32=4) + pbi_pgid(u32=4) + pbi_pjobc(u32=4) +
    // pbi_e_tdev(u32=4) + pbi_e_tpgid(u32=4) + pbi_nice(i32=4) +
    // pbi_start_tvsec(u64=8) + pbi_start_tvusec(u64=8) = 52 bytes.
    // Wait, we need the correct offset. Let's use a simpler approach.
    //
    // On macOS, /proc doesn't exist. Use the nix crate to call getpwuid_r
    // after getting the UID from proc_pidinfo.
    //
    // proc_bsdinfo layout (from XNU bsd/sys/proc_info.h):
    //   uint32_t pbi_flags          offset 0
    //   uint32_t pbi_status         offset 4
    //   uint32_t pbi_xstatus        offset 8
    //   uint32_t pbi_pid            offset 12
    //   uint32_t pbi_ppid           offset 16
    //   uid_t    pbi_uid            offset 20
    //   gid_t    pbi_gid            offset 24
    //   uid_t    pbi_ruid           offset 28
    //   gid_t    pbi_rgid           offset 32
    const OFF_PBI_RUID: usize = 28;

    let mut buf = [0u8; PROC_BSDINFO_SIZE];
    let ret = unsafe {
        macos_proc::proc_pidinfo(
            pid as c_int,
            PROC_PIDTBSDINFO,
            0,
            buf.as_mut_ptr() as *mut c_void,
            PROC_BSDINFO_SIZE as c_int,
        )
    };
    if ret <= 0 {
        return None;
    }

    let uid = u32::from_ne_bytes([
        buf[OFF_PBI_RUID],
        buf[OFF_PBI_RUID + 1],
        buf[OFF_PBI_RUID + 2],
        buf[OFF_PBI_RUID + 3],
    ]);

    let user = nix::unistd::User::from_uid(nix::unistd::Uid::from_raw(uid)).ok()??;
    Some(user.name)
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

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn resolve_owner_impl(_pid: u32) -> Option<String> {
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

    #[test]
    fn test_resolve_process_owner_current_process() {
        // Resolve the owner of the current process. Should return a non-empty
        // username string regardless of whether running as root or normal user.
        let pid = std::process::id();
        let owner = resolve_process_owner(pid);
        assert!(owner.is_some(), "should resolve current process owner");
        let name = owner.unwrap();
        assert!(!name.is_empty(), "owner name should not be empty");
    }

    #[test]
    fn test_resolve_process_owner_pid_zero() {
        // PID 0 should return None (caller handles fallback).
        assert!(resolve_process_owner(0).is_none());
    }

    #[test]
    fn test_resolve_process_owner_nonexistent_pid() {
        // A PID that almost certainly does not exist should return None
        // without panicking.
        assert!(resolve_process_owner(999_999_999).is_none());
    }

    #[test]
    fn test_resolve_process_owner_consistent() {
        // Calling resolve_process_owner twice on the same PID should return
        // the same result both times.
        let pid = std::process::id();
        let first = resolve_process_owner(pid);
        let second = resolve_process_owner(pid);
        assert_eq!(first, second);
    }
}
