// Package writer handles writing events to ClickHouse and updating Postgres.
package writer

import (
	"context"
	"fmt"
	"time"

	"github.com/ClickHouse/clickhouse-go/v2"
	"github.com/google/uuid"
	"gorm.io/gorm"

	"github.com/pykul/ai-ranger/workers/internal/constants"
	"github.com/pykul/ai-ranger/workers/internal/models"

	rangerpb "github.com/pykul/ai-ranger/proto/gen/go/ranger/v1"
)

// Writer writes events to ClickHouse and updates agent metadata in Postgres.
type Writer struct {
	ch clickhouse.Conn
	pg *gorm.DB
}

// New creates a Writer with the given ClickHouse and Postgres connections.
func New(ch clickhouse.Conn, pg *gorm.DB) *Writer {
	return &Writer{ch: ch, pg: pg}
}

// WriteEvents inserts events from an EventBatch into ClickHouse and updates
// the agent's last_seen_at in Postgres.
func (w *Writer) WriteEvents(batch *rangerpb.EventBatch) error {
	if len(batch.Events) == 0 {
		return nil
	}

	// Insert events into ClickHouse.
	ctx := context.Background()
	chBatch, err := w.ch.PrepareBatch(ctx, fmt.Sprintf(
		"INSERT INTO %s", constants.ClickHouseEventsTable,
	))
	if err != nil {
		return fmt.Errorf("prepare clickhouse batch: %w", err)
	}

	var agentID uuid.UUID
	for _, e := range batch.Events {
		parsed, _ := uuid.Parse(e.AgentId)
		agentID = parsed
		orgID, _ := uuid.Parse(batch.AgentId) // batch-level agent_id used for org lookup

		dm := detectionMethodString(e.DetectionMethod)
		cm := captureModeString(e.CaptureMode)

		if err := chBatch.Append(
			orgID,                                                  // org_id (placeholder - resolved from agent record)
			parsed,                                                 // agent_id
			e.MachineHostname,                                      // hostname
			e.OsUsername,                                           // os_username
			e.OsType,                                               // os_type
			time.UnixMilli(e.TimestampMs),                          // timestamp
			e.Provider,                                             // provider
			e.ProviderHost,                                         // provider_host
			stringOrEmpty(e.ModelHint),                              // model_hint
			e.ProcessName,                                          // process_name
			stringOrEmpty(e.ProcessPath),                            // process_path
			dm,                                                     // detection_method
			cm,                                                     // capture_mode
		); err != nil {
			return fmt.Errorf("append to clickhouse batch: %w", err)
		}
	}

	if err := chBatch.Send(); err != nil {
		return fmt.Errorf("send clickhouse batch: %w", err)
	}

	// Update agent last_seen_at in Postgres via GORM.
	if agentID != uuid.Nil {
		now := time.Now().UTC()
		w.pg.Model(&models.Agent{}).Where("id = ?", agentID).Update("last_seen_at", now)
	}

	return nil
}

func detectionMethodString(dm rangerpb.DetectionMethod) string {
	switch dm {
	case rangerpb.DetectionMethod_SNI:
		return "sni"
	case rangerpb.DetectionMethod_DNS:
		return "dns"
	case rangerpb.DetectionMethod_IP_RANGE:
		return "ip_range"
	case rangerpb.DetectionMethod_TCP_HEURISTIC:
		return "tcp_heuristic"
	default:
		return "sni"
	}
}

func captureModeString(cm rangerpb.CaptureMode) string {
	switch cm {
	case rangerpb.CaptureMode_DNS_SNI:
		return "dns_sni"
	case rangerpb.CaptureMode_MITM:
		return "mitm"
	default:
		return "dns_sni"
	}
}

func stringOrEmpty(s *string) string {
	if s == nil {
		return ""
	}
	return *s
}
