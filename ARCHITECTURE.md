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
- Measure bytes sent/received per connection (traffic volume proxy)
- Associate activity with a specific enrolled machine and OS user
- Aggregate into per-user, per-provider dashboards with traffic metrics
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
- How many bytes were sent and received
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
│       ├── python/             # Generated Python classes (betterproto)
│       └── go/                 # Generated Go structs (protoc-gen-go)
│
├── agent/                      # Rust - the on-machine capture agent
│   ├── Makefile
│   ├── src/
│   │   ├── main.rs
│   │   ├── capture/
│   │   │   ├── mod.rs
│   │   │   ├── sni.rs          # TLS ClientHello parser, SNI extractor
│   │   │   ├── dns.rs          # DNS response parser
│   │   │   ├── pcap.rs         # libpcap / npcap interface
│   │   │   └── mitm/           # DO NOT IMPLEMENT - Phase 5+ only
│   │   │       └── mod.rs      # Stub file with a single comment explaining scope
│   │   ├── process/
│   │   │   ├── mod.rs
│   │   │   └── resolver.rs     # pid -> process name, per OS
│   │   ├── classifier/
│   │   │   ├── mod.rs
│   │   │   └── providers.rs    # Provider registry loader and matcher
│   │   ├── output/
│   │   │   ├── mod.rs
│   │   │   ├── sink.rs         # EventSink trait definition
│   │   │   ├── stdout.rs       # Default output (no config needed)
│   │   │   ├── file.rs         # --output file:/path
│   │   │   ├── http.rs         # --output http://backend-url
│   │   │   ├── webhook.rs      # Custom webhook sink
│   │   │   └── fanout.rs       # Fan events to multiple sinks concurrently
│   │   ├── identity/
│   │   │   ├── mod.rs
│   │   │   └── config.rs       # Enrollment token, agent ID, machine metadata
│   │   └── buffer/
│   │       ├── mod.rs
│   │       └── store.rs        # SQLite local event buffer (http mode only)
│   └── Cargo.toml
│
├── gateway/                    # Python + Flask - thin agent-facing gateway
│   ├── Makefile
│   ├── app/
│   │   ├── __init__.py
│   │   ├── routes/
│   │   │   ├── ingest.py       # POST /v1/ingest - receive agent batches
│   │   │   └── enroll.py       # POST /v1/agents/enroll
│   │   ├── auth.py             # Bearer token validation
│   │   ├── queue.py            # RabbitMQ publisher (pika)
│   │   └── proto_utils.py      # Protobuf deserialize helpers
│   ├── proto/                  # Symlink to proto/gen/python
│   ├── requirements.txt
│   └── gunicorn.conf.py
│
├── workers/                    # Go - async processing and query API
│   ├── Makefile
│   ├── cmd/
│   │   ├── ingest/
│   │   │   └── main.go         # Ingest worker binary entry point
│   │   └── api/
│   │       └── main.go         # Query API binary entry point
│   ├── internal/
│   │   ├── consumer/
│   │   │   └── rabbitmq.go     # RabbitMQ consumer, worker pool
│   │   ├── writer/
│   │   │   ├── clickhouse.go   # Batch write events to ClickHouse
│   │   │   └── postgres.go     # Update agent last_seen, fleet metadata
│   │   ├── api/
│   │   │   ├── router.go       # Chi router setup
│   │   │   ├── dashboard.go    # Dashboard query handlers
│   │   │   ├── fleet.go        # Fleet management handlers
│   │   │   └── tokens.go       # Token management handlers
│   │   └── store/
│   │       ├── clickhouse.go   # ClickHouse query helpers
│   │       └── postgres.go     # Postgres query helpers
│   ├── proto/                  # Symlink to proto/gen/go
│   └── go.mod
│
├── dashboard/                  # React + TypeScript
│   ├── Makefile
│   ├── src/
│   │   ├── pages/
│   │   │   ├── Overview.tsx        # Fleet overview landing page
│   │   │   ├── Providers.tsx       # Provider breakdown
│   │   │   ├── Users.tsx           # Per-user activity table
│   │   │   ├── Traffic.tsx         # Traffic timeseries charts
│   │   │   └── Fleet.tsx           # Agent management + token generation
│   │   ├── components/
│   │   └── api/                    # TanStack Query hooks
│   └── package.json
│
├── providers/
│   └── providers.toml          # THE community-maintained provider registry
│
├── docker/
│   ├── Makefile                # Targets for bring-up, teardown, logs, reset
│   ├── docker-compose.yml      # Full stack - one command
│   ├── docker-compose.dev.yml  # Dev overrides - mounts source, hot reload
│   ├── postgres/
│   │   └── init.sql            # Schema bootstrap
│   ├── clickhouse/
│   │   └── config.xml
│   └── rabbitmq/
│       └── definitions.json    # Pre-configured queues and exchanges
│
├── docs/                       # Docusaurus
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
                              │ HTTPS POST
                              │ protobuf EventBatch
                              │ Bearer: <agent_id>
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│              Flask Gateway (Python)                              │
│                                                                  │
│  - Verify agent_id Bearer token (Postgres lookup)               │
│  - Deserialize protobuf EventBatch                              │
│  - Publish raw bytes to RabbitMQ exchange                       │
│  - Return 200 immediately                                        │
│  - Nothing else. No processing, no DB writes.                   │
│                                                                  │
│  Gunicorn + gevent workers for concurrency                      │
└───────────────────────────┬─────────────────────────────────────┘
                             │
                         RabbitMQ
                      (ranger.events)
                             │
              ┌──────────────┴──────────────┐
              │                             │
              ▼                             ▼
