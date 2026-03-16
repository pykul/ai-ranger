use ipnet::IpNet;
use serde::Deserialize;
use std::net::IpAddr;
use std::sync::OnceLock;

/// Provider loading priority (Phase 1):
///
/// 1. Fetch from `providers_url` in config at startup (if configured and reachable).
///    This allows centralized fleet-wide provider list updates without redeploying agents.
///    Requires async runtime - wired in main.rs when tokio is available.
///
/// 2. Fall back to a local file in the OS config directory:
///    - Linux:   ~/.config/ai-ranger/providers.toml
///    - macOS:   ~/Library/Application Support/ai-ranger/providers.toml
///    - Windows: %APPDATA%\ai-ranger\providers.toml
///      This allows per-machine customization and survives network outages.
///
/// 3. Fall back to the compile-time bundled copy (include_str! of providers/providers.toml).
///    This guarantees the agent always has a working provider list even on first run
///    with no config directory and no network.
///
/// The loaded providers are stored in a global OnceLock and never change after init.
const BUNDLED_PROVIDERS: &str = include_str!("../../../providers/providers.toml");

static REGISTRY: OnceLock<ProviderRegistry> = OnceLock::new();

#[derive(Deserialize)]
struct ProvidersFile {
    providers: Vec<ProviderEntry>,
}

#[derive(Deserialize, Clone)]
#[allow(dead_code)] // display_name used by output sinks in later steps
pub(crate) struct ProviderEntry {
    pub name: String,
    pub display_name: String,
    pub hostnames: Vec<String>,
    /// CIDR ranges for providers with dedicated IP space. Used as a fallback
    /// when SNI and DNS detection both fail (e.g. browser ECH+DoH).
    /// Only populated for providers with dedicated IPs - never for CDN-backed providers.
    #[serde(default)]
    pub ip_ranges: Vec<String>,
}

/// Parsed provider with pre-computed CIDR networks for fast IP matching.
struct ParsedProvider {
    entry: ProviderEntry,
    networks: Vec<IpNet>,
}

pub(crate) struct ProviderRegistry {
    providers: Vec<ParsedProvider>,
}

impl ProviderRegistry {
    fn from_toml(toml_str: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file: ProvidersFile = toml::from_str(toml_str)?;
        let providers = file
            .providers
            .into_iter()
            .map(|entry| {
                let networks: Vec<IpNet> = entry
                    .ip_ranges
                    .iter()
                    .filter_map(|r| r.parse().ok())
                    .collect();
                ParsedProvider { entry, networks }
            })
            .collect();
        Ok(Self { providers })
    }
}

/// Initialize the provider registry with the full 3-tier priority chain.
///
/// 1. `fetched_content` - TOML string fetched from `providers_url` by the caller.
/// 2. `local_config_path` - local file in the OS config directory.
/// 3. Compile-time bundled copy.
///
/// Call once at startup. Subsequent calls are no-ops (OnceLock).
pub fn init_with_fetched(
    fetched_content: Option<&str>,
    local_config_path: Option<&std::path::Path>,
) {
    REGISTRY.get_or_init(|| {
        // Priority 1: content fetched from providers_url
        if let Some(content) = fetched_content {
            if let Ok(registry) = ProviderRegistry::from_toml(content) {
                eprintln!(
                    "[ai-ranger] Loaded {} providers from providers_url",
                    registry.providers.len()
                );
                return registry;
            }
            eprintln!("[ai-ranger] Failed to parse fetched providers, trying local file");
        }

        // Priority 2: local file in config directory
        if let Some(path) = local_config_path {
            if let Ok(contents) = std::fs::read_to_string(path) {
                if let Ok(registry) = ProviderRegistry::from_toml(&contents) {
                    eprintln!(
                        "[ai-ranger] Loaded {} providers from {}",
                        registry.providers.len(),
                        path.display()
                    );
                    return registry;
                }
                eprintln!(
                    "[ai-ranger] Failed to parse {}, falling back to bundled providers",
                    path.display()
                );
            }
        }

        // Priority 3: compile-time bundle
        let registry = ProviderRegistry::from_toml(BUNDLED_PROVIDERS)
            .expect("bundled providers.toml must be valid");
        eprintln!(
            "[ai-ranger] Loaded {} providers (bundled)",
            registry.providers.len()
        );
        registry
    });
}

/// Return the provider `name` if `hostname` matches a known AI provider, else `None`.
///
/// Matches exact hostnames and subdomains (e.g. "foo.api.openai.com" → "openai").
pub fn classify(hostname: &str) -> Option<&'static str> {
    let registry = REGISTRY
        .get()
        .expect("providers not initialized - call init() first");
    let hostname = hostname.trim_end_matches('.');
    for parsed in &registry.providers {
        for known in &parsed.entry.hostnames {
            if hostname == known
                || (hostname.len() > known.len()
                    && hostname.ends_with(known.as_str())
                    && hostname.as_bytes()[hostname.len() - known.len() - 1] == b'.')
            {
                // SAFETY: registry lives in a static OnceLock, so &str has 'static lifetime.
                let name: &str = &parsed.entry.name;
                return Some(unsafe { &*(name as *const str) });
            }
        }
    }
    None
}

