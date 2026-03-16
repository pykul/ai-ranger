mod buffer;
mod capture;
mod classifier;
mod config;
mod dedup;
mod event;
mod identity;
mod output;
mod process;

use chrono::Utc;
use clap::Parser;
use config::{AppConfig, OutputConfig};
use event::{AiConnectionEvent, CaptureMode, DetectionMethod};
use output::sink::EventSink;
use std::path::PathBuf;
use std::sync::Arc;

/// Capacity of the mpsc channel between the capture thread and the async dispatch loop.
/// 1024 provides enough headroom for burst traffic without unbounded memory growth.
const EVENT_CHANNEL_CAPACITY: usize = 1024;

/// How long a connection_id remains in the dedup cache before expiring.
/// 5 seconds is generous enough to handle the boundary-crossing scenario where DNS
/// and SNI land in different 2-second buckets. See DECISIONS.md for full rationale.
const DEDUP_CACHE_TTL_SECS: u64 = 5;

/// Default timeout for fetching providers.toml from a remote URL at startup.
/// 10 seconds balances startup latency against slow networks.
const PROVIDERS_FETCH_TIMEOUT_SECS: u64 = 10;

/// Default interval between SQLite buffer drain attempts.
/// 30 seconds balances latency to the backend against unnecessary polling.
pub const DRAIN_INTERVAL_SECS: u64 = 30;

/// Maximum backoff interval for the drain loop after repeated failures.
/// 5 minutes caps the worst-case delay before retrying a backend connection.
pub const DRAIN_MAX_BACKOFF_SECS: u64 = 300;

/// Multiplier for exponential backoff in the drain loop.
const DRAIN_BACKOFF_MULTIPLIER: u64 = 2;

/// Default maximum events read from the SQLite buffer per drain cycle.
/// 500 balances memory usage against upload efficiency.
pub const DRAIN_BATCH_SIZE: usize = 500;

#[derive(Parser)]
#[command(name = "ai-ranger", about = "Passive AI provider detection agent")]
struct Cli {
    /// Path to config.toml
    #[arg(long, default_value = "config.toml")]
    config: PathBuf,

    /// Capture mode (dns-sni)
    #[arg(long, default_value = "dns-sni")]
    mode: String,

    /// Enroll with a backend
    #[arg(long)]
    enroll: bool,

    /// Enrollment token
    #[arg(long)]
    token: Option<String>,

    /// Backend URL for enrollment
    #[arg(long)]
    backend: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Load app config (config.toml)
    let app_config = AppConfig::load(&cli.config).unwrap_or_else(|e| {
        eprintln!("[ai-ranger] Warning: could not load config: {e}");
        AppConfig::default()
    });

    // Initialize provider registry (3-tier: fetch URL → local file → bundled)
    let providers_timeout = app_config
        .agent
        .providers_fetch_timeout_secs
        .unwrap_or(PROVIDERS_FETCH_TIMEOUT_SECS);
    let fetched_providers =
        fetch_providers_url(app_config.agent.providers_url.as_deref(), providers_timeout).await;
    let local_providers = identity::config::config_dir().map(|d| d.join("providers.toml"));
    classifier::providers::init_with_fetched(
        fetched_providers.as_deref(),
        local_providers.as_deref(),
    );

    // Handle enrollment
    if cli.enroll {
        match (cli.token.as_deref(), cli.backend.as_deref()) {
            (Some(token), Some(backend)) => {
                let agent_config = identity::config::AgentConfig {
                    agent_id: uuid::Uuid::new_v4().to_string(),
                    org_id: String::new(), // populated by backend in Phase 2
                    backend_url: backend.to_string(),
                    machine_hostname: identity::config::machine_hostname(),
                    os_username: identity::config::os_username(),
                    enrolled_at: Utc::now().timestamp_millis(),
                };
                if let Err(e) = agent_config.save() {
                    eprintln!("[ai-ranger] Failed to save enrollment config: {e}");
                    std::process::exit(1);
                }
                eprintln!(
                    "[ai-ranger] Enrolled as {} (token: {token})",
                    agent_config.agent_id
                );
                eprintln!("[ai-ranger] Config saved. Backend enrollment will complete in Phase 2.");
                return;
            }
            _ => {
                eprintln!("[ai-ranger] --enroll requires --token and --backend");
                std::process::exit(1);
            }
        }
    }

