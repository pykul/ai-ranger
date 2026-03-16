"""Application contract constants for the AI Ranger gateway.

Route paths, queue names, exchange names, header names, and protocol values.
These are application contracts — they do not change between environments.

Runtime configuration (hosts, ports, credentials, timeouts) lives in config.py
and is loaded from environment variables via pydantic-settings.
"""

# -- API route paths -------------------------------------------------------

ROUTE_INGEST = "/v1/ingest"
"""POST: receive protobuf EventBatch from agents."""

ROUTE_ENROLL = "/v1/agents/enroll"
"""POST: enroll a new agent with a token."""

ROUTE_PROVIDERS = "/v1/agents/providers"
"""GET: serve the latest providers.toml."""

ROUTE_HEALTH = "/health"
"""GET: health check endpoint for readiness/liveness probes."""

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