┌─────────────────────┐        ┌────────────────────────┐
│  Go Ingest Workers  │        │   Go Query API          │
│                     │        │                         │
│  Goroutine pool     │        │  Chi router             │
│  Consume from queue │        │  Goroutine per request  │
│  Batch write to     │        │  Dashboard endpoints    │
│  ClickHouse         │        │  Fleet management       │
│  Update Postgres    │        │  Token management       │
│  agent last_seen    │        │                         │
└──────────┬──────────┘        └────────────┬────────────┘
           │                                │
           ▼                                ▼
┌────────────────────┐          ┌────────────────────┐
│    ClickHouse      │          │      Postgres       │
│    (events)        │◄─────────│    (identity)       │
└────────────────────┘          └────────────────────┘
                                          │
                                          ▼
                               ┌────────────────────┐
                               │  Dashboard (React)  │
                               │  talks to Go API    │
                               │  only               │
                               └────────────────────┘
```

---

## Backend Language Split - Rules

This boundary must stay clean. If it drifts, the architecture falls apart.

**Flask Gateway (Python) is responsible for:**
- Receiving HTTP requests from agents
- Verifying Bearer tokens
- Deserializing protobuf payloads
- Publishing messages to RabbitMQ
- Responding to the agent

That is all Flask does. No exceptions. If you find yourself writing a database
query, a ClickHouse insert, a data aggregation, or any business logic inside
a Flask route, stop. That code belongs in the Go workers.

**Go Workers are responsible for:**
- Consuming messages from RabbitMQ
- Writing events to ClickHouse in batches
- Updating agent metadata in Postgres
- Serving all dashboard and fleet management API endpoints
- Any future async processing (enrichment, alerting)

The dashboard talks to Go only. It never talks to Flask directly.

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
  TCP_HEURISTIC = 2;
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

  // Traffic
  uint64 bytes_sent = 12;
  uint64 bytes_received = 13;

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

    // Timing
    pub timestamp_ms: i64,              // Phase 0
    pub duration_ms: Option<u64>,       // Phase 1

    // Provider
    pub provider: String,               // Phase 0 - "anthropic", "openai" ...
    pub provider_host: String,          // Phase 0 - raw SNI e.g. "api.anthropic.com"
    pub model_hint: Option<String>,     // Phase 1 - derived from hostname

    // Process
    pub process_name: String,           // Phase 0 - "unknown" until Phase 1
    pub process_pid: u32,               // Phase 0
    pub process_path: Option<String>,   // Phase 1

    // Network
    pub src_ip: String,                 // Phase 0 - source IP of the connection

    // Traffic
    pub bytes_sent: u64,                // Phase 1
    pub bytes_received: u64,            // Phase 1

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
    pub enrollment_token: String,   // provided during install, invalidated after use
    pub org_id: String,             // returned by backend at enrollment
    pub backend_url: String,
    pub machine_hostname: String,
    pub os_username: String,
    pub enrolled_at: i64,           // unix ms
}
```

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

