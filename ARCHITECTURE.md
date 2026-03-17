# AI Ranger - Architecture Document

> **For Claude Code:** Read this entire document before writing any code. This is the
> single source of truth for all architectural decisions. When in doubt, refer back here.
> Every component has a Makefile. If you are writing a component that does not have one, stop and create it first.

---

## Executive Summary

AI Ranger is a **passive network observability tool** that discovers which AI
models and providers are being used across machines in an organization, without any
TLS interception, MITM proxying, or content inspection.

It works by capturing TLS ClientHello packets (which are sent in plaintext before
encryption is established) and extracting the SNI hostname. This is matched against
a community-maintained registry of known AI provider hostnames. The calling process
is identified via OS APIs. No prompt or response content is ever read.

This is a **pure open source community tool**. Apache-2.0 license. Everything
self-hostable. The agent has zero required dependencies on any external infrastructure.

---

## What This System Does (And Deliberately Does Not Do)

**Does:**
- Detect connections to AI providers via SNI hostname extraction
- Detect connections via DNS query monitoring (fallback/corroboration)
- Identify which process and application is making the call
- Associate activity with a specific enrolled machine and OS user
- Aggregate into per-user, per-provider dashboards
- Support custom output sinks via a plugin/webhook system

**Does NOT (in default DNS/SNI mode):**
- Read prompt or response content
- Require a proxy, CA certificate installation, or MITM setup
- Break any existing tooling or require app reconfiguration
- Contact any external URL by default (zero call-home behavior)
- Require the agent to be connected to any backend to function

---

## How Detection Works (Plain Language)

If you heard "reads network traffic" and tensed up, this section is for you.

When your browser connects to a website over HTTPS, it first sends a small plaintext
greeting to the server called a TLS ClientHello. Tucked inside that greeting is the
destination hostname - for example, `api.anthropic.com`. This field is called the SNI
(Server Name Indication), and it exists so that servers hosting multiple domains know
which certificate to present. It is sent before any encryption is established, and it
is visible to every network device between you and the destination - your router, your
ISP, your corporate firewall. It was never private.

AI Ranger reads that greeting. That is all.

Think of it like a postal worker reading the address on the envelope. The letter inside
is sealed. The postal worker never opens it, never reads it, never knows what it says.
They only see where it is going. AI Ranger works the same way. It sees that a connection
went to `api.anthropic.com`. It does not see the prompt you sent or the response you
received. Those are encrypted and stay encrypted. We never touch them.

This is categorically different from a MITM (man-in-the-middle) proxy, which works by
intercepting the connection, decrypting the traffic, reading the contents, and
re-encrypting it. That approach requires installing a custom certificate on your machine
and actively modifying your traffic. AI Ranger does none of that. There is no certificate,
no proxy, no interception. The agent sits passively and observes metadata that is already
public at the network layer.

What AI Ranger sees:
- Which AI provider was contacted (from the SNI hostname)
- Which process on your machine made the call
- When it happened

What AI Ranger never sees:
- The content of any prompt or message
- Any response from the AI provider
- Anything inside the encrypted connection

---

## Repository Structure (Monorepo)

```
ai-ranger/
├── Makefile                    # Root Makefile - orchestrates everything
│
├── proto/                      # Protobuf definitions - shared across all components
│   ├── Makefile                # Compiles .proto files for all target languages
│   ├── ranger/
│   │   └── v1/
│   │       ├── events.proto    # AiConnectionEvent, EventBatch
│   │       └── agent.proto     # EnrollmentRequest, EnrollmentResponse
│   └── gen/                    # Generated code - committed to repo
│       ├── rust/               # Generated Rust structs (prost)
│       ├── python/             # Generated Python classes (protoc --python_out)
│       └── go/                 # Generated Go structs (protoc-gen-go)
│
├── agent/                      # Rust - the on-machine capture agent
│   ├── Makefile
│   ├── src/
│   │   ├── main.rs             # Thin wiring - CLI, config, task spawning, shutdown
│   │   ├── event.rs            # AiConnectionEvent struct + constructor
│   │   ├── config.rs           # config.toml parsing (AppConfig, AgentSection, OutputConfig)
│   │   ├── pipeline.rs         # Packet-to-event transformation (classify, resolve, construct)
│   │   ├── dedup.rs            # connection_id hashing + DedupCache
│   │   ├── capture/
│   │   │   ├── mod.rs
│   │   │   ├── constants.rs    # Shared protocol constants (TLS, DNS, IP, Ethernet)
│   │   │   ├── sni.rs          # TLS ClientHello parser, SNI extractor
│   │   │   ├── dns.rs          # DNS response parser
│   │   │   ├── pcap.rs         # OS-native raw sockets (AF_PACKET on Linux, BPF on macOS, SIO_RCVALL + ETW on Windows)
│   │   │   ├── etw_dns.rs      # Windows ETW DNS-Client provider (IPv6 + DNS hostname capture)
│   │   │   └── mitm/           # DO NOT IMPLEMENT - Phase 5+ only
│   │   │       └── mod.rs      # Stub file with a single comment explaining scope
│   │   ├── process/
│   │   │   └── mod.rs          # pid -> process name, per OS (Linux, macOS, Windows)
│   │   ├── classifier/
│   │   │   ├── mod.rs          # Re-exports + fetch_providers_url
│   │   │   └── providers.rs    # Provider registry loader and matcher
│   │   ├── output/
│   │   │   ├── mod.rs          # build_sinks() + module declarations
│   │   │   ├── sink.rs         # EventSink trait definition
│   │   │   ├── stdout.rs       # Default output (no config needed)
│   │   │   ├── file.rs         # JSON-lines file output
│   │   │   ├── http.rs         # POST JSON batches to backend gateway
│   │   │   ├── webhook.rs      # Custom webhook sink
│   │   │   └── fanout.rs       # Fan events to multiple sinks concurrently
│   │   ├── identity/
│   │   │   ├── mod.rs
│   │   │   ├── config.rs       # AgentConfig, machine metadata, OS config paths
│   │   │   └── enroll.rs       # Enrollment flow + identity loading
│   │   └── buffer/
│   │       ├── mod.rs
│   │       ├── store.rs        # SQLite local event buffer (http mode only)
│   │       └── drain.rs        # Background drain loop with exponential backoff
│   └── Cargo.toml
│
├── gateway/                    # Python + FastAPI - thin agent-facing gateway
│   ├── Makefile
│   ├── main.py                # FastAPI app instance + startup
│   ├── database.py            # Async SQLAlchemy engine and session factory
│   ├── dependencies.py        # Shared FastAPI dependencies (auth, DB session, queue)
│   ├── routers/
│   │   ├── ingest.py          # POST /v1/ingest - receive agent batches
│   │   ├── enroll.py          # POST /v1/agents/enroll
│   │   └── providers.py       # GET /v1/agents/providers
│   ├── models/                # SQLAlchemy ORM models + Pydantic request/response schemas
│   │   ├── orm.py             # SQLAlchemy models (Organization, EnrollmentToken, Agent)
│   │   ├── events.py          # Pydantic schemas for event-related requests/responses
│   │   └── enrollment.py      # Pydantic schemas for enrollment requests/responses
│   ├── alembic/               # Versioned Postgres migrations (source of truth for schema)
│   │   ├── env.py
│   │   └── versions/          # Migration scripts generated by `alembic revision`
│   ├── alembic.ini
│   ├── queue.py               # RabbitMQ publisher (aio-pika)
│   ├── proto_utils.py         # Protobuf deserialize helpers
│   ├── constants.py           # Named constants (routes, queue names, ports)
│   ├── proto/                 # Symlink to proto/gen/python
│   └── requirements.txt
│
├── workers/                    # Go - async processing and query API
│   ├── Makefile
│   ├── Dockerfile.dev          # Dev image with CompileDaemon for hot reload
│   ├── cmd/
│   │   ├── ingest/
│   │   │   ├── main.go         # Ingest worker binary entry point
│   │   │   └── Dockerfile      # Produces a single-binary image for the ingest worker
│   │   └── api/
│   │       ├── main.go         # Query API binary entry point
│   │       └── Dockerfile      # Produces a single-binary image for the API server
│   ├── internal/
│   │   ├── models/
│   │   │   └── models.go       # GORM structs mirroring gateway SQLAlchemy models
│   │   ├── consumer/
│   │   │   └── rabbitmq.go     # RabbitMQ consumer, worker pool
│   │   ├── writer/
│   │   │   ├── clickhouse.go   # Batch write events to ClickHouse (plain SQL, no ORM)
│   │   │   └── postgres.go     # Update agent last_seen via GORM
│   │   ├── api/
│   │   │   ├── router.go       # Chi router setup
│   │   │   ├── dashboard.go    # Dashboard query handlers
│   │   │   ├── fleet.go        # Fleet management handlers
│   │   │   └── tokens.go       # Token management handlers
│   │   └── store/
│   │       ├── clickhouse.go   # ClickHouse query helpers (plain SQL via clickhouse-go)
│   │       └── postgres.go     # Postgres query helpers via GORM
│   ├── proto/                  # Symlink to proto/gen/go
│   └── go.mod
│
├── dashboard/                  # React + TypeScript dashboard
│   ├── Makefile
│   ├── Dockerfile              # Multi-stage: node build → nginx serve static
│   ├── nginx.conf              # SPA fallback for client-side routing
│   ├── src/
│   │   ├── pages/
│   │   │   ├── Overview.tsx        # Dashboard: stats, chart, ranked lists
│   │   │   ├── Events.tsx          # Raw event search with pagination
│   │   │   ├── Admin.tsx           # Fleet + token management (tabs)
│   │   │   └── Login.tsx           # Login page (production only)
│   │   ├── layouts/
│   │   │   └── DashboardLayout.tsx # Sidebar (Dashboard, Events, Admin) + time range
│   │   ├── lib/                    # API client, auth, formatting, types
│   │   ├── hooks/                  # Auth, dashboard data, events, time range
│   │   └── components/             # TimeRangeSelector
│   └── package.json
│
├── providers/
│   └── providers.toml          # THE community-maintained provider registry
│
├── docker/
│   ├── Makefile                # Targets for bring-up, teardown, logs, reset
│   ├── docker-compose.yml      # Base stack (all services)
│   ├── docker-compose.dev.yml  # Dev overrides (source mounts, hot reload)
│   ├── docker-compose.prod.yml # Production overrides (TLS, no direct ports)
│   ├── nginx/
│   │   ├── nginx.dev.conf      # Dev routing (port 8000, no TLS)
│   │   └── nginx.prod.conf     # Production routing (port 443, TLS)
│   ├── clickhouse/
│   │   └── init.sql            # ClickHouse schema (plain SQL - no ORM for ClickHouse)
│   └── rabbitmq/
│       └── definitions.json    # Pre-configured queues and exchanges
│
├── docs/                       # Docusaurus (Phase 4+)
│   └── docs/
│       ├── getting-started.md
│       ├── agent/
│       ├── backend/
│       ├── self-hosting.md
│       └── contributing-providers.md
│
└── .github/
    ├── workflows/
    │   ├── ci.yml              # Test + lint on every PR
    │   └── release.yml         # Build agent binaries for all targets on tag
    └── ISSUE_TEMPLATE/
        ├── bug_report.md
        ├── provider_request.md # Most common issue type
        └── feature_request.md
```

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        USER MACHINE                              │
│                                                                  │
│  ┌────────────────────────────────────────────────────────┐    │
│  │                   Rust Agent                            │    │
│  │                                                         │    │
│  │  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐ │    │
│  │  │   Packet    │  │   Process    │  │   Identity &  │ │    │
│  │  │   Capture   │  │   Resolver   │  │   Config      │ │    │
│  │  │ (SNI + DNS) │  │ (pid->name)  │  │   (optional)  │ │    │
│  │  └──────┬──────┘  └──────┬───────┘  └───────┬───────┘ │    │
│  │         └────────────────┴──────────────────┘         │    │
│  │                          │                             │    │
│  │                ┌─────────▼─────────┐                  │    │
│  │                │    Classifier      │                  │    │
│  │                │  (providers.toml)  │                  │    │
│  │                └─────────┬─────────┘                  │    │
│  │                          │                             │    │
│  │                ┌─────────▼─────────┐                  │    │
│  │                │   EventSink        │                  │    │
│  │                │   Fan-out          │                  │    │
│  │                └──┬──────┬──────┬──┘                  │    │
│  │                   │      │      │                      │    │
│  │              stdout    file   http (protobuf)          │    │
│  └────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
         │                                     │
         │ (once) POST /v1/agents/enroll       │ (ongoing) POST /v1/ingest
         │ JSON + enrollment token             │ protobuf EventBatch
         │                                     │ Bearer: <agent_id>
         ▼                                     ▼
