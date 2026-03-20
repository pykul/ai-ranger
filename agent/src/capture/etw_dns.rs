/// Windows ETW DNS-Client provider integration.
///
/// Subscribes to Microsoft-Windows-DNS-Client (Event ID 3008: query completed)
/// to detect AI provider DNS resolutions. This captures hostname→provider matches
/// for connections that go over IPv6 (which SIO_RCVALL cannot see) and for any
/// DNS resolution that uses the Windows system DNS client.
///
/// Limitation: browsers that use their own internal DoH resolver (Chrome, Firefox,
/// Edge, Brave) bypass the Windows DNS client entirely, so ETW DNS-Client events
/// do not fire for those connections.
#[cfg(windows)]
use crate::classifier;
#[cfg(windows)]
use crate::dedup;
#[cfg(windows)]
use crate::event::{AiConnectionEvent, CaptureMode, DetectionMethod};
#[cfg(windows)]
use crate::process;

#[cfg(windows)]
use ferrisetw::parser::Parser;
#[cfg(windows)]
use ferrisetw::provider::Provider;
#[cfg(windows)]
use ferrisetw::schema_locator::SchemaLocator;
#[cfg(windows)]
use ferrisetw::trace::UserTrace;
#[cfg(windows)]
use ferrisetw::EventRecord;

#[cfg(windows)]
const TRACE_NAME: &str = "ai-ranger-dns";

/// Microsoft-Windows-DNS-Client ETW provider GUID.
/// Fires events for every DNS resolution through the Windows system DNS client.
#[cfg(windows)]
const MS_DNS_CLIENT_PROVIDER_GUID: &str = "1c95126e-7eea-49a9-a3fe-a378b03ddb4d";

/// ETW event ID for "DNS query completed" from the DNS-Client provider.
#[cfg(windows)]
const ETW_DNS_QUERY_COMPLETED: u16 = 3008;

/// Start the ETW DNS-Client trace. Runs in the background via ferrisetw's
/// internal thread. Sends matched AI provider events through the provided channel.
///
/// Returns a handle that must be kept alive - dropping it stops the trace.
#[cfg(windows)]
pub fn start(
    tx: tokio::sync::mpsc::Sender<AiConnectionEvent>,
    machine_hostname: String,
    os_username: String,
    agent_id: String,
    os_type: String,
) -> Result<UserTrace, Box<dyn std::error::Error + Send + Sync>> {
    // Clean up any stale session from a previous crash before attempting to start.
    // This is a no-op if no stale session exists.
    let _ = std::process::Command::new("logman")
        .args(["stop", TRACE_NAME, "-ets"])
        .output();

    let dns_provider = Provider::by_guid(MS_DNS_CLIENT_PROVIDER_GUID)
        .add_callback(
            move |record: &EventRecord, schema_locator: &SchemaLocator| {
                if record.event_id() != ETW_DNS_QUERY_COMPLETED {
                    return;
                }

                let schema = match schema_locator.event_schema(record) {
                    Ok(s) => s,
                    Err(_) => return,
                };

                let parser = Parser::create(record, &schema);

                let query_name: String = match parser.try_parse("QueryName") {
                    Ok(name) => name,
                    Err(_) => return,
                };

                // Classify the resolved hostname against known AI providers.
                let Some(provider) = classifier::classify(&query_name) else {
                    return;
                };

                // Get process name, path, and owner from the ETW event's process ID.
                let pid = record.process_id();
                let process_name = process::name_by_pid(pid);
                let proc_path = process::process_path(pid);

                // Resolve the process owner per-connection. Falls back to the
                // agent's own username when PID is 0 or resolution fails.
                let event_username = if pid == 0 {
                    os_username.clone()
                } else {
                    process::resolve_process_owner(pid).unwrap_or_else(|| "unknown".to_string())
                };

                let timestamp_ms = chrono::Utc::now().timestamp_millis();
                // ETW DNS events have no source IP - use empty string for the hash.
                // The SNI path will have the real src_ip, so if both fire the connection_id
                // will differ. Dedup still works because (empty, host, bucket) is consistent
                // across ETW events for the same resolution.
                let connection_id = dedup::compute_connection_id("", &query_name, timestamp_ms);

                let event = AiConnectionEvent::new(
                    agent_id.clone(),
                    machine_hostname.clone(),
                    event_username,
                    os_type.clone(),
                    connection_id,
                    timestamp_ms,
                    provider.to_string(),
                    query_name,
                    process_name,
                    pid,
                    proc_path,
                    String::new(), // ETW DNS events don't have a source IP
                    DetectionMethod::Dns,
                    CaptureMode::DnsSni,
                );

                // Send through channel - blocking_send is safe here because ferrisetw
                // callbacks run on a dedicated ETW processing thread, not a tokio thread.
                let _ = tx.blocking_send(event);
            },
        )
        .build();

    let trace = UserTrace::new()
        .named(String::from(TRACE_NAME))
        .enable(dns_provider)
        .start_and_process()
        .map_err(|e| format!("ETW trace start failed: {e:?}"))?;

    eprintln!("[ai-ranger] ETW DNS-Client monitoring active (Windows DNS resolution events)");

    Ok(trace)
}
