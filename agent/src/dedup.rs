use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

/// Width of the time bucket used to group related capture events (DNS + SNI)
/// into a single logical connection. 2 seconds handles realistic ETW DNS-Client
/// latency (1-3s) without over-collapsing distinct connections.
/// See DECISIONS.md "Why 2-second buckets" for the full rationale.
const DEDUP_BUCKET_MS: i64 = 2000;

/// Compute a connection_id that identifies a logical connection attempt.
///
/// Key: (src_ip, provider_host, timestamp_ms / 2000).
///
/// src_ip + provider_host identify "who is connecting to what." Rounding
/// timestamp to 2-second buckets groups DNS + SNI events that fire close
/// together in time. 2 seconds handles realistic ETW DNS-Client latency
/// without over-collapsing distinct connections.
///
/// Returns empty string if provider_host is empty (IP-range-only events
/// cannot be reliably deduplicated).
pub fn compute_connection_id(src_ip: &str, provider_host: &str, timestamp_ms: i64) -> String {
    if provider_host.is_empty() {
        return String::new();
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src_ip.hash(&mut hasher);
    provider_host.hash(&mut hasher);
    (timestamp_ms / DEDUP_BUCKET_MS).hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Short-window deduplication cache.
///
/// Tracks connection_ids that have already been dispatched. The first event
/// for a connection passes through; subsequent events with the same
/// connection_id within the TTL window are dropped.
///
/// Expired entries are swept inline on every `is_duplicate()` call via
/// `HashMap::retain`. No background thread needed - the cache stays small
/// because entries expire quickly.
pub struct DedupCache {
    seen: HashMap<String, Instant>,
    ttl: Duration,
}

impl DedupCache {
    /// Create a new dedup cache. Entries older than `ttl` are swept on each lookup.
    pub fn new(ttl: Duration) -> Self {
        Self {
            seen: HashMap::new(),
            ttl,
        }
    }

    /// Returns `true` if this connection_id was already seen within the TTL.
    /// Empty connection_id always returns `false` (never suppressed).
    pub fn is_duplicate(&mut self, connection_id: &str) -> bool {
        if connection_id.is_empty() {
            return false;
        }

        let now = Instant::now();

        // Sweep expired entries
        let ttl = self.ttl;
        self.seen.retain(|_, ts| now.duration_since(*ts) < ttl);

        // Check-and-insert
        if self.seen.contains_key(connection_id) {
            true
        } else {
            self.seen.insert(connection_id.to_string(), now);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_connection_id_never_deduped() {
        let mut cache = DedupCache::new(Duration::from_secs(5));
        assert!(!cache.is_duplicate(""));
        assert!(!cache.is_duplicate(""));
    }

    #[test]
    fn duplicate_detected_within_ttl() {
        let mut cache = DedupCache::new(Duration::from_secs(5));
        assert!(!cache.is_duplicate("abc123"));
        assert!(cache.is_duplicate("abc123"));
    }

    #[test]
    fn different_ids_not_deduped() {
        let mut cache = DedupCache::new(Duration::from_secs(5));
        assert!(!cache.is_duplicate("abc"));
        assert!(!cache.is_duplicate("def"));
    }

    #[test]
    fn same_connection_same_bucket() {
        let ts = 1773506947460i64;
        let id1 = compute_connection_id("10.0.0.1", "api.anthropic.com", ts);
        let id2 = compute_connection_id("10.0.0.1", "api.anthropic.com", ts + 500);
        assert_eq!(id1, id2, "same 2-second bucket should produce same ID");
    }

    #[test]
    fn different_bucket_different_id() {
        let ts = 1773506946000i64; // exactly on a 2s boundary
        let id1 = compute_connection_id("10.0.0.1", "api.anthropic.com", ts);
        let id2 = compute_connection_id("10.0.0.1", "api.anthropic.com", ts + 2000);
        assert_ne!(
            id1, id2,
            "different 2-second bucket should produce different ID"
        );
    }

    #[test]
    fn different_host_different_id() {
        let ts = 1773506947460i64;
        let id1 = compute_connection_id("10.0.0.1", "api.anthropic.com", ts);
        let id2 = compute_connection_id("10.0.0.1", "api.openai.com", ts);
        assert_ne!(id1, id2);
    }

    #[test]
    fn different_src_ip_different_id() {
        let ts = 1773506947460i64;
        let id1 = compute_connection_id("10.0.0.1", "api.anthropic.com", ts);
        let id2 = compute_connection_id("10.0.0.2", "api.anthropic.com", ts);
        assert_ne!(id1, id2);
    }

    #[test]
    fn empty_host_returns_empty_id() {
        let id = compute_connection_id("10.0.0.1", "", 1773506947460);
        assert!(id.is_empty());
    }
}
