# Kubernetes Deployment Guide

Kubernetes manifests are not provided yet, but all AI Ranger backend services are
designed to be k8s-compatible. This document gives future contributors everything
needed to write manifests.

---

## Design Principles

- **Stateless application pods.** Gateway, ingest-worker, and api-server store no
  local state. They can be scaled horizontally and restarted without data loss.
- **Config from environment.** All runtime configuration comes from environment
  variables. Map these to k8s ConfigMaps (non-sensitive) and Secrets (credentials).
- **Health endpoints.** Gateway and api-server expose `GET /health` for readiness
  and liveness probes. Infrastructure services use their native health mechanisms.
- **Graceful shutdown.** All services handle SIGTERM and drain in-flight work
  within `SHUTDOWN_TIMEOUT_SECS` (default 30s). Set `terminationGracePeriodSeconds`
  in pod specs to match.
- **No host filesystem dependencies.** Application images are self-contained.
  Volumes are for data persistence only (Postgres, ClickHouse).

---

## Services

### Gateway (FastAPI)

| Property | Value |
|----------|-------|
| Image | Built from `gateway/Dockerfile` |
| Port | 8080 |
| Health endpoint | `GET /health` → `{"status": "ok", "service": "gateway"}` |
| Readiness probe | `httpGet: { path: /health, port: 8080 }` |
| Liveness probe | Same as readiness |
| Replicas | Stateless — scale as needed |

**Environment variables:**

| Variable | Required | Description |
|----------|----------|-------------|
| `DATABASE_URL` | Yes | SQLAlchemy async Postgres URL (`postgresql+asyncpg://...`) |
| `RABBITMQ_URL` | Yes | AMQP connection URL |
| `ENVIRONMENT` | No | Set to `development` for seed data (default: `production`) |
| `SEED_TOKEN` | No | Plaintext token to seed (only when `ENVIRONMENT=development`) |
| `GATEWAY_PORT` | No | Listen port (default: `8080`) |
| `PROVIDERS_TOML_PATH` | No | Path to providers.toml (default: `providers/providers.toml`) |
| `SHUTDOWN_TIMEOUT_SECS` | No | Graceful shutdown timeout (default: `30`) |

**Startup command:** `sh -c "alembic upgrade head && uvicorn main:app --host 0.0.0.0 --port 8080"`

Only one replica should run Alembic migrations at a time. Use an init container or
a k8s Job for migrations in production.

### Ingest Worker (Go)

| Property | Value |
|----------|-------|
| Image | Built from `workers/Dockerfile` (target: `ingest`) |
| Port | None (no HTTP server) |
| Health endpoint | None (use process liveness check) |
| Liveness probe | Process check — the worker exits on fatal errors |
| Replicas | Stateless — scale to match RabbitMQ queue depth |

**Environment variables:**

| Variable | Required | Description |
|----------|----------|-------------|
| `DATABASE_URL` | Yes | GORM Postgres DSN (`host=... port=... user=...`) |
| `CLICKHOUSE_ADDR` | Yes | ClickHouse native protocol address (`host:9000`) |
| `CLICKHOUSE_DATABASE` | No | ClickHouse database name (default: `default`) |
| `RABBITMQ_URL` | Yes | AMQP connection URL |
| `SHUTDOWN_TIMEOUT_SECS` | No | Graceful shutdown timeout (default: `30`) |

### API Server (Go)

| Property | Value |
|----------|-------|
| Image | Built from `workers/Dockerfile` (target: `api`) |
| Port | 8081 |
| Health endpoint | `GET /health` → `{"status": "ok", "service": "api"}` |
| Readiness probe | `httpGet: { path: /health, port: 8081 }` |
| Liveness probe | Same as readiness |
| Replicas | Stateless — scale as needed |

**Environment variables:**

| Variable | Required | Description |
|----------|----------|-------------|
| `DATABASE_URL` | Yes | GORM Postgres DSN |
| `CLICKHOUSE_ADDR` | Yes | ClickHouse native protocol address |
| `CLICKHOUSE_DATABASE` | No | ClickHouse database name (default: `default`) |
| `API_SERVER_PORT` | No | Listen port (default: `8081`) |
| `SHUTDOWN_TIMEOUT_SECS` | No | Graceful shutdown timeout (default: `30`) |

---

## Infrastructure

### Postgres

Use a managed Postgres service (RDS, Cloud SQL, etc.) or the Bitnami Postgres Helm chart.
The gateway runs Alembic migrations on startup to create/update the schema.

### ClickHouse

Use the official ClickHouse Helm chart or a managed service. Load
`docker/clickhouse/init.sql` as an init script on first boot.

### RabbitMQ

Use the official RabbitMQ Helm chart or a managed service (CloudAMQP, Amazon MQ).
Load `docker/rabbitmq/definitions.json` for exchange/queue topology. Set credentials
via `RABBITMQ_DEFAULT_USER` and `RABBITMQ_DEFAULT_PASS` environment variables.

---

## Secret Management

Map credentials to k8s Secrets:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: ai-ranger-db
type: Opaque
stringData:
  DATABASE_URL: "postgresql+asyncpg://user:pass@postgres:5432/ranger"
  WORKERS_DATABASE_URL: "host=postgres port=5432 user=user password=pass dbname=ranger sslmode=require"

---
apiVersion: v1
kind: Secret
metadata:
  name: ai-ranger-rabbitmq
type: Opaque
stringData:
  RABBITMQ_URL: "amqp://user:pass@rabbitmq:5672/"
```

Reference these in pod specs via `envFrom: [{ secretRef: { name: ai-ranger-db } }]`.
