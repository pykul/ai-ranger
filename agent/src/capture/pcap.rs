/// Data extracted from a captured packet.
pub struct PacketInfo {
    /// Hostname extracted via SNI or DNS. Empty if neither produced a hostname
    /// (e.g. ECH-encrypted ClientHello) — IP range fallback may still match.
    pub hostname: String,
    pub src_ip: String,
    pub dst_ip: String,
    pub src_port: u16,
    /// "sni" if extracted from TLS ClientHello, "dns" if from DNS response,
    /// empty string if no hostname was extracted (IP-only packet).
    pub detection_method: &'static str,
}

/// Parse an IPv4 packet and attempt to extract a hostname via SNI or DNS.
/// Shared by all platform capture backends.
fn parse_ipv4_packet(ip: &[u8]) -> Option<PacketInfo> {
    if ip.len() < 20 {
        return None;
    }
    let proto = ip[9];
    let ihl = ((ip[0] & 0x0f) as usize) * 4;
    let src_ip = format!("{}.{}.{}.{}", ip[12], ip[13], ip[14], ip[15]);
    let dst_ip = format!("{}.{}.{}.{}", ip[16], ip[17], ip[18], ip[19]);
    let transport = ip.get(ihl..)?;

    match proto {
        6 => parse_tcp_sni(transport, src_ip, dst_ip),
        17 => parse_udp_dns(transport, src_ip, dst_ip),
        _ => None,
    }
}

/// Extract SNI from a TCP segment destined for port 443.
///
/// If the payload is a TLS record (starts with 0x16) but SNI extraction fails
/// (e.g. ECH hid the hostname), returns a PacketInfo with an empty hostname so
/// the IP range fallback in main.rs can attempt classification.
///
/// Non-TLS packets (SYN, ACK, data after handshake) return None — they are not
/// connection-initiating events and should not generate duplicate detections.
fn parse_tcp_sni(tcp: &[u8], src_ip: String, dst_ip: String) -> Option<PacketInfo> {
    if tcp.len() < 20 {
        return None;
    }
    let dst_port = u16::from_be_bytes([tcp[2], tcp[3]]);
    if dst_port != 443 {
        return None;
    }
    let src_port = u16::from_be_bytes([tcp[0], tcp[1]]);
    let doff = ((tcp[12] >> 4) as usize) * 4;
    let payload = tcp.get(doff..)?;

    // Only process TLS handshake records (content type 0x16).
    // Non-TLS or empty payloads (SYN, ACK, data) are skipped entirely.
    if payload.is_empty() || payload[0] != 0x16 {
        return None;
    }

    match super::sni::extract_sni(payload) {
        Some(hostname) => Some(PacketInfo {
            hostname,
            src_ip,
            dst_ip,
            src_port,
            detection_method: "sni",
        }),
        // TLS handshake record present but no SNI found (ECH or malformed).
        // Return empty hostname for IP range fallback.
        None => Some(PacketInfo {
            hostname: String::new(),
            src_ip,
            dst_ip,
            src_port,
            detection_method: "",
        }),
    }
}

/// Extract queried hostname from a DNS response (source port 53).
fn parse_udp_dns(udp: &[u8], src_ip: String, dst_ip: String) -> Option<PacketInfo> {
    // UDP header: src_port(2) + dst_port(2) + length(2) + checksum(2) = 8 bytes
    if udp.len() < 8 {
        return None;
    }
    let src_port = u16::from_be_bytes([udp[0], udp[1]]);
    if src_port != 53 {
        return None;
    }
    let dst_port = u16::from_be_bytes([udp[2], udp[3]]);
    let payload = udp.get(8..)?;
    let hostname = super::dns::extract_dns_hostname(payload)?;
    Some(PacketInfo {
        hostname,
        // For DNS events, report the client IP and client port (dst of the response).
        // src_port is set to the client's ephemeral port so process resolution can try,
        // though it usually resolves to the system DNS resolver, not the application.
        src_ip,
        dst_ip,
        src_port: dst_port,
        detection_method: "dns",
    })
}