    // Load agent identity (if enrolled)
    let agent_config = identity::config::AgentConfig::load();
    let agent_id = agent_config
        .as_ref()
        .map(|c| c.agent_id.clone())
        .unwrap_or_default();
    let machine_hostname = identity::config::machine_hostname();
    let os_username = identity::config::os_username();
    let os_type = std::env::consts::OS.to_string();

    // Determine if any HTTP output is configured (activates SQLite buffer)
    let has_http_output = app_config
        .outputs
        .iter()
        .any(|o| matches!(o, OutputConfig::Http { .. }));

    // Build sinks from config
    let http_batch = app_config.agent.http_batch_size.map(|v| v as usize);
    let webhook_batch_default = app_config.agent.webhook_batch_size.map(|v| v as usize);
    let sinks = build_sinks(
        &app_config.outputs,
        &agent_id,
        http_batch,
        webhook_batch_default,
    );
    let sink: Arc<dyn EventSink> = if sinks.len() == 1 {
        sinks.into_iter().next().unwrap()
    } else {
        Arc::new(output::fanout::FanoutSink::new(sinks))
    };

    // Set up SQLite buffer if HTTP output is configured
    let event_buffer: Option<Arc<buffer::store::EventBuffer>> = if has_http_output {
        let buf_path = identity::config::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("buffer.db");
        match buffer::store::EventBuffer::open(&buf_path) {
            Ok(buf) => {
                eprintln!("[ai-ranger] SQLite buffer active at {}", buf_path.display());
                Some(Arc::new(buf))
            }
            Err(e) => {
                eprintln!("[ai-ranger] Warning: could not open buffer DB: {e}");
                eprintln!("[ai-ranger] HTTP events will be sent directly (no buffering)");
                None
            }
        }
    } else {
        None
    };

    // Spawn buffer drain task
    let drain_interval = app_config
        .agent
        .drain_interval_secs
        .unwrap_or(DRAIN_INTERVAL_SECS);
    let drain_batch = app_config
        .agent
        .drain_batch_size
        .unwrap_or(DRAIN_BATCH_SIZE as u64) as usize;
    if let Some(ref buf) = event_buffer {
        let buf_clone = Arc::clone(buf);
        let sink_clone = Arc::clone(&sink);
        tokio::spawn(async move {
            drain_loop(buf_clone, sink_clone, drain_interval, drain_batch).await;
        });
    }

    eprintln!("[ai-ranger] AI provider detection agent");
    eprintln!("[ai-ranger] Mode: {}", app_config.agent.mode);
    eprintln!("[ai-ranger] Press Ctrl+C to stop.\n");

    // Use a channel to pass events from the blocking capture thread to async sinks.
    // This avoids Handle::block_on() inside spawn_blocking, which can deadlock.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<AiConnectionEvent>(EVENT_CHANNEL_CAPACITY);

    // Async task: receive events from channel, deduplicate, dispatch to sinks + buffer.
    let sink_for_dispatch = Arc::clone(&sink);
    let buffer_for_dispatch = event_buffer.clone();
    let dispatch_task = tokio::spawn(async move {
        let mut dedup_cache =
            dedup::DedupCache::new(std::time::Duration::from_secs(DEDUP_CACHE_TTL_SECS));
        while let Some(event) = rx.recv().await {
            if dedup_cache.is_duplicate(&event.connection_id) {
                continue;
            }
            if let Some(ref buf) = buffer_for_dispatch {
                if let Err(e) = buf.insert(&event) {
                    eprintln!("[ai-ranger] Buffer insert error: {e}");
                }
            }
            if let Err(e) = sink_for_dispatch.send(&event).await {
                eprintln!("[ai-ranger] Sink error: {e}");
            }
        }
    });

