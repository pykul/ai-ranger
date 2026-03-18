// Package store provides query helpers for dashboard data.
package store

import (
	"context"
	"fmt"
	"time"

	"github.com/ClickHouse/clickhouse-go/v2"
	"github.com/ClickHouse/clickhouse-go/v2/lib/driver"

	"github.com/pykul/ai-ranger/workers/internal/constants"
)

// maxProviderResults caps the number of providers returned to prevent unbounded queries.
const maxProviderResults = 50

// maxTimeseriesBuckets caps the number of timeseries buckets returned.
const maxTimeseriesBuckets = 1000

// eventsQueryTimeout is the maximum duration for events queries.
// Prevents pathological wildcard searches from hanging indefinitely on large datasets.
const eventsQueryTimeout = 30 * time.Second

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
		WHERE timestamp > now() - INTERVAL ? DAY
	`, constants.ClickHouseEventsTable)

	row := s.conn.QueryRow(ctx, query, days)
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
		WHERE timestamp > now() - INTERVAL ? DAY
		GROUP BY provider
		ORDER BY connections DESC
		LIMIT ?
	`, constants.ClickHouseEventsTable)

	rows, err := s.conn.Query(ctx, query, days, maxProviderResults)
	if err != nil {
		return nil, fmt.Errorf("query providers: %w", err)
	}
	defer func() { _ = rows.Close() }()

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
	var query string
	var args []any

	if provider != "" {
		query = fmt.Sprintf(`
			SELECT os_username, count() AS connections
			FROM %s
			WHERE timestamp > now() - INTERVAL ? DAY AND provider = ?
			GROUP BY os_username
			ORDER BY connections DESC
		`, constants.ClickHouseEventsTable)
		args = []any{days, provider}
	} else {
		query = fmt.Sprintf(`
			SELECT os_username, count() AS connections
			FROM %s
			WHERE timestamp > now() - INTERVAL ? DAY
			GROUP BY os_username
			ORDER BY connections DESC
		`, constants.ClickHouseEventsTable)
		args = []any{days}
	}

	rows, err := s.conn.Query(ctx, query, args...)
	if err != nil {
		return nil, fmt.Errorf("query users: %w", err)
	}
	defer func() { _ = rows.Close() }()

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
		WHERE timestamp > now() - INTERVAL ? DAY
		GROUP BY ts, provider
		ORDER BY ts
		LIMIT ?
	`, constants.ClickHouseEventsTable)

	rows, err := s.conn.Query(ctx, query, days, maxTimeseriesBuckets)
	if err != nil {
		return nil, fmt.Errorf("query traffic: %w", err)
	}
	defer func() { _ = rows.Close() }()

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
	ctx, cancel := context.WithTimeout(ctx, eventsQueryTimeout)
	defer cancel()

	// Validate sort column to prevent injection (only table names use fmt.Sprintf).
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

	if q != "" {
		return s.getEventsWithSearch(ctx, q, days, sortCol, orderDir, limit, offset)
	}
	return s.getEventsNoSearch(ctx, days, sortCol, orderDir, limit, offset)
}

// getEventsNoSearch handles the common case with no search filter.
func (s *ClickHouseStore) getEventsNoSearch(ctx context.Context, days int, sortCol, orderDir string, limit, offset int) (*EventsResult, error) {
	// Count query.
	countQuery := fmt.Sprintf("SELECT count() FROM %s WHERE timestamp > now() - INTERVAL ? DAY",
		constants.ClickHouseEventsTable)
	var total uint64
	if err := s.conn.QueryRow(ctx, countQuery, days).Scan(&total); err != nil {
		return nil, fmt.Errorf("count events: %w", err)
	}

	// Data query. sortCol and orderDir are validated enum values, not user input.
	dataQuery := fmt.Sprintf(`
		SELECT timestamp, os_username, hostname, provider, provider_host, process_name,
			os_type, detection_method, src_ip, model_hint, process_path, capture_mode
		FROM %s
		WHERE timestamp > now() - INTERVAL ? DAY
		ORDER BY %s %s
		LIMIT ? OFFSET ?
	`, constants.ClickHouseEventsTable, sortCol, orderDir)

	rows, err := s.conn.Query(ctx, dataQuery, days, limit, offset)
	if err != nil {
		return nil, fmt.Errorf("query events: %w", err)
	}
	defer func() { _ = rows.Close() }()

	events := s.scanEventRows(rows)
	return &EventsResult{Events: events, Total: total, Page: (offset / limit) + 1, Limit: limit}, rows.Err()
}

// getEventsWithSearch handles the search filter case using parameterized ILIKE.
func (s *ClickHouseStore) getEventsWithSearch(ctx context.Context, q string, days int, sortCol, orderDir string, limit, offset int) (*EventsResult, error) {
	like := "%" + q + "%"

	// Count query with search.
	countQuery := fmt.Sprintf(`SELECT count() FROM %s WHERE timestamp > now() - INTERVAL ? DAY AND (
		provider ILIKE ? OR provider_host ILIKE ? OR process_name ILIKE ? OR
		hostname ILIKE ? OR os_username ILIKE ? OR src_ip ILIKE ?
	)`, constants.ClickHouseEventsTable)
	var total uint64
	if err := s.conn.QueryRow(ctx, countQuery, days, like, like, like, like, like, like).Scan(&total); err != nil {
		return nil, fmt.Errorf("count events: %w", err)
	}

	// Data query with search. sortCol and orderDir are validated enum values.
	dataQuery := fmt.Sprintf(`
		SELECT timestamp, os_username, hostname, provider, provider_host, process_name,
			os_type, detection_method, src_ip, model_hint, process_path, capture_mode
		FROM %s
		WHERE timestamp > now() - INTERVAL ? DAY AND (
			provider ILIKE ? OR provider_host ILIKE ? OR process_name ILIKE ? OR
			hostname ILIKE ? OR os_username ILIKE ? OR src_ip ILIKE ?
		)
		ORDER BY %s %s
		LIMIT ? OFFSET ?
	`, constants.ClickHouseEventsTable, sortCol, orderDir)

	rows, err := s.conn.Query(ctx, dataQuery, days, like, like, like, like, like, like, limit, offset)
	if err != nil {
		return nil, fmt.Errorf("query events: %w", err)
	}
	defer func() { _ = rows.Close() }()

	events := s.scanEventRows(rows)
	return &EventsResult{Events: events, Total: total, Page: (offset / limit) + 1, Limit: limit}, rows.Err()
}

// scanEventRows scans all rows from an event query into a slice.
func (s *ClickHouseStore) scanEventRows(rows driver.Rows) []EventRow {
	var events []EventRow
	for rows.Next() {
		var e EventRow
		if err := rows.Scan(
			&e.Timestamp, &e.OsUsername, &e.MachineHostname, &e.Provider, &e.ProviderHost,
			&e.ProcessName, &e.OsType, &e.DetectionMethod, &e.SrcIP, &e.ModelHint,
			&e.ProcessPath, &e.CaptureMode,
		); err != nil {
			// Log and skip malformed rows rather than failing the entire query.
			continue
		}
		events = append(events, e)
	}
	return events
}