┌─────────────────────────────────────────────────────────────────┐
│              FastAPI Gateway (Python)                            │
│                                                                  │
│  POST /v1/agents/enroll  →  validate token, write to Postgres,  │
│                              return org_id + agent_id (sync)     │
│                              No RabbitMQ.                        │
│                                                                  │
│  POST /v1/ingest         →  verify Bearer, deserialize protobuf,│
│                              publish to RabbitMQ, return 200.    │
│                              No DB writes.                       │
│                                                                  │
│  Uvicorn + async workers for concurrency                        │
│  Swagger UI at /docs (auto-generated from Pydantic models)      │
└───────────────┬─────────────────────────────┬───────────────────┘
                │ enrollment                   │ ingest
                ▼                              ▼
           Postgres                        RabbitMQ
         (direct write)                 (ranger.events)
                                               │
                                               ▼
                                   ┌──────────────────────┐
                                   │  Go Ingest Worker     │
                                   │                       │
                                   │  Consume from queue   │
                                   │  Write events to CH   │
                                   │  Update PG last_seen  │
                                   │  (independently)      │
                                   └─────┬───────────┬─────┘
                                         │           │
                          (independent)  │           │  (independent)
                                         ▼           ▼
                               ┌────────────┐ ┌────────────┐
                               │ ClickHouse │ │  Postgres  │
                               │  (events)  │ │ (identity) │
                               └─────┬──────┘ └──────┬─────┘
                                     │               │
                                     ▼               ▼
                               ┌────────────────────────────┐
                               │      Go Query API          │
                               │                            │
                               │  Reads ClickHouse (events) │
                               │  Reads Postgres (fleet)    │
                               │  No gateway, no RabbitMQ   │
                               └──────────────┬─────────────┘
                                              │
                                              ▼
                                   ┌────────────────────┐
                                   │  Dashboard (React)  │
                                   │  talks to Go API    │
                                   │  only               │
                                   └────────────────────┘
```

---

## nginx - Single Ingress Point

All external traffic enters through nginx. Internal services are not exposed
directly in production. In development, direct ports remain available for
debugging and integration tests.

### Routing table

| Path prefix | Upstream | Strip prefix | Description |
|---|---|---|---|
| `/` | `dashboard:3000` | No | React SPA (static files + SPA fallback) |
| `/api/` | `api-server:8081` | Yes (strip `/api`) | Go Query API (dashboard data, auth) |
| `/ingest/` | `gateway:8080` | Yes (strip `/ingest`) | FastAPI gateway (agent enrollment + ingest) |

In production, agents point their `--backend` flag to `https://yourdomain.com/ingest`
instead of directly to the gateway port. nginx handles TLS termination.

### nginx configuration files

| File | Environment | Description |
|---|---|---|
| `docker/nginx/nginx.dev.conf` | Development | Port 8000, no TLS |
| `docker/nginx/nginx.prod.conf` | Production | Port 443 with TLS, port 80 redirects to 443 |
| `dashboard/nginx.conf` | Both | Dashboard container internal config (SPA fallback) |

---

## Backend Language Split - Rules

This boundary must stay clean. If it drifts, the architecture falls apart.

**FastAPI Gateway (Python) is responsible for:**
- Receiving HTTP requests from agents
- Verifying Bearer tokens
- Deserializing protobuf payloads
- Publishing messages to RabbitMQ
- Responding to the agent

That is all the gateway does. No exceptions. If you find yourself writing a database
query, a ClickHouse insert, a data aggregation, or any business logic inside
a FastAPI route, stop. That code belongs in the Go workers.

**Go Workers are responsible for:**
- Consuming messages from RabbitMQ
- Writing events to ClickHouse in batches
- Updating agent metadata in Postgres
- Serving all dashboard and fleet management API endpoints
- Any future async processing (enrichment, alerting)

The dashboard talks to Go only. It never talks to the FastAPI gateway directly.

---

## Wire Format - Protobuf

All communication between the agent and the gateway uses protobuf over HTTPS.
The `.proto` files are the contract between all components. A change to a `.proto`
file requires running `make proto` and committing the regenerated code before
any other work proceeds.

