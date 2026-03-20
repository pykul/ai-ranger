//! Packet-to-event transformation pipeline.
//!
//! Runs the full detection pipeline on a captured packet: classify hostname
//! via SNI or DNS, fall back to IP range matching, resolve the owning process,
//! compute a connection_id, and construct an AiConnectionEvent.

use crate::capture::pcap::PacketInfo;
use crate::classifier;
use crate::dedup;
use crate::event::{AiConnectionEvent, CaptureMode, DetectionMethod};
use crate::process;
use chrono::Utc;

/// Shared context for the detection pipeline, computed once at startup.
#[derive(Clone)]
pub(crate) struct PipelineContext {
    pub agent_id: String,
    pub machine_hostname: String,
    pub os_username: String,
    pub os_type: String,
}

/// Transform a captured packet into an AiConnectionEvent.
///
/// Detection priority: SNI hostname -> DNS hostname -> IP range fallback.
/// Returns None if the packet does not match any known AI provider.
pub(crate) fn handle_packet(
    packet: PacketInfo,
    ctx: &PipelineContext,
) -> Option<AiConnectionEvent> {
    // Classify: try hostname first, fall back to IP range.
    let (provider, provider_host, detection_method) = if !packet.hostname.is_empty() {
        let provider = classifier::classify(&packet.hostname)?;
        let dm = match packet.detection_method {
            "dns" => DetectionMethod::Dns,
            _ => DetectionMethod::Sni,
        };
        (provider, packet.hostname.clone(), dm)
    } else {
        let (provider, synth_host) = classifier::classify_ip(&packet.dst_ip)?;
        (provider, synth_host.to_string(), DetectionMethod::IpRange)
    };

    // Resolve process ownership.
    let (process_pid, process_name) = process::pid_and_name(packet.src_port);
    let proc_path = process::process_path(process_pid);

    // Resolve the OS username of the process owner. Falls back to the agent's
    // own username (ctx.os_username) when PID is 0 (DNS events without process
    // attribution) or when resolution fails.
    let os_username = if process_pid == 0 {
        ctx.os_username.clone()
    } else {
        process::resolve_process_owner(process_pid).unwrap_or_else(|| "unknown".to_string())
    };

    // Compute connection_id for dedup and construct event.
    let timestamp_ms = Utc::now().timestamp_millis();
    let connection_id = dedup::compute_connection_id(&packet.src_ip, &provider_host, timestamp_ms);

    Some(AiConnectionEvent::new(
        ctx.agent_id.clone(),
        ctx.machine_hostname.clone(),
        os_username,
        ctx.os_type.clone(),
        connection_id,
        timestamp_ms,
        provider.to_string(),
        provider_host,
        process_name,
        process_pid,
        proc_path,
        packet.src_ip,
        detection_method,
        CaptureMode::DnsSni,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> PipelineContext {
        PipelineContext {
            agent_id: "test-agent".to_string(),
            machine_hostname: "test-host".to_string(),
            os_username: "testuser".to_string(),
            os_type: "linux".to_string(),
        }
    }

    fn ensure_providers() {
        classifier::providers::init_with_fetched(None, None);
    }

    #[test]
    fn sni_match_produces_event() {
        ensure_providers();
        let packet = PacketInfo {
            hostname: "api.anthropic.com".to_string(),
            src_ip: "10.0.0.1".to_string(),
            dst_ip: "160.79.104.1".to_string(),
            src_port: 0,
            detection_method: "sni",
        };
        let event = handle_packet(packet, &test_ctx()).unwrap();
        assert_eq!(event.provider, "anthropic");
        assert_eq!(event.provider_host, "api.anthropic.com");
        assert_eq!(event.detection_method, DetectionMethod::Sni);
        assert_eq!(event.os_type, "linux");
        assert!(!event.connection_id.is_empty());
    }

    #[test]
    fn unknown_hostname_returns_none() {
        ensure_providers();
        let packet = PacketInfo {
            hostname: "example.com".to_string(),
            src_ip: "10.0.0.1".to_string(),
            dst_ip: "93.184.216.34".to_string(),
            src_port: 0,
            detection_method: "sni",
        };
        assert!(handle_packet(packet, &test_ctx()).is_none());
    }

    #[test]
    fn ip_range_match_produces_event_with_ip_range_method() {
        ensure_providers();
        // Empty hostname forces IP range fallback. 160.79.104.1 is in Anthropic's range.
        let packet = PacketInfo {
            hostname: String::new(),
            src_ip: "10.0.0.1".to_string(),
            dst_ip: "160.79.104.1".to_string(),
            src_port: 0,
            detection_method: "",
        };
        let event = handle_packet(packet, &test_ctx()).unwrap();
        assert_eq!(event.provider, "anthropic");
        assert_eq!(event.detection_method, DetectionMethod::IpRange);
    }

    #[test]
    fn dns_detection_method_set_correctly() {
        ensure_providers();
        let packet = PacketInfo {
            hostname: "api.openai.com".to_string(),
            src_ip: "10.0.0.1".to_string(),
            dst_ip: "104.18.0.1".to_string(),
            src_port: 0,
            detection_method: "dns",
        };
        let event = handle_packet(packet, &test_ctx()).unwrap();
        assert_eq!(event.provider, "openai");
        assert_eq!(event.detection_method, DetectionMethod::Dns);
    }
}
