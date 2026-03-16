//! Enrollment and identity loading.
//!
//! Handles the `--enroll` CLI flow: POST to the gateway's enrollment endpoint,
//! receive org_id, save AgentConfig locally, then exit. On subsequent runs the
//! agent loads the saved config and uses agent_id as Bearer token for all requests.

use super::config::{self, AgentConfig};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// JSON body sent to POST /v1/agents/enroll.
#[derive(Serialize)]
struct EnrollmentRequest {
    token: String,
    agent_id: String,
    hostname: String,
    os_username: String,
    os: String,
    agent_version: String,
}

/// JSON body returned by the gateway on successful enrollment.
#[derive(Deserialize)]
struct EnrollmentResponse {
    org_id: String,
    agent_id: String,
}

/// Enrollment endpoint path on the gateway.
const ENROLL_PATH: &str = "/v1/agents/enroll";

/// Handle enrollment or load existing identity.
///
/// If `enroll` is true, generates a new agent_id, POSTs to the gateway's
/// enrollment endpoint, saves the returned config locally, and exits.
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
                let agent_id = uuid::Uuid::new_v4().to_string();
                let hostname = config::machine_hostname();
                let os_username = config::os_username();

                let req = EnrollmentRequest {
                    token: token.to_string(),
                    agent_id: agent_id.clone(),
                    hostname: hostname.clone(),
                    os_username: os_username.clone(),
                    os: std::env::consts::OS.to_string(),
                    agent_version: env!("CARGO_PKG_VERSION").to_string(),
                };

                let enroll_url = format!("{}{}", backend.trim_end_matches('/'), ENROLL_PATH);
                eprintln!("[ai-ranger] Enrolling with backend at {backend}...");

                let client = reqwest::blocking::Client::new();
                let resp = match client.post(&enroll_url).json(&req).send() {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("[ai-ranger] Enrollment failed: could not reach backend: {e}");
                        std::process::exit(1);
                    }
                };

                if !resp.status().is_success() {
                    let status = resp.status();
                    let body = resp.text().unwrap_or_default();
                    eprintln!("[ai-ranger] Enrollment failed: HTTP {status}: {body}");
                    std::process::exit(1);
                }

                let enrollment: EnrollmentResponse = match resp.json() {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("[ai-ranger] Enrollment failed: invalid response: {e}");
                        std::process::exit(1);
                    }
                };

                let agent_config = AgentConfig {
                    agent_id: enrollment.agent_id.clone(),
                    org_id: enrollment.org_id,
                    backend_url: backend.to_string(),
                    machine_hostname: hostname,
                    os_username,
                    enrolled_at: Utc::now().timestamp_millis(),
                };

                if let Err(e) = agent_config.save() {
                    eprintln!("[ai-ranger] Failed to save enrollment config: {e}");
                    std::process::exit(1);
                }

                eprintln!(
                    "[ai-ranger] Enrolled as {} (org: {})",
                    enrollment.agent_id, agent_config.org_id
                );
                eprintln!("[ai-ranger] Config saved to {:?}", config::config_dir().unwrap_or_default());
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
