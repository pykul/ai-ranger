use crate::event::AiConnectionEvent;
use crate::output::sink::EventSink;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::Mutex;

/// POST JSON arrays of events to an arbitrary URL with configurable headers.
///
/// Designed for integration with external services (Datadog, Splunk, custom APIs).
/// Events are batched up to `batch_size` (default 100) before sending.
pub struct WebhookSink {
    url: String,
    headers: HashMap<String, String>,
    client: reqwest::Client,
    batch: Mutex<Vec<serde_json::Value>>,
    batch_size: usize,
}

impl WebhookSink {
    pub fn new(url: String, headers: HashMap<String, String>, batch_size: Option<usize>) -> Self {
        Self {
            url,
            headers,
            client: reqwest::Client::new(),
            batch: Mutex::new(Vec::new()),
            batch_size: batch_size.unwrap_or(100),
        }
    }

    async fn send_batch(
        &self,
        events: &[serde_json::Value],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if events.is_empty() {
            return Ok(());
        }
        let body = serde_json::to_vec(events)?;
        let mut req = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .body(body);
        for (key, value) in &self.headers {
            req = req.header(key, value);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Err(format!("Webhook sink: server returned {}", resp.status()).into());
        }
        Ok(())
    }
}

#[async_trait]
impl EventSink for WebhookSink {
    async fn send(
        &self,
        event: &AiConnectionEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let value = serde_json::to_value(event)?;
        let should_flush;
        {
            let mut batch = self.batch.lock().await;
            batch.push(value);
            should_flush = batch.len() >= self.batch_size;
        }
        if should_flush {
            self.flush().await?;
        }
        Ok(())
    }

    async fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let events: Vec<serde_json::Value> = {
            let mut batch = self.batch.lock().await;
            std::mem::take(&mut *batch)
        };
        self.send_batch(&events).await
    }
}