/// Parse an IPv6 packet and attempt to extract a hostname via SNI or DNS.
///
/// IPv6 fixed header: 40 bytes.
///   Byte 0:     version (high nibble = 6)
///   Byte 6:     next-header (6=TCP, 17=UDP, or extension header type)
///   Bytes 8–23: source address (16 bytes)
///   Bytes 24–39: destination address (16 bytes)
///   Byte 40+:   transport header or extension headers
///
/// Extension headers are walked until TCP or UDP is found. Unknown extension
/// header types cause the packet to be skipped gracefully.
#[cfg(not(windows))] // Windows uses ETW DNS-Client for IPv6, not raw packet capture
fn parse_ipv6_packet(ip6: &[u8]) -> Option<PacketInfo> {
    if ip6.len() < 40 {
        return None;
    }
    if (ip6[0] >> 4) != 6 {
        return None;
    }

    let src_ip = std::net::Ipv6Addr::from(<[u8; 16]>::try_from(&ip6[8..24]).ok()?).to_string();
    let dst_ip = std::net::Ipv6Addr::from(<[u8; 16]>::try_from(&ip6[24..40]).ok()?).to_string();

    // Walk extension header chain to find the transport protocol.
    let mut next_header = ip6[6];
    let mut offset: usize = 40;

    loop {
        match next_header {
            6 => {
                // TCP
                let transport = ip6.get(offset..)?;
                return parse_tcp_sni(transport, src_ip, dst_ip);
            }
            17 => {
                // UDP
                let transport = ip6.get(offset..)?;
                return parse_udp_dns(transport, src_ip, dst_ip);
            }
            // Known extension header types — walk past them.
            // Each has next-header in byte 0 and length in byte 1 (in 8-byte units, excluding first 8).
            0 | 43 | 60 => {
                // 0=Hop-by-Hop, 43=Routing, 60=Destination Options
                if offset + 2 > ip6.len() {
                    return None;
                }
                next_header = ip6[offset];
                let ext_len = (ip6[offset + 1] as usize + 1) * 8;
                offset += ext_len;
                if offset > ip6.len() {
                    return None;
                }
            }
            44 => {
                // Fragment header — fixed 8 bytes
                if offset + 8 > ip6.len() {
                    return None;
                }
                next_header = ip6[offset];
                offset += 8;
            }
            other => {
                // Unknown extension header type — cannot walk past safely.
                eprintln!(
                    "[ai-ranger] IPv6: unknown next-header type {} at offset {}, skipping packet",
                    other, offset
                );
                return None;
            }
        }
    }
}

/// Parse an Ethernet frame and extract hostname from IPv4 or IPv6 payload.
/// Used by Linux (AF_PACKET) and macOS (BPF) which both deliver raw Ethernet frames.
#[cfg(unix)]
fn parse_eth_frame(data: &[u8]) -> Option<PacketInfo> {
    if data.len() < 14 {
        return None;
    }
    let ethertype = u16::from_be_bytes([data[12], data[13]]);
    match ethertype {
        0x0800 => parse_ipv4_packet(&data[14..]),
        0x86DD => parse_ipv6_packet(&data[14..]),
        _ => None,
    }
}

// ── Linux: AF_PACKET raw socket ───────────────────────────────────────────────
//
// Opens an AF_PACKET/SOCK_RAW socket (kernel built-in, no libpcap).
// Attaches a BPF filter for "tcp dst port 443 OR udp src port 53" via SO_ATTACH_FILTER.
// Receives raw Ethernet frames directly from the kernel.
// Requires root.

#[cfg(target_os = "linux")]
mod platform {
    use super::PacketInfo;
    use libc::{
        c_void, close, recv, setsockopt, sock_filter, sock_fprog, socket, AF_PACKET, ETH_P_ALL,
        SOCK_RAW, SOL_SOCKET, SO_ATTACH_FILTER,
    };
    use std::mem;

