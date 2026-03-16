use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Agent identity and enrollment state, persisted to the OS config directory.
///
/// Created during enrollment (`ai-ranger enroll --token=... --backend=...`).
/// Loaded at startup if present. Absent in standalone/stdout-only mode.
#[derive(Serialize, Deserialize, Debug)]
pub struct AgentConfig {
    pub agent_id: String,
    pub org_id: String,
    pub backend_url: String,
    pub machine_hostname: String,
    pub os_username: String,
    pub enrolled_at: i64, // unix ms
}

impl AgentConfig {
    /// Load config from the OS-specific path. Returns None if not enrolled.
    pub fn load() -> Option<Self> {
        let path = config_file_path()?;
        let contents = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    /// Save config to the OS-specific path.
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let path = config_file_path().ok_or("could not determine config directory")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }
}

/// OS-specific config file path.
///
/// Linux:   ~/.config/ai-ranger/config.json
/// macOS:   ~/Library/Application Support/ai-ranger/config.json
/// Windows: %APPDATA%\ai-ranger\config.json
pub fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        dirs::config_dir().map(|d| d.join("ai-ranger"))
    }
    #[cfg(target_os = "macos")]
    {
        // ~/Library/Application Support/ai-ranger
        dirs::config_dir().map(|d| d.join("ai-ranger"))
    }
    #[cfg(windows)]
    {
        // %APPDATA%\ai-ranger
        dirs::config_dir().map(|d| d.join("ai-ranger"))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        None
    }
}

fn config_file_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("config.json"))
}

/// Get the local machine hostname.
pub fn machine_hostname() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Get the current OS username.
pub fn os_username() -> String {
    whoami::username()
}
