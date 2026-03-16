//! Shared protocol constants for the capture pipeline.
//!
//! These are standard protocol values from IETF RFCs. They are shared across
//! pcap.rs, sni.rs, and dns.rs to eliminate magic numbers in packet parsing.

// ── Ethernet ────────────────────────────────────────────────────────────────

/// Minimum Ethernet frame header size (dst MAC + src MAC + ethertype).
pub const ETH_HEADER_SIZE: usize = 14;

/// Ethertype for IPv4 (RFC 894).
pub const ETH_TYPE_IPV4: u16 = 0x0800;

/// Ethertype for IPv6 (RFC 2464).
pub const ETH_TYPE_IPV6: u16 = 0x86DD;

// ── IPv4 ────────────────────────────────────────────────────────────────────

/// Minimum IPv4 header size in bytes (no options).
pub const IPV4_HEADER_MIN_SIZE: usize = 20;

/// Byte offset of the protocol field in the IPv4 header.
pub const IPV4_PROTOCOL_OFFSET: usize = 9;

/// Mask to extract IHL (Internet Header Length) from the first byte of an IPv4 header.
pub const IPV4_IHL_MASK: u8 = 0x0F;

/// IPv4 version number (high nibble of first byte).
/// Used by Windows SIO_RCVALL path to identify IPv4 packets.
#[cfg(windows)]
pub const IPV4_VERSION: u8 = 4;

// ── IPv6 ────────────────────────────────────────────────────────────────────

/// Fixed IPv6 header size in bytes (RFC 8200).
pub const IPV6_HEADER_SIZE: usize = 40;

/// IPv6 version number (high nibble of first byte).
pub const IPV6_VERSION: u8 = 6;

/// IPv6 extension header: Hop-by-Hop Options (RFC 8200).
pub const IPV6_EXT_HOP_BY_HOP: u8 = 0;

/// IPv6 extension header: Routing (RFC 8200).
pub const IPV6_EXT_ROUTING: u8 = 43;

/// IPv6 extension header: Fragment (RFC 8200). Fixed 8-byte header.
pub const IPV6_EXT_FRAGMENT: u8 = 44;

/// IPv6 extension header: Destination Options (RFC 8200).
pub const IPV6_EXT_DESTINATION: u8 = 60;

/// Size of the IPv6 Fragment extension header in bytes.
pub const IPV6_FRAGMENT_HEADER_SIZE: usize = 8;

// ── IP protocol numbers ─────────────────────────────────────────────────────

/// IP protocol number for TCP (RFC 793).
pub const PROTO_TCP: u8 = 6;

/// IP protocol number for UDP (RFC 768).
pub const PROTO_UDP: u8 = 17;

// ── TCP / UDP ports ─────────────────────────────────────────────────────────

/// HTTPS port. The agent filters TCP traffic to this port for TLS ClientHello capture.
pub const HTTPS_PORT: u16 = 443;

/// DNS port. The agent captures DNS responses from this source port.
pub const DNS_PORT: u16 = 53;

// ── TLS ─────────────────────────────────────────────────────────────────────

/// TLS record content type for Handshake (RFC 8446 section 5.1).
pub const TLS_CONTENT_TYPE_HANDSHAKE: u8 = 0x16;

/// TLS handshake type for ClientHello (RFC 8446 section 4).
pub const TLS_HANDSHAKE_CLIENT_HELLO: u8 = 0x01;

/// TLS extension type for Server Name Indication (RFC 6066).
pub const TLS_EXT_SNI: u16 = 0x0000;

/// SNI entry type for host_name (RFC 6066 section 3).
pub const SNI_HOST_NAME_TYPE: u8 = 0x00;

/// Byte offset past the ClientHello version(2) + random(32) fields.
pub const CLIENT_HELLO_FIXED_PREFIX: usize = 34;

// ── DNS ─────────────────────────────────────────────────────────────────────

/// DNS header size in bytes (RFC 1035 section 4.1.1).
pub const DNS_HEADER_SIZE: usize = 12;

/// Bitmask for the QR (Query/Response) flag in DNS header flags.
pub const DNS_FLAG_QR: u16 = 0x8000;

/// Bitmask for the RCODE field in DNS header flags.
pub const DNS_FLAG_RCODE_MASK: u16 = 0x000F;

/// DNS record type A (host address, RFC 1035).
pub const DNS_TYPE_A: u16 = 1;

/// DNS record type CNAME (canonical name, RFC 1035).
pub const DNS_TYPE_CNAME: u16 = 5;

/// DNS record type AAAA (IPv6 host address, RFC 3596).
pub const DNS_TYPE_AAAA: u16 = 28;

/// Maximum allowed compression pointer jumps in DNS name parsing.
/// Prevents infinite loops from malformed packets.
pub const DNS_MAX_COMPRESSION_JUMPS: u32 = 10;

/// Bitmask to detect a DNS compression pointer (top 2 bits set).
pub const DNS_COMPRESSION_MARKER: u8 = 0xC0;

/// Bitmask to extract the offset from a DNS compression pointer.
pub const DNS_COMPRESSION_OFFSET_MASK: u8 = 0x3F;

// ── Capture buffer ──────────────────────────────────────────────────────────

/// Size of the packet receive buffer in bytes. 65536 = maximum IP packet size.
/// Used by Linux AF_PACKET, macOS BPF, and Windows SIO_RCVALL capture loops.
pub const CAPTURE_BUFFER_SIZE: usize = 65536;

// ── macOS BPF ───────────────────────────────────────────────────────────────

/// Minimum BPF header size in bytes (timeval32(8) + caplen(4) + datalen(4) + hdrlen(2)).
#[cfg(target_os = "macos")]
pub const BPF_HEADER_MIN_SIZE: usize = 18;

/// Minimum BPF buffer size fallback when the kernel reports 0 or a very small value.
#[cfg(target_os = "macos")]
pub const BPF_MIN_BUFFER_SIZE: usize = 4096;

/// Maximum number of /dev/bpf* devices to try opening on macOS.
#[cfg(target_os = "macos")]
pub const BPF_MAX_DEVICES: usize = 16;
