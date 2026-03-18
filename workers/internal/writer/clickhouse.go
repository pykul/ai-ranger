// Package writer handles writing events to ClickHouse.
package writer

import (
	"context"
	"fmt"
	"time"

	"github.com/ClickHouse/clickhouse-go/v2"
	"github.com/google/uuid"

	"github.com/pykul/ai-ranger/workers/internal/constants"

	rangerpb "github.com/pykul/ai-ranger/proto/gen/go/ranger/v1"
)

// ClickHouseWriter writes events to ClickHouse.
type ClickHouseWriter struct {
	ch clickhouse.Conn
}

// NewClickHouseWriter creates a ClickHouseWriter with the given connection.
func NewClickHouseWriter(ch clickhouse.Conn) *ClickHouseWriter {
	return &ClickHouseWriter{ch: ch}
}

// WriteEvents inserts events from an EventBatch into ClickHouse.
// Returns the agent UUID extracted from the batch for use by other writers.
func (w *ClickHouseWriter) WriteEvents(batch *rangerpb.EventBatch) (uuid.UUID, error) {
	if len(batch.Events) == 0 {
		return uuid.Nil, nil
	}
	return w.insertClickHouseEvents(batch)
}

// insertClickHouseEvents batch-inserts all events into ClickHouse.
// Returns the parsed agent UUID from the last event.
func (w *ClickHouseWriter) insertClickHouseEvents(batch *rangerpb.EventBatch) (uuid.UUID, error) {
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
			e.SrcIp,
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