    // BPF bytecode: accept IPv4 (0x0800) and IPv6 (0x86DD) Ethernet frames.
    //
    // Port filtering (TCP 443, UDP 53) is done in userspace because IPv4 and
    // IPv6 have different header lengths, making a single BPF program that
    // checks ports for both protocols complex and error-prone. The ethertype
    // check alone eliminates most non-IP traffic at the kernel level.
    //
    //   0: ldh  [12]                ; ethertype
    //   1: jeq  #0x0800, +2, +0    ; IPv4 → accept
    //   2: jeq  #0x86DD, +1, +0    ; IPv6 → accept
    //   3: ret  #0                  ; reject
    //   4: ret  #0xffff             ; accept
    const FILTER: [sock_filter; 5] = [
        sock_filter {
            code: 0x28,
            jt: 0,
            jf: 0,
            k: 12,
        }, // ldh [12]
        sock_filter {
            code: 0x15,
            jt: 2,
            jf: 0,
            k: 0x0800,
        }, // jeq #0x0800 → accept
        sock_filter {
            code: 0x15,
            jt: 1,
            jf: 0,
            k: 0x86DD,
        }, // jeq #0x86DD → accept
        sock_filter {
            code: 0x06,
            jt: 0,
            jf: 0,
            k: 0,
        }, // reject
        sock_filter {
            code: 0x06,
            jt: 0,
            jf: 0,
            k: 0xffff,
        }, // accept
    ];

    pub fn capture<F: FnMut(PacketInfo)>(
        mut on_packet: F,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            // ETH_P_ALL in network byte order
            let proto = (ETH_P_ALL as u16).to_be() as i32;
            let sock = socket(AF_PACKET, SOCK_RAW, proto);
            if sock < 0 {
                return Err(format!(
                    "socket(AF_PACKET) failed: {} — run with sudo",
                    *libc::__errno_location()
                )
                .into());
            }

            let fprog = sock_fprog {
                len: FILTER.len() as u16,
                filter: FILTER.as_ptr() as *mut sock_filter,
            };
            if setsockopt(
                sock,
                SOL_SOCKET,
                SO_ATTACH_FILTER,
                &fprog as *const _ as *const c_void,
                mem::size_of::<sock_fprog>() as u32,
            ) < 0
            {
                close(sock);
                return Err(
                    format!("SO_ATTACH_FILTER failed: {}", *libc::__errno_location()).into(),
                );
            }

            eprintln!("[ai-ranger] Capturing on all interfaces (AF_PACKET raw socket)");
            let mut buf = vec![0u8; 65536];
            loop {
                let n = recv(sock, buf.as_mut_ptr() as *mut c_void, buf.len(), 0);
                if n <= 0 {
                    break;
                }
                if let Some(info) = super::parse_eth_frame(&buf[..n as usize]) {
                    on_packet(info);
                }
            }
            close(sock);
        }
        Ok(())
    }
}

// ── macOS: BPF device ─────────────────────────────────────────────────────────
//
// MACOS-UNVERIFIED: This entire platform block requires compile-test on Apple
// hardware. Specifically:
//   - detect_interface() uses getifaddrs FFI (new in Phase 1, untested)
//   - BPF filter updated for IPv4+IPv6 dual-stack (untested)
//   - IPv6 packet parsing via parse_ipv6_packet (untested on macOS)
//   - All BPF ioctl constants and struct layouts carried from Phase 0 (worked then)
//
// Opens /dev/bpf*, attaches to the primary network interface via BIOCSETIF,
// and installs a BPF filter for "tcp dst port 443 OR udp src port 53" via BIOCSETF.
// Receives raw Ethernet frames wrapped in a bpf_hdr.
// Requires root.

#[cfg(target_os = "macos")]
mod platform {
    use super::PacketInfo;
    use libc::{c_void, close, ifreq, ioctl, open, read, IFNAMSIZ, O_RDWR};
    use std::{ffi::CString, mem};

    // BPF ioctl codes (macOS 64-bit, computed from <net/bpf.h>)
    const BIOCSETIF: libc::c_ulong = 0x8020_426c; // _IOW('B', 108, ifreq)
    const BIOCIMMEDIATE: libc::c_ulong = 0x8004_4270; // _IOW('B', 112, u_int)
    const BIOCSETF: libc::c_ulong = 0x8010_4267; // _IOW('B', 103, bpf_program)
    const BIOCGBLEN: libc::c_ulong = 0x4004_4266; // _IOR('B', 102, u_int)
    const BIOCSSEESENT: libc::c_ulong = 0x8004_4277; // _IOW('B', 119, u_int)

