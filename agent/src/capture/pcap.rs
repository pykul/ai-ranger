/// Data extracted from a captured packet that matched an SNI hostname.
pub struct PacketInfo {
    pub sni_hostname: String,
    pub src_ip: String,
    pub src_port: u16,
}

/// Parse an IPv4/TCP packet starting at the IP header and extract SNI.
/// Shared by all platform capture backends.
fn parse_ipv4_tcp(ip: &[u8]) -> Option<PacketInfo> {
    if ip.len() < 20 || ip[9] != 6 {
        return None; // TCP only
    }
    let ihl = ((ip[0] & 0x0f) as usize) * 4;
    let src_ip = format!("{}.{}.{}.{}", ip[12], ip[13], ip[14], ip[15]);
    let tcp = ip.get(ihl..)?;
    if tcp.len() < 20 {
        return None;
    }
    let src_port = u16::from_be_bytes([tcp[0], tcp[1]]);
    let doff = ((tcp[12] >> 4) as usize) * 4;
    let payload = tcp.get(doff..)?;
    let sni = super::sni::extract_sni(payload)?;
    Some(PacketInfo {
        sni_hostname: sni,
        src_ip,
        src_port,
    })
}

/// Parse an Ethernet frame, validate IPv4 ethertype, and extract SNI.
/// Used by Linux (AF_PACKET) and macOS (BPF) which both deliver raw Ethernet frames.
fn parse_eth_frame(data: &[u8]) -> Option<PacketInfo> {
    if data.len() < 14 {
        return None;
    }
    if u16::from_be_bytes([data[12], data[13]]) != 0x0800 {
        return None; // IPv4 only
    }
    parse_ipv4_tcp(&data[14..])
}

