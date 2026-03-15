use crate::event::AiConnectionEvent;
use crate::output::sink::EventSink;
use async_trait::async_trait;
use tokio::sync::Mutex;

/// POST protobuf-encoded EventBatch to the gateway.
/// Events are batched internally and flushed periodically or when flush() is called.
///
/// Uses JSON encoding for now (Phase 1). Will switch to protobuf when prost
/// and the proto/gen/rust types are wired in Phase 2.
pub struct HttpSink {
    url: String,
    agent_id: String,
    client: reqwest::Client,
    batch: Mutex<Vec<AiConnectionEvent>>,
    batch_size: usize,
}

impl HttpSink {
    pub fn new(url: String, agent_id: String) -> Self {
        Self {
            url,
            agent_id,
            client: reqwest::Client::new(),
            batch: Mutex::new(Vec::new()),
            batch_size: 100,
        }
    }

    async fn send_batch(
        &self,
        events: &[AiConnectionEvent],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if events.is_empty() {
            return Ok(());
        }
        let body = serde_json::to_vec(events)?;
        let resp = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .bearer_auth(&self.agent_id)
            .body(body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(format!("HTTP sink: server returned {}", resp.status()).into());
        }
        Ok(())
    }
}

#[async_trait]
impl EventSink for HttpSink {
    async fn send(&self, event: &AiConnectionEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let should_flush;
        {
            let mut batch = self.batch.lock().await;
            batch.push(clone_event(event));
            should_flush = batch.len() >= self.batch_size;
        }
        if should_flush {
            self.flush().await?;
        }
        Ok(())
    }

    async fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let events: Vec<AiConnectionEvent> = {
            let mut batch = self.batch.lock().await;
            std::mem::take(&mut *batch)
        };
        self.send_batch(&events).await
    }

}

/// Clone an event. AiConnectionEvent doesn't derive Clone to keep it lightweight
/// in the common case; this is only needed for batching sinks.
fn clone_event(e: &AiConnectionEvent) -> AiConnectionEvent {
    AiConnectionEvent {
        agent_id: e.agent_id.clone(),
        machine_hostname: e.machine_hostname.clone(),
        os_username: e.os_username.clone(),
        connection_id: e.connection_id.clone(),
        timestamp_ms: e.timestamp_ms,
        duration_ms: e.duration_ms,
        provider: e.provider.clone(),
        provider_host: e.provider_host.clone(),
        model_hint: e.model_hint.clone(),
        process_name: e.process_name.clone(),
        process_pid: e.process_pid,
        process_path: e.process_path.clone(),
        src_ip: e.src_ip.clone(),
        detection_method: e.detection_method,
        capture_mode: e.capture_mode,
        content_available: e.content_available,
        payload_ref: e.payload_ref.clone(),
        model_exact: e.model_exact.clone(),
        token_count_input: e.token_count_input,
        token_count_output: e.token_count_output,
        latency_ttfb_ms: e.latency_ttfb_ms,
    }
}
