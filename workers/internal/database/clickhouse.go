package database

import (
	"context"
	"fmt"
	"os"

	"github.com/ClickHouse/clickhouse-go/v2"
)

// Default ClickHouse address matches docker-compose.yml clickhouse service.
const defaultClickHouseAddr = "localhost:9000"

// ConnectClickHouse opens a connection to ClickHouse via the native protocol.
// Reads CLICKHOUSE_ADDR from the environment, falling back to the local default.
func ConnectClickHouse() (clickhouse.Conn, error) {
	addr := os.Getenv("CLICKHOUSE_ADDR")
	if addr == "" {
		addr = defaultClickHouseAddr
	}

	conn, err := clickhouse.Open(&clickhouse.Options{
		Addr: []string{addr},
		Auth: clickhouse.Auth{
			Database: "default",
		},
	})
	if err != nil {
		return nil, fmt.Errorf("open clickhouse: %w", err)
	}

	if err := conn.Ping(context.Background()); err != nil {
		return nil, fmt.Errorf("ping clickhouse: %w", err)
	}

	return conn, nil
}
