# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Essential Reading

Read `ARCHITECTURE.md` before writing any code. It is the single source of truth for all architectural decisions.

## Build Commands

Polyglot monorepo: Rust (agent), Python/Flask (gateway), Go (workers), React/TypeScript (dashboard). The root Makefile delegates to each component's Makefile. Always use `make` as the top-level entry point.

```bash
make build          # Build all components
make test           # Run all tests
make lint           # Lint all components
make clean          # Clean all build artifacts
```

### Agent (Rust) — `agent/`

```bash
cargo build                     # Debug build
cargo test                      # Unit tests (no elevated privileges needed)
cargo clippy -- -D warnings     # Lint
cargo fmt --check               # Format check
cargo run                       # Run agent (requires sudo / Administrator)
cargo test -p ai-ranger test_name  # Run a single test
```

Running the agent binary requires root/Administrator (raw socket access for packet capture).

## Current State: Phase 0

Only the Rust agent exists. The backend components (gateway, workers, dashboard, proto, docker) described in ARCHITECTURE.md are not yet created — they are Phase 2+. The root Makefile has `dev`, `proto`, `down`, and `logs` targets commented out for this reason. When the full stack lands, `make dev` starts everything via Docker Compose and the dashboard is at `http://localhost:3000`.

## Architecture

AI Ranger is a passive network observability tool that detects AI provider usage via TLS SNI hostname extraction. No TLS interception, no MITM, no content inspection.

**Agent pipeline** (`agent/src/`):
1. **Packet capture** (`capture/pcap.rs`) — Platform-specific raw socket capture filtered to TCP port 443. Linux: `AF_PACKET`, macOS: `/dev/bpf*`, Windows: `SIO_RCVALL`. The `pcap` crate is explicitly forbidden — no libpcap, npcap, or WinPcap. Only OS built-in APIs.
2. **SNI extraction** (`capture/sni.rs`) — Pure byte-level parser of TLS ClientHello to extract the SNI hostname.
3. **DNS monitoring** (`capture/dns.rs`, planned Phase 1) — DNS query parser as fallback/corroboration for SNI.
4. **Classification** (`classifier/providers.rs`) — Matches hostname against a hardcoded provider list (Phase 0; will load from `providers/providers.toml` in Phase 1). Matches exact hostnames and subdomains.
5. **Process resolution** (`main.rs`) — Maps source port to PID/process name via OS APIs (`/proc/net/tcp` on Linux, `lsof` on macOS, `netstat` on Windows).
6. **Output** — JSON events to stdout. Phase 1 adds the `EventSink` trait with stdout, file, HTTP, and webhook sinks via fan-out.

The agent is fully standalone by default — it outputs JSON events to stdout with no backend required. When a backend is configured, the agent sends events to it; otherwise nothing leaves the machine.

**Planned full system** (Phase 2+): Agent → Flask Gateway → RabbitMQ → Go Workers → ClickHouse/Postgres → React Dashboard. See ARCHITECTURE.md for details.

## Key Constraints

- **Every component must have a Makefile.** If creating a new component, create its Makefile first.
- **MITM mode is Phase 5+ only.** `capture/mitm/mod.rs` is an intentional stub. Do not implement it.
- **No external capture dependencies.** The agent uses only OS built-in APIs. The `pcap` crate is explicitly forbidden — no libpcap, npcap, or WinPcap.
- **Zero call-home by default.** The agent never contacts any URL unless explicitly configured with a backend.
- **Backend language boundary** (when implemented): Flask gateway handles only HTTP receipt, token verification, protobuf deserialization, and RabbitMQ publishing. All business logic, DB writes, and dashboard API endpoints go in Go workers. Dashboard talks to Go only, never Flask.
- **Proto changes require `make proto`** and committing regenerated code before other work proceeds.