```protobuf
// proto/ranger/v1/events.proto
syntax = "proto3";
package ranger.v1;

enum DetectionMethod {
  SNI = 0;
  DNS = 1;
  IP_RANGE = 2;       // Fallback: matched destination IP against provider CIDR ranges
  TCP_HEURISTIC = 3;
}

enum CaptureMode {
  DNS_SNI = 0;
  MITM = 1;  // Phase 5+, reserved - do not use
}

message AiConnectionEvent {
  // Identity
  string agent_id = 1;
  string machine_hostname = 2;
  string os_username = 3;
  string os_type = 24;  // "linux", "macos", or "windows" - compile-time constant

  // Timing
  int64 timestamp_ms = 4;
  optional uint64 duration_ms = 5;

  // Provider
  string provider = 6;
  string provider_host = 7;
  optional string model_hint = 8;

  // Process
  string process_name = 9;
  uint32 process_pid = 10;
  optional string process_path = 11;

  // Dedup
  string connection_id = 23;  // hash of (src_ip, provider_host, timestamp_ms / 2000)

  // Detection
  DetectionMethod detection_method = 14;
  CaptureMode capture_mode = 15;

  // Network
  string src_ip = 22;  // source IP of the connection

  // Phase 5 - MITM only. Do not populate these fields until Phase 5.
  bool content_available = 16;
  optional string payload_ref = 17;
  optional string model_exact = 18;
  optional uint32 token_count_input = 19;
  optional uint32 token_count_output = 20;
  optional uint32 latency_ttfb_ms = 21;
}

message EventBatch {
  string agent_id = 1;
  int64 sent_at_ms = 2;
  repeated AiConnectionEvent events = 3;
}
```

```protobuf
// proto/ranger/v1/agent.proto
syntax = "proto3";
package ranger.v1;

message EnrollmentRequest {
  string token = 1;
  string agent_id = 2;
  string hostname = 3;
  string os_username = 4;
  string os = 5;
  string agent_version = 6;
}

message EnrollmentResponse {
  string org_id = 1;
  string agent_id = 2;
}
```

Generated code lives in `proto/gen/` and is committed to the repo. Contributors
should not need to install protoc to work on the project. `make proto` regenerates
everything if the `.proto` files change.

---

## Core Data Structures

### `AiConnectionEvent` - the fundamental unit of data

> **For Claude Code:** Implement all fields marked `// Phase 1`. Fields marked
> `// Phase 5 - MITM only` must be defined in the struct (so the data model is
> future-proof) but always set to their default/None values. Do not wire them
> to any capture logic yet.

```rust
// Use the prost-generated types from proto/gen/rust/ directly.
// This is shown here for documentation clarity only.

pub struct AiConnectionEvent {
    // Identity - Phase 1 (Phase 0 leaves these as empty string)
    pub agent_id: String,
    pub machine_hostname: String,
    pub os_username: String,
    pub os_type: String,                // "linux", "macos", or "windows" - from std::env::consts::OS

    // Dedup
    pub connection_id: String,          // Hash of (src_ip, provider_host, timestamp_ms / 2000). Omitted from JSON when empty.

    // Timing
    pub timestamp_ms: i64,              // Phase 0
    pub duration_ms: Option<u64>,       // Deferred - requires TCP session lifecycle tracking. See DECISIONS.md

    // Provider
    pub provider: String,               // Phase 0 - "anthropic", "openai" ...
    pub provider_host: String,          // Phase 0 - raw SNI e.g. "api.anthropic.com"
    pub model_hint: Option<String>,     // Phase 5 - populated from request body in MITM mode

    // Process
    pub process_name: String,           // "unknown" if process exited before lookup
    pub process_pid: u32,               // Phase 0
    pub process_path: Option<String>,   // Phase 1

    // Network
    pub src_ip: String,                 // Phase 0 - source IP of the connection

    // Detection
    pub detection_method: DetectionMethod,  // Phase 0
    pub capture_mode: CaptureMode,          // Phase 0 - always DnsSni until Phase 5

    // Phase 5 - MITM only. Always default/None until Phase 5.
    pub content_available: bool,
    pub payload_ref: Option<String>,
    pub model_exact: Option<String>,
    pub token_count_input: Option<u32>,
    pub token_count_output: Option<u32>,
    pub latency_ttfb_ms: Option<u32>,
}
```

### `AgentConfig` - enrollment and identity

```rust
pub struct AgentConfig {
    pub agent_id: String,           // generated once at enrollment, never changes
    pub org_id: String,             // returned by backend at enrollment
    pub backend_url: String,
    pub machine_hostname: String,
    pub os_username: String,
    pub enrolled_at: i64,           // unix ms
}
```

The enrollment token is passed via `--token` and `--backend` on the command line.
Two enrollment modes:
- `ai-ranger --token=tok_abc --backend=http://...` — enroll and start capturing in one step.
  If already enrolled, the flags are ignored and the saved identity is used.
- `ai-ranger --enroll --token=tok_abc --backend=http://...` — enroll and exit without
  capturing. Used by installer scripts that start the daemon separately.

The token is consumed during enrollment and not persisted in `AgentConfig` — after
enrollment completes, the agent uses `agent_id` for all subsequent authentication.

---

## The `EventSink` Trait - Output Abstraction

Every output destination implements this single trait. This is what makes custom
telemetry possible without modifying core agent code.

```rust
#[async_trait]
pub trait EventSink: Send + Sync {
    async fn send(&self, event: &AiConnectionEvent) -> Result<()>;
    async fn flush(&self) -> Result<()> { Ok(()) }  // optional, for batched sinks
    async fn close(&self) -> Result<()> { Ok(()) }
}
```

**Built-in implementations (ship with agent):**
- `StdoutSink` - default, no config required
- `FileSink` - write JSON lines to a file
- `HttpSink` - POST protobuf batches to the gateway
- `WebhookSink` - POST JSON to any arbitrary URL with configurable headers
- `FanoutSink` - wraps multiple sinks, sends to all concurrently

**Future (deferred until community demand):**
- `WasmPluginSink` - load a `.wasm` plugin for custom logic (uses `wasmtime`)

---

## Agent Capture Modes

> **For Claude Code:** In Phases 0-4, implement DNS/SNI mode only. The MITM mode
> section below is architectural documentation for future reference. Do not implement
> any code in `capture/mitm/` until explicitly told to start Phase 5. If you find
> yourself writing a proxy, cert generation, or HTTP/2 parser, stop - you are out of scope.

### Mode 1: DNS/SNI (Default - Phases 0-4)

```
ai-ranger                        # default, DNS/SNI mode
ai-ranger --mode dns-sni         # explicit
```

- Passive packet capture only
- Extracts SNI from TLS ClientHello (plaintext, no decryption)
- DNS query monitoring as fallback
- No CA cert, no proxy, no app configuration changes
- `capture_mode` on all events: `DNS_SNI`
- `content_available` always `false`

### Mode 2: MITM (Opt-in - Phase 5+, NOT YET IMPLEMENTED)

```
ai-ranger --mode mitm            # requires completed cert installation flow
```

**This mode does not exist yet. It is planned for Phase 5.**

When it is eventually built, it will:
- Generate a local CA certificate on first run
- Guide the user through OS trust store installation (separate flow per OS)
- Intercept outbound port 443 connections to known AI provider hostnames only
- Terminate TLS, inspect plaintext HTTP/2 payload, re-encrypt outbound
- Extract exact model name, token counts, latency, error rates
- Store content references in object storage (S3/R2), not inline in events
- Set `capture_mode: MITM` and `content_available: true` on events
- Never intercept connections to non-AI-provider hostnames

**Known hard problems for Phase 5 (documented so they are not forgotten):**
- Cert pinning: Cursor and some Electron apps will reject the local CA and break
- HTTP/2 + SSE streaming: chunked responses must be buffered and reassembled
- Storage: content payloads need a separate object store, not ClickHouse
- PII exposure: prompt content may contain sensitive data, needs explicit user consent
- The trust story changes: README and docs need a clear two-mode explanation

---

## Agent Configuration (`config.toml`)

```toml
[agent]
# Capture mode: "dns-sni" (default) or "mitm" (Phase 5+, NOT YET IMPLEMENTED)
# Do not implement mitm support - this field is a placeholder for future use only
mode = "dns-sni"

# URL to fetch fresh providers.toml on startup (optional)
# If not set, uses the bundled providers.toml
providers_url = "https://raw.githubusercontent.com/pykul/ai-ranger/main/providers/providers.toml"

# ── Tuning parameters (all optional, defaults shown) ──────────────────────
# drain_interval_secs = 30          # How often the SQLite buffer uploads to the backend (seconds)
# drain_batch_size = 500            # Maximum events per upload batch
# http_batch_size = 100             # Maximum events buffered per HTTP sink flush
# webhook_batch_size = 100          # Default maximum events buffered per webhook sink flush
# providers_fetch_timeout_secs = 10 # Timeout for fetching providers.toml from URL (seconds)

# Multiple outputs supported - events fan out to all of them
[[outputs]]
type = "stdout"     # default, always works with zero config

[[outputs]]
type = "http"
url = "http://localhost:8080"   # use https:// in production
# Authentication uses agent_id as Bearer token, set during enrollment.
# No token field needed here - the agent authenticates with the identity
# established by `ai-ranger --token=... --backend=...`.

[[outputs]]
type = "webhook"
url = "https://http-intake.logs.datadoghq.com/api/v2/logs"
headers = { "DD-API-KEY" = "your-key", "Content-Type" = "application/json" }
batch_size = 100

# Future (Phase 5+):
# [[outputs]]
# type = "plugin"
# path = "/etc/ai-ranger/plugins/custom.wasm"
```

