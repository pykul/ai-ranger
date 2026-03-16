// Package writer handles writing events to ClickHouse and updating Postgres.
package writer

import (
	"context"
	"fmt"
	"log"
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

	agentID, err := w.insertClickHouseEvents(batch)
	if err != nil {
		return err
	}

	w.updateAgentLastSeen(agentID)
	return nil
}

// insertClickHouseEvents batch-inserts all events into ClickHouse.
// Returns the parsed agent UUID from the last event for Postgres update.
func (w *Writer) insertClickHouseEvents(batch *rangerpb.EventBatch) (uuid.UUID, error) {
	ctx := context.Background()
	chBatch, err := w.ch.PrepareBatch(ctx, fmt.Sprintf(
		"INSERT INTO %s", constants.ClickHouseEventsTable,
	))
	if err != nil {
		return uuid.Nil, fmt.Errorf("prepare clickhouse batch: %w", err)
	}

	var agentID uuid.UUID
	for _, e := range batch.Events {
		parsed, _ := uuid.Parse(e.AgentId)
		agentID = parsed
		orgID, _ := uuid.Parse(batch.AgentId)

		if err := chBatch.Append(
			orgID, parsed, e.MachineHostname, e.OsUsername, e.OsType,
			time.UnixMilli(e.TimestampMs), e.Provider, e.ProviderHost,
			stringOrEmpty(e.ModelHint), e.ProcessName, stringOrEmpty(e.ProcessPath),
			detectionMethodString(e.DetectionMethod), captureModeString(e.CaptureMode),
		); err != nil {
			return uuid.Nil, fmt.Errorf("append to clickhouse batch: %w", err)
		}
	}

	if err := chBatch.Send(); err != nil {
		return uuid.Nil, fmt.Errorf("send clickhouse batch: %w", err)
	}
	return agentID, nil
}

// updateAgentLastSeen sets the agent's last_seen_at timestamp in Postgres.
// Logs errors rather than returning them — a failed metadata update should
// not prevent event processing.
func (w *Writer) updateAgentLastSeen(agentID uuid.UUID) {
	if agentID == uuid.Nil {
		return
	}
	now := time.Now().UTC()
	if err := w.pg.Model(&models.Agent{}).Where("id = ?", agentID).Update("last_seen_at", now).Error; err != nil {
		log.Printf("[writer] Failed to update agent last_seen_at: %v", err)
	}
}

// detectionMethodString converts the protobuf enum to the ClickHouse Enum8 string value.
func detectionMethodString(dm rangerpb.DetectionMethod) string {
	switch dm {
	case rangerpb.DetectionMethod_SNI:
		return constants.DetectionMethodSNI
	case rangerpb.DetectionMethod_DNS:
		return constants.DetectionMethodDNS
	case rangerpb.DetectionMethod_IP_RANGE:
		return constants.DetectionMethodIPRange
	case rangerpb.DetectionMethod_TCP_HEURISTIC:
		return constants.DetectionMethodTCPHeuristic
	default:
		return constants.DetectionMethodSNI
	}
}

// captureModeString converts the protobuf enum to the ClickHouse Enum8 string value.
func captureModeString(cm rangerpb.CaptureMode) string {
	switch cm {
	case rangerpb.CaptureMode_DNS_SNI:
		return constants.CaptureModeDNSSNI
	case rangerpb.CaptureMode_MITM:
		return constants.CaptureModeMITM
	default:
		return constants.CaptureModeDNSSNI
	}
}

func stringOrEmpty(s *string) string {
	if s == nil {
		return ""
	}
	return *s
}
