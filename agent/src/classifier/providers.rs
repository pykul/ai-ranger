/// A known AI provider and the hostnames it uses.
///
/// Phase 0: hardcoded list for the spike. Phase 1 will load this from providers/providers.toml.
#[allow(dead_code)] // display_name used in Phase 1 provider registry
pub struct Provider {
    pub name: &'static str,
    pub display_name: &'static str,
    pub hostnames: &'static [&'static str],
}

static PROVIDERS: &[Provider] = &[
    Provider {
        name: "anthropic",
        display_name: "Anthropic / Claude",
        hostnames: &["api.anthropic.com", "claude.ai"],
    },
    Provider {
        name: "openai",
        display_name: "OpenAI",
        hostnames: &["api.openai.com", "chat.openai.com", "chatgpt.com"],
    },
    Provider {
        name: "cursor",
        display_name: "Cursor",
        hostnames: &["api2.cursor.sh", "repo.cursor.sh"],
    },
    Provider {
        name: "github_copilot",
        display_name: "GitHub Copilot",
        hostnames: &["copilot-proxy.githubusercontent.com", "githubcopilot.com"],
    },
    Provider {
        name: "google_gemini",
        display_name: "Google Gemini",
        hostnames: &["generativelanguage.googleapis.com", "aistudio.google.com"],
    },
];

/// Return the provider `name` if `hostname` matches a known AI provider, else `None`.
///
/// Matches exact hostnames and subdomains (e.g. "foo.api.openai.com" → "openai").
pub fn classify(hostname: &str) -> Option<&'static str> {
    let hostname = hostname.trim_end_matches('.');
    for provider in PROVIDERS {
        for &known in provider.hostnames {
            if hostname == known || hostname.ends_with(&format!(".{known}")) {
                return Some(provider.name);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_anthropic() {
        assert_eq!(classify("api.anthropic.com"), Some("anthropic"));
    }

    #[test]
    fn exact_match_openai() {
        assert_eq!(classify("api.openai.com"), Some("openai"));
    }

    #[test]
    fn exact_match_cursor() {
        assert_eq!(classify("api2.cursor.sh"), Some("cursor"));
    }

    #[test]
    fn exact_match_copilot() {
        assert_eq!(classify("githubcopilot.com"), Some("github_copilot"));
    }

    #[test]
    fn exact_match_gemini() {
        assert_eq!(
            classify("generativelanguage.googleapis.com"),
            Some("google_gemini")
        );
    }

    #[test]
    fn subdomain_match() {
        assert_eq!(classify("eu.api.anthropic.com"), Some("anthropic"));
    }

    #[test]
    fn trailing_dot_stripped() {
        assert_eq!(classify("api.openai.com."), Some("openai"));
    }

    #[test]
    fn unknown_host_returns_none() {
        assert_eq!(classify("github.com"), None);
        assert_eq!(classify("google.com"), None);
        assert_eq!(classify("example.com"), None);
    }
}