# Multiple outputs supported - events fan out to all of them
[[outputs]]
type = "stdout"     # default, always works with zero config

[[outputs]]
type = "http"
url = "http://localhost:8080"   # use https:// in production
token = "tok_abc123"

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
    "timestamp_ms": 1773506947460,
    "duration_ms": null,
    "provider": "anthropic",
    "provider_host": "api.anthropic.com",
    "model_hint": null,
    "process_name": "claude",
    "process_pid": 1867,
    "process_path": null,
    "src_ip": "172.27.151.106",
    "bytes_sent": 0,
    "bytes_received": 0,
    "detection_method": "SNI",
    "capture_mode": "DNS_SNI",
    "content_available": false,
    "payload_ref": null,
    "model_exact": null,
    "token_count_input": null,
    "token_count_output": null,
    "latency_ttfb_ms": null
  }
]
```

Fields populated only in MITM mode (Phase 5+) will always be `null` in the current
version. `bytes_sent` and `bytes_received` are populated in Phase 1 and will be `0`
in Phase 0 output. The `batch_size` config key controls the maximum number of events
per POST. If not set, the default is 100.

---

## Provider Registry (`providers/providers.toml`)

```toml
# CONTRIBUTING: To add a provider, open a PR adding an entry below.
# Required fields: name, display_name, hostnames
# Please include a source link (docs_url) for any hostname you add.

[[providers]]
name = "anthropic"
display_name = "Anthropic / Claude"
hostnames = ["api.anthropic.com", "claude.ai"]
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

[[providers]]
name = "mistral"
display_name = "Mistral"
hostnames = ["api.mistral.ai"]

[[providers]]
name = "ollama"
display_name = "Ollama (Local)"
hostnames = ["localhost"]
ports = [11434]
tls = false     # No SNI extraction needed - plain TCP
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
| Linux   | `AF_PACKET` raw socket | `socket2` crate + manual filter | No external deps, requires root |
| macOS   | BPF device (`/dev/bpf*`) | `socket2` or direct `libc` syscalls | No external deps, requires root |
| Windows | ETW (Event Tracing for Windows) | `ferrisetw` crate | No driver install needed, built into Windows |

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
    // ETW implementation via ferrisetw
}

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
   c. Runs: ai-ranger enroll --token=tok_abc123 --backend=https://your-instance.com
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

```sql
CREATE TABLE organizations (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,
    slug        TEXT UNIQUE NOT NULL,
    created_at  TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE enrollment_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID REFERENCES organizations(id),
    token_hash  TEXT NOT NULL UNIQUE,   -- SHA256, never store raw token
    label       TEXT,
    created_by  UUID,
    expires_at  TIMESTAMPTZ,
    max_uses    INT DEFAULT 1,
    used_count  INT DEFAULT 0,
    created_at  TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE agents (
    id              UUID PRIMARY KEY,   -- agent_id generated on device
    org_id          UUID REFERENCES organizations(id),
    hostname        TEXT NOT NULL,
    os_username     TEXT NOT NULL,
    os              TEXT NOT NULL,      -- "macos" | "linux" | "windows"
    agent_version   TEXT NOT NULL,
    enrolled_at     TIMESTAMPTZ DEFAULT now(),
    last_seen_at    TIMESTAMPTZ,
    status          TEXT DEFAULT 'active'  -- "active" | "revoked"
);
```

