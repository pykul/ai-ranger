//! Enrollment and identity loading.
//!
//! Handles the `--enroll` CLI flow (create + save AgentConfig, then exit)
//! and the normal startup flow (load existing config or return None for
//! standalone mode).

use super::config::{self, AgentConfig};
use chrono::Utc;

/// Handle enrollment or load existing identity.
///
/// If `enroll` is true, creates a new AgentConfig, saves it, prints
/// confirmation, and exits the process (enrollment is a one-shot command).
/// Otherwise loads existing config from disk, returning None if the agent
/// has not been enrolled (standalone mode).
pub(crate) fn load_or_enroll(
    enroll: bool,
    token: Option<&str>,
    backend: Option<&str>,
) -> Option<AgentConfig> {
    if enroll {
        match (token, backend) {
            (Some(token), Some(backend)) => {
                let agent_config = AgentConfig {
                    agent_id: uuid::Uuid::new_v4().to_string(),
                    org_id: String::new(), // populated by backend in Phase 2
                    backend_url: backend.to_string(),
                    machine_hostname: config::machine_hostname(),
                    os_username: config::os_username(),
                    enrolled_at: Utc::now().timestamp_millis(),
                };
                if let Err(e) = agent_config.save() {
                    eprintln!("[ai-ranger] Failed to save enrollment config: {e}");
                    std::process::exit(1);
                }
                eprintln!(
                    "[ai-ranger] Enrolled as {} (token: {token})",
                    agent_config.agent_id
                );
                eprintln!("[ai-ranger] Config saved. Backend enrollment will complete in Phase 2.");
                std::process::exit(0);
            }
            _ => {
                eprintln!("[ai-ranger] --enroll requires --token and --backend");
                std::process::exit(1);
            }
        }
    }

    AgentConfig::load()
}