### Webhook sink payload

When the webhook sink fires, it POSTs a JSON array of events to the configured URL.
Each element in the array is a serialized `AiConnectionEvent`. The `Content-Type`
header is `application/json` unless overridden in the config.

Example payload:

```json
[
  {
    "agent_id": "3f2a1b4c-...",
    "machine_hostname": "alices-macbook",
    "os_username": "alice",
    "os_type": "macos",
    "connection_id": "a1b2c3d4e5f67890",
    "timestamp_ms": 1773506947460,
    "provider": "anthropic",
    "provider_host": "api.anthropic.com",
    "process_name": "claude",
    "process_pid": 1867,
    "src_ip": "172.27.151.106",
    "detection_method": "SNI",
    "capture_mode": "DNS_SNI"
  }
]
```

Optional fields (`duration_ms`, `model_hint`, `process_path`) are omitted from the JSON
when null. Phase 5 fields (`content_available`, `payload_ref`, `model_exact`,
`token_count_input`, `token_count_output`, `latency_ttfb_ms`) are omitted entirely in
the current version. They will appear when populated in MITM mode.

The `batch_size` config key controls the maximum number of events per POST.
If not set, the default is 100.

---

## Provider Registry (`providers/providers.toml`)

```toml
# CONTRIBUTING: To add a provider, open a PR adding an entry below.
# Required fields: name, display_name, hostnames
# Optional: ip_ranges - CIDR ranges for providers with dedicated IP space.
#   Used as a last-resort fallback when both SNI and DNS detection fail
#   (e.g. applications using ECH+DoH, currently primarily browsers). Only add ip_ranges for providers
#   with dedicated IPs. Do NOT add ranges for CDN-backed providers - shared
#   IPs cause false positives.
# Please include a source link (docs_url) for any hostname you add.

[[providers]]
name = "anthropic"
display_name = "Anthropic / Claude"
hostnames = ["api.anthropic.com", "claude.ai"]
ip_ranges = ["160.79.104.0/23", "2607:6bc0::/48"]
docs_url = "https://docs.anthropic.com"

[[providers]]
name = "openai"
display_name = "OpenAI"
hostnames = ["api.openai.com", "chat.openai.com", "chatgpt.com"]
docs_url = "https://platform.openai.com/docs"

[[providers]]
name = "cursor"
display_name = "Cursor"
hostnames = ["api2.cursor.sh", "repo.cursor.sh"]

[[providers]]
name = "github_copilot"
display_name = "GitHub Copilot"
hostnames = ["copilot-proxy.githubusercontent.com", "githubcopilot.com"]

[[providers]]
name = "google_gemini"
display_name = "Google Gemini"
hostnames = ["generativelanguage.googleapis.com", "aistudio.google.com"]
docs_url = "https://ai.google.dev/docs"

[[providers]]
name = "mistral"
display_name = "Mistral"
hostnames = ["api.mistral.ai"]
docs_url = "https://docs.mistral.ai"

[[providers]]
name = "cohere"
display_name = "Cohere"
hostnames = ["api.cohere.ai", "api.cohere.com"]
docs_url = "https://docs.cohere.com"

[[providers]]
name = "huggingface"
display_name = "Hugging Face"
hostnames = ["api-inference.huggingface.co", "huggingface.co"]
docs_url = "https://huggingface.co/docs"

[[providers]]
name = "replicate"
display_name = "Replicate"
hostnames = ["api.replicate.com"]
docs_url = "https://replicate.com/docs"

[[providers]]
name = "together"
display_name = "Together AI"
hostnames = ["api.together.xyz"]
docs_url = "https://docs.together.ai"

[[providers]]
name = "perplexity"
display_name = "Perplexity"
hostnames = ["api.perplexity.ai", "www.perplexity.ai"]
docs_url = "https://docs.perplexity.ai"

[[providers]]
name = "deepseek"
display_name = "DeepSeek"
hostnames = ["api.deepseek.com", "chat.deepseek.com"]

[[providers]]
name = "xai"
display_name = "xAI / Grok"
hostnames = ["api.x.ai"]

[[providers]]
name = "ai21"
display_name = "AI21 Labs"
hostnames = ["api.ai21.com"]
docs_url = "https://docs.ai21.com"

[[providers]]
name = "amazon_bedrock"
display_name = "Amazon Bedrock"
hostnames = ["bedrock-runtime.us-east-1.amazonaws.com", "bedrock-runtime.us-west-2.amazonaws.com", "bedrock-runtime.eu-west-1.amazonaws.com", "bedrock.us-east-1.amazonaws.com", "bedrock.us-west-2.amazonaws.com"]
docs_url = "https://docs.aws.amazon.com/bedrock"

[[providers]]
name = "azure_openai"
display_name = "Azure OpenAI"
hostnames = ["openai.azure.com"]
docs_url = "https://learn.microsoft.com/en-us/azure/ai-services/openai"

[[providers]]
name = "stability"
display_name = "Stability AI"
hostnames = ["api.stability.ai"]
docs_url = "https://platform.stability.ai/docs"

[[providers]]
name = "ollama"
display_name = "Ollama (Local)"
hostnames = ["localhost"]
ports = [11434]       # parsed but not yet implemented in the agent
tls = false           # parsed but not yet implemented in the agent
```

---

## SNI Parser - Core Algorithm

The TLS ClientHello is sent entirely in plaintext before encryption begins.
Extracting the SNI field requires zero cryptography - it is pure byte parsing.

```
TCP SYN ->
TLS ClientHello -> [SNI: "api.anthropic.com"]  <- readable, plaintext
                -> [Encrypted session data]     <- not read, not needed
```

The parser reads:
1. TLS record header (type byte must be `0x16` = handshake)
2. Handshake header (type byte must be `0x01` = ClientHello)
3. Skip: version, random, session ID, cipher suites, compression methods
4. Walk extensions until `type == 0x0000` (SNI extension)
5. Extract the hostname string

This is approximately 80 lines of Rust with no external C library dependencies.

---

## Packet Capture Backend - Per OS

> **This is a hard requirement:** the agent binary must be fully standalone with zero
> external C library dependencies. The `pcap` crate is explicitly forbidden because it
> requires libpcap on Linux/macOS and npcap on Windows to be installed separately on
> the target machine. Use the approaches below instead.

The capture layer in `capture/pcap.rs` uses OS-native APIs via conditional compilation.
The SNI parser in `capture/sni.rs` receives raw bytes and does not care how they arrived.
Only the capture backend is OS-specific.

| OS      | Capture method | Rust approach | Notes |
|---------|---------------|---------------|-------|
| Linux   | `AF_PACKET` raw socket (IPv4 + IPv6) | direct `libc` syscalls + BPF filter | No external deps, requires root |
| macOS   | BPF device (`/dev/bpf*`) (IPv4 + IPv6) | direct `libc` syscalls | No external deps, requires root |
| Windows | `SIO_RCVALL` raw socket (IPv4 packets) + ETW `Microsoft-Windows-DNS-Client` (DNS resolutions) | `winapi` + `ferrisetw` crate | Dual-path: IPv4 packets via raw socket, DNS hostnames via ETW. Requires Administrator |

The `pnet` crate is an alternative to `socket2` for Linux and macOS if direct BPF/AF_PACKET
access proves complex. It abstracts raw sockets without requiring libpcap. Either is acceptable
as long as no external C library installation is required on the target machine.

**Conditional compilation structure in `capture/pcap.rs`:**

```rust
#[cfg(target_os = "linux")]
mod platform {
    // AF_PACKET raw socket implementation
}

#[cfg(target_os = "macos")]
mod platform {
    // BPF device implementation
}

#[cfg(target_os = "windows")]
mod platform {
    // SIO_RCVALL raw socket for IPv4 packet capture
}

// capture/etw_dns.rs (Windows only):
// ETW Microsoft-Windows-DNS-Client for DNS resolution monitoring
// Covers IPv6 connections that SIO_RCVALL cannot see

pub use platform::capture_packets;
```

**Why not npcap/libpcap:**
npcap requires a separate driver installation on Windows and its free license prohibits
static linking, making redistribution without installer complexity impossible. libpcap
on Linux/macOS is often already present but cannot be assumed. Both break the standalone
binary requirement. Do not use them.



Mapping a network connection back to the process that made it is OS-specific.

| OS      | Primary method                          | Rust crate/API          |
|---------|-----------------------------------------|-------------------------|
| Linux   | `/proc/net/tcp` + `/proc/<pid>/fd`      | `procfs` crate          |
| macOS   | `proc_pidinfo(PROC_PIDLISTFDS)`         | `libproc` bindings      |
| Windows | `GetExtendedTcpTable` from iphlpapi.dll | `windows` crate         |

The `sysinfo` crate covers most of this cross-platform. Use it first, fall back to
OS-specific APIs for edge cases.

---

## Deployment & Identity

