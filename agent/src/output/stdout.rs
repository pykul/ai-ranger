use crate::event::AiConnectionEvent;
use crate::output::sink::EventSink;
use async_trait::async_trait;

/// Default output sink: JSON-serialize each event to stdout, one per line.
pub struct StdoutSink;

#[async_trait]
impl EventSink for StdoutSink {
    async fn send(&self, event: &AiConnectionEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let json = serde_json::to_string(event)?;
        println!("{json}");
        Ok(())
    }
}