    // Windows: start ETW DNS-Client monitoring for IPv6 DNS resolution events.
    // This runs in parallel with the SIO_RCVALL IPv4 capture below.
    // The ETW trace handle must be kept alive - dropping it stops the trace.
    #[cfg(windows)]
    let _etw_trace = {
        let tx_etw = tx.clone();
        match capture::etw_dns::start(
            tx_etw,
            machine_hostname.clone(),
            os_username.clone(),
            agent_id.clone(),
            os_type.clone(),
        ) {
            Ok(trace) => Some(trace),
            Err(e) => {
                eprintln!("[ai-ranger] ETW DNS-Client monitoring unavailable: {e}");
                None
            }
        }
    };

    // Capture loop runs in a blocking thread since raw socket recv() blocks.
    let capture_result = tokio::task::spawn_blocking(move || -> Result<(), String> {
        capture::pcap::capture(|packet| {
            // Detection priority: SNI → DNS → IP range fallback.
            let (provider, provider_host, detection_method) = if !packet.hostname.is_empty() {
                // SNI or DNS produced a hostname - try to classify it.
                if let Some(provider) = classifier::classify(&packet.hostname) {
                    let dm = match packet.detection_method {
                        "dns" => DetectionMethod::Dns,
                        _ => DetectionMethod::Sni,
                    };
                    (provider, packet.hostname.clone(), dm)
                } else {
                    return; // hostname present but not a known provider
                }
            } else {
                // No hostname (ECH hid SNI, no DNS match) - try IP range fallback.
                if let Some((provider, synth_host)) = classifier::classify_ip(&packet.dst_ip) {
                    (provider, synth_host.to_string(), DetectionMethod::IpRange)
                } else {
                    return; // no match by any method
                }
            };

            let (process_pid, process_name) = process::pid_and_name(packet.src_port);
            let proc_path = process::process_path(process_pid);
            let timestamp_ms = Utc::now().timestamp_millis();
            let connection_id =
                dedup::compute_connection_id(&packet.src_ip, &provider_host, timestamp_ms);

            let event = AiConnectionEvent {
                agent_id: agent_id.clone(),
                machine_hostname: machine_hostname.clone(),
                os_username: os_username.clone(),
                os_type: os_type.clone(),
                connection_id,
                timestamp_ms,
                duration_ms: None,
                provider: provider.to_string(),
                provider_host,
                model_hint: None,
                process_name,
                process_pid,
                process_path: proc_path,
                src_ip: packet.src_ip,
                detection_method,
                capture_mode: CaptureMode::DnsSni,
                content_available: false,
                payload_ref: None,
                model_exact: None,
                token_count_input: None,
                token_count_output: None,
                latency_ttfb_ms: None,
            };

            // Send event through channel - non-blocking, no Handle::block_on needed.
            if let Err(e) = tx.blocking_send(event) {
                eprintln!("[ai-ranger] Channel send error: {e}");
            }
        })
        .map_err(|e| e.to_string())
    })
    .await;

    // Capture ended - drop sender to close channel, wait for dispatch to finish.
    drop(capture_result); // ensure tx is dropped via the closure
    let _ = dispatch_task.await;

    // Flush remaining buffer on shutdown
    if let Some(ref buf) = event_buffer {
        if let Err(e) = drain_once(buf, &sink, drain_batch).await {
            eprintln!("[ai-ranger] Final drain error: {e}");
        }
    }
    if let Err(e) = sink.flush().await {
        eprintln!("[ai-ranger] Final flush error: {e}");
    }
}