    #[repr(C)]
    struct BpfInsn {
        code: u16,
        jt: u8,
        jf: u8,
        k: u32,
    }

    // #[repr(C)] ensures 4-byte padding between bf_len and bf_insns, matching
    // struct bpf_program { u_int bf_len; struct bpf_insn *bf_insns; } on 64-bit.
    #[repr(C)]
    struct BpfProgram {
        bf_len: u32,
        bf_insns: *const BpfInsn,
    }

    // MACOS-UNVERIFIED: BPF filter for IPv4+IPv6 dual-stack.
    // Accepts IPv4 (0x0800) and IPv6 (0x86DD) frames. Port filtering in userspace.
    // Same logic as Linux filter — see comments there.
    const FILTER: [BpfInsn; 5] = [
        BpfInsn {
            code: 0x28,
            jt: 0,
            jf: 0,
            k: 12,
        }, // ldh [12]
        BpfInsn {
            code: 0x15,
            jt: 2,
            jf: 0,
            k: 0x0800,
        }, // jeq #0x0800 → accept
        BpfInsn {
            code: 0x15,
            jt: 1,
            jf: 0,
            k: 0x86DD,
        }, // jeq #0x86DD → accept
        BpfInsn {
            code: 0x06,
            jt: 0,
            jf: 0,
            k: 0,
        }, // reject
        BpfInsn {
            code: 0x06,
            jt: 0,
            jf: 0,
            k: 0xffff,
        }, // accept
    ];

    pub fn capture<F: FnMut(PacketInfo)>(
        mut on_packet: F,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe { capture_impl(&mut on_packet) }
    }

    unsafe fn capture_impl<F: FnMut(PacketInfo)>(
        on_packet: &mut F,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fd = open_bpf()?;

        let iface = detect_interface().unwrap_or_else(|| "en0".to_string());
        eprintln!("[ai-ranger] Capturing on {iface} (BPF device)");

        // Attach to interface
        let mut ifr: ifreq = mem::zeroed();
        let name_bytes = iface.as_bytes();
        let copy_len = name_bytes.len().min(IFNAMSIZ - 1);
        for (i, &b) in name_bytes[..copy_len].iter().enumerate() {
            ifr.ifr_name[i] = b as libc::c_char;
        }
        if ioctl(fd, BIOCSETIF, &ifr) < 0 {
            close(fd);
            return Err(format!("BIOCSETIF failed: {}", *libc::__error()).into());
        }

        // Immediate mode: deliver packets as soon as they arrive
        let one: u32 = 1;
        ioctl(fd, BIOCIMMEDIATE, &one);

        // See outgoing packets (required to capture TLS ClientHello from this host)
        ioctl(fd, BIOCSSEESENT, &one);

        // Install BPF filter
        let prog = BpfProgram {
            bf_len: FILTER.len() as u32,
            bf_insns: FILTER.as_ptr(),
        };
        if ioctl(fd, BIOCSETF, &prog) < 0 {
            close(fd);
            return Err(format!("BIOCSETF failed: {}", *libc::__error()).into());
        }

        // Query kernel buffer size
        let mut buf_len: u32 = 0;
        ioctl(fd, BIOCGBLEN, &mut buf_len);
        eprintln!("[ai-ranger] BPF buffer size: {buf_len}");
        let mut buf = vec![0u8; buf_len.max(4096) as usize];

        let mut read_count: u64 = 0;
        loop {
            let n = read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                eprintln!("[ai-ranger] BPF read returned {n} (errno: {})", *libc::__error());
                break;
            }
            read_count += 1;
            if read_count <= 5 {
                eprintln!("[ai-ranger] BPF read #{read_count}: {n} bytes");
            }
            drain_bpf_buf(&buf[..n as usize], on_packet);
        }

