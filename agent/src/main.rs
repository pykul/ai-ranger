mod buffer;
mod capture;
mod classifier;
mod config;
mod dedup;
mod event;
mod identity;
mod output;
mod pipeline;
mod process;

use config::{AppConfig, OutputConfig};
use event::AiConnectionEvent;
use output::sink::EventSink;
use pipeline::PipelineContext;
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

#[derive(clap::Parser)]
#[command(name = "ai-ranger", about = "Passive AI provider detection agent")]
struct Cli {
    #[arg(long, default_value = "config.toml")]
    config: std::path::PathBuf,
    #[arg(long)]
    enroll: bool,
    #[arg(long)]
    token: Option<String>,
    #[arg(long)]
    backend: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = <Cli as clap::Parser>::parse();

    let app_config = AppConfig::load(&cli.config).unwrap_or_else(|e| {
        eprintln!("[ai-ranger] Warning: could not load config: {e}");
        AppConfig::default()
    });

    // Initialize provider registry (3-tier: fetch URL -> local file -> bundled)
    let timeout = app_config
        .agent
        .providers_fetch_timeout_secs
        .unwrap_or(PROVIDERS_FETCH_TIMEOUT_SECS);
    let fetched =
        classifier::fetch_providers_url(app_config.agent.providers_url.as_deref(), timeout).await;
    let local = identity::config::config_dir().map(|d| d.join("providers.toml"));
    classifier::providers::init_with_fetched(fetched.as_deref(), local.as_deref());

    // Enrollment or identity loading (exits process if --enroll)
    let agent_config =
        identity::enroll::load_or_enroll(cli.enroll, cli.token.as_deref(), cli.backend.as_deref());
    let agent_id = agent_config
        .as_ref()
        .map(|c| c.agent_id.clone())
        .unwrap_or_default();

    let ctx = PipelineContext {
        agent_id: agent_id.clone(),
        machine_hostname: identity::config::machine_hostname(),
        os_username: identity::config::os_username(),
        os_type: std::env::consts::OS.to_string(),
    };

    // Build sinks and optional SQLite buffer
    let http_batch = app_config.agent.http_batch_size.map(|v| v as usize);
    let webhook_batch = app_config.agent.webhook_batch_size.map(|v| v as usize);
    let sinks = output::build_sinks(&app_config.outputs, &agent_id, http_batch, webhook_batch);
    let sink: Arc<dyn EventSink> = if sinks.len() == 1 {
        sinks.into_iter().next().unwrap()
    } else {
        Arc::new(output::fanout::FanoutSink::new(sinks))
    };

    let has_http = app_config
        .outputs
        .iter()
        .any(|o| matches!(o, OutputConfig::Http { .. }));
    let event_buffer = buffer::store::open_if_needed(has_http);

    // Spawn buffer drain task
    let drain_interval = app_config
        .agent
        .drain_interval_secs
        .unwrap_or(buffer::drain::DRAIN_INTERVAL_SECS);
    let drain_batch = app_config
        .agent
        .drain_batch_size
        .unwrap_or(buffer::drain::DRAIN_BATCH_SIZE as u64) as usize;
    if let Some(ref buf) = event_buffer {
        let b = Arc::clone(buf);
        let s = Arc::clone(&sink);
        tokio::spawn(buffer::drain::drain_loop(b, s, drain_interval, drain_batch));
    }

    eprintln!("[ai-ranger] AI provider detection agent");
    eprintln!("[ai-ranger] Mode: {}", app_config.agent.mode);
    eprintln!("[ai-ranger] Press Ctrl+C to stop.\n");

    // Channel: capture thread -> async dispatch loop
    let (tx, mut rx) = tokio::sync::mpsc::channel::<AiConnectionEvent>(EVENT_CHANNEL_CAPACITY);

    // Dispatch task: deduplicate, buffer, and fan out to sinks
    let sink_d = Arc::clone(&sink);
    let buf_d = event_buffer.clone();
    let dispatch = tokio::spawn(async move {
        let mut cache =
            dedup::DedupCache::new(std::time::Duration::from_secs(DEDUP_CACHE_TTL_SECS));
        while let Some(event) = rx.recv().await {
            if cache.is_duplicate(&event.connection_id) {
                continue;
            }
            if let Some(ref b) = buf_d {
                if let Err(e) = b.insert(&event) {
                    eprintln!("[ai-ranger] Buffer insert error: {e}");
                }
            }
            if let Err(e) = sink_d.send(&event).await {
                eprintln!("[ai-ranger] Sink error: {e}");
            }
        }
    });

    // Windows: start ETW DNS-Client monitoring for IPv6 DNS resolution events
    #[cfg(windows)]
    let _etw = {
        let tx_etw = tx.clone();
        match capture::etw_dns::start(
            tx_etw,
            ctx.machine_hostname.clone(),
            ctx.os_username.clone(),
            ctx.agent_id.clone(),
            ctx.os_type.clone(),
        ) {
            Ok(trace) => Some(trace),
            Err(e) => {
                eprintln!("[ai-ranger] ETW DNS-Client monitoring unavailable: {e}");
                None
            }
        }
    };

    // Capture loop (blocking thread - raw socket recv() blocks)
    let capture_result = tokio::task::spawn_blocking(move || -> Result<(), String> {
        capture::pcap::capture(|packet| {
            if let Some(event) = pipeline::handle_packet(packet, &ctx) {
                if let Err(e) = tx.blocking_send(event) {
                    eprintln!("[ai-ranger] Channel send error: {e}");
                }
            }
        })
        .map_err(|e| e.to_string())
    })
    .await;

    // Shutdown: close channel, drain remaining buffer, flush sinks
    drop(capture_result);
    let _ = dispatch.await;
    if let Some(ref buf) = event_buffer {
        if let Err(e) = buffer::drain::drain_once(buf, &sink, drain_batch).await {
            eprintln!("[ai-ranger] Final drain error: {e}");
        }
    }
    if let Err(e) = sink.flush().await {
        eprintln!("[ai-ranger] Final flush error: {e}");
    }
}
