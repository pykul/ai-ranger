// Package database provides connection setup for Postgres (GORM) and ClickHouse.
package database

import (
	"fmt"
	"os"

	"gorm.io/driver/postgres"
	"gorm.io/gorm"
)

// PostgresDSN default matches docker-compose.yml postgres service.
const defaultPostgresDSN = "host=localhost port=5432 user=ranger password=ranger dbname=ranger sslmode=disable"

// MaxOpenConns is the maximum number of open connections in the pool.
const MaxOpenConns = 10

// MaxIdleConns is the maximum number of idle connections in the pool.
const MaxIdleConns = 5

// ConnectPostgres opens a GORM connection to Postgres.
// Reads DATABASE_URL from the environment, falling back to the local default.
func ConnectPostgres() (*gorm.DB, error) {
	dsn := os.Getenv("DATABASE_URL")
	if dsn == "" {
		dsn = defaultPostgresDSN
	}

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
