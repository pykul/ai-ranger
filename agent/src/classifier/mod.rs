pub mod providers;
pub use providers::{classify, classify_ip};

/// Fetch providers.toml from a remote URL. Returns None on any failure.
pub(crate) async fn fetch_providers_url(url: Option<&str>, timeout_secs: u64) -> Option<String> {
    let url = url?;
    eprintln!("[ai-ranger] Fetching providers from {url}");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .ok()?;
    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.text().await {
            Ok(body) => {
                eprintln!("[ai-ranger] Fetched providers.toml ({} bytes)", body.len());
                Some(body)
            }
            Err(e) => {
                eprintln!("[ai-ranger] Failed to read providers response: {e}");
                None
            }
        },
        Ok(resp) => {
            eprintln!(
                "[ai-ranger] Providers fetch returned HTTP {}, falling back",
                resp.status()
            );
            None
        }
        Err(e) => {
            eprintln!("[ai-ranger] Providers fetch failed: {e}");
            None
        }
    }
}