/// Fetch providers.toml from a remote URL. Returns None on any failure.
async fn fetch_providers_url(url: Option<&str>, timeout_secs: u64) -> Option<String> {
    let url = url?;
    eprintln!("[ai-ranger] Fetching providers from {url}");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .ok()?;
    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.text().await {
            Ok(body) => {
                eprintln!("[ai-ranger] Fetched providers.toml ({} bytes)", body.len());
                Some(body)
            }
            Err(e) => {
                eprintln!("[ai-ranger] Failed to read providers response: {e}");
                None
            }
        },
        Ok(resp) => {
            eprintln!(
                "[ai-ranger] Providers fetch returned HTTP {}, falling back",
                resp.status()
            );
            None
        }
        Err(e) => {
            eprintln!("[ai-ranger] Providers fetch failed: {e}");
            None
        }
    }
}

/// Background drain loop: periodically reads buffered events and POSTs them.
/// Uses exponential backoff on failure up to DRAIN_MAX_BACKOFF_SECS.
async fn drain_loop(
    buf: Arc<buffer::store::EventBuffer>,
    sink: Arc<dyn EventSink>,
    base_interval: u64,
    batch_size: usize,
) {
    let mut interval_secs: u64 = base_interval;

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;

        match drain_once(&buf, &sink, batch_size).await {
            Ok(drained) => {
                if drained > 0 {
                    eprintln!("[ai-ranger] Buffer drain: uploaded {drained} events");
                }
                interval_secs = base_interval; // reset backoff on success
            }
            Err(e) => {
                eprintln!("[ai-ranger] Buffer drain failed: {e}");
                interval_secs =
                    (interval_secs * DRAIN_BACKOFF_MULTIPLIER).min(DRAIN_MAX_BACKOFF_SECS);
                eprintln!("[ai-ranger] Next drain attempt in {interval_secs}s (backoff)");
            }
        }
    }
}

/// Drain up to `batch_size` events from the buffer, send via sink, delete on success.
/// Returns the number of events successfully drained.
async fn drain_once(
    buf: &buffer::store::EventBuffer,
    sink: &Arc<dyn EventSink>,
    batch_size: usize,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let batch = buf.read_batch(batch_size)?;
    if batch.is_empty() {
        return Ok(0);
    }

    let ids: Vec<i64> = batch.iter().map(|(id, _)| *id).collect();

    // Deserialize and send each event through the sink
    for (_, json) in &batch {
        let event: AiConnectionEvent = serde_json::from_str(json)?;
        sink.send(&event).await?;
    }
    sink.flush().await?;

    // All sent successfully - delete from buffer
    buf.delete_batch(&ids)?;
    Ok(batch.len())
}

fn build_sinks(
    outputs: &[OutputConfig],
    agent_id: &str,
    http_batch: Option<usize>,
    webhook_batch_default: Option<usize>,
) -> Vec<Arc<dyn EventSink>> {
    let mut sinks: Vec<Arc<dyn EventSink>> = Vec::new();

    for output in outputs {
        match output {
            OutputConfig::Stdout => {
                sinks.push(Arc::new(output::stdout::StdoutSink));
            }
            OutputConfig::File { path } => {
                sinks.push(Arc::new(output::file::FileSink::new(PathBuf::from(path))));
            }
            OutputConfig::Http { url, .. } => {
                sinks.push(Arc::new(output::http::HttpSink::new(
                    url.clone(),
                    agent_id.to_string(),
                    http_batch,
                )));
            }
            OutputConfig::Webhook {
                url,
                headers,
                batch_size,
            } => {
                // Per-sink batch_size in config overrides the global webhook_batch_size.
                let effective_batch = batch_size.or(webhook_batch_default);
                sinks.push(Arc::new(output::webhook::WebhookSink::new(
                    url.clone(),
                    headers.clone(),
                    effective_batch,
                )));
            }
        }
    }

    if sinks.is_empty() {
        sinks.push(Arc::new(output::stdout::StdoutSink));
    }

    sinks
}
