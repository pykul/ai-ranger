// Package constants defines application contract constants for the AI Ranger workers.
//
// Queue names, table names, route paths, and protocol values live here.
// These are application contracts — they do not change between environments.
//
// Runtime configuration (hosts, ports, credentials, timeouts) lives in
// config.Config and is loaded from environment variables at startup.
package constants

// -- RabbitMQ -----------------------------------------------------------------

// RabbitMQExchange is the direct exchange for event batches from the gateway.
const RabbitMQExchange = "ranger.events"

// RabbitMQQueue is the durable queue that ingest workers consume from.
const RabbitMQQueue = "ranger.ingest"

// RabbitMQRoutingKey is the routing key used by the gateway when publishing.
const RabbitMQRoutingKey = "ingest"

// RabbitMQDLQ is the dead-letter queue for failed messages after retries.
const RabbitMQDLQ = "ranger.dlq"

// -- ClickHouse ---------------------------------------------------------------

// ClickHouseEventsTable is the table name for AI connection events.
const ClickHouseEventsTable = "ai_events"

// ClickHouseBatchSize is the number of events to buffer before flushing to ClickHouse.
// Balances memory usage against insert efficiency.
const ClickHouseBatchSize = 500

// -- API routes ---------------------------------------------------------------

// RouteDashboardOverview returns org-wide summary stats.
const RouteDashboardOverview = "/v1/dashboard/overview"

// RouteDashboardProviders returns provider breakdown with traffic.
const RouteDashboardProviders = "/v1/dashboard/providers"

// RouteDashboardUsers returns per-user activity table.
const RouteDashboardUsers = "/v1/dashboard/users"

// RouteDashboardTraffic returns hourly/daily traffic by provider.
const RouteDashboardTraffic = "/v1/dashboard/traffic/timeseries"

// RouteDashboardFleet returns all enrolled agents and status.
const RouteDashboardFleet = "/v1/dashboard/fleet"

// RouteAdminTokensCreate creates a new enrollment token.
const RouteAdminTokensCreate = "/v1/admin/tokens"

// RouteAdminTokensDelete revokes an enrollment token.
const RouteAdminTokensDelete = "/v1/admin/tokens/{id}"

// RouteAdminAgentsDelete revokes an agent.
const RouteAdminAgentsDelete = "/v1/admin/agents/{id}"

// RouteHealth is the health check endpoint for readiness/liveness probes.
const RouteHealth = "/health"

// -- Retry --------------------------------------------------------------------

// ConsumerPrefetchCount is the number of messages to prefetch from RabbitMQ.
const ConsumerPrefetchCount = 10

// MaxRetries is the number of times to retry a failed operation (e.g. RabbitMQ connect).
const MaxRetries = 10
