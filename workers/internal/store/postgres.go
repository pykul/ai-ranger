package store

import (
	"fmt"

	"github.com/google/uuid"
	"gorm.io/gorm"

	"github.com/pykul/ai-ranger/workers/internal/constants"
	"github.com/pykul/ai-ranger/workers/internal/models"
)

// PostgresStore provides GORM query helpers for fleet and token management.
type PostgresStore struct {
	db *gorm.DB
}

// NewPostgresStore creates a PostgresStore with the given GORM connection.
func NewPostgresStore(db *gorm.DB) *PostgresStore {
	return &PostgresStore{db: db}
}

// FleetAgent represents an agent in the fleet listing.
type FleetAgent struct {
	models.Agent
}

// GetFleet returns all enrolled agents.
func (s *PostgresStore) GetFleet() ([]FleetAgent, error) {
	var agents []models.Agent
	if err := s.db.Order("enrolled_at DESC").Find(&agents).Error; err != nil {
		return nil, fmt.Errorf("query fleet: %w", err)
	}
	result := make([]FleetAgent, len(agents))
	for i, a := range agents {
		result[i] = FleetAgent{Agent: a}
	}
	return result, nil
}

// CreateToken creates a new enrollment token.
func (s *PostgresStore) CreateToken(token *models.EnrollmentToken) error {
	if err := s.db.Create(token).Error; err != nil {
		return fmt.Errorf("create token: %w", err)
	}
	return nil
}

// DeleteToken revokes an enrollment token by ID.
func (s *PostgresStore) DeleteToken(id uuid.UUID) error {
	if err := s.db.Delete(&models.EnrollmentToken{}, "id = ?", id).Error; err != nil {
		return fmt.Errorf("delete token: %w", err)
	}
	return nil
}

// RevokeAgent sets an agent's status to "revoked".
func (s *PostgresStore) RevokeAgent(id uuid.UUID) error {
	result := s.db.Model(&models.Agent{}).Where("id = ?", id).Update("status", constants.AgentStatusRevoked)
	if result.Error != nil {
		return fmt.Errorf("revoke agent: %w", result.Error)
	}
	if result.RowsAffected == 0 {
		return fmt.Errorf("agent not found: %s", id)
	}
	return nil
}