### ClickHouse - events and timeseries

```sql
CREATE TABLE ai_events (
    org_id          UUID,
    agent_id        UUID,
    hostname        String,
    os_username     LowCardinality(String),
    timestamp       DateTime64(3, 'UTC'),
    provider        LowCardinality(String),
    provider_host   String,
    model_hint      LowCardinality(String),
    process_name    LowCardinality(String),
    process_path    String,
    bytes_sent      UInt64,
    bytes_received  UInt64,
    detection_method Enum8('sni'=1, 'dns'=2, 'tcp'=3),
    capture_mode    Enum8('dns_sni'=1, 'mitm'=2)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (org_id, timestamp, agent_id, provider)
TTL timestamp + INTERVAL 1 YEAR;
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
PROTO_SRC   := ranger/v1/events.proto ranger/v1/agent.proto
GEN_RUST    := gen/rust
GEN_PYTHON  := gen/python
GEN_GO      := gen/go

.PHONY: all clean

all:               ## Regenerate all protobuf bindings for all languages
	mkdir -p $(GEN_RUST) $(GEN_PYTHON) $(GEN_GO)
	protoc --rust_out=$(GEN_RUST) $(PROTO_SRC)
	protoc --python_betterproto_out=$(GEN_PYTHON) $(PROTO_SRC)
	protoc --go_out=$(GEN_GO) --go_opt=paths=source_relative $(PROTO_SRC)

clean:             ## Remove all generated files
	rm -rf $(GEN_RUST) $(GEN_PYTHON) $(GEN_GO)
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
	python -c "from app import create_app; create_app()"

run:               ## Run gateway with gunicorn
	gunicorn -c gunicorn.conf.py "app:create_app()"

run-dev:           ## Run in dev mode with auto-reload
	FLASK_ENV=development flask run --port 8080

test:              ## Run tests
	pytest tests/ -v

lint:              ## Lint with ruff and type-check with mypy
	ruff check app/
	mypy app/

clean:             ## Remove cached files
	find . -type d -name __pycache__ -exec rm -rf {} +
	find . -name "*.pyc" -delete
```

### `workers/Makefile`

```makefile
INGEST_BIN  := bin/ingest-worker
API_BIN     := bin/api-server

.PHONY: build run-ingest run-api test lint clean

build:             ## Build all Go binaries
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

Ingest endpoints are served by the Flask gateway. All query and management
endpoints are served by the Go API server. The dashboard never talks to Flask.

```
# Flask Gateway - agent-facing only
POST /v1/ingest              -> receive protobuf EventBatch, enqueue to RabbitMQ
POST /v1/agents/enroll       -> enrollment (token in body, returns org_id)
GET  /v1/agents/providers    -> fetch latest providers.toml
GET  /v1/agents/version      -> check for agent updates

# Go Query API - dashboard and admin facing
GET  /v1/dashboard/overview           -> org-wide summary stats
GET  /v1/dashboard/providers          -> provider breakdown with traffic
GET  /v1/dashboard/users              -> per-user activity table
GET  /v1/dashboard/traffic/timeseries -> hourly/daily traffic by provider
GET  /v1/dashboard/fleet              -> all enrolled agents + status

