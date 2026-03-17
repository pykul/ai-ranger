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

// RouteAuthLogin is the login endpoint for dashboard authentication.
const RouteAuthLogin = "/v1/auth/login"

// RouteAuthRefresh exchanges a refresh token for a new access token.
const RouteAuthRefresh = "/v1/auth/refresh"

// RouteHealth is the health check endpoint for readiness/liveness probes.
const RouteHealth = "/health"

// EnvironmentDevelopment is the value of ENVIRONMENT that disables auth.
const EnvironmentDevelopment = "development"

// -- Agent status -------------------------------------------------------------

// AgentStatusActive is the status of a healthy enrolled agent.
const AgentStatusActive = "active"

// AgentStatusRevoked is the status of a revoked agent that can no longer submit events.
const AgentStatusRevoked = "revoked"

// -- ClickHouse enum values ---------------------------------------------------
// These must match the Enum8 values in docker/clickhouse/init.sql.

// DetectionMethodSNI is the ClickHouse enum value for SNI-based detection.
const DetectionMethodSNI = "sni"

// DetectionMethodDNS is the ClickHouse enum value for DNS-based detection.
const DetectionMethodDNS = "dns"

// DetectionMethodIPRange is the ClickHouse enum value for IP range fallback detection.
const DetectionMethodIPRange = "ip_range"

// DetectionMethodTCPHeuristic is the ClickHouse enum value for TCP heuristic detection (future).
const DetectionMethodTCPHeuristic = "tcp_heuristic"

// CaptureModeDNSSNI is the ClickHouse enum value for passive DNS/SNI capture mode.
const CaptureModeDNSSNI = "dns_sni"

// CaptureModeMITM is the ClickHouse enum value for MITM capture mode (Phase 5+).
const CaptureModeMITM = "mitm"

// -- Retry --------------------------------------------------------------------

// ConsumerPrefetchCount is the number of messages to prefetch from RabbitMQ.
const ConsumerPrefetchCount = 10

// MaxRetries is the number of times to retry a failed operation (e.g. RabbitMQ connect).
const MaxRetries = 10
