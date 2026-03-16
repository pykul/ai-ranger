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

    // Compute connection_id for dedup and construct event.
    let timestamp_ms = Utc::now().timestamp_millis();
    let connection_id = dedup::compute_connection_id(&packet.src_ip, &provider_host, timestamp_ms);

    Some(AiConnectionEvent::new(
        ctx.agent_id.clone(),
        ctx.machine_hostname.clone(),
        ctx.os_username.clone(),
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
