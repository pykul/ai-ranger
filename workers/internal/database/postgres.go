// Package database provides connection setup for Postgres (GORM) and ClickHouse.
package database

import (
	"fmt"

	"gorm.io/driver/postgres"
	"gorm.io/gorm"
)

// MaxOpenConns is the maximum number of open connections in the pool.
const MaxOpenConns = 10

// MaxIdleConns is the maximum number of idle connections in the pool.
const MaxIdleConns = 5

// ConnectPostgres opens a GORM connection to Postgres using the provided DSN.
// The DSN comes from config.Config, not from environment variables directly.
func ConnectPostgres(dsn string) (*gorm.DB, error) {
	db, err := gorm.Open(postgres.Open(dsn), &gorm.Config{})
	if err != nil {
		return nil, fmt.Errorf("connect postgres: %w", err)
	}

	sqlDB, err := db.DB()
	if err != nil {
		return nil, fmt.Errorf("get underlying sql.DB: %w", err)
	}
	sqlDB.SetMaxOpenConns(MaxOpenConns)
	sqlDB.SetMaxIdleConns(MaxIdleConns)

	return db, nil
}
