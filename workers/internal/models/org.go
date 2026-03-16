// Package models defines GORM structs mirroring the SQLAlchemy models in gateway/models/.
//
// The gateway's SQLAlchemy models are the source of truth for the Postgres schema.
// These GORM structs MUST be kept in exact sync. Any Alembic migration that changes
// a column must be accompanied by a corresponding update here in the same commit.
package models

import (
	"time"

	"github.com/google/uuid"
)

// Organization represents a customer organization that enrolls agents.
type Organization struct {
	ID        uuid.UUID `gorm:"type:uuid;primaryKey"`
	Name      string    `gorm:"not null"`
	Slug      string    `gorm:"uniqueIndex;not null"`
	CreatedAt time.Time `gorm:"autoCreateTime"`
}

// TableName overrides the default GORM table name.
func (Organization) TableName() string { return "organizations" }

// EnrollmentToken is a single-use or multi-use token for agent enrollment.
// Tokens are stored as SHA256 hashes -- the plaintext is never persisted.
type EnrollmentToken struct {
	ID        uuid.UUID  `gorm:"type:uuid;primaryKey"`
	OrgID     uuid.UUID  `gorm:"type:uuid;not null;index"`
	TokenHash string     `gorm:"uniqueIndex;not null"`
	Label     *string    `gorm:""`
	CreatedBy *uuid.UUID `gorm:"type:uuid"`
	ExpiresAt *time.Time `gorm:""`
	MaxUses   int        `gorm:"default:1"`
	UsedCount int        `gorm:"default:0"`
	CreatedAt time.Time  `gorm:"autoCreateTime"`
}

// TableName overrides the default GORM table name.
func (EnrollmentToken) TableName() string { return "enrollment_tokens" }

// Agent represents an enrolled agent reporting from a machine.
type Agent struct {
	ID           uuid.UUID  `gorm:"type:uuid;primaryKey"` // generated on device
	OrgID        uuid.UUID  `gorm:"type:uuid;not null;index"`
	Hostname     string     `gorm:"not null"`
	OsUsername   string     `gorm:"column:os_username;not null"`
	Os           string     `gorm:"not null"` // "linux" | "macos" | "windows"
	AgentVersion string     `gorm:"column:agent_version;not null"`
	EnrolledAt   time.Time  `gorm:"autoCreateTime"`
	LastSeenAt   *time.Time `gorm:""`
	Status       string     `gorm:"default:'active'"` // "active" | "revoked"
}

// TableName overrides the default GORM table name.
func (Agent) TableName() string { return "agents" }
