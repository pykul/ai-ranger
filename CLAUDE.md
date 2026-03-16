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

## Current State: Phase 1 complete, Phase 2 in progress

The Rust agent is complete with all Phase 1 deliverables: SNI + DNS capture pipeline, ETW DNS-Client on Windows, provider classifier with 19 providers, process resolver on all platforms, all output sinks (stdout, file, http, webhook, fanout), enrollment flow, SQLite buffer with drain loop, and installer scripts. The backend components (gateway, workers, dashboard, proto, docker) described in ARCHITECTURE.md are not yet created - they are Phase 2. The root Makefile has `dev`, `proto`, `down`, and `logs` targets commented out for this reason. When the full stack lands, `make dev` starts everything via Docker Compose and the dashboard is at `http://localhost:3000`.

## Architecture

AI Ranger is a passive network observability tool that detects AI provider usage via TLS SNI hostname extraction. No TLS interception, no MITM, no content inspection.

**Agent pipeline** (`agent/src/`):
1. **Packet capture** (`capture/pcap.rs`) - Platform-specific raw socket capture filtered to TCP port 443. Linux: `AF_PACKET`, macOS: `/dev/bpf*`, Windows: `SIO_RCVALL`. The `pcap` crate is explicitly forbidden - no libpcap, npcap, or WinPcap. Only OS built-in APIs.
2. **SNI extraction** (`capture/sni.rs`) - Pure byte-level parser of TLS ClientHello to extract the SNI hostname.
3. **DNS monitoring** (`capture/dns.rs`, planned Phase 1) - DNS query parser as fallback/corroboration for SNI.
4. **Classification** (`classifier/providers.rs`) - Matches hostname against a hardcoded provider list (Phase 0; will load from `providers/providers.toml` in Phase 1). Matches exact hostnames and subdomains.
5. **Process resolution** (`process/mod.rs`) - Maps source port to PID/process name via OS APIs (`/proc/net/tcp` on Linux, `proc_pidinfo` on macOS, `GetExtendedTcpTable` on Windows).
6. **Output** - JSON events to stdout. Phase 1 adds the `EventSink` trait with stdout, file, HTTP, and webhook sinks via fan-out.

The agent is fully standalone by default - it outputs JSON events to stdout with no backend required. When a backend is configured, the agent sends events to it; otherwise nothing leaves the machine.

**Planned full system** (Phase 2+): Agent → FastAPI Gateway → RabbitMQ → Go Workers → ClickHouse/Postgres → React Dashboard. See ARCHITECTURE.md for details.

## Key Constraints

- **Every component must have a Makefile.** If creating a new component, create its Makefile first.
- **MITM mode is Phase 5+ only.** `capture/mitm/mod.rs` is an intentional stub. Do not implement it.
- **No external capture dependencies.** The agent uses only OS built-in APIs. The `pcap` crate is explicitly forbidden - no libpcap, npcap, or WinPcap.
- **Zero call-home by default.** The agent never contacts any URL unless explicitly configured with a backend.
- **Backend language boundary** (when implemented): FastAPI gateway handles only HTTP receipt, token verification, protobuf deserialization, and RabbitMQ publishing. All business logic, DB writes, and dashboard API endpoints go in Go workers. Dashboard talks to Go only, never FastAPI.
- **Proto changes require `make proto`** and committing regenerated code before other work proceeds.
- **No magic numbers or magic strings.** Every literal value that represents a configuration parameter, a protocol constant, a timeout, a buffer size, a GUID, or a port number must be defined as a named constant with a doc comment explaining what it is and why the default was chosen. Tuneable operational values that admins may want to adjust must be exposed as optional fields in config.toml with the constant as the fallback default. This applies to all components - Rust agent, Python gateway, and Go workers. When adding a new feature, define its constants before writing the implementation.
- **Code quality standards.** main.rs is thin - it wires components together but contains no business logic. Every file has a single clear responsibility. Functions longer than 50 lines should be broken into smaller named pieces unless the length is driven by unavoidable sequential steps (protocol parsers, FFI ceremony). Logic that appears in more than one place must be extracted into a shared function. Constructors must hide internal defaults - only accept parameters that genuinely vary at call time. Visibility is minimal by default - use pub(crate) unless external access is required, and pub only when the item is part of a deliberate public interface.
- **Database access uses ORMs.** Python components use SQLAlchemy 2.0 async with Alembic for migrations. Go components use GORM. Raw SQL against Postgres is not permitted except in Alembic migration files and GORM model definitions. ClickHouse is the intentional exception - use the clickhouse-go driver directly with named query constants. Never hardcode table names, column names, or query strings outside of model definitions and named constants. The SQLAlchemy models in gateway/models/ are the source of truth for the Postgres schema. The GORM structs in workers/internal/models/ must mirror them exactly. Any Alembic migration that adds or changes a column must be accompanied by the corresponding GORM struct update in the same commit.
