# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Decision Log

DECISIONS.md is a living document. Whenever a significant decision is made, a direction changes, an approach is researched and rejected, or a planned path is confirmed or abandoned, add an entry to DECISIONS.md. Keep entries concise - explain what was decided, what alternatives were considered, and why. Do not duplicate what is in ARCHITECTURE.md. That document explains what the system is. DECISIONS.md explains why it is that way. Every session that results in a meaningful architectural or implementation decision should end with a DECISIONS.md update.

## Essential Reading

Read `ARCHITECTURE.md` before writing any code. It is the single source of truth for all architectural decisions.

## Build Commands

Polyglot monorepo: Rust (agent), Python/FastAPI (gateway), Go (workers), React/TypeScript (dashboard). The root Makefile delegates to each component's Makefile. Always use `make` as the top-level entry point.

```bash
make build          # Build all components
make test           # Run all tests
make lint           # Lint all components
make clean          # Clean all build artifacts
```

### Agent (Rust) - `agent/`

```bash
cargo build                     # Debug build
cargo test                      # Unit tests (no elevated privileges needed)
cargo clippy -- -D warnings     # Lint
cargo fmt --check               # Format check
cargo run                       # Run agent (requires sudo / Administrator)
cargo test -p ai-ranger test_name  # Run a single test
```

Running the agent binary requires root/Administrator (raw socket access for packet capture).

## Current State: Phase 3 complete (dashboard MVP)

The Rust agent is complete with all Phase 1 deliverables plus the Phase 2 protobuf switch. The full platform is operational: nginx as single entry point, FastAPI gateway (ingest, enrollment, providers endpoints), Go workers (ingest consumer writing to ClickHouse, API server with dashboard and admin endpoints, JWT auth with environment-aware bypass), Docker Compose stack (nginx, Postgres, ClickHouse, RabbitMQ, gateway, ingest-worker, api-server, dashboard), protobuf schema with generated code for Python/Go/Rust, integration test suite, and CI pipeline. The React dashboard is complete with Overview (stats, charts, ranked lists), Events (paginated search with sort), Admin (fleet and token management), and Login (production JWT auth). `make dev` starts the full 8-service stack via Docker Compose.

## Architecture

AI Ranger is a passive network observability tool that detects AI provider usage via TLS SNI hostname extraction. No TLS interception, no MITM, no content inspection.

**Agent pipeline** (`agent/src/`):
1. **Packet capture** (`capture/pcap.rs`) - Platform-specific raw socket capture filtered to TCP port 443. Linux: `AF_PACKET`, macOS: `/dev/bpf*`, Windows: `SIO_RCVALL`. The `pcap` crate is explicitly forbidden - no libpcap, npcap, or WinPcap. Only OS built-in APIs.
2. **SNI extraction** (`capture/sni.rs`) - Pure byte-level parser of TLS ClientHello to extract the SNI hostname.
3. **DNS monitoring** (`capture/dns.rs`) - DNS query parser as fallback/corroboration for SNI.
4. **Classification** (`classifier/providers.rs`) - Matches hostname against the provider registry (`providers/providers.toml`). Matches exact hostnames and subdomains.
5. **Process resolution** (`process/mod.rs`) - Maps source port to PID/process name via OS APIs (`/proc/net/tcp` on Linux, `proc_pidinfo` on macOS, `GetExtendedTcpTable` on Windows).
6. **Output** - Events routed to configurable output sinks (stdout, file, http, webhook) via the EventSink trait and FanoutSink.

The agent is fully standalone by default - it outputs JSON events to stdout with no backend required. When a backend is configured, the agent sends events to it; otherwise nothing leaves the machine.

**Full system**: Agent → FastAPI Gateway → RabbitMQ → Go Workers → ClickHouse/Postgres → React Dashboard. All traffic enters through nginx. See ARCHITECTURE.md for details.

## Key Constraints

- **Every component must have a Makefile.** If creating a new component, create its Makefile first.
- **MITM mode is Phase 5+ only.** `capture/mitm/mod.rs` is an intentional stub. Do not implement it.
- **No external capture dependencies.** The agent uses only OS built-in APIs. The `pcap` crate is explicitly forbidden - no libpcap, npcap, or WinPcap.
- **Zero call-home by default.** The agent never contacts any URL unless explicitly configured with a backend.
- **Backend language boundary**: FastAPI gateway handles only HTTP receipt, token verification, protobuf deserialization, and RabbitMQ publishing. All business logic, DB writes, and dashboard API endpoints go in Go workers. Dashboard talks to Go only, never FastAPI.
- **Proto changes require `make proto`** and committing regenerated code before other work proceeds.
- **No magic numbers or magic strings.** Every literal value that represents a configuration parameter, a protocol constant, a timeout, a buffer size, a GUID, or a port number must be defined as a named constant with a doc comment explaining what it is and why the default was chosen. Tuneable operational values that admins may want to adjust must be exposed as optional fields in config.toml with the constant as the fallback default. This applies to all components - Rust agent, Python gateway, and Go workers. When adding a new feature, define its constants before writing the implementation.
- **Code quality standards.** main.rs is thin - it wires components together but contains no business logic. Every file has a single clear responsibility. Functions longer than 50 lines should be broken into smaller named pieces unless the length is driven by unavoidable sequential steps (protocol parsers, FFI ceremony). Logic that appears in more than one place must be extracted into a shared function. Constructors must hide internal defaults - only accept parameters that genuinely vary at call time. Visibility is minimal by default - use pub(crate) unless external access is required, and pub only when the item is part of a deliberate public interface.
- **Database access uses ORMs.** Python components use SQLAlchemy 2.0 async with Alembic for migrations. Go components use GORM. Raw SQL against Postgres is not permitted except in Alembic migration files and GORM model definitions. ClickHouse is the intentional exception - use the clickhouse-go driver directly with named query constants. Never hardcode table names, column names, or query strings outside of model definitions and named constants. The SQLAlchemy models in gateway/models/ are the source of truth for the Postgres schema. The GORM structs in workers/internal/models/ must mirror them exactly. Any Alembic migration that adds or changes a column must be accompanied by the corresponding GORM struct update in the same commit.
- **All runtime configuration comes from environment variables.** No hardcoded hostnames, ports, credentials, or secrets in application code. Python services use pydantic-settings with a Settings class in config.py. Go services use a config struct loaded at startup. Constants files are for application contract constants only (queue names, route paths, protocol values) — not for runtime configuration.
- **Every HTTP service exposes GET /health returning 200 with no auth required.** This endpoint is used by Docker Compose health checks and k8s probes.
- **Services are designed to be k8s-compatible:** stateless, all config from environment, graceful SIGTERM handling, no host filesystem dependencies at runtime.
- **Integration tests live in tests/integration/. Unit tests live alongside the code they test.** Never use time.sleep() in tests — use wait_for_condition() from tests/integration/helpers/wait.py. Every test must be independent and clean up after itself. Tests requiring root must be marked with @pytest.mark.skipif(not is_root(), ...). Tests requiring external network must be marked with @pytest.mark.network.