POST   /v1/admin/tokens       -> create enrollment token
DELETE /v1/admin/tokens/:id   -> revoke token
DELETE /v1/admin/agents/:id   -> revoke agent
```

---

## Dashboard Views

**1. Fleet Overview** - admin landing page
- Total active agents, last-seen distribution
- Org-wide connections today vs. yesterday
- Top providers by connection count (bar chart)

**2. Provider Breakdown**
- Table: Provider | Connections | Unique Users | Bytes Sent | Bytes Received | Trend
- Time series: connections over time, stacked by provider
- Date range filter

**3. User Activity Table**
- Rows: one per (user x provider x process) combination
- Columns: User | Machine | Provider | App | Connections | Data Sent | Data Received | Last Active
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

Services started:

```yaml
# docker/docker-compose.yml
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: ranger
      POSTGRES_USER: ranger
      POSTGRES_PASSWORD: ranger
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./postgres/init.sql:/docker-entrypoint-initdb.d/init.sql

  clickhouse:
    image: clickhouse/clickhouse-server:24
    volumes:
      - clickhouse_data:/var/lib/clickhouse
      - ./clickhouse/config.xml:/etc/clickhouse-server/config.d/config.xml

  rabbitmq:
    image: rabbitmq:3-management-alpine
    ports:
      - "5672:5672"     # AMQP
      - "15672:15672"   # Management UI at localhost:15672
    volumes:
      - ./rabbitmq/definitions.json:/etc/rabbitmq/definitions.json

  gateway:
    build: ../gateway
    ports:
      - "8080:8080"
    environment:
      DATABASE_URL: postgres://ranger:ranger@postgres/ranger
      RABBITMQ_URL: amqp://guest:guest@rabbitmq:5672/
    depends_on: [postgres, rabbitmq]

  workers:
    build: ../workers
    ports:
      - "9090:9090"     # Go query API
    environment:
      DATABASE_URL: postgres://ranger:ranger@postgres/ranger
      CLICKHOUSE_URL: http://clickhouse:8123
      RABBITMQ_URL: amqp://guest:guest@rabbitmq:5672/
    depends_on: [postgres, clickhouse, rabbitmq]

  dashboard:
    build: ../dashboard
    ports:
      - "3000:3000"
    environment:
      VITE_API_URL: http://localhost:9090
    depends_on: [workers]

volumes:
  postgres_data:
  clickhouse_data:
```

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
- Full SNI + DNS capture pipeline
- Provider classifier with `providers.toml` (15-20 providers seeded)
- Process resolver (Linux + macOS; Windows in Phase 4)
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
- Flask gateway: ingest + enroll endpoints
- RabbitMQ: exchange and queue configured via definitions.json
- Go ingest workers: consume from RabbitMQ, write to ClickHouse
- Go query API: basic dashboard endpoints
- gateway/Makefile and workers/Makefile working
- `make dev` brings up the full stack end to end

### Phase 3 - Dashboard MVP (3-4 weeks)
- Fleet overview page
- Provider breakdown page
- User activity table
- Traffic timeseries charts
- Enrollment token generation UI
- dashboard/Makefile working

### Phase 4 - Polish + Windows (2-3 weeks)
- Windows agent + installer (PowerShell)
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

## Key Architectural Decisions (and Why)

| Decision | Chosen | Rejected | Reason |
|---|---|---|---|
| Default capture method | SNI extraction (passive) | TLS MITM proxy | No cert installation, no broken apps, no legal grey area |
| Capture backend | OS-native raw sockets + ETW | libpcap / npcap | Standalone binary, no external C library deps, no driver install |
| MITM mode | Planned Phase 5, opt-in only | Built-in from day 1 | Ship value fast, earn trust first, complex problems deferred |
| Agent language | Rust | Go, Python | Memory safety for a privileged process, single binary, deterministic performance, author learning goal |
| Gateway language | Python + Flask | Go, Node | Thin gateway pattern, battle-tested at scale, matches SentinelOne model |
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

2. **No content inspection in default mode.** DNS/SNI mode reads hostnames and byte
   counts only. It never reads, buffers, or transmits any part of the TLS payload.

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
- Byte volume is a proxy for token usage, not a precise token count
- Provider hostnames change occasionally; registry needs community maintenance
- Requires root on Linux and macOS, elevated service permissions on Windows

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
