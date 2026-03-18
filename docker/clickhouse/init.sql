-- AI Ranger ClickHouse schema: event timeseries.
--
-- No ORM for ClickHouse - this is plain SQL loaded on container startup.
-- See ARCHITECTURE.md § Storage Design and DECISIONS.md § ORMs for Postgres.
--
-- bytes_sent and bytes_received are intentionally omitted from the initial schema.
-- See DECISIONS.md § bytes_sent and bytes_received removed from ClickHouse schema.

CREATE TABLE IF NOT EXISTS ai_events (
    org_id          UUID,
    agent_id        UUID,
    hostname        String,
    os_username     LowCardinality(String),
    os_type         LowCardinality(String),
    timestamp       DateTime64(3, 'UTC'),
    provider        LowCardinality(String),
    provider_host   String,
    model_hint      LowCardinality(String),
    process_name    LowCardinality(String),
    process_path    String,
    src_ip          String,
    detection_method Enum8('sni'=1, 'dns'=2, 'ip_range'=3, 'tcp_heuristic'=4),
    capture_mode    Enum8('dns_sni'=1, 'mitm'=2)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (org_id, timestamp, agent_id, provider)
TTL toDateTime(timestamp) + INTERVAL 1 YEAR;
