use crate::event::AiConnectionEvent;
use crate::output::sink::EventSink;
use crate::proto::ranger_v1;
use async_trait::async_trait;
use prost::Message;
use tokio::sync::Mutex;

/// Default number of events to buffer before flushing to the HTTP backend.
/// 100 balances memory usage against the overhead of small HTTP requests.
pub(crate) const DEFAULT_HTTP_BATCH_SIZE: usize = 100;

/// Content-Type header value for protobuf payloads.
const CONTENT_TYPE_PROTOBUF: &str = "application/x-protobuf";

/// Timeout for individual HTTP requests to the backend.
/// 30 seconds is generous enough for slow networks while preventing indefinite hangs
/// that would stall the buffer drain loop.
const HTTP_TIMEOUT_SECS: u64 = 30;

/// Ingest endpoint path on the gateway.
const INGEST_PATH: &str = "/v1/ingest";

/// POST protobuf-encoded EventBatch to the gateway.
/// Events are batched internally and flushed periodically or when flush() is called.
///
/// Encodes events as a protobuf EventBatch using prost-generated types.
pub struct HttpSink {
    url: String,
    agent_id: String,
    client: reqwest::Client,
    batch: Mutex<Vec<AiConnectionEvent>>,
    batch_size: usize,
}

impl HttpSink {
    pub fn new(url: String, agent_id: String, batch_size: Option<usize>) -> Self {
        let ingest_url = format!("{}{}", url.trim_end_matches('/'), INGEST_PATH);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            url: ingest_url,
            agent_id,
            client,
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

        // Convert internal events to protobuf types and wrap in EventBatch.
        let proto_events: Vec<ranger_v1::AiConnectionEvent> = events
            .iter()
            .map(ranger_v1::AiConnectionEvent::from)
            .collect();

        let batch = ranger_v1::EventBatch {
            agent_id: self.agent_id.clone(),
            sent_at_ms: chrono::Utc::now().timestamp_millis(),
            events: proto_events,
        };

        let body = batch.encode_to_vec();

        let resp = self
            .client
            .post(&self.url)
            .header("Content-Type", CONTENT_TYPE_PROTOBUF)
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