### Token Lifecycle

```
1. Admin opens dashboard -> creates enrollment token
   Platform generates:  tok_<random_32_bytes>
   Stores:              { token_hash, org_id, created_by, expires_at, max_uses }

2. Admin shares token with user or machine

3. User runs installer with token:
   curl -sSL https://your-instance.com/install.sh | sh -s -- --token=tok_abc123

4. Installer:
   a. Downloads correct binary for OS/arch from GitHub Releases
   b. Verifies SHA256 checksum
   c. Runs: ai-ranger --token=tok_abc123 --backend=https://your-instance.com
   d. Backend validates token -> creates agent record -> returns org_id
   e. Agent stores agent_id + config locally
   f. Token use_count++ (invalidated if single-use)
   g. Installs as system daemon

5. Agent uses agent_id (not token) for all subsequent requests
```

### Config file locations (post-enrollment)

| OS      | Path                                                   |
|---------|--------------------------------------------------------|
| macOS   | `~/Library/Application Support/ai-ranger/config.json` |
| Linux   | `~/.config/ai-ranger/config.json`                      |
| Windows | `%APPDATA%\ai-ranger\config.json`                      |

### Daemon installation

| OS      | Mechanism                                           |
|---------|-----------------------------------------------------|
| macOS   | `/Library/LaunchDaemons/com.ai-ranger.agent.plist`  |
| Linux   | `/etc/systemd/system/ai-ranger.service`             |
| Windows | Windows Service via `sc.exe`                        |

---

## Storage Design

Two databases. Different workloads, different tools.

### Postgres - identity and configuration

The Postgres schema is managed by **SQLAlchemy 2.0 async models** in `gateway/models/orm.py`
with **Alembic** handling versioned migrations in `gateway/alembic/versions/`. There is no
`init.sql` for Postgres -- `alembic upgrade head` runs on gateway container startup and
creates or migrates tables automatically.

Three tables:

- **organizations** - id (UUID PK), name, slug (unique), created_at
- **enrollment_tokens** - id (UUID PK), org_id (FK -> organizations), token_hash (SHA256, unique), label, created_by, expires_at, max_uses, used_count, created_at
- **agents** - id (UUID PK, generated on device), org_id (FK -> organizations), hostname, os_username, os, agent_version, enrolled_at, last_seen_at, status

The SQLAlchemy models in `gateway/models/orm.py` are the **source of truth** for the Postgres
schema. The GORM structs in `workers/internal/models/models.go` must mirror them exactly.
Any Alembic migration that adds or changes a column must be accompanied by the corresponding
GORM struct update in the same commit.

### ClickHouse - events and timeseries

> **No ORM for ClickHouse.** No mature ORM exists for ClickHouse in Go. ClickHouse access
> uses the `clickhouse-go` driver with plain SQL and named query constants. This is an
> intentional exception to the ORM rule. The schema is defined in `docker/clickhouse/init.sql`
> and loaded on container startup.

```sql
CREATE TABLE ai_events (
    org_id          UUID,
    agent_id        UUID,
    hostname        String,
    os_username     LowCardinality(String),
    os_type         LowCardinality(String),
    timestamp       DateTime64(3, 'UTC'),
    provider        LowCardinality(String),
    provider_host   String,
    model_hint      LowCardinality(String),
    process_name    LowCardinality(String),
    process_path    String,
    src_ip          String,
    detection_method Enum8('sni'=1, 'dns'=2, 'ip_range'=3, 'tcp_heuristic'=4),
    capture_mode    Enum8('dns_sni'=1, 'mitm'=2)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (org_id, timestamp, agent_id, provider)
TTL timestamp + INTERVAL 1 YEAR;
```

**Schema changes require volume recreation.** ClickHouse loads `init.sql` only on
first container start. To apply schema changes, destroy the volume and restart:

```bash
make dev-reset    # tears down all volumes, restarts with fresh schemas
```

### RabbitMQ - event queue

```
Exchange:   ranger.events  (type: direct, durable: true)
Queue:      ranger.ingest  (durable: true, dead-letter -> ranger.dlq)
Queue:      ranger.dlq     (dead letter queue for failed events)
```

The gateway publishes raw protobuf bytes to `ranger.events`.
Go ingest workers consume from `ranger.ingest` in a goroutine pool.
Failed messages after retries go to `ranger.dlq` for inspection.

Pre-configured via `docker/rabbitmq/definitions.json` so no manual setup is needed.
The management UI at `localhost:15672` lets contributors inspect queue depth and
dead letters without any extra tooling.

---

## Makefile Structure

Every component has its own Makefile. The root Makefile ties them together.
This is a hard requirement. Every operation a developer might need must be
reachable via `make <target>` without reading documentation first.

### Root `Makefile`

```makefile
.PHONY: all build test lint clean proto dev down logs help

help:              ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

proto:             ## Compile .proto files and regenerate code for all languages
	$(MAKE) -C proto

build:             ## Build all components
	$(MAKE) -C agent build
	$(MAKE) -C gateway build
	$(MAKE) -C workers build
	$(MAKE) -C dashboard build

test:              ## Run all tests
	$(MAKE) -C agent test
	$(MAKE) -C gateway test
	$(MAKE) -C workers test

lint:              ## Lint all components
	$(MAKE) -C agent lint
	$(MAKE) -C gateway lint
	$(MAKE) -C workers lint
	$(MAKE) -C dashboard lint

dev:               ## Start full local dev environment
	$(MAKE) -C docker dev

down:              ## Stop local dev environment
	$(MAKE) -C docker down

logs:              ## Tail logs from all services
	$(MAKE) -C docker logs

clean:             ## Clean all build artifacts
	$(MAKE) -C agent clean
	$(MAKE) -C gateway clean
	$(MAKE) -C workers clean
	$(MAKE) -C dashboard clean
	$(MAKE) -C proto clean
```

### `proto/Makefile`

```makefile
# Rust code generation is handled by prost-build in agent/build.rs
# (idiomatic Rust: compile-time generation via cargo build, not make proto).

PROTO_SRC   := ranger/v1/events.proto ranger/v1/agent.proto
GEN_PYTHON  := gen/python
GEN_GO      := gen/go

.PHONY: all clean

all:               ## Regenerate protobuf bindings for Python and Go
	mkdir -p $(GEN_PYTHON) $(GEN_GO)
	python3 -m grpc_tools.protoc -I. --python_out=$(GEN_PYTHON) --pyi_out=$(GEN_PYTHON) $(PROTO_SRC)
	protoc -I. --go_out=$(GEN_GO) --go_opt=paths=source_relative $(PROTO_SRC)

clean:             ## Remove all generated files
	rm -rf $(GEN_PYTHON) $(GEN_GO)
```

### `agent/Makefile`

```makefile
BINARY      := ai-ranger
TARGETS     := x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu \
               x86_64-apple-darwin aarch64-apple-darwin \
               x86_64-pc-windows-msvc

.PHONY: build test lint clean release run

build:             ## Build debug binary for current platform
	cargo build

run:               ## Run agent in stdout mode (requires root/sudo for pcap)
	sudo cargo run -- --mode dns-sni

test:              ## Run all tests
	cargo test

lint:              ## Run clippy and rustfmt check
	cargo clippy -- -D warnings
	cargo fmt --check

release:           ## Build release binaries for all targets
	@for target in $(TARGETS); do \
		echo "Building $$target..."; \
		cargo build --release --target $$target; \
	done

clean:             ## Remove build artifacts
	cargo clean
```

### `gateway/Makefile`

```makefile
.PHONY: build run run-dev test lint install clean

install:           ## Install Python dependencies
	pip install -r requirements.txt

build: install     ## Install deps and verify the app loads cleanly
	python -c "from main import app"

migrate:           ## Run Alembic migrations (creates/updates Postgres schema)
	alembic upgrade head

run: migrate       ## Run gateway with uvicorn (migrates DB first)
	uvicorn main:app --host 0.0.0.0 --port 8080

run-dev: migrate   ## Run in dev mode with auto-reload (migrates DB first)
	uvicorn main:app --host 0.0.0.0 --port 8080 --reload

test:              ## Run tests
	pytest tests/ -v

lint:              ## Lint with ruff and type-check with mypy
	ruff check .
	mypy .

clean:             ## Remove cached files
	find . -type d -name __pycache__ -exec rm -rf {} +
	find . -name "*.pyc" -delete
```

### `workers/Makefile`

```makefile
INGEST_BIN  := bin/ingest-worker
API_BIN     := bin/api-server

.PHONY: build run-ingest run-api test lint clean

build:             ## Build all Go binaries (includes Swagger generation)
	swag init -g cmd/api/main.go -o cmd/api/docs --parseDependency
	go build -o $(INGEST_BIN) ./cmd/ingest
	go build -o $(API_BIN) ./cmd/api

run-ingest:        ## Run ingest worker locally
	go run ./cmd/ingest

run-api:           ## Run query API server locally
	go run ./cmd/api

test:              ## Run all tests
	go test ./... -v

lint:              ## Run golangci-lint
	golangci-lint run ./...

clean:             ## Remove build artifacts
	rm -rf bin/
```

