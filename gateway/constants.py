"""Named constants for the AI Ranger gateway.

Every route path, queue name, exchange name, and configuration key lives here.
No magic strings anywhere else in the gateway codebase.
"""

# -- API route paths -------------------------------------------------------

ROUTE_INGEST = "/v1/ingest"
"""POST: receive protobuf EventBatch from agents."""

ROUTE_ENROLL = "/v1/agents/enroll"
"""POST: enroll a new agent with a token."""

ROUTE_PROVIDERS = "/v1/agents/providers"
"""GET: serve the latest providers.toml."""

# -- RabbitMQ --------------------------------------------------------------

RABBITMQ_EXCHANGE = "ranger.events"
"""Direct exchange for event batches."""

RABBITMQ_ROUTING_KEY = "ingest"
"""Routing key used when publishing to the exchange."""

# -- HTTP headers ----------------------------------------------------------

AUTH_HEADER = "Authorization"
"""Header carrying the Bearer token."""

AUTH_SCHEME = "Bearer"
"""Expected authentication scheme prefix."""

CONTENT_TYPE_PROTOBUF = "application/x-protobuf"
"""Content-Type for protobuf payloads from the agent."""

# -- Token hashing ---------------------------------------------------------

TOKEN_HASH_ALGORITHM = "sha256"
"""Algorithm used to hash enrollment tokens before storage."""

# -- Providers file --------------------------------------------------------

PROVIDERS_TOML_PATH = "providers/providers.toml"
"""Path to the community provider registry, relative to the repo root."""

# -- Server ----------------------------------------------------------------

GATEWAY_PORT = 8080
"""Default port for the gateway server."""
