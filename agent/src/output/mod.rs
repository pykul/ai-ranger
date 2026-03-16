pub mod fanout;
pub mod file;
pub mod http;
pub mod sink;
pub mod stdout;
pub mod webhook;

use crate::config::OutputConfig;
use sink::EventSink;
use std::path::PathBuf;
use std::sync::Arc;

/// Build output sinks from config. Returns at least one sink (stdout if none configured).
pub(crate) fn build_sinks(
    outputs: &[OutputConfig],
    agent_id: &str,
    http_batch: Option<usize>,
    webhook_batch_default: Option<usize>,
) -> Vec<Arc<dyn EventSink>> {
    let mut sinks: Vec<Arc<dyn EventSink>> = Vec::new();

    for output in outputs {
        match output {
            OutputConfig::Stdout => {
                sinks.push(Arc::new(stdout::StdoutSink));
            }
            OutputConfig::File { path } => {
                sinks.push(Arc::new(file::FileSink::new(PathBuf::from(path))));
            }
            OutputConfig::Http { url, .. } => {
                sinks.push(Arc::new(http::HttpSink::new(
                    url.clone(),
                    agent_id.to_string(),
                    http_batch,
                )));
            }
            OutputConfig::Webhook {
                url,
                headers,
                batch_size,
            } => {
                let effective_batch = batch_size.or(webhook_batch_default);
                sinks.push(Arc::new(webhook::WebhookSink::new(
                    url.clone(),
                    headers.clone(),
                    effective_batch,
                )));
            }
        }
    }

    if sinks.is_empty() {
        sinks.push(Arc::new(stdout::StdoutSink));
    }

    sinks
}