### `dashboard/Makefile`

```makefile
.PHONY: install build run test lint clean

install:           ## Install npm dependencies
	npm install

build: install     ## Build production bundle
	npm run build

run:               ## Start dev server with hot reload
	npm run dev

test:              ## Run tests
	npm run test

lint:              ## Lint and type-check
	npm run lint
	npm run typecheck

clean:             ## Remove build artifacts
	rm -rf dist/ node_modules/
```

### `docker/Makefile`

```makefile
COMPOSE         := docker compose -f docker-compose.yml
COMPOSE_DEV     := docker compose -f docker-compose.yml -f docker-compose.dev.yml

.PHONY: up dev down logs reset ps build

up:                ## Start full stack (production-like)
	$(COMPOSE) up -d

dev:               ## Start full stack with dev overrides (source mounts, hot reload)
	$(COMPOSE_DEV) up

down:              ## Stop all services
	$(COMPOSE) down

logs:              ## Tail logs from all services
	$(COMPOSE) logs -f

ps:                ## Show status of all services
	$(COMPOSE) ps

build:             ## Rebuild all Docker images
	$(COMPOSE) build

reset:             ## Tear down everything including volumes (destructive)
	$(COMPOSE) down -v
	$(COMPOSE) up -d
```

---

## Backend API Endpoints

Ingest endpoints are served by the FastAPI gateway (Swagger UI at `http://localhost:8080/docs`).
All query and management endpoints are served by the Go API server (Swagger UI at
`http://localhost:8081/docs`). The dashboard never talks to the gateway directly.

In development, these are also accessible via nginx: `http://localhost:8000/ingest/docs`
for the gateway and `http://localhost:8000/api/docs` for the API server.

```
# FastAPI Gateway - agent-facing only (Swagger: http://localhost:8080/docs)
POST /v1/ingest              -> receive protobuf EventBatch, enqueue to RabbitMQ
POST /v1/agents/enroll       -> enrollment (token in body, returns org_id)
GET  /v1/agents/providers    -> fetch latest providers.toml
GET  /v1/agents/version      -> check for agent updates

# Go Query API - dashboard and admin facing (Swagger: http://localhost:8081/docs)
GET  /v1/dashboard/overview           -> summary stats (?days=7|30|90)
GET  /v1/dashboard/providers          -> provider breakdown (?days=7)
GET  /v1/dashboard/users              -> per-user activity (?days=7&provider=openai)
GET  /v1/dashboard/traffic/timeseries -> hourly traffic by provider (?days=7)
GET  /v1/dashboard/fleet              -> all enrolled agents + status
GET  /v1/events                       -> paginated raw events (?q=term&days=7&page=1&limit=25&sort=timestamp&order=desc)

GET    /v1/admin/tokens       -> list enrollment tokens
POST   /v1/admin/tokens       -> create enrollment token
DELETE /v1/admin/tokens/:id   -> revoke token
DELETE /v1/admin/agents/:id   -> revoke agent

POST /v1/auth/login           -> authenticate, returns JWT + refresh token
POST /v1/auth/refresh         -> exchange refresh token for new JWT
```

---

## Dashboard Views

**1. Fleet Overview** - admin landing page
- Total active agents, last-seen distribution
- Org-wide connections today vs. yesterday
- Top providers by connection count (bar chart)

**2. Provider Breakdown**
- Table: Provider | Connections | Unique Users | Bytes Sent | Bytes Received | Trend (Bytes Sent/Received require byte counting implementation)
- Time series: connections over time, stacked by provider
- Date range filter

**3. User Activity Table**
- Rows: one per (user x provider x process) combination
- Columns: User | Machine | Provider | App | Connections | Data Sent | Data Received | Last Active (Data Sent/Received require byte counting implementation)
- Sortable, filterable, click-through to user timeline

**4. Traffic Over Time**
- Stacked area chart: bytes per provider per hour/day
- Useful for spotting anomalies (large uploads to AI providers at odd hours)

**5. Agent Fleet Management**
- All enrolled agents: hostname, user, OS, version, last seen, status
- Generate enrollment tokens (label, expiry, max-use settings)
- Revoke agents or tokens

### Dashboard tech stack
- React + TypeScript
- TanStack Query (data fetching)
- TanStack Table (sortable, filterable tables)
- Recharts (charts)
- shadcn/ui (components)

---

## Local Buffer & Upload Logic (Agent)

The agent writes events to a local SQLite database immediately on capture.
A background async task drains the buffer every 30 seconds.
Network outages do not lose data. Backend restarts do not matter.

```
Capture event
    -> write to SQLite immediately (< 1ms, non-blocking)

Every 30s:
    -> read up to 500 events from SQLite
    -> serialize to protobuf EventBatch
    -> POST to gateway with agent_id Bearer auth
    -> on 2xx: mark events as uploaded, delete from SQLite
    -> on failure: leave in SQLite, retry next cycle (exponential backoff)
```

SQLite buffer is only active when an HTTP output sink is configured.
In stdout or file mode, there is no local buffer.

---

## Self-Hosting

The entire platform runs with two commands. This is a hard requirement.

```bash
git clone https://github.com/pykul/ai-ranger
cd ai-ranger
make dev
```

Services started (8 total):

| Service | Role |
|---|---|
| `nginx` | Single entry point, routes `/` to dashboard, `/api/` to API server, `/ingest/` to gateway |
| `postgres` | Identity data (orgs, agents, tokens), schema via Alembic |
| `clickhouse` | Event timeseries storage |
| `rabbitmq` | Message queue between gateway and ingest workers |
| `gateway` | FastAPI agent-facing ingest and enrollment |
| `ingest-worker` | Consumes RabbitMQ, writes to ClickHouse and Postgres |
| `api-server` | Go Query API for dashboard data and admin operations |
| `dashboard` | React SPA served by nginx |

Configuration is split across three compose files:
- `docker-compose.yml` - base (all services)
- `docker-compose.dev.yml` - dev overrides (source mounts, hot reload)
- `docker-compose.prod.yml` - production overrides (TLS, no direct port exposure)

---

## Release Pipeline (GitHub Actions)

On every git tag, build and attach agent binaries to GitHub Release:

```
Targets:
  x86_64-unknown-linux-gnu
  aarch64-unknown-linux-gnu
  x86_64-apple-darwin
  aarch64-apple-darwin        (Apple Silicon)
  x86_64-pc-windows-msvc
```

Users download a single pre-built binary. No Rust toolchain required.
This is required for community adoption.

---

## Phased Delivery Plan

### Phase 0 - Spike (2-3 weeks)
**Goal:** Prove the core technique works end to end.
- Rust binary that captures SNI from real traffic and prints to stdout
- Validate against: Claude Code, Cursor, ChatGPT browser, Copilot
- Confirm process attribution works on primary dev OS
- Deliverable: ~200 line Rust program, no config, prints detected AI connections
- Root Makefile and agent/Makefile created at the start of this phase

### Phase 1 - Agent MVP (4-5 weeks)
- Full SNI + DNS capture pipeline (including ETW DNS-Client on Windows for IPv6 coverage)
- Provider classifier with `providers.toml` (15-20 providers seeded)
- Process resolver (Linux, macOS, and Windows)
- All output sinks: stdout, file, http (protobuf)
- WebhookSink for custom telemetry
- FanoutSink (fan events to multiple outputs)
- Enrollment flow + identity + AgentConfig
- SQLite buffer + batch uploader (http mode only)
- Installer scripts: macOS + Linux
- GitHub Actions release pipeline (do this early, not at the end)

### Phase 2 - Backend MVP (3-4 weeks, parallel with Phase 1)
- Protobuf schema finalized, generated code committed for all three languages
- proto/Makefile working (`make proto` regenerates everything)
- Postgres schema: orgs, tokens, agents
- ClickHouse schema
- FastAPI gateway: ingest + enroll endpoints (Swagger UI at /docs)
- RabbitMQ: exchange and queue configured via definitions.json
- Go ingest workers: consume from RabbitMQ, write to ClickHouse
- Go query API: basic dashboard endpoints
- gateway/Makefile and workers/Makefile working
- `make dev` brings up the full stack end to end

### Phase 3 - Dashboard MVP (3-4 weeks)
- Authentication: JWT, single admin user, environment-aware (disabled in dev)
- Login page in the dashboard (production only)
- nginx as single ingress point (port 8000 dev, port 443 prod with TLS)
- Dashboard page: stat cards, timeseries chart, top providers/users ranked lists
- Events page: full-text search, paginated table, sortable, expandable row detail
- Admin section: fleet management + enrollment token management
- Time range selector (7d/30d/90d) applies globally to all views
- Provider filter via chart legend click filters all dashboard data
- GET /v1/events endpoint with search, pagination, time filtering
- All dashboard endpoints support ?days query parameter
- dashboard/Makefile working, CI job enabled

