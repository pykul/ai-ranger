package writer

// Postgres writes are handled inline in WriteEvents via GORM (updating agent.last_seen_at).
// This file exists as a placeholder for future Postgres-specific write operations
// (e.g. fleet metadata updates) that may be separated from the event write path.
