use crate::event::AiConnectionEvent;
use async_trait::async_trait;

/// Every output destination implements this trait.
///
/// Built-in implementations: StdoutSink, FileSink, HttpSink, WebhookSink.
/// FanoutSink wraps multiple sinks and sends to all concurrently.
#[allow(dead_code)] // close() is part of the public API per ARCHITECTURE.md, not yet called
#[async_trait]
pub trait EventSink: Send + Sync {
    async fn send(
        &self,
        event: &AiConnectionEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Flush any buffered data. Default is a no-op for non-batched sinks.
    async fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Close the sink and release resources. Default is a no-op.
    async fn close(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}
