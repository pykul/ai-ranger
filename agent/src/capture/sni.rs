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

    /// Build a minimal but valid TLS ClientHello containing an SNI extension.
    fn build_client_hello(hostname: &str) -> Vec<u8> {
        // SNI extension body: list_length(2) + type(1) + name_length(2) + hostname
        let name_bytes = hostname.as_bytes();
        let sni_entry_len = 1 + 2 + name_bytes.len(); // type + name_length + name
        let sni_list_len = sni_entry_len;
        let sni_ext_data_len = 2 + sni_entry_len; // list_length + entry

        // Extension: type(2) + length(2) + data
        let ext_len = 4 + sni_ext_data_len;

        // ClientHello body: version(2) + random(32) + session_id_len(1) +
        //   cipher_suites_len(2) + one_suite(2) + comp_methods_len(1) + null_comp(1) +
        //   extensions_len(2) + extension
        let hello_body_len = 2 + 32 + 1 + 2 + 2 + 1 + 1 + 2 + ext_len;

        // Handshake header: type(1) + length(3)
        let handshake_len = 4 + hello_body_len;

        // TLS record: type(1) + version(2) + length(2)
        let mut buf = Vec::with_capacity(5 + handshake_len);

        // TLS record header
        buf.push(0x16); // handshake
        buf.extend_from_slice(&[0x03, 0x01]); // TLS 1.0 (record layer version)
        buf.extend_from_slice(&(handshake_len as u16).to_be_bytes());

        // Handshake header
        buf.push(0x01); // ClientHello
        let hl = hello_body_len as u32;
        buf.extend_from_slice(&[
            ((hl >> 16) & 0xff) as u8,
            ((hl >> 8) & 0xff) as u8,
            (hl & 0xff) as u8,
        ]);

        // ClientHello body
        buf.extend_from_slice(&[0x03, 0x03]); // TLS 1.2
        buf.extend_from_slice(&[0u8; 32]); // random
        buf.push(0); // session ID length = 0
        buf.extend_from_slice(&2u16.to_be_bytes()); // cipher suites length = 2
        buf.extend_from_slice(&[0x00, 0x2f]); // TLS_RSA_WITH_AES_128_CBC_SHA
        buf.push(1); // compression methods length = 1
        buf.push(0); // null compression

        // Extensions
        buf.extend_from_slice(&(ext_len as u16).to_be_bytes());

        // SNI extension (type 0x0000)
        buf.extend_from_slice(&[0x00, 0x00]); // extension type = SNI
        buf.extend_from_slice(&(sni_ext_data_len as u16).to_be_bytes());
        buf.extend_from_slice(&(sni_list_len as u16).to_be_bytes()); // server name list length
        buf.push(0x00); // host_name type
        buf.extend_from_slice(&(name_bytes.len() as u16).to_be_bytes());
        buf.extend_from_slice(name_bytes);

        buf
    }

    #[test]
    fn extracts_sni_from_valid_client_hello() {
        let hello = build_client_hello("api.anthropic.com");
        assert_eq!(extract_sni(&hello), Some("api.anthropic.com".to_string()));
    }

    #[test]
    fn extracts_long_hostname() {
        let hello = build_client_hello("generativelanguage.googleapis.com");
        assert_eq!(
            extract_sni(&hello),
            Some("generativelanguage.googleapis.com".to_string())
        );
    }

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
