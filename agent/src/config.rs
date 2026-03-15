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

#[derive(Deserialize, Debug)]
pub struct AgentSection {
    #[serde(default = "default_mode")]
    pub mode: String,
    pub providers_url: Option<String>,
}

impl Default for AgentSection {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            providers_url: None,
        }
    }
}

fn default_mode() -> String {
    "dns-sni".to_string()
}

#[derive(Deserialize, Debug)]
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
