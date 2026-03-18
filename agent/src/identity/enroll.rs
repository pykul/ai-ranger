//! Enrollment and identity loading.
//!
//! Two enrollment modes:
//! - **Enroll and run** (default): pass `--token` and `--backend` to enroll on first
//!   run, then immediately start capturing. If already enrolled, the flags are ignored.
//! - **Enroll only**: pass `--enroll --token --backend` to enroll and exit without
//!   capturing. Used by installer scripts that start the daemon separately.
//!
//! On subsequent runs the agent loads the saved config and uses agent_id as Bearer
//! token for all requests. No flags needed.

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

/// Warn if a non-localhost backend URL uses plaintext HTTP.
fn warn_if_insecure(url: &str) {
    if url.starts_with("http://") {
        let host_part = url.trim_start_matches("http://");
        let host = host_part.split('/').next().unwrap_or("");
        let host = host.split(':').next().unwrap_or("");
        if host != "localhost" && host != "127.0.0.1" && host != "::1" {
            eprintln!("WARNING: Backend URL uses HTTP. Event data and enrollment tokens will be sent in plaintext. Use HTTPS in production.");
        }
    }
}

/// Perform enrollment by POSTing to the gateway.
/// Returns the saved AgentConfig on success, or exits the process on failure.
fn do_enroll(token: &str, backend: &str) -> AgentConfig {
    warn_if_insecure(backend);

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
    eprintln!(
        "[ai-ranger] Config saved to {:?}",
        config::config_dir().unwrap_or_default()
    );

    agent_config
}

/// Resolve the agent identity. Three paths:
///
/// 1. `--enroll --token T --backend B` → enroll and exit (installer scripts)
/// 2. `--token T --backend B` (no --enroll) → enroll if not already enrolled, return config
/// 3. No flags → load existing config or return None (standalone mode)
pub(crate) fn resolve_identity(
    enroll_only: bool,
    token: Option<&str>,
    backend: Option<&str>,
) -> Option<AgentConfig> {
    // Mode 1: explicit enroll-and-exit
    if enroll_only {
        match (token, backend) {
            (Some(t), Some(b)) => {
                do_enroll(t, b);
                std::process::exit(0);
            }
            _ => {
                eprintln!("[ai-ranger] --enroll requires --token and --backend");
                std::process::exit(1);
            }
        }
    }

    // Mode 2: auto-enroll if --token and --backend provided but not yet enrolled
    if let (Some(t), Some(b)) = (token, backend) {
        if let Some(existing) = AgentConfig::load() {
            eprintln!(
                "[ai-ranger] Already enrolled as {} — ignoring --token/--backend",
                existing.agent_id
            );
            return Some(existing);
        }
        return Some(do_enroll(t, b));
    }

    // Mode 3: load existing or standalone
    AgentConfig::load()
}
