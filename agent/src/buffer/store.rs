use crate::event::AiConnectionEvent;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

/// SQLite-backed event buffer for the HTTP sink.
///
/// Events are written immediately on capture (< 1ms). A background task drains
/// the buffer every 30 seconds, serializes to JSON, POSTs to the gateway, and
/// deletes on success. Network outages do not lose data.
///
/// Only active when an HTTP output sink is configured. In stdout or file mode,
/// this module is not used.
pub struct EventBuffer {
    conn: Mutex<Connection>,
}

impl EventBuffer {
    pub fn open(path: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                json TEXT NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
            )",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Insert an event into the buffer. Fast (< 1ms).
    pub fn insert(&self, event: &AiConnectionEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let json = serde_json::to_string(event)?;
        let conn = self.conn.lock().map_err(|e| format!("lock: {e}"))?;
        conn.execute("INSERT INTO events (json) VALUES (?1)", params![json])?;
        Ok(())
    }

    /// Read up to `limit` events from the buffer. Returns (id, json) pairs.
    pub fn read_batch(&self, limit: usize) -> Result<Vec<(i64, String)>, Box<dyn std::error::Error + Send + Sync>> {
        let conn = self.conn.lock().map_err(|e| format!("lock: {e}"))?;
        let mut stmt = conn.prepare("SELECT id, json FROM events ORDER BY id LIMIT ?1")?;
        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Return the number of events currently in the buffer.
    #[cfg(test)]
    pub fn count(&self) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let conn = self.conn.lock().map_err(|e| format!("lock: {e}"))?;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Delete events by IDs after successful upload.
    pub fn delete_batch(&self, ids: &[i64]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if ids.is_empty() {
            return Ok(());
        }
        let conn = self.conn.lock().map_err(|e| format!("lock: {e}"))?;
        // Build a parameterized IN clause
        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        let sql = format!("DELETE FROM events WHERE id IN ({})", placeholders.join(","));
        let params: Vec<Box<dyn rusqlite::types::ToSql>> =
            ids.iter().map(|id| Box::new(*id) as Box<dyn rusqlite::types::ToSql>).collect();
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())?;
        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{CaptureMode, DetectionMethod};

    fn test_event() -> AiConnectionEvent {
        AiConnectionEvent {
            agent_id: "test-agent".to_string(),
            machine_hostname: "test-host".to_string(),
            os_username: "testuser".to_string(),
            connection_id: String::new(),
            timestamp_ms: 1234567890,
            duration_ms: None,
            provider: "openai".to_string(),
            provider_host: "api.openai.com".to_string(),
            model_hint: None,
            process_name: "curl".to_string(),
            process_pid: 1234,
            process_path: None,
            src_ip: "192.168.1.100".to_string(),
            detection_method: DetectionMethod::Sni,
            capture_mode: CaptureMode::DnsSni,
            content_available: false,
            payload_ref: None,
            model_exact: None,
            token_count_input: None,
            token_count_output: None,
            latency_ttfb_ms: None,
        }
    }

    #[test]
    fn insert_and_read() {
        let buf = EventBuffer::open(Path::new(":memory:")).unwrap();
        let event = test_event();
        buf.insert(&event).unwrap();
        buf.insert(&event).unwrap();

        assert_eq!(buf.count().unwrap(), 2);

        let batch = buf.read_batch(10).unwrap();
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn delete_batch() {
        let buf = EventBuffer::open(Path::new(":memory:")).unwrap();
        let event = test_event();
        buf.insert(&event).unwrap();
        buf.insert(&event).unwrap();
        buf.insert(&event).unwrap();

        let batch = buf.read_batch(2).unwrap();
        let ids: Vec<i64> = batch.iter().map(|(id, _)| *id).collect();
        buf.delete_batch(&ids).unwrap();

        assert_eq!(buf.count().unwrap(), 1);
    }

    #[test]
    fn read_respects_limit() {
        let buf = EventBuffer::open(Path::new(":memory:")).unwrap();
        let event = test_event();
        for _ in 0..10 {
            buf.insert(&event).unwrap();
        }
        let batch = buf.read_batch(3).unwrap();
        assert_eq!(batch.len(), 3);
    }

    #[test]
    fn roundtrip_serialization() {
        // Verifies the buffer drain path: event → JSON → SQLite → JSON → event
        let buf = EventBuffer::open(Path::new(":memory:")).unwrap();
        let event = test_event();
        buf.insert(&event).unwrap();

        let batch = buf.read_batch(1).unwrap();
        assert_eq!(batch.len(), 1);

        let (_, json) = &batch[0];
        let deserialized: AiConnectionEvent = serde_json::from_str(json).unwrap();
        assert_eq!(deserialized.provider, "openai");
        assert_eq!(deserialized.provider_host, "api.openai.com");
        assert_eq!(deserialized.process_name, "curl");
        assert_eq!(deserialized.detection_method, DetectionMethod::Sni);
        assert_eq!(deserialized.capture_mode, CaptureMode::DnsSni);
        assert!(!deserialized.content_available);
    }
}
