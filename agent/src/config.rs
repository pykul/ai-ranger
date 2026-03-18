use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Top-level agent configuration from config.toml.
///
/// See ARCHITECTURE.md § Agent Configuration for the full schema.
#[derive(Deserialize, Debug)]
pub struct AppConfig {
    #[serde(default)]
    pub agent: AgentSection,
    #[serde(default)]
    pub outputs: Vec<OutputConfig>,
}

/// Agent-specific configuration from the `[agent]` section of config.toml.
///
/// All timing/sizing fields are optional. When absent, the hardcoded constant
/// in the owning module is used as the default.
#[derive(Deserialize, Debug)]
pub struct AgentSection {
    #[serde(default = "default_mode")]
    pub mode: String,
    pub providers_url: Option<String>,
    /// How often (seconds) the SQLite buffer uploads events to the backend.
    pub drain_interval_secs: Option<u64>,
    /// Maximum events read from the SQLite buffer per drain cycle.
    pub drain_batch_size: Option<u64>,
    /// Maximum events the HTTP sink buffers before flushing.
    pub http_batch_size: Option<u64>,
    /// Default maximum events the webhook sink buffers before flushing.
    /// Per-sink `batch_size` in `[[outputs]]` overrides this.
    pub webhook_batch_size: Option<u64>,
    /// Timeout (seconds) for fetching providers.toml from `providers_url`.
    pub providers_fetch_timeout_secs: Option<u64>,
    /// Maximum number of events to keep in the SQLite buffer.
    /// When the buffer exceeds this limit, the oldest events are dropped.
    pub max_buffer_events: Option<usize>,
}

impl Default for AgentSection {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            providers_url: None,
            drain_interval_secs: None,
            drain_batch_size: None,
            http_batch_size: None,
            webhook_batch_size: None,
            providers_fetch_timeout_secs: None,
            max_buffer_events: None,
        }
    }
}

fn default_mode() -> String {
    "dns-sni".to_string()
}

/// Output sink configuration from `[[outputs]]` entries in config.toml.
///
/// Each variant corresponds to a sink type. Events fan out to all configured outputs.
#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum OutputConfig {
    #[serde(rename = "stdout")]
    Stdout,
    #[serde(rename = "file")]
    File { path: String },
    #[serde(rename = "http")]
    Http { url: String },
    #[serde(rename = "webhook")]
    Webhook {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        batch_size: Option<usize>,
    },
}

impl AppConfig {
    /// Load config from a TOML file. Returns default config if the file doesn't exist.
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&contents)?;
        Ok(config)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            agent: AgentSection::default(),
            outputs: vec![OutputConfig::Stdout],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let toml_str = r#"
[agent]
mode = "dns-sni"

[[outputs]]
type = "stdout"
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.agent.mode, "dns-sni");
        assert_eq!(config.outputs.len(), 1);
    }

    #[test]
    fn parse_full_config() {
        let toml_str = r#"
[agent]
mode = "dns-sni"
providers_url = "https://example.com/providers.toml"

[[outputs]]
type = "stdout"

[[outputs]]
type = "file"
path = "/tmp/events.jsonl"

[[outputs]]
type = "http"
url = "http://localhost:8080"
token = "tok_abc123"

[[outputs]]
type = "webhook"
url = "https://example.com/hook"
headers = { "X-API-Key" = "secret" }
batch_size = 50
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.outputs.len(), 4);
        assert!(config.agent.providers_url.is_some());
    }

    #[test]
    fn default_config_has_stdout() {
        let config = AppConfig::default();
        assert_eq!(config.agent.mode, "dns-sni");
        assert_eq!(config.outputs.len(), 1);
    }
}
