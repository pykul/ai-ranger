// Package store provides query helpers for dashboard data.
package store

import (
	"context"
	"fmt"
	"time"

	"github.com/ClickHouse/clickhouse-go/v2"

	"github.com/pykul/ai-ranger/workers/internal/constants"
)

// ClickHouseStore provides read queries against the ai_events table.
type ClickHouseStore struct {
	conn clickhouse.Conn
}

// NewClickHouseStore creates a ClickHouseStore with the given connection.
func NewClickHouseStore(conn clickhouse.Conn) *ClickHouseStore {
	return &ClickHouseStore{conn: conn}
}

// OverviewStats holds summary data for the dashboard overview endpoint.
type OverviewStats struct {
	TotalEvents      uint64 `json:"total_events"`
	TotalProviders   uint64 `json:"total_providers"`
	TotalAgents      uint64 `json:"total_agents"`
	EventsLast24h    uint64 `json:"events_last_24h"`
}

// GetOverview returns org-wide summary stats.
func (s *ClickHouseStore) GetOverview(ctx context.Context) (*OverviewStats, error) {
	var stats OverviewStats
	query := fmt.Sprintf(`
		SELECT
			count() AS total_events,
			uniq(provider) AS total_providers,
			uniq(agent_id) AS total_agents,
			countIf(timestamp > now() - INTERVAL 1 DAY) AS events_last_24h
		FROM %s
	`, constants.ClickHouseEventsTable)

	row := s.conn.QueryRow(ctx, query)
	if err := row.Scan(&stats.TotalEvents, &stats.TotalProviders, &stats.TotalAgents, &stats.EventsLast24h); err != nil {
		return nil, fmt.Errorf("query overview: %w", err)
	}
	return &stats, nil
}

// ProviderBreakdown holds per-provider stats.
type ProviderBreakdown struct {
	Provider    string `json:"provider"`
	Connections uint64 `json:"connections"`
	UniqueUsers uint64 `json:"unique_users"`
}

// GetProviders returns provider breakdown.
func (s *ClickHouseStore) GetProviders(ctx context.Context) ([]ProviderBreakdown, error) {
	query := fmt.Sprintf(`
		SELECT provider, count() AS connections, uniq(os_username) AS unique_users
		FROM %s
		GROUP BY provider
		ORDER BY connections DESC
	`, constants.ClickHouseEventsTable)

	rows, err := s.conn.Query(ctx, query)
	if err != nil {
		return nil, fmt.Errorf("query providers: %w", err)
	}
	defer rows.Close()

	var results []ProviderBreakdown
	for rows.Next() {
		var p ProviderBreakdown
		if err := rows.Scan(&p.Provider, &p.Connections, &p.UniqueUsers); err != nil {
			return nil, fmt.Errorf("scan provider row: %w", err)
		}
		results = append(results, p)
	}
	return results, nil
}

// UserActivity holds per-user activity data.
type UserActivity struct {
	OsUsername  string    `json:"os_username"`
	Hostname   string    `json:"hostname"`
	Provider   string    `json:"provider"`
	ProcessName string   `json:"process_name"`
	Connections uint64   `json:"connections"`
	LastActive  time.Time `json:"last_active"`
}

// GetUsers returns per-user activity.
func (s *ClickHouseStore) GetUsers(ctx context.Context) ([]UserActivity, error) {
	query := fmt.Sprintf(`
		SELECT os_username, hostname, provider, process_name, count() AS connections, max(timestamp) AS last_active
		FROM %s
		GROUP BY os_username, hostname, provider, process_name
		ORDER BY last_active DESC
	`, constants.ClickHouseEventsTable)

	rows, err := s.conn.Query(ctx, query)
	if err != nil {
		return nil, fmt.Errorf("query users: %w", err)
	}
	defer rows.Close()

	var results []UserActivity
	for rows.Next() {
		var u UserActivity
		if err := rows.Scan(&u.OsUsername, &u.Hostname, &u.Provider, &u.ProcessName, &u.Connections, &u.LastActive); err != nil {
			return nil, fmt.Errorf("scan user row: %w", err)
		}
		results = append(results, u)
	}
	return results, nil
}

// TrafficPoint holds a single timeseries data point.
type TrafficPoint struct {
	Timestamp   time.Time `json:"timestamp"`
	Provider    string    `json:"provider"`
	Connections uint64    `json:"connections"`
}

// GetTrafficTimeseries returns hourly traffic by provider.
func (s *ClickHouseStore) GetTrafficTimeseries(ctx context.Context) ([]TrafficPoint, error) {
	query := fmt.Sprintf(`
		SELECT toStartOfHour(timestamp) AS ts, provider, count() AS connections
		FROM %s
		WHERE timestamp > now() - INTERVAL 7 DAY
		GROUP BY ts, provider
		ORDER BY ts
	`, constants.ClickHouseEventsTable)

	rows, err := s.conn.Query(ctx, query)
	if err != nil {
		return nil, fmt.Errorf("query traffic: %w", err)
	}
	defer rows.Close()

	var results []TrafficPoint
	for rows.Next() {
		var t TrafficPoint
		if err := rows.Scan(&t.Timestamp, &t.Provider, &t.Connections); err != nil {
			return nil, fmt.Errorf("scan traffic row: %w", err)
		}
		results = append(results, t)
	}
	return results, nil
}
