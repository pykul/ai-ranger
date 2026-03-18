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
mod proto;

use config::{AppConfig, OutputConfig};
use event::AiConnectionEvent;
use output::sink::EventSink;
use pipeline::PipelineContext;
use std::sync::Arc;

/// Capacity of the mpsc channel between the capture thread and the async dispatch loop.
/// 1024 provides enough headroom for burst traffic without unbounded memory growth.
const EVENT_CHANNEL_CAPACITY: usize = 1024;

/// Interval for the background sink flush timer.
/// Events sitting in the HTTP batch would otherwise wait for the drain loop or
/// the batch size threshold (10 events). This timer ensures a single event is
/// visible in the dashboard within 500ms under light load.
const FLUSH_INTERVAL_MS: u64 = 500;

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
    /// Path to config.toml
    #[arg(long, default_value = "config.toml")]
    config: std::path::PathBuf,

    /// Enroll and exit without capturing (for installer scripts).
    /// For normal use, just pass --token and --backend to enroll and start in one step.
    #[arg(long)]
    enroll: bool,

    /// Enrollment token. If provided with --backend, the agent enrolls on first run
    /// and then starts capturing. If already enrolled, the flags are ignored.
    #[arg(long)]
    token: Option<String>,

    /// Backend URL (e.g. http://localhost:8080). Required with --token for enrollment.
    #[arg(long)]
    backend: Option<String>,
}

fn main() {
    let cli = <Cli as clap::Parser>::parse();

    // Enrollment uses reqwest::blocking which creates its own runtime.
    // Handle it before entering the tokio async runtime to avoid nesting.
    // This covers both --enroll (exit after) and auto-enroll (continue to capture).
    let needs_enrollment = cli.enroll || (cli.token.is_some() && cli.backend.is_some());
    let pre_enrolled = if needs_enrollment {
        let local = identity::config::config_dir().map(|d| d.join("providers.toml"));
        classifier::providers::init_with_fetched(None, local.as_deref());

        // resolve_identity exits the process if --enroll is set.
        // For auto-enroll it returns the config so we can continue to capture.
        identity::enroll::resolve_identity(cli.enroll, cli.token.as_deref(), cli.backend.as_deref())
    } else {
        None
    };

    // Normal operation: start the async runtime.
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime")
        .block_on(async_main(cli, pre_enrolled));
}

async fn async_main(cli: Cli, pre_enrolled: Option<identity::config::AgentConfig>) {
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

    // Use pre-enrolled config if available, otherwise load from disk.
    let agent_config =
        pre_enrolled.or_else(|| identity::enroll::resolve_identity(false, None, None));
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
    let event_buffer = buffer::store::open_if_needed(has_http, app_config.agent.max_buffer_events);

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

    // Flush all sinks every 500ms so events do not sit in batches waiting for
    // the size threshold. Under light load this is the primary delivery trigger.
    let sink_flush = Arc::clone(&sink);
    tokio::spawn(async move {
        let interval = std::time::Duration::from_millis(FLUSH_INTERVAL_MS);
        loop {
            tokio::time::sleep(interval).await;
            let _ = sink_flush.flush().await;
        }
    });

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