### Phase 4 - Polish + Windows Installer (2-3 weeks)
- Windows installer (PowerShell) + Windows service registration
- Agent auto-update mechanism
- Agent version tracking in fleet view
- Alerting (new provider first seen, unusual volume)
- WASM plugin sink (if community demand exists)

### Phase 5 - MITM Mode (future, scope TBD)

> **For Claude Code: Do not begin Phase 5 work unless explicitly instructed.
> Do not create any files inside `agent/src/capture/mitm/` beyond the stub.
> Do not implement cert generation, a local proxy, or HTTP/2 parsing at any
> point during Phases 0-4. If asked to "add MITM" during those phases,
> refuse and point to this section.**

When community demand justifies it, MITM mode will add:
- Local CA cert generation + OS trust store installation flow
- Per-OS installer extensions (macOS Keychain, Windows cert store, Linux NSS)
- Local TLS proxy for known AI provider hostnames only
- HTTP/2 + SSE stream reassembly and parser
- Exact model name extraction (from request body)
- Token count extraction (from response headers)
- Latency measurement (time to first byte)
- Content storage in object storage - not ClickHouse, payloads are too large
- Dashboard views for content-level analytics
- Explicit user consent flow and PII warning on install

Hard problems to solve in Phase 5:
- Cert pinning breakage (Cursor, some Electron apps)
- HTTP/2 stream reassembly for chunked SSE responses
- PII exposure policy and user acknowledgment
- Trust story update across all docs and README

### Docs (runs in parallel with Phase 3-4)
- Getting started guide (under 5 minutes from zero to first event)
- Self-hosting guide
- Agent configuration reference
- Provider contribution guide (`CONTRIBUTING.md` + `providers/CONTRIBUTING.md`)
- README that explains the project in 30 seconds

---

## Known Limitations

- **Process attribution resolves the process that owns the socket at the moment of capture.** Short-lived child processes (e.g. `curl` spawned from a shell) may appear as their parent process or as `"unknown"` if the process exits before the name can be resolved. The PID is always accurate regardless. This is expected behavior and not a bug. In practice it does not affect real AI tool detection since tools like Cursor, Claude Code, and Python scripts own their sockets directly.

- **Traffic volume (bytes sent/received) is not measured.** Accurate byte counting requires full TCP session tracking across the connection lifecycle, which is not currently implemented.

- **IP range matching only covers providers with dedicated IP space.** Providers behind shared CDNs (OpenAI, Claude, Cursor, Copilot, Gemini) cannot be detected via IP matching without causing false positives. Currently only the Anthropic API (`api.anthropic.com`) has a known dedicated range (`160.79.104.0/23`).

- **On Windows, DNS-based detection relies on the Windows DNS client service.** Applications that use their own internal DoH resolver, bypassing the OS DNS client, are invisible to ETW DNS-Client events. Currently this is primarily browsers (Chrome, Firefox, Edge, Brave) but the limitation applies to any application that implements DoH internally. CLI tools, SDKs, and desktop AI applications use the system DNS resolver and are detected normally.

- **Ollama local model detection is not implemented.** The `ports` and `tls` fields in `providers.toml` are parsed but ignored. Connections to localhost on port 11434 without TLS cannot be detected via SNI. See DECISIONS.md for the planned TCP heuristic approach.

---

## Key Architectural Decisions (and Why)

| Decision | Chosen | Rejected | Reason |
|---|---|---|---|
| Default capture method | SNI extraction (passive) | TLS MITM proxy | No cert installation, no broken apps, no legal grey area |
| Capture backend | OS-native raw sockets + ETW | libpcap / npcap | Standalone binary, no external C library deps, no driver install |
| MITM mode | Planned Phase 5, opt-in only | Built-in from day 1 | Ship value fast, earn trust first, complex problems deferred |
| Agent language | Rust | Go, Python | Memory safety for a privileged process, single binary, deterministic performance, author learning goal |
| Gateway language | Python + FastAPI | Go, Node, Flask | Thin gateway pattern, async-native, auto-generated Swagger docs from Pydantic types |
| Worker language | Go | Python, Node | Native goroutines for concurrency, great ClickHouse and RabbitMQ clients |
| Message queue | RabbitMQ | Kafka, NATS, Redis | Widely understood, durable, purpose-built, simple Docker setup |
| Wire format | Protobuf | JSON | Smaller on the wire, schema enforced across all three languages |
| Event storage | ClickHouse | TimescaleDB, InfluxDB | Best-in-class OLAP, handles billions of rows, free |
| Identity storage | Postgres | SQLite, MongoDB | Relational integrity for orgs/tokens/agents, boring and reliable |
| Output abstraction | Trait-based sinks | Hardcoded outputs | Enables custom telemetry without forking the agent |
| Custom telemetry | WebhookSink (Phase 1), WASM (deferred) | Plugin API from day 1 | Ship value now, WASM when there is demand |
| Build system | Makefiles everywhere | Scripts, Taskfile | Universal, no extra tooling required, contributor-friendly |
| License | Apache-2.0 | MIT, BUSL, GPL | Patent grant included, permissive, no commercial restrictions |
| Repo structure | Monorepo | Separate repos | Simpler contributor experience, shared issues and PRs |
| Providers list | Community TOML file | Hardcoded, DB-driven | Easy to contribute, no code required, version controlled |

---

## Trust & Privacy Principles (Non-Negotiable)

These are not just design goals. They are the reason this tool is trustworthy enough
to install on machines that monitor network traffic.

1. **Zero call-home by default.** The agent never contacts any URL unless explicitly
   configured. Running `ai-ranger` with no config file produces stdout output only.

2. **No content inspection in default mode.** DNS/SNI mode reads hostnames only.
   It never reads, buffers, or transmits any part of the TLS payload.

3. **MITM mode is always explicit opt-in.** It requires a separate install step, a
   separate flag (`--mode mitm`), and a consent flow. It is never the default.
   It is not yet implemented (Phase 5+).

4. **Auditable.** The agent is fully open source. Every line of network-touching code
   can be reviewed by anyone. This is the correct answer to "how do I know you are not
   spying on me?"

5. **Explicit configuration required.** Backend URL and enrollment token must be
   explicitly provided by the user. They are never bundled, hardcoded, or defaulted.

6. **Local-first.** The SQLite buffer means data stays on the machine until successfully
   delivered. Nothing is sent to any third party.

---

## Known Limitations (Be Upfront About These)

- Does not detect AI usage over non-standard ports
- Does not detect AI calls made through a VPN that terminates on-machine
- Ollama (local models) requires TCP heuristic, not SNI (no TLS)
- Process attribution can have brief gaps under very high connection rates
- Provider hostnames change occasionally; registry needs community maintenance
- Requires root on Linux and macOS, elevated service permissions on Windows
- ECH (Encrypted Client Hello) and DoH (DNS over HTTPS) are TLS and DNS privacy features that any application can implement. Applications using ECH hide the SNI hostname; applications using DoH bypass UDP port 53 DNS capture. When both are active simultaneously, passive detection produces no events. Browsers (Chrome, Firefox, Edge, Brave) are the primary current deployers of both features. Anthropic API connections are partially recoverable via IP range matching. Full visibility for ECH+DoH applications requires MITM mode (Phase 5). CLI tools, SDKs, and desktop AI apps do not currently implement ECH and are unaffected.

---

## Authentication

Authentication is **environment-aware**. The behavior differs between development
and production to preserve the local developer experience unchanged.

**Development (`ENVIRONMENT=development`):** Auth is disabled entirely. The Go Query
API accepts all requests without a token. The dashboard has no login screen. This is
the default behavior in the local Docker Compose stack and must not change.

**Production (`ENVIRONMENT=production`):** JWT authentication is required on every
Go Query API request except `/health` and `/v1/auth/*`. The dashboard shows a login
screen.

### Single admin user

The dashboard has a single admin user. There is no user management system, no users
table, no Alembic migration, and no invite or password reset flows. The admin
credentials are set via environment variables and checked directly at login time.

### Auth endpoints (Go Query API)

Two auth endpoints are added to the Go Query API:

- **`POST /v1/auth/login`** - Accepts `{ "email": "...", "password": "..." }`. Checks
  the submitted email and password against `ADMIN_EMAIL` and `ADMIN_PASSWORD` environment
  variables. `ADMIN_PASSWORD` is plaintext in the environment and hashed once in memory
  at startup via bcrypt. On success, returns a signed
  JWT access token (24-hour expiry) and a refresh token.

- **`POST /v1/auth/refresh`** - Accepts `{ "refresh_token": "..." }`. Returns a new
  access token if the refresh token is valid.

### Auth middleware

