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

// ListTokens returns all enrollment tokens.
func (s *PostgresStore) ListTokens() ([]models.EnrollmentToken, error) {
	var tokens []models.EnrollmentToken
	if err := s.db.Order("created_at DESC").Find(&tokens).Error; err != nil {
		return nil, fmt.Errorf("list tokens: %w", err)
	}
	return tokens, nil
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

// GetOrgSettings returns the settings for the first organization.
// In a single-org deployment (the standard case) this returns the only org's settings.
func (s *PostgresStore) GetOrgSettings() (*models.OrgSettings, error) {
	var settings models.OrgSettings
	err := s.db.Order("created_at ASC").First(&settings).Error
	if err != nil {
		return nil, err
	}
	return &settings, nil
}

// UpsertOrgSettings creates or updates the webhook URL for an organization.
func (s *PostgresStore) UpsertOrgSettings(orgID uuid.UUID, webhookURL *string) error {
	var existing models.OrgSettings
	err := s.db.Where("org_id = ?", orgID).First(&existing).Error
	if err != nil {
		// No existing row -- create one.
		record := models.OrgSettings{
			ID:         uuid.New(),
			OrgID:      orgID,
			WebhookURL: webhookURL,
		}
		if err := s.db.Create(&record).Error; err != nil {
			return fmt.Errorf("create org settings: %w", err)
		}
		return nil
	}

	// Update existing row.
	existing.WebhookURL = webhookURL
	if err := s.db.Save(&existing).Error; err != nil {
		return fmt.Errorf("update org settings: %w", err)
	}
	return nil
}
