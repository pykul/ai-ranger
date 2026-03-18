// Package store provides query helpers for dashboard data.
package store

import (
	"context"
	"fmt"
	"strings"
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
	TotalConnections uint64 `json:"total_connections"`
	ActiveUsers      uint64 `json:"active_users"`
	ProviderCount    uint64 `json:"provider_count"`
}

// GetOverview returns org-wide summary stats for the given time range.
func (s *ClickHouseStore) GetOverview(ctx context.Context, days int) (*OverviewStats, error) {
	var stats OverviewStats
	query := fmt.Sprintf(`
		SELECT
			count() AS total_connections,
			uniq(os_username) AS active_users,
			uniq(provider) AS provider_count
		FROM %s
		WHERE timestamp > now() - INTERVAL %d DAY
	`, constants.ClickHouseEventsTable, days)

	row := s.conn.QueryRow(ctx, query)
	if err := row.Scan(&stats.TotalConnections, &stats.ActiveUsers, &stats.ProviderCount); err != nil {
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

// GetProviders returns provider breakdown for the given time range.
func (s *ClickHouseStore) GetProviders(ctx context.Context, days int) ([]ProviderBreakdown, error) {
	query := fmt.Sprintf(`
		SELECT provider, count() AS connections, uniq(os_username) AS unique_users
		FROM %s
		WHERE timestamp > now() - INTERVAL %d DAY
		GROUP BY provider
		ORDER BY connections DESC
	`, constants.ClickHouseEventsTable, days)

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
	OsUsername  string `json:"os_username"`
	Connections uint64 `json:"connections"`
}

// GetUsers returns per-user connection counts for the given time range.
// If provider is non-empty, filters to that provider only.
func (s *ClickHouseStore) GetUsers(ctx context.Context, days int, provider string) ([]UserActivity, error) {
	where := fmt.Sprintf("timestamp > now() - INTERVAL %d DAY", days)
	if provider != "" {
		where += fmt.Sprintf(" AND provider = '%s'", escapeSingleQuote(provider))
	}

	query := fmt.Sprintf(`
		SELECT os_username, count() AS connections
		FROM %s
		WHERE %s
		GROUP BY os_username
		ORDER BY connections DESC
	`, constants.ClickHouseEventsTable, where)

	rows, err := s.conn.Query(ctx, query)
	if err != nil {
		return nil, fmt.Errorf("query users: %w", err)
	}
	defer rows.Close()

	var results []UserActivity
	for rows.Next() {
		var u UserActivity
		if err := rows.Scan(&u.OsUsername, &u.Connections); err != nil {
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

// GetTrafficTimeseries returns hourly traffic by provider for the given time range.
func (s *ClickHouseStore) GetTrafficTimeseries(ctx context.Context, days int) ([]TrafficPoint, error) {
	query := fmt.Sprintf(`
		SELECT toStartOfHour(timestamp) AS ts, provider, count() AS connections
		FROM %s
		WHERE timestamp > now() - INTERVAL %d DAY
		GROUP BY ts, provider
		ORDER BY ts
	`, constants.ClickHouseEventsTable, days)

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

// EventRow holds a single raw event for the events table.
type EventRow struct {
	Timestamp       time.Time `json:"timestamp"`
	OsUsername      string    `json:"os_username"`
	MachineHostname string    `json:"machine_hostname"`
	Provider        string    `json:"provider"`
	ProviderHost    string    `json:"provider_host"`
	ProcessName     string    `json:"process_name"`
	OsType          string    `json:"os_type"`
	DetectionMethod string    `json:"detection_method"`
	SrcIP           string    `json:"src_ip"`
	ModelHint       string    `json:"model_hint"`
	ProcessPath     string    `json:"process_path"`
	CaptureMode     string    `json:"capture_mode"`
}

// EventsResult holds paginated event results.
type EventsResult struct {
	Events []EventRow `json:"events"`
	Total  uint64     `json:"total"`
	Page   int        `json:"page"`
	Limit  int        `json:"limit"`
}

// GetEvents returns paginated raw events with optional search and time filter.
func (s *ClickHouseStore) GetEvents(ctx context.Context, q string, days, page, limit int, sort, order string) (*EventsResult, error) {
	where := fmt.Sprintf("timestamp > now() - INTERVAL %d DAY", days)
	if q != "" {
		escaped := escapeSingleQuote(q)
		like := fmt.Sprintf("'%%%s%%'", escaped)
		where += fmt.Sprintf(` AND (
			provider ILIKE %s OR provider_host ILIKE %s OR process_name ILIKE %s OR
			hostname ILIKE %s OR os_username ILIKE %s OR src_ip ILIKE %s
		)`, like, like, like, like, like, like)
	}

	// Validate sort column to prevent injection.
	sortCol := "timestamp"
	switch sort {
	case "timestamp", "os_username", "provider", "process_name":
		sortCol = sort
	}
	orderDir := "DESC"
	if order == "asc" {
		orderDir = "ASC"
	}

	offset := (page - 1) * limit

	// Count query.
	countQuery := fmt.Sprintf("SELECT count() FROM %s WHERE %s", constants.ClickHouseEventsTable, where)
	var total uint64
	if err := s.conn.QueryRow(ctx, countQuery).Scan(&total); err != nil {
		return nil, fmt.Errorf("count events: %w", err)
	}

	// Data query.
	dataQuery := fmt.Sprintf(`
		SELECT timestamp, os_username, hostname, provider, provider_host, process_name,
			os_type, detection_method, src_ip, model_hint, process_path, capture_mode
		FROM %s
		WHERE %s
		ORDER BY %s %s
		LIMIT %d OFFSET %d
	`, constants.ClickHouseEventsTable, where, sortCol, orderDir, limit, offset)

	rows, err := s.conn.Query(ctx, dataQuery)
	if err != nil {
		return nil, fmt.Errorf("query events: %w", err)
	}
	defer rows.Close()

	var events []EventRow
	for rows.Next() {
		var e EventRow
		if err := rows.Scan(
			&e.Timestamp, &e.OsUsername, &e.MachineHostname, &e.Provider, &e.ProviderHost,
			&e.ProcessName, &e.OsType, &e.DetectionMethod, &e.SrcIP, &e.ModelHint,
			&e.ProcessPath, &e.CaptureMode,
		); err != nil {
			return nil, fmt.Errorf("scan event row: %w", err)
		}
		events = append(events, e)
	}

	return &EventsResult{
		Events: events,
		Total:  total,
		Page:   page,
		Limit:  limit,
	}, nil
}

func escapeSingleQuote(s string) string {
	return strings.ReplaceAll(s, "'", "\\'")
}
