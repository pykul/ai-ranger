package models

import (
	"time"

	"github.com/google/uuid"
)

// ClickHouseEvent is a plain Go struct for inserting events into ClickHouse.
// This is NOT a GORM model -- ClickHouse uses the clickhouse-go driver with plain SQL.
// Field tags use `ch` for clickhouse-go column mapping.
type ClickHouseEvent struct {
	OrgID           uuid.UUID `ch:"org_id"`
	AgentID         uuid.UUID `ch:"agent_id"`
	Hostname        string    `ch:"hostname"`
	OsUsername      string    `ch:"os_username"`
	OsType          string    `ch:"os_type"`
	Timestamp       time.Time `ch:"timestamp"`
	Provider        string    `ch:"provider"`
	ProviderHost    string    `ch:"provider_host"`
	ModelHint       string    `ch:"model_hint"`
	ProcessName     string    `ch:"process_name"`
	ProcessPath     string    `ch:"process_path"`
	DetectionMethod string    `ch:"detection_method"` // "sni", "dns", "ip_range", "tcp_heuristic"
	CaptureMode     string    `ch:"capture_mode"`     // "dns_sni", "mitm"
}
