package database

import (
	"context"
	"fmt"

	"github.com/ClickHouse/clickhouse-go/v2"
)

// ConnectClickHouse opens a connection to ClickHouse via the native protocol.
// Address and database name come from config.Config, not from environment variables directly.
func ConnectClickHouse(addr, database string) (clickhouse.Conn, error) {
	conn, err := clickhouse.Open(&clickhouse.Options{
		Addr: []string{addr},
		Auth: clickhouse.Auth{
			Database: database,
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
