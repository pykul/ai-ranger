// Package writer handles writing events to ClickHouse and updating Postgres.
package writer

import (
	"log"
	"time"

	"github.com/google/uuid"
	"gorm.io/gorm"

	"github.com/pykul/ai-ranger/workers/internal/models"
)

// PostgresWriter updates agent metadata in Postgres independently of event writes.
type PostgresWriter struct {
	pg *gorm.DB
}

// NewPostgresWriter creates a PostgresWriter with the given GORM connection.
func NewPostgresWriter(pg *gorm.DB) *PostgresWriter {
	return &PostgresWriter{pg: pg}
}

// UpdateAgentLastSeen sets the agent's last_seen_at timestamp in Postgres.
// Logs errors rather than returning them — a failed metadata update should
// not prevent event processing.
func (pw *PostgresWriter) UpdateAgentLastSeen(agentID uuid.UUID) {
	if agentID == uuid.Nil {
		return
	}
	now := time.Now().UTC()
	if err := pw.pg.Model(&models.Agent{}).Where("id = ?", agentID).Update("last_seen_at", now).Error; err != nil {
		log.Printf("[postgres] Failed to update agent last_seen_at: %v", err)
	}
}