/// Fallback: match a destination IP against provider ip_ranges.
///
/// Called only when both SNI and DNS detection produced no match.
/// Returns (provider_name, synthetic_provider_host) where provider_host is the
/// first hostname from the provider entry.
pub fn classify_ip(dst_ip: &str) -> Option<(&'static str, &'static str)> {
    let registry = REGISTRY
        .get()
        .expect("providers not initialized - call init() first");
    let ip: IpAddr = dst_ip.parse().ok()?;
    for parsed in &registry.providers {
        if parsed.networks.is_empty() {
            continue;
        }
        for net in &parsed.networks {
            if net.contains(&ip) {
                let name: &str = &parsed.entry.name;
                let host: &str = parsed
                    .entry
                    .hostnames
                    .first()
                    .map(|s| s.as_str())
                    .unwrap_or("");
                // SAFETY: registry lives in a static OnceLock, so &str has 'static lifetime.
                return Some(unsafe { (&*(name as *const str), &*(host as *const str)) });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ensure_init() {
        // init is idempotent via OnceLock
        init_with_fetched(None, None);
    }

    #[test]
    fn exact_match_anthropic() {
        ensure_init();
        assert_eq!(classify("api.anthropic.com"), Some("anthropic"));
    }

    #[test]
    fn exact_match_openai() {
        ensure_init();
        assert_eq!(classify("api.openai.com"), Some("openai"));
    }

    #[test]
    fn exact_match_cursor() {
        ensure_init();
        assert_eq!(classify("api2.cursor.sh"), Some("cursor"));
    }

    #[test]
    fn exact_match_copilot() {
        ensure_init();
        assert_eq!(classify("githubcopilot.com"), Some("github_copilot"));
    }

    #[test]
    fn exact_match_gemini() {
        ensure_init();
        assert_eq!(
            classify("generativelanguage.googleapis.com"),
            Some("google_gemini")
        );
    }

    #[test]
    fn subdomain_match() {
        ensure_init();
        assert_eq!(classify("eu.api.anthropic.com"), Some("anthropic"));
    }

    #[test]
    fn trailing_dot_stripped() {
        ensure_init();
        assert_eq!(classify("api.openai.com."), Some("openai"));
    }

    #[test]
    fn unknown_host_returns_none() {
        ensure_init();
        assert_eq!(classify("github.com"), None);
        assert_eq!(classify("google.com"), None);
        assert_eq!(classify("example.com"), None);
    }

    #[test]
    fn new_providers_from_toml() {
        ensure_init();
        assert_eq!(classify("api.mistral.ai"), Some("mistral"));
        assert_eq!(classify("api.cohere.ai"), Some("cohere"));
        assert_eq!(classify("api.together.xyz"), Some("together"));
        assert_eq!(classify("api.perplexity.ai"), Some("perplexity"));
        assert_eq!(classify("api.deepseek.com"), Some("deepseek"));
        assert_eq!(classify("api.x.ai"), Some("xai"));
        assert_eq!(classify("api.ai21.com"), Some("ai21"));
        assert_eq!(classify("api.stability.ai"), Some("stability"));
        assert_eq!(classify("openai.azure.com"), Some("azure_openai"));
    }

    #[test]
    fn azure_openai_subdomain() {
        ensure_init();
        assert_eq!(
            classify("my-deployment.openai.azure.com"),
            Some("azure_openai")
        );
    }

    #[test]
    fn bedrock_regions() {
        ensure_init();
        assert_eq!(
            classify("bedrock-runtime.us-east-1.amazonaws.com"),
            Some("amazon_bedrock")
        );
        assert_eq!(
            classify("bedrock-runtime.us-west-2.amazonaws.com"),
            Some("amazon_bedrock")
        );
    }

    #[test]
    fn bundled_toml_parses() {
        let registry = ProviderRegistry::from_toml(BUNDLED_PROVIDERS).unwrap();
        assert!(registry.providers.len() >= 15);
    }

    // ── IP range matching tests ──────────────────────────────────────────────

    #[test]
    fn ip_range_matches_anthropic() {
        ensure_init();
        // 160.79.104.0/23 covers 160.79.104.0 - 160.79.105.255
        let result = classify_ip("160.79.104.1");
        assert_eq!(result, Some(("anthropic", "api.anthropic.com")));

        let result = classify_ip("160.79.105.200");
        assert_eq!(result, Some(("anthropic", "api.anthropic.com")));
    }

    #[test]
    fn ip_range_matches_anthropic_ipv6() {
        ensure_init();
        // 2607:6bc0::/48 covers 2607:6bc0:0000::-2607:6bc0:00ff:ffff:...
        let result = classify_ip("2607:6bc0::10");
        assert_eq!(result, Some(("anthropic", "api.anthropic.com")));

        let result = classify_ip("2607:6bc0:0000:1::1");
        assert_eq!(result, Some(("anthropic", "api.anthropic.com")));
    }

    #[test]
    fn ip_range_rejects_outside() {
        ensure_init();
        // Just outside the /23 range
        assert_eq!(classify_ip("160.79.106.0"), None);
        // Completely unrelated
        assert_eq!(classify_ip("8.8.8.8"), None);
        assert_eq!(classify_ip("172.66.0.243"), None); // Cloudflare - no ip_ranges
                                                       // IPv6 outside Anthropic's range
        assert_eq!(classify_ip("2607:6bc1::1"), None);
    }

    #[test]
    fn ip_range_returns_none_for_invalid() {
        ensure_init();
        assert_eq!(classify_ip("not-an-ip"), None);
        assert_eq!(classify_ip(""), None);
    }
}
