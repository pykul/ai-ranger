// Package config provides centralized environment variable loading for all workers.
//
// All runtime configuration comes from environment variables loaded into the Config
// struct at startup. No os.Getenv calls exist outside this package. Constants in
// constants.go are for application contract values only (queue names, route paths).
package config

import (
	"log"
	"os"
	"strconv"

	"golang.org/x/crypto/bcrypt"
)

// Config holds all runtime configuration loaded from environment variables.
// Created once at startup in main.go and passed to all components.
type Config struct {
	// DatabaseURL is the GORM-compatible Postgres DSN.
	DatabaseURL string

	// ClickHouseAddr is the ClickHouse native protocol address (host:port).
	ClickHouseAddr string

	// ClickHouseDatabase is the ClickHouse database name.
	ClickHouseDatabase string

	// RabbitMQURL is the AMQP connection URL for RabbitMQ.
	RabbitMQURL string

	// APIServerPort is the port the query API server listens on.
	APIServerPort int

	// ShutdownTimeoutSecs is how long to wait for in-flight work before force-stopping.
	ShutdownTimeoutSecs int

	// Environment is "development" or "production". Controls auth behavior.
	// In development, auth middleware is bypassed entirely.
	Environment string

	// JWTSecret is the HMAC-SHA256 signing key for JWT tokens (production only).
	JWTSecret string

	// AdminEmail is the single admin user's email (production only).
	AdminEmail string

	// AdminPassword is the single admin user's plaintext password (production only).
	// Hashed once in memory at startup via bcrypt. The plaintext is not retained.
	AdminPassword string

	// AdminPasswordHash is the bcrypt hash of AdminPassword, computed once at startup.
	// Used by the login handler for comparison. Empty in development mode.
	AdminPasswordHash []byte
}

// defaultDatabaseURL is the fallback DSN for local development.
const defaultDatabaseURL = "host=localhost port=5432 user=ranger password=ranger dbname=ranger sslmode=disable"

// defaultClickHouseAddr is the fallback ClickHouse address for local development.
const defaultClickHouseAddr = "localhost:9000"

// defaultClickHouseDatabase is the fallback ClickHouse database name.
const defaultClickHouseDatabase = "default"

// defaultRabbitMQURL is the fallback AMQP URL for local development.
const defaultRabbitMQURL = "amqp://guest:guest@localhost:5672/"

// defaultAPIServerPort is the fallback port for the query API server.
const defaultAPIServerPort = 8081

// defaultShutdownTimeoutSecs is the fallback graceful shutdown timeout.
const defaultShutdownTimeoutSecs = 30

// defaultEnvironment is the fallback environment mode.
// Development disables auth; production requires JWT.
const defaultEnvironment = "development"

// Load reads all environment variables and returns a Config struct.
// Missing variables fall back to sensible defaults for local development.
// In production, ADMIN_PASSWORD is hashed once via bcrypt and the plaintext
// is not retained in the struct.
func Load() Config {
	env := envOrDefault("ENVIRONMENT", defaultEnvironment)
	plainPassword := os.Getenv("ADMIN_PASSWORD")

	var passwordHash []byte
	if env != "development" && plainPassword != "" {
		var err error
		passwordHash, err = bcrypt.GenerateFromPassword([]byte(plainPassword), bcrypt.DefaultCost)
		if err != nil {
			log.Fatalf("[config] Failed to hash ADMIN_PASSWORD: %v", err)
		}
	}

	return Config{
		DatabaseURL:         envOrDefault("DATABASE_URL", defaultDatabaseURL),
		ClickHouseAddr:      envOrDefault("CLICKHOUSE_ADDR", defaultClickHouseAddr),
		ClickHouseDatabase:  envOrDefault("CLICKHOUSE_DATABASE", defaultClickHouseDatabase),
		RabbitMQURL:         envOrDefault("RABBITMQ_URL", defaultRabbitMQURL),
		APIServerPort:       envOrDefaultInt("API_SERVER_PORT", defaultAPIServerPort),
		ShutdownTimeoutSecs: envOrDefaultInt("SHUTDOWN_TIMEOUT_SECS", defaultShutdownTimeoutSecs),
		Environment:         env,
		JWTSecret:           os.Getenv("JWT_SECRET"),
		AdminEmail:          os.Getenv("ADMIN_EMAIL"),
		AdminPasswordHash:   passwordHash,
	}
}

// envOrDefault returns the environment variable value or the fallback.
func envOrDefault(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

// envOrDefaultInt returns the environment variable value as int or the fallback.
func envOrDefaultInt(key string, fallback int) int {
	v := os.Getenv(key)
	if v == "" {
		return fallback
	}
	n, err := strconv.Atoi(v)
	if err != nil {
		return fallback
	}
	return n
}
