/// Parse a raw TCP payload and extract the SNI hostname from a TLS ClientHello.
///
/// The TLS ClientHello is sent in plaintext before encryption begins. Extracting
/// the SNI requires zero cryptography — it is pure byte parsing of a known structure.
///
/// Returns `None` if the payload is not a TLS ClientHello or contains no SNI extension.
pub fn extract_sni(data: &[u8]) -> Option<String> {
    // TLS record header: content_type(1) + version(2) + length(2) = 5 bytes
    if data.len() < 5 {
        return None;
    }
    // 0x16 = handshake record type
    if data[0] != 0x16 {
        return None;
    }

    let record_len = u16::from_be_bytes([data[3], data[4]]) as usize;
    if data.len() < 5 + record_len {
        return None;
    }

    let handshake = &data[5..5 + record_len];

    // Handshake header: type(1) + length(3)
    if handshake.len() < 4 {
        return None;
    }
    // 0x01 = ClientHello
    if handshake[0] != 0x01 {
        return None;
    }

    let hello_len = u32::from_be_bytes([0, handshake[1], handshake[2], handshake[3]]) as usize;
    if handshake.len() < 4 + hello_len {
        return None;
    }

    let hello = &handshake[4..4 + hello_len];

    // ClientHello body: version(2) + random(32) = 34 bytes to skip
    if hello.len() < 34 {
        return None;
    }
    let mut pos = 34;

    // Session ID: length(1) + data
    if pos >= hello.len() {
        return None;
    }
    pos += 1 + hello[pos] as usize;

    // Cipher suites: length(2) + data
    if pos + 2 > hello.len() {
        return None;
    }
    let cs_len = u16::from_be_bytes([hello[pos], hello[pos + 1]]) as usize;
    pos += 2 + cs_len;

    // Compression methods: length(1) + data
    if pos >= hello.len() {
        return None;
    }
    pos += 1 + hello[pos] as usize;

    // Extensions: length(2) + data
    if pos + 2 > hello.len() {
        return None;
    }
    let ext_total = u16::from_be_bytes([hello[pos], hello[pos + 1]]) as usize;
    pos += 2;
    let ext_end = pos + ext_total;
    if ext_end > hello.len() {
        return None;
    }

    // Walk extensions looking for type 0x0000 (SNI)
    while pos + 4 <= ext_end {
        let ext_type = u16::from_be_bytes([hello[pos], hello[pos + 1]]);
        let ext_len = u16::from_be_bytes([hello[pos + 2], hello[pos + 3]]) as usize;
        pos += 4;
        if pos + ext_len > ext_end {
            return None;
        }
        if ext_type == 0x0000 {
            return parse_sni_extension(&hello[pos..pos + ext_len]);
        }
        pos += ext_len;
    }

    None
}

/// Parse the SNI extension body and return the hostname.
///
/// Structure: list_length(2) + entry_type(1) + name_length(2) + hostname
fn parse_sni_extension(data: &[u8]) -> Option<String> {
    if data.len() < 5 {
        return None;
    }
    // entry_type 0x00 = host_name
    if data[2] != 0x00 {
        return None;
    }
    let name_len = u16::from_be_bytes([data[3], data[4]]) as usize;
    if data.len() < 5 + name_len {
        return None;
    }
    String::from_utf8(data[5..5 + name_len].to_vec()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_payload() {
        assert_eq!(extract_sni(&[]), None);
    }

    #[test]
    fn rejects_non_tls_record() {
        // First byte is not 0x16
        let data = vec![0x17u8, 0x03, 0x03, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(extract_sni(&data), None);
    }

    #[test]
    fn rejects_non_clienthello_handshake() {
        // Valid TLS record header, but handshake type 0x02 = ServerHello
        let mut data = vec![0u8; 50];
        data[0] = 0x16; // handshake record
        data[3] = 0x00;
        data[4] = 0x05; // record length = 5
        data[5] = 0x02; // ServerHello, not ClientHello
        assert_eq!(extract_sni(&data), None);
    }

    #[test]
    fn rejects_short_payload() {
        assert_eq!(extract_sni(&[0x16, 0x03, 0x03]), None);
    }
}
