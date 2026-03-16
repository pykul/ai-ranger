use crate::event::AiConnectionEvent;
use crate::output::sink::EventSink;
use async_trait::async_trait;
use std::sync::Arc;

/// Wraps multiple sinks and sends each event to all of them concurrently.
pub struct FanoutSink {
    sinks: Vec<Arc<dyn EventSink>>,
}

impl FanoutSink {
    pub fn new(sinks: Vec<Arc<dyn EventSink>>) -> Self {
        Self { sinks }
    }
}

#[async_trait]
impl EventSink for FanoutSink {
    async fn send(
        &self,
        event: &AiConnectionEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let futures: Vec<_> = self.sinks.iter().map(|s| s.send(event)).collect();
        let results = futures::future::join_all(futures).await;
        for result in results {
            result?;
        }
        Ok(())
    }

    async fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let futures: Vec<_> = self.sinks.iter().map(|s| s.flush()).collect();
        let results = futures::future::join_all(futures).await;
        for result in results {
            result?;
        }
        Ok(())
    }
}
