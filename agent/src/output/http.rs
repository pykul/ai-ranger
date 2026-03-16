use crate::event::AiConnectionEvent;
use crate::output::sink::EventSink;
use async_trait::async_trait;
use tokio::sync::Mutex;

/// Default number of events to buffer before flushing to the HTTP backend.
/// 100 balances memory usage against the overhead of small HTTP requests.
pub(crate) const DEFAULT_HTTP_BATCH_SIZE: usize = 100;

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
    pub fn new(url: String, agent_id: String, batch_size: Option<usize>) -> Self {
        Self {
            url,
            agent_id,
            client: reqwest::Client::new(),
            batch: Mutex::new(Vec::new()),
            batch_size: batch_size.unwrap_or(DEFAULT_HTTP_BATCH_SIZE),
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
    async fn send(
        &self,
        event: &AiConnectionEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let should_flush;
        {
            let mut batch = self.batch.lock().await;
            batch.push(event.clone());
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
