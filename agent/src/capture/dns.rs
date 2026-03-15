/// Parse a DNS response and extract queried hostnames that received A/AAAA/CNAME answers.
///
/// DNS responses are sent in plaintext over UDP port 53. Parsing them supplements
/// SNI extraction — useful when ECH (Encrypted Client Hello) hides the real SNI.
///
/// Returns `None` if the packet is not a valid DNS response or contains no answers.
pub fn extract_dns_hostname(data: &[u8]) -> Option<String> {
    // DNS header: 12 bytes minimum
    //   [0..2]  Transaction ID
    //   [2..4]  Flags (bit 15 = QR, 1 = response)
    //   [4..6]  QDCOUNT (questions)
    //   [6..8]  ANCOUNT (answers)
    //   [8..10] NSCOUNT
    //   [10..12] ARCOUNT
    if data.len() < 12 {
        return None;
    }

    let flags = u16::from_be_bytes([data[2], data[3]]);
    // QR bit must be 1 (response), RCODE must be 0 (no error)
    if flags & 0x8000 == 0 || flags & 0x000F != 0 {
        return None;
    }

    let qdcount = u16::from_be_bytes([data[4], data[5]]) as usize;
    let ancount = u16::from_be_bytes([data[6], data[7]]) as usize;
    if qdcount == 0 || ancount == 0 {
        return None;
    }

    let mut pos = 12;

    // Parse question section to extract the queried hostname
    let (hostname, new_pos) = parse_name(data, pos)?;
    pos = new_pos;

    // Skip QTYPE (2) + QCLASS (2)
    if pos + 4 > data.len() {
        return None;
    }
    pos += 4;

    // Skip remaining questions (if more than 1)
    for _ in 1..qdcount {
        let (_, new_pos) = parse_name(data, pos)?;
        pos = new_pos;
        if pos + 4 > data.len() {
            return None;
        }
        pos += 4;
    }

    // Walk answers to confirm at least one A (1), AAAA (28), or CNAME (5) record exists
    for _ in 0..ancount {
        let (_, new_pos) = parse_name(data, pos)?;
        pos = new_pos;
        if pos + 10 > data.len() {
            return None;
        }
        let rtype = u16::from_be_bytes([data[pos], data[pos + 1]]);
        let rdlength = u16::from_be_bytes([data[pos + 8], data[pos + 9]]) as usize;
        pos += 10;
        if pos + rdlength > data.len() {
            return None;
        }

        if rtype == 1 || rtype == 28 || rtype == 5 {
            // Valid answer record — return the queried hostname
            return Some(hostname);
        }
        pos += rdlength;
    }

    None
}

/// Parse a DNS name from the packet, handling compression pointers.
/// Returns the decoded hostname and the position after the name in the original data.
fn parse_name(data: &[u8], start: usize) -> Option<(String, usize)> {
    let mut labels: Vec<String> = Vec::new();
    let mut pos = start;
    let mut jumped = false;
    let mut end_pos = 0;
    let mut jumps = 0;

    loop {
        if pos >= data.len() || jumps > 10 {
            return None;
        }
        let len = data[pos] as usize;

        if len == 0 {
            if !jumped {
                end_pos = pos + 1;
            }
            break;
        }

        // Compression pointer: top 2 bits are 11
        if len & 0xC0 == 0xC0 {
            if pos + 1 >= data.len() {
                return None;
            }
            if !jumped {
                end_pos = pos + 2;
            }
            let offset = ((len & 0x3F) << 8) | data[pos + 1] as usize;
            pos = offset;
            jumped = true;
            jumps += 1;
            continue;
        }

        pos += 1;
        if pos + len > data.len() {
            return None;
        }
        labels.push(String::from_utf8_lossy(&data[pos..pos + len]).to_string());
        pos += len;
    }

    if labels.is_empty() {
        return None;
    }

    Some((labels.join("."), end_pos))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal DNS response for a single A record query.
    fn build_dns_response(hostname: &str, has_answer: bool) -> Vec<u8> {
        let mut buf = Vec::new();

        // Transaction ID
        buf.extend_from_slice(&[0x12, 0x34]);
        // Flags: QR=1, RCODE=0 (standard response, no error)
        buf.extend_from_slice(&[0x81, 0x80]);
        // QDCOUNT=1
        buf.extend_from_slice(&[0x00, 0x01]);
        // ANCOUNT
        buf.extend_from_slice(&[0x00, if has_answer { 0x01 } else { 0x00 }]);
        // NSCOUNT=0, ARCOUNT=0
        buf.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        // Question: hostname
        for label in hostname.split('.') {
            buf.push(label.len() as u8);
            buf.extend_from_slice(label.as_bytes());
        }
        buf.push(0x00); // end of name
        buf.extend_from_slice(&[0x00, 0x01]); // QTYPE=A
        buf.extend_from_slice(&[0x00, 0x01]); // QCLASS=IN

        if has_answer {
            // Answer: compression pointer to question name (offset 12)
            buf.extend_from_slice(&[0xC0, 0x0C]);
            buf.extend_from_slice(&[0x00, 0x01]); // TYPE=A
            buf.extend_from_slice(&[0x00, 0x01]); // CLASS=IN
            buf.extend_from_slice(&[0x00, 0x00, 0x00, 0x3C]); // TTL=60
            buf.extend_from_slice(&[0x00, 0x04]); // RDLENGTH=4
            buf.extend_from_slice(&[93, 184, 216, 34]); // RDATA=93.184.216.34
        }

        buf
    }

    #[test]
    fn extracts_hostname_from_a_record_response() {
        let pkt = build_dns_response("api.anthropic.com", true);
        assert_eq!(
            extract_dns_hostname(&pkt),
            Some("api.anthropic.com".to_string())
        );
    }

    #[test]
    fn rejects_response_with_no_answers() {
        let pkt = build_dns_response("api.anthropic.com", false);
        assert_eq!(extract_dns_hostname(&pkt), None);
    }

    #[test]
    fn rejects_query_packet() {
        let mut pkt = build_dns_response("example.com", true);
        // Clear QR bit (make it a query, not response)
        pkt[2] = 0x01;
        assert_eq!(extract_dns_hostname(&pkt), None);
    }

    #[test]
    fn rejects_short_packet() {
        assert_eq!(extract_dns_hostname(&[0u8; 5]), None);
    }

    #[test]
    fn rejects_error_response() {
        let mut pkt = build_dns_response("example.com", true);
        // Set RCODE=3 (NXDOMAIN)
        pkt[3] = 0x83;
        assert_eq!(extract_dns_hostname(&pkt), None);
    }
}
