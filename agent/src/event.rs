use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(dead_code)] // TcpHeuristic reserved for future use
pub enum DetectionMethod {
    Sni,
    Dns,
    IpRange,
    TcpHeuristic,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(dead_code)] // Mitm reserved for Phase 5
pub enum CaptureMode {
    DnsSni,
    Mitm, // Phase 5+ - reserved, do not use
}

fn is_false(v: &bool) -> bool {
    !v
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AiConnectionEvent {
    // Identity
    pub agent_id: String,
    pub machine_hostname: String,
    pub os_username: String,
    pub os_type: String,

    // Dedup
    /// Hash of (src_ip, provider_host, timestamp_ms / 2000). Empty for
    /// IP-range-only events that cannot be reliably deduplicated.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub connection_id: String,

    // Timing
    pub timestamp_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    // Provider
    pub provider: String,
    pub provider_host: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_hint: Option<String>, // Phase 5 - populated from request body in MITM mode

    // Process
    pub process_name: String,
    pub process_pid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_path: Option<String>,

    // Network
    pub src_ip: String,

    // Detection
    pub detection_method: DetectionMethod,
    pub capture_mode: CaptureMode,

    // Phase 5 - MITM only. Always default/None until Phase 5.
    // Omitted from JSON output when empty via skip_serializing_if.
    // default on deserialize so roundtrip through the SQLite buffer works.
    #[serde(default, skip_serializing_if = "is_false")]
    pub content_available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_exact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_count_input: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_count_output: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ttfb_ms: Option<u32>,
}

impl AiConnectionEvent {
    /// Construct a new event with Phase 5 fields defaulted to None/false.
    ///
    /// Only accepts fields that genuinely vary at construction time.
    /// Phase 5 MITM fields (content_available, payload_ref, model_exact,
    /// token_count_input, token_count_output, latency_ttfb_ms) are always
    /// default -- they will be populated when MITM mode ships in Phase 5.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        agent_id: String,
        machine_hostname: String,
        os_username: String,
        os_type: String,
        connection_id: String,
        timestamp_ms: i64,
        provider: String,
        provider_host: String,
        process_name: String,
        process_pid: u32,
        process_path: Option<String>,
        src_ip: String,
        detection_method: DetectionMethod,
        capture_mode: CaptureMode,
    ) -> Self {
        Self {
            agent_id,
            machine_hostname,
            os_username,
            os_type,
            connection_id,
            timestamp_ms,
            duration_ms: None,
            provider,
            provider_host,
            model_hint: None,
            process_name,
            process_pid,
            process_path,
            src_ip,
            detection_method,
            capture_mode,
            content_available: false,
            payload_ref: None,
            model_exact: None,
            token_count_input: None,
            token_count_output: None,
            latency_ttfb_ms: None,
        }
    }
}