        close(fd);
        Ok(())
    }

    /// Walk all bpf_hdr-prefixed records in a BPF read buffer.
    ///
    /// bpf_hdr layout on macOS 64-bit:
    ///   [0..8]   tv_sec  (long = 8 bytes)
    ///   [8..12]  tv_usec (int  = 4 bytes)
    ///   [12..16] padding (4 bytes, struct timeval padded to 16)
    ///   [16..20] bh_caplen  (u32)
    ///   [20..24] bh_datalen (u32)
    ///   [24..26] bh_hdrlen  (u16) — actual header size (≥26, word-aligned)
    fn drain_bpf_buf<F: FnMut(PacketInfo)>(mut buf: &[u8], on_packet: &mut F) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static FRAME_COUNT: AtomicU64 = AtomicU64::new(0);

        while buf.len() >= 26 {
            let caplen = u32::from_ne_bytes([buf[16], buf[17], buf[18], buf[19]]) as usize;
            let hdrlen = u16::from_ne_bytes([buf[24], buf[25]]) as usize;

            if hdrlen < 26 || buf.len() < hdrlen + caplen {
                break;
            }

            let frame = &buf[hdrlen..hdrlen + caplen];
            let count = FRAME_COUNT.fetch_add(1, Ordering::Relaxed);
            if count < 5 {
                let ethertype = if frame.len() >= 14 {
                    u16::from_be_bytes([frame[12], frame[13]])
                } else {
                    0
                };
                eprintln!(
                    "[ai-ranger] BPF frame #{}: caplen={caplen} hdrlen={hdrlen} framelen={} ethertype=0x{ethertype:04x}",
                    count + 1,
                    frame.len()
                );
            }
            if let Some(info) = super::parse_eth_frame(frame) {
                on_packet(info);
            }

            // Advance to next word-aligned record
            let advance = bpf_wordalign(hdrlen + caplen);
            if advance == 0 || advance > buf.len() {
                break;
            }
            buf = &buf[advance..];
        }
    }

    // BPF_WORDALIGN: round up to sizeof(long) = 8 on 64-bit macOS
    fn bpf_wordalign(x: usize) -> usize {
        (x + 7) & !7
    }

    unsafe fn open_bpf() -> Result<libc::c_int, Box<dyn std::error::Error>> {
        for i in 0..16 {
            let path = CString::new(format!("/dev/bpf{i}")).unwrap();
            let fd = open(path.as_ptr(), O_RDWR);
            if fd >= 0 {
                return Ok(fd);
            }
        }
        Err("could not open any /dev/bpf* device — run with sudo".into())
    }

    /// Detect the primary active non-loopback IPv4 interface via getifaddrs.
    /// Falls back to None if no suitable interface is found.
    // MACOS-UNVERIFIED: getifaddrs FFI — see file-level comment.
    fn detect_interface() -> Option<String> {
        use libc::{freeifaddrs, getifaddrs, ifaddrs, sockaddr_in, AF_INET, IFF_LOOPBACK, IFF_UP};
        use std::ptr;

        unsafe {
            let mut addrs: *mut ifaddrs = ptr::null_mut();
            if getifaddrs(&mut addrs) != 0 {
                return None;
            }

            let mut current = addrs;
            let mut result = None;

            while !current.is_null() {
                let ifa = &*current;
                let flags = ifa.ifa_flags as i32;

                // Want: up, not loopback, has an IPv4 address
                if (flags & IFF_UP != 0)
                    && (flags & IFF_LOOPBACK == 0)
                    && !ifa.ifa_addr.is_null()
                    && (*ifa.ifa_addr).sa_family as i32 == AF_INET
                {
                    // Check for a non-zero IPv4 address
                    let sin = &*(ifa.ifa_addr as *const sockaddr_in);
                    if sin.sin_addr.s_addr != 0 {
                        let name = std::ffi::CStr::from_ptr(ifa.ifa_name);
                        result = Some(name.to_string_lossy().into_owned());
                        break;
                    }
                }
                current = ifa.ifa_next;
            }

            freeifaddrs(addrs);
            result
        }
    }
}

// ── Windows: WinSock2 raw socket + SIO_RCVALL ─────────────────────────────────
//
// Opens a raw IP socket (AF_INET/SOCK_RAW/IPPROTO_IP) and enables SIO_RCVALL
// to receive all IPv4 packets on the interface. Uses only ws2_32.dll — no npcap,
// no driver, no installer. Requires Administrator.
//
// SIO_RCVALL only captures IPv4. IPv6 connections are covered by the ETW
// DNS-Client path in capture/etw_dns.rs, which detects AI provider hostname
// resolutions regardless of IP version.

