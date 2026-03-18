//! Background drain loop for the SQLite event buffer.
//!
//! Periodically reads buffered events, sends them through the configured sink,
//! and deletes successfully uploaded events. Uses exponential backoff on failure.

use crate::buffer::store::EventBuffer;
use crate::event::AiConnectionEvent;
use crate::output::sink::EventSink;
use std::sync::Arc;

/// Default interval between SQLite buffer drain attempts.
/// 1 second keeps dashboard latency under 2 seconds for captured events.
/// Configurable via `drain_interval_secs` in config.toml.
pub(crate) const DRAIN_INTERVAL_SECS: u64 = 1;

/// Maximum backoff interval for the drain loop after repeated failures.
/// 5 minutes caps the worst-case delay before retrying a backend connection.
const DRAIN_MAX_BACKOFF_SECS: u64 = 300;

/// Multiplier for exponential backoff in the drain loop.
const DRAIN_BACKOFF_MULTIPLIER: u64 = 2;

/// Default maximum events read from the SQLite buffer per drain cycle.
/// 500 balances memory usage against upload efficiency.
pub(crate) const DRAIN_BATCH_SIZE: usize = 500;

/// Background drain loop: periodically reads buffered events and POSTs them.
/// Uses exponential backoff on failure up to DRAIN_MAX_BACKOFF_SECS.
pub(crate) async fn drain_loop(
    buf: Arc<EventBuffer>,
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
pub(crate) async fn drain_once(
    buf: &EventBuffer,
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