In production, the Go Query API auth middleware validates the `Authorization: Bearer <jwt>`
header on every protected request. In development, the middleware is a no-op that passes
all requests through. The environment check reads `ENVIRONMENT` from the Go config struct
at startup - there is no per-request overhead.

### Auth environment variables

| Variable | Description |
|----------|-------------|
| `JWT_SECRET` | Secret key for signing JWT tokens (production only) |
| `ADMIN_EMAIL` | Admin login email (production only) |
| `ADMIN_PASSWORD` | Admin password, plaintext - hashed in memory at startup (production only) |

These have no effect in development. They are not required in `.env` for local
development and are intentionally absent from the default Docker Compose configuration.

---

## Configuration and Secrets

All runtime configuration comes from environment variables. No hardcoded hostnames,
ports, credentials, or secrets exist in application code. The `.env.example` file at
the repo root documents every variable with comments and safe local development defaults.

**Python gateway:** Uses `pydantic-settings` with a `Settings` class in `gateway/config.py`.
All environment variables are typed, validated at startup, and injected via FastAPI
dependency injection. No `os.environ` calls elsewhere in the codebase.

**Go workers:** Uses a `Config` struct in `workers/internal/config/config.go` loaded
via `config.Load()` at startup in each `main.go`. The struct is passed to all components.
No `os.Getenv` calls outside the config package.

**Docker Compose:** The `docker/docker-compose.yml` file references variables from `../.env`
via `env_file` and `${VAR}` interpolation. No credentials are hardcoded in the compose file.

### Environment Variables by Service

**All services:**

| Variable | Description |
|----------|-------------|
| `SHUTDOWN_TIMEOUT_SECS` | Graceful shutdown timeout in seconds (default: 30) |

**Gateway:**

| Variable | Description |
|----------|-------------|
| `DATABASE_URL` | SQLAlchemy async Postgres URL |
| `RABBITMQ_URL` | AMQP connection URL |
| `GATEWAY_PORT` | Listen port (default: 8080) |
| `PROVIDERS_TOML_PATH` | Path to providers.toml (default: `providers/providers.toml`) |
| `ENVIRONMENT` | `development` enables seed data (default: `production`) |
| `SEED_TOKEN` | Plaintext token to seed (only in development) |

**Workers (ingest + API):**

| Variable | Description |
|----------|-------------|
| `DATABASE_URL` | GORM Postgres DSN |
| `CLICKHOUSE_ADDR` | ClickHouse native protocol address (default: `localhost:9000`) |
| `CLICKHOUSE_DATABASE` | ClickHouse database name (default: `default`) |
| `RABBITMQ_URL` | AMQP connection URL (ingest only) |
| `API_SERVER_PORT` | API server listen port (default: 8081) |
| `ENVIRONMENT` | `development` disables auth; `production` requires JWT (default: `development`) |
| `JWT_SECRET` | Secret key for signing JWT tokens (production only, no effect in dev) |
| `ADMIN_EMAIL` | Admin login email (production only, no effect in dev) |
| `ADMIN_PASSWORD` | Admin password, plaintext - hashed in memory at startup (production only, no effect in dev) |

**Infrastructure (Docker Compose):**

| Variable | Description |
|----------|-------------|
| `POSTGRES_USER` | Postgres superuser name |
| `POSTGRES_PASSWORD` | Postgres superuser password |
| `POSTGRES_DB` | Postgres database name |
| `POSTGRES_HOST` | Postgres hostname (used in docker-compose.yml to construct connection URLs) |
| `POSTGRES_PORT` | Postgres port (used in docker-compose.yml to construct connection URLs) |
| `CLICKHOUSE_HOST` | ClickHouse hostname (used in docker-compose.yml to construct CLICKHOUSE_ADDR) |
| `CLICKHOUSE_PORT` | ClickHouse native protocol port (used in docker-compose.yml to construct CLICKHOUSE_ADDR) |
| `RABBITMQ_DEFAULT_USER` | RabbitMQ default user (read natively by the image) |
| `RABBITMQ_DEFAULT_PASS` | RabbitMQ default password |

---

## Testing Strategy

Three levels of testing, each with different scope and requirements:

### Unit tests

Per-component tests with no external dependencies. Run with `make test`.

- **Agent (Rust):** 49 tests covering SNI parsing, DNS parsing, provider classification,
  IP range matching, SQLite buffer operations, and the pipeline. No root required.
- **Gateway (Python):** pytest tests in `gateway/tests/` when present.
- **Workers (Go):** `go test ./...` covering models and query helpers.

### Integration tests

Full Docker Compose stack tests in `tests/integration/`. Two layers:

- **Synthetic event tests:** Send protobuf EventBatch messages directly to the gateway
  HTTP endpoint. Test the full pipeline from HTTP ingest through RabbitMQ to ClickHouse
  without requiring a real agent or raw socket access. Run on all environments.
- **Real agent tests:** Run the actual compiled agent binary, trigger real network traffic,
  and verify events flow through the full pipeline. Require root for raw socket capture.
  Automatically skipped when not running as root.

Run everything in one command:

```bash
make test-integration
```

This builds the agent, starts the Docker Compose stack (waiting for health checks),
installs test dependencies, and runs all integration tests including real agent tests
with sudo. Platform-specific scripts live in `tests/integration/scripts/`.

See `tests/README.md` for details on test layers and adding new tests.

### CI

Integration tests run automatically in CI after component builds pass:

- **Linux** (`integration-tests` job): Runs `make test-integration` which executes
  the full suite — synthetic, real agent, dashboard, and pipeline tests. GitHub
  Actions Linux runners are root by default.
- **Windows** (`integration-tests-windows` job): Runs only the real agent tests
  (`test_ingest_real_agent.py`) via `tests/integration/scripts/run-windows.ps1`
  to validate the Windows detection path (SIO_RCVALL + ETW DNS) against the full
  backend. GitHub Actions Windows runners have Administrator by default.

### Mapping to Kubernetes

For k8s deployments, map non-sensitive variables to ConfigMaps and credentials to
Secrets. See `k8s/README.md` for complete deployment guidance including probe
configuration, replica guidance, and Secret manifests.

---

## Health Checks

Every HTTP service exposes `GET /health` returning `200 OK` with no authentication
required. These endpoints are used by Docker Compose health checks and translate
directly to Kubernetes readiness and liveness probes.

| Service | Endpoint | Response |
|---------|----------|----------|
| Gateway | `GET http://localhost:8080/health` | `{"status": "ok", "service": "gateway"}` |
| API Server | `GET http://localhost:8081/health` | `{"status": "ok", "service": "api"}` |
| Postgres | `pg_isready -U $POSTGRES_USER` | Exit code 0 |
| ClickHouse | `clickhouse-client --query 'SELECT 1'` | Exit code 0 |
| RabbitMQ | `rabbitmq-diagnostics -q ping` | Exit code 0 |

The ingest worker has no HTTP server. Its health is inferred from process liveness —
it exits on fatal errors and Docker restarts it via `restart: unless-stopped`.

---

## Kubernetes Compatibility

All backend services are designed to be k8s-compatible without modification:

- **Stateless pods.** Gateway, ingest-worker, and api-server store no local state.
  They can be scaled horizontally and restarted at any time.
- **Config from environment.** All runtime configuration comes from environment
  variables, mapping naturally to k8s ConfigMaps and Secrets.
- **Health endpoints.** Gateway and api-server expose `GET /health` for readiness
  and liveness probes.
- **Graceful shutdown.** All services handle SIGTERM and drain in-flight work
  within `SHUTDOWN_TIMEOUT_SECS`. Set `terminationGracePeriodSeconds` to match.
- **No host filesystem dependencies.** Docker images are self-contained. The
  gateway bundles providers.toml in the image; it is not required as a volume mount.

See `k8s/README.md` for complete deployment guidance.

---

## Starting Point for Claude Code Sessions

**Always begin a new session with:**

> "Read ARCHITECTURE.md. We are working on [specific phase/component].
> Do not implement anything outside the scope of what I describe.
> If something is marked Phase 5 or MITM, do not touch it.
> Every component needs a Makefile - create it before writing any other code."

**Phase 0 session prompt:**
> "Read ARCHITECTURE.md. Implement Phase 0 only: a Rust binary called `ai-ranger`
> that captures TLS ClientHello packets on port 443, extracts the SNI hostname,
> matches it against a hardcoded list of 5 AI provider hostnames, and prints
> matching connections to stdout as JSON. Start by creating the root Makefile and
> agent/Makefile. No config file, no enrollment, no upload, no MITM code.
> Just capture, classify, print."

**When asked to start Phase 5:**
> "Read ARCHITECTURE.md, paying close attention to the Phase 5 MITM Mode section
> and the Agent Capture Modes section. We are now starting Phase 5. Before writing
> any code, outline your implementation plan for the MITM capture pipeline and the
> cert installation flow. We will review and approve the plan before any code is written."
