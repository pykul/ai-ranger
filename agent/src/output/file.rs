use crate::event::AiConnectionEvent;
use crate::output::sink::EventSink;
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

/// Write JSON-lines to a file. Each event is appended as one line.
pub struct FileSink {
    path: PathBuf,
    file: Mutex<Option<tokio::fs::File>>,
}

impl FileSink {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            file: Mutex::new(None),
        }
    }

    async fn ensure_open(
        &self,
    ) -> Result<
        tokio::sync::MutexGuard<'_, Option<tokio::fs::File>>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let mut guard = self.file.lock().await;
        if guard.is_none() {
            if let Some(parent) = self.path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            let f = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .await?;
            *guard = Some(f);
        }
        Ok(guard)
    }
}

#[async_trait]
impl EventSink for FileSink {
    async fn send(
        &self,
        event: &AiConnectionEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut json = serde_json::to_string(event)?;
        json.push('\n');
        let mut guard = self.ensure_open().await?;
        if let Some(file) = guard.as_mut() {
            file.write_all(json.as_bytes()).await?;
        }
        Ok(())
    }

    async fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut guard = self.file.lock().await;
        if let Some(file) = guard.as_mut() {
            file.flush().await?;
        }
        Ok(())
    }
}
