mod capture;
mod classifier;
mod event;
mod process;

use chrono::Utc;
use event::AiConnectionEvent;

fn main() {
    eprintln!("[ai-ranger] Phase 0 spike — SNI-based AI provider detection");
    eprintln!("[ai-ranger] Monitoring outbound port 443 for connections to known AI providers.");
    eprintln!("[ai-ranger] Requires administrator (Windows) or sudo (Linux/macOS).");
    eprintln!("[ai-ranger] Press Ctrl+C to stop.\n");

    if let Err(e) = capture::pcap::capture(|packet| {
        let Some(provider) = classifier::classify(&packet.sni_hostname) else {
            return;
        };

        let (process_pid, process_name) = process::pid_and_name(packet.src_port);

        let event = AiConnectionEvent {
            agent_id: String::new(),
            machine_hostname: String::new(),
            os_username: String::new(),
            timestamp_ms: Utc::now().timestamp_millis(),
            provider: provider.to_string(),
            provider_host: packet.sni_hostname,
            detection_method: "sni",
            process_name,
            process_pid,
            src_ip: packet.src_ip,
            capture_mode: "DNS_SNI",
        };

        println!(
            "{}",
            serde_json::to_string(&event).expect("event serialization")
        );
    }) {
        eprintln!("[ai-ranger] Capture error: {e}");
        eprintln!("[ai-ranger] Windows: run as Administrator.");
        eprintln!("[ai-ranger] Linux/macOS: run with sudo.");
        std::process::exit(1);
    }
}