#[cfg(windows)]
mod platform {
    use super::PacketInfo;
    use std::net::UdpSocket;
    use winapi::{
        shared::{
            minwindef::{DWORD, LPVOID},
            ws2def::{AF_INET, IPPROTO_IP, SOCKADDR, SOCKADDR_IN},
        },
        um::winsock2::{
            bind, closesocket, recv, socket, WSACleanup, WSAGetLastError, WSAIoctl, WSAStartup,
            INVALID_SOCKET, SOCK_RAW, WSADATA,
        },
    };

    // _WSAIOW(IOC_VENDOR, 1) — receive all IP packets on the interface
    const SIO_RCVALL: DWORD = 0x9800_0001;

    pub fn capture<F: FnMut(PacketInfo)>(
        mut on_packet: F,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let mut wsa: WSADATA = std::mem::zeroed();
            if WSAStartup(0x0202, &mut wsa) != 0 {
                return Err("WSAStartup failed".into());
            }

            let sock = socket(AF_INET, SOCK_RAW, IPPROTO_IP);
            if sock == INVALID_SOCKET {
                return Err(format!(
                    "socket() failed: {} — run as Administrator",
                    WSAGetLastError()
                )
                .into());
            }

            let local_ip = local_ipv4().ok_or("could not determine local IPv4 address")?;

            let mut addr: SOCKADDR_IN = std::mem::zeroed();
            addr.sin_family = AF_INET as u16;
            *addr.sin_addr.S_un.S_addr_mut() = u32::from_ne_bytes(local_ip);

            if bind(
                sock,
                &addr as *const SOCKADDR_IN as *const SOCKADDR,
                std::mem::size_of::<SOCKADDR_IN>() as i32,
            ) != 0
            {
                closesocket(sock);
                return Err(format!("bind() failed: {}", WSAGetLastError()).into());
            }

            let mut in_val: u32 = 1; // RCVALL_ON
            let mut out_val: u32 = 0;
            let mut bytes: DWORD = 0;
            if WSAIoctl(
                sock,
                SIO_RCVALL,
                &mut in_val as *mut u32 as LPVOID,
                std::mem::size_of::<u32>() as DWORD,
                &mut out_val as *mut u32 as LPVOID,
                std::mem::size_of::<u32>() as DWORD,
                &mut bytes,
                std::ptr::null_mut(),
                None,
            ) != 0
            {
                closesocket(sock);
                return Err(format!(
                    "WSAIoctl(SIO_RCVALL) failed: {} — run as Administrator",
                    WSAGetLastError()
                )
                .into());
            }

            eprintln!(
                "[ai-ranger] Capturing on {}.{}.{}.{} (raw IP socket + SIO_RCVALL)",
                local_ip[0], local_ip[1], local_ip[2], local_ip[3]
            );
            eprintln!(
                "[ai-ranger] Note: SIO_RCVALL captures IPv4 only. IPv6 connections are detected via ETW DNS-Client monitoring."
            );

            let mut buf = vec![0u8; 65536];
            loop {
                let n = recv(sock, buf.as_mut_ptr() as *mut i8, buf.len() as i32, 0);
                if n <= 0 {
                    break;
                }
                let data = &buf[..n as usize];
                if data.len() >= 20 && (data[0] >> 4) == 4 {
                    if let Some(info) = super::parse_ipv4_packet(data) {
                        on_packet(info);
                    }
                }
            }

            closesocket(sock);
            WSACleanup();
        }
        Ok(())
    }

    /// UDP connect trick: connect a UDP socket to an external address (no data sent)
    /// and read back the local address the OS selected — that is our capture IP.
    fn local_ipv4() -> Option<[u8; 4]> {
        let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
        sock.connect("8.8.8.8:80").ok()?;
        match sock.local_addr().ok()? {
            std::net::SocketAddr::V4(a) => Some(a.ip().octets()),
            _ => None,
        }
    }
}

// ── Unsupported platform stub ─────────────────────────────────────────────────

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod platform {
    pub fn capture<F: FnMut(super::PacketInfo)>(_: F) -> Result<(), Box<dyn std::error::Error>> {
        Err("packet capture is not implemented for this platform".into())
    }
}

pub use platform::capture;