// ── Linux: AF_PACKET raw socket ───────────────────────────────────────────────
//
// Opens an AF_PACKET/SOCK_RAW socket (kernel built-in, no libpcap).
// Attaches a BPF filter for "tcp dst port 443" via SO_ATTACH_FILTER.
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

    // BPF bytecode equivalent to: tcp dst port 443 (on Ethernet frames)
    //   ldh  [12]           ; ethertype
    //   jeq  #0x0800, +0, +5  ; IPv4 → continue, else drop
    //   ldb  [23]           ; IP protocol
    //   jeq  #6, +0, +3    ; TCP → continue, else drop
    //   ldh  [36]           ; TCP dst port (eth:14 + ip_min:20 + dst_port_off:2)
    //   jeq  #443, +0, +1  ; port 443 → continue, else drop
    //   ret  #0xffff        ; accept
    //   ret  #0             ; reject
    //
    // Note: offset 36 assumes a 20-byte IP header (no options). If IP options are
    // present (IHL > 5), this filter may pass non-443 packets or miss 443 packets.
    // The userspace parser handles variable IHL correctly, so this is safe — just
    // slightly less efficient as a pre-filter.
    const FILTER: [sock_filter; 8] = [
        sock_filter {
            code: 0x28,
            jt: 0,
            jf: 0,
            k: 12,
        },
        sock_filter {
            code: 0x15,
            jt: 0,
            jf: 5,
            k: 0x0800,
        },
        sock_filter {
            code: 0x30,
            jt: 0,
            jf: 0,
            k: 23,
        },
        sock_filter {
            code: 0x15,
            jt: 0,
            jf: 3,
            k: 6,
        },
        sock_filter {
            code: 0x28,
            jt: 0,
            jf: 0,
            k: 36,
        },
        sock_filter {
            code: 0x15,
            jt: 0,
            jf: 1,
            k: 443,
        },
        sock_filter {
            code: 0x06,
            jt: 0,
            jf: 0,
            k: 0xffff,
        },
        sock_filter {
            code: 0x06,
            jt: 0,
            jf: 0,
            k: 0,
        },
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
// Opens /dev/bpf*, attaches to the primary network interface via BIOCSETIF,
// and installs a BPF filter for "tcp dst port 443" via BIOCSETF.
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

    // Same BPF filter as Linux: tcp dst port 443 on Ethernet frames
    const FILTER: [BpfInsn; 8] = [
        BpfInsn {
            code: 0x28,
            jt: 0,
            jf: 0,
            k: 12,
        },
        BpfInsn {
            code: 0x15,
            jt: 0,
            jf: 5,
            k: 0x0800,
        },
        BpfInsn {
            code: 0x30,
            jt: 0,
            jf: 0,
            k: 23,
        },
        BpfInsn {
            code: 0x15,
            jt: 0,
            jf: 3,
            k: 6,
        },
        BpfInsn {
            code: 0x28,
            jt: 0,
            jf: 0,
            k: 36,
        },
        BpfInsn {
            code: 0x15,
            jt: 0,
            jf: 1,
            k: 443,
        },
        BpfInsn {
            code: 0x06,
            jt: 0,
            jf: 0,
            k: 0xffff,
        },
        BpfInsn {
            code: 0x06,
            jt: 0,
            jf: 0,
            k: 0,
        },
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

        // TODO Phase 1: detect primary interface dynamically via getifaddrs.
        // en0 is the primary Ethernet/Wi-Fi interface on almost all Macs.
        let iface = "en0";
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
        let mut buf = vec![0u8; buf_len.max(4096) as usize];

        loop {
            let n = read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
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
        while buf.len() >= 26 {
            let caplen = u32::from_ne_bytes([buf[16], buf[17], buf[18], buf[19]]) as usize;
            let hdrlen = u16::from_ne_bytes([buf[24], buf[25]]) as usize;

            if hdrlen < 26 || buf.len() < hdrlen + caplen {
                break;
            }

            let frame = &buf[hdrlen..hdrlen + caplen];
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
}

// ── Windows: WinSock2 raw socket + SIO_RCVALL ─────────────────────────────────
//
// Opens a raw IP socket (AF_INET/SOCK_RAW/IPPROTO_IP) and enables SIO_RCVALL
// to receive all IP packets on the interface. Uses only ws2_32.dll which is
// built into every Windows install — no npcap, no driver, no installer.
// Receives raw IP packets (no Ethernet header; starts at IP header).
// Requires Administrator.
//
// Note: SIO_RCVALL is the pragmatic standalone approach for Phase 0.
// Phase 1 can upgrade to ETW via ferrisetw (Microsoft-Windows-NDIS-PacketCapture)
// if per-interface or inbound/outbound separation is needed.

#[cfg(windows)]
mod platform {
    use super::PacketInfo;
    use std::{mem, net::UdpSocket};
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
        unsafe { capture_impl(&mut on_packet) }
    }

    unsafe fn capture_impl<F: FnMut(PacketInfo)>(
        on_packet: &mut F,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut wsa: WSADATA = mem::zeroed();
        if WSAStartup(0x0202, &mut wsa) != 0 {
            return Err("WSAStartup failed".into());
        }

        // AF_INET is c_int (i32) in winapi::shared::ws2def, matching socket()'s af parameter.
        let sock = socket(AF_INET, SOCK_RAW, IPPROTO_IP);
        if sock == INVALID_SOCKET {
            return Err(format!(
                "socket() failed: {} — run as Administrator",
                WSAGetLastError()
            )
            .into());
        }

        let local_ip = local_ipv4().ok_or("could not determine local IPv4 address")?;

        let mut addr: SOCKADDR_IN = mem::zeroed();
        // sin_family is ADDRESS_FAMILY (u16); AF_INET is c_int (i32) — cast required.
        addr.sin_family = AF_INET as u16;
        *addr.sin_addr.S_un.S_addr_mut() = u32::from_ne_bytes(local_ip);

        if bind(
            sock,
            &addr as *const SOCKADDR_IN as *const SOCKADDR,
            mem::size_of::<SOCKADDR_IN>() as i32,
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
            mem::size_of::<u32>() as DWORD,
            &mut out_val as *mut u32 as LPVOID,
            mem::size_of::<u32>() as DWORD,
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

        let mut buf = vec![0u8; 65536];
        loop {
            let n = recv(sock, buf.as_mut_ptr() as *mut i8, buf.len() as i32, 0);
            if n <= 0 {
                break;
            }
            if let Some(info) = parse_ip_packet(&buf[..n as usize]) {
                on_packet(info);
            }
        }

        closesocket(sock);
        WSACleanup();
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

    /// SIO_RCVALL gives raw IPv4 packets with no Ethernet header.
    /// Filters to port 443 in userspace (no kernel BPF on Windows with SIO_RCVALL).
    fn parse_ip_packet(data: &[u8]) -> Option<PacketInfo> {
        if data.len() < 20 {
            return None;
        }
        if (data[0] >> 4) != 4 {
            return None; // IPv4 only
        }

        // Check dst port 443 before calling shared parser (Windows has no kernel filter)
        let ihl = ((data[0] & 0x0f) as usize) * 4;
        let tcp = data.get(ihl..)?;
        if tcp.len() < 4 {
            return None;
        }
        let dst_port = u16::from_be_bytes([tcp[2], tcp[3]]);
        if dst_port != 443 {
            return None;
        }

        super::parse_ipv4_tcp(data)
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
