//! Prost-generated protobuf types for agent-to-gateway communication.
//!
//! Generated at compile time by build.rs from proto/ranger/v1/*.proto.

/// The ranger.v1 protobuf package containing AiConnectionEvent, EventBatch, etc.
#[allow(dead_code)] // EnrollmentRequest/Response not used by the agent yet (enrollment uses JSON).
pub mod ranger_v1 {
    include!(concat!(env!("OUT_DIR"), "/ranger.v1.rs"));
}

use crate::event;

impl From<&event::AiConnectionEvent> for ranger_v1::AiConnectionEvent {
    /// Convert the agent's internal AiConnectionEvent to the protobuf type.
    fn from(e: &event::AiConnectionEvent) -> Self {
        Self {
            agent_id: e.agent_id.clone(),
            machine_hostname: e.machine_hostname.clone(),
            os_username: e.os_username.clone(),
            os_type: e.os_type.clone(),
            timestamp_ms: e.timestamp_ms,
            duration_ms: e.duration_ms,
            provider: e.provider.clone(),
            provider_host: e.provider_host.clone(),
            model_hint: e.model_hint.clone(),
            process_name: e.process_name.clone(),
            process_pid: e.process_pid,
            process_path: e.process_path.clone(),
            connection_id: e.connection_id.clone(),
            detection_method: match e.detection_method {
                event::DetectionMethod::Sni => ranger_v1::DetectionMethod::Sni as i32,
                event::DetectionMethod::Dns => ranger_v1::DetectionMethod::Dns as i32,
                event::DetectionMethod::IpRange => ranger_v1::DetectionMethod::IpRange as i32,
                event::DetectionMethod::TcpHeuristic => {
                    ranger_v1::DetectionMethod::TcpHeuristic as i32
                }
            },
            capture_mode: match e.capture_mode {
                event::CaptureMode::DnsSni => ranger_v1::CaptureMode::DnsSni as i32,
                event::CaptureMode::Mitm => ranger_v1::CaptureMode::Mitm as i32,
            },
            src_ip: e.src_ip.clone(),
            content_available: e.content_available,
            payload_ref: e.payload_ref.clone(),
            model_exact: e.model_exact.clone(),
            token_count_input: e.token_count_input,
            token_count_output: e.token_count_output,
            latency_ttfb_ms: e.latency_ttfb_ms,
        }
    }
}
