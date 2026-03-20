# AI Ranger - Decisions Log

This document records the reasoning behind significant architectural decisions, pivots,
and dead ends in the development of AI Ranger. It is a living document. Every time a
meaningful decision is made or a direction changes, an entry is added here.

The architecture document (ARCHITECTURE.md) describes what the system is.
This document describes why it is that way.

---

## Project Framing

### Why passive network observation instead of TLS interception

The original goal was full visibility into AI usage - prompts, responses, models, token
counts. The technically complete solution would have been a MITM proxy that decrypts
traffic. That was rejected as the default approach for three reasons: it requires
installing a custom CA certificate on every monitored machine, it breaks applications
that use certificate pinning (Cursor, some Electron apps), and it fundamentally changes
the trust relationship with users - you are no longer observing metadata, you are
reading content.

The realization was that the most important question organizations actually need answered
is not "what did the developer say to Claude?" but "which AI tools is my team using, and
how heavily?" That question can be answered entirely from network metadata without any
content inspection. SNI hostnames are transmitted in plaintext before TLS encryption
begins - reading them requires no decryption and no certificates.

MITM mode is planned for Phase 5 as an explicit opt-in for organizations that need
prompt-level visibility. It will never be the default.

### The envelope analogy

The core trust communication for the project: the agent reads the address on the
envelope, not the letter inside. The SNI field in a TLS ClientHello is the network
equivalent of a postal address - visible to every router and firewall between you and
the destination, never private. This framing was established early and carried through
all documentation.

### Scoping down from content to metadata

An early decision was to descope the initial build to provider/model detection only,
deferring prompt and response content to later phases. This was the right call because:
it removed the need for TLS interception entirely from the core product, it produced
a trustworthy tool faster, and it turned out to be the most commercially valuable
signal anyway - organizations largely do not know which AI tools their teams are using.

---

## Language Choices

### Agent: Rust (reconsidered from Go, returned to Rust)

The agent was initially planned in Rust, then reconsidered in favor of Go for community
contribution reasons - Go has a lower barrier to entry and the project is open source.
The final decision returned to Rust for the following reasons: the agent runs as a
privileged process with root/admin access, and Rust's memory safety guarantees are
genuinely valuable in that context. An additional factor was learning Rust through
a real project.

The Go argument was not wrong - it would have produced more contributors. The Rust
decision was made with eyes open to that tradeoff.

### Backend: Python (FastAPI) + Go, not Rust throughout

The backend uses a thin-gateway pattern: a lightweight Python service handles
agent-facing ingest (auth, deserialize, enqueue) and Go workers handle everything
else (async processing, storage writes, dashboard API).

Python was chosen for the gateway because it is battle-tested for this thin-gateway
pattern and familiar to ops teams. FastAPI was chosen over Flask for its native async
support, automatic OpenAPI/Swagger documentation from Pydantic type annotations, and
built-in request validation. Go was chosen for workers because goroutines handle the
concurrent workload well and the ClickHouse and RabbitMQ client libraries are mature.
The rule: no processing logic ever lives in the gateway. The moment a database write
or business logic appears in a FastAPI route, it belongs in Go.

### Wire format: Protobuf

JSON was considered and rejected for agent-to-gateway communication. Protobuf was
chosen because: it is smaller on the wire (matters for agents sending batches every 30
seconds), the schema is enforced across all three languages (Rust agent, Python gateway,
Go workers), and adding a field to AiConnectionEvent is a single .proto change that
propagates everywhere. The generated code for all three languages is committed to the
repo so contributors do not need protoc installed.

The HttpSink was switched from JSON to protobuf encoding in Phase 2. Events are
serialized as a protobuf EventBatch using prost-generated types and sent with
Content-Type: application/x-protobuf.

---

## Infrastructure Choices

### Message queue: RabbitMQ over Kafka and NATS

Kafka was considered and rejected - it is well understood from prior experience but the
operational overhead (Kafka + ZooKeeper/KRaft) is significant for a community open
source tool where `docker compose up` is a hard promise. NATS JetStream was considered
and rejected as too niche - contributors will not be familiar with it. RabbitMQ was
chosen as widely understood, durable, purpose-built, and trivial to run in Docker with
the management UI included. The management UI at localhost:15672 is a genuine benefit
for contributors debugging locally.

### Storage: Postgres + ClickHouse, not a single database

Two databases for two fundamentally different workloads. Postgres handles identity data
(organizations, agents, enrollment tokens) where relational integrity matters. ClickHouse
handles event timeseries where OLAP query performance at scale matters. A single database
would mean either compromising on relational integrity or on analytics performance.
TimescaleDB was considered as an alternative to ClickHouse but rejected - ClickHouse is
faster for this workload, handles billions of rows well, and is free.

### Makefiles everywhere

Every component has a Makefile. This was a deliberate decision to make the project
accessible to contributors without requiring knowledge of any specific build tool. Make
is universal. `make dev`, `make test`, `make lint`, `make proto` work the same way
regardless of which component you are working on. The root Makefile orchestrates all
component Makefiles.

---

## Agent Architecture Decisions

### Standalone first, backend optional

The agent is useful with zero configuration - it captures AI connections and prints JSON
to stdout with no backend required. The backend is an opt-in addition, not a requirement.
This was a deliberate community trust decision: a tool that phones home by default would
not be trusted for a network monitoring agent. The backend URL and enrollment token must
be explicitly configured.

### Output sink abstraction (EventSink trait)

Rather than hardcoding output destinations, every output implements a single EventSink
trait. This enables custom telemetry without forking the agent. Built-in sinks: stdout
(default), file, http (backend), webhook (any HTTPS endpoint). FanoutSink wraps multiple
sinks and sends to all concurrently. A WASM plugin sink is planned for later phases when
community demand justifies the complexity.

### Three-tier provider loading

The provider registry loads in priority order: fetch from providers_url at startup if
reachable, fall back to a local file in the config directory, fall back to the
compile-time bundled copy as a last resort. This means the agent always starts
successfully even when offline, but automatically picks up community provider additions
on every restart when network is available.

### Providers override mechanism: rejected

An admin override file (`providers.override.toml`) was considered to let admins add
private internal endpoints without touching the community registry. This was rejected
because it adds unnecessary complexity - admins who run their own backend simply point
`providers_url` at their own hosted TOML file. That file can contain anything they want.
The override mechanism solves a problem that does not exist given the URL configuration.

### SQLite local buffer

When the HTTP sink is configured, events are written to a local SQLite database
immediately on capture. A background drain loop uploads batches every 30 seconds and
deletes successfully uploaded events. This means network outages and backend restarts
do not lose data. SQLite uses the `bundled` feature to compile SQLite into the binary -
no system SQLite dependency, consistent with the standalone binary requirement.

---

## Packet Capture Decisions

### No libpcap / npcap

The `pcap` crate was used in early Phase 0 development and immediately rejected. It
requires libpcap on Linux/macOS and npcap on Windows to be installed separately on the
target machine. This breaks the standalone binary requirement. npcap also prohibits
static linking in its free license. The replacement: OS-native raw sockets on Linux
(AF_PACKET) and macOS (BPF device), and ETW on Windows.

### Windows capture: SIO_RCVALL + ETW DNS-Client (two pivots)

The Windows capture story involved two significant pivots worth documenting in full.

**Pivot 1: SIO_RCVALL works, but only for IPv4**

The initial Windows implementation used SIO_RCVALL on an AF_INET socket - the standard
Windows approach for raw packet capture. This worked for IPv4 traffic. The problem was
discovered when Anthropic connections were invisible: Anthropic has AAAA records, Windows
prefers IPv6, and SIO_RCVALL on AF_INET6 does not deliver packets. This is a confirmed,
unfixable Microsoft limitation - a Microsoft engineer confirmed "raw socket does not
receive any IPv6 headers."

**Pivot 2: ETW NDIS-PacketCapture cannot be started without netsh**

The natural fix seemed to be ETW Microsoft-Windows-NDIS-PacketCapture, which delivers
raw Ethernet frames at the NDIS layer including IPv6. Research confirmed it works in
principle, but there is a critical requirement: the NdisCap driver must be activated via
private undocumented IOCTLs that `netsh trace start capture=yes` performs internally.
Simply enabling the ETW provider results in a valid session with zero events. Starting
the agent with a `netsh` subprocess dependency was rejected - it creates a system-wide
exclusive session conflict (only one netsh trace session at a time), leaves traces on
disk, and fails if another tool is already tracing.

**Resolution: ETW Microsoft-Windows-DNS-Client**

The correct solution turned out to not need raw packet capture at all. The
Microsoft-Windows-DNS-Client ETW provider (GUID: 1C95126E-7EEA-49A9-A3FE-A378B03DDB4D)
fires Event ID 3008 for every DNS resolution through the Windows DNS client, delivering
the queried hostname and the resolving PID directly. No NdisCap activation, no netsh,
no subprocess dependency - just standard ETW provider consumption via ferrisetw.

This is actually better than raw packet capture for the use case: the hostname arrives
directly without SNI parsing, and PID attribution comes for free. The only gap: applications
that implement their own internal DoH resolver, bypassing the OS DNS client -- currently
primarily browsers (Chrome, Firefox, Edge, Brave) -- cause ETW DNS-Client events to not
fire for those connections. This is a limitation of any application that bypasses the OS
DNS resolver, not a Windows limitation.

Final Windows capture architecture: SIO_RCVALL for IPv4 packets (existing, proven, zero
latency) running in parallel with ETW DNS-Client for hostname visibility across all
protocols including IPv6 (1-3 second ETW buffering latency, acceptable for observability).

### macOS: written blind, MACOS-UNVERIFIED

The project is developed on WSL (Windows). macOS binaries cannot be compiled on
non-Apple hardware because Apple's SDK licensing prevents cross-compilation for any code
touching libc FFI. The macOS-specific code (BPF device capture, proc_pidinfo process
attribution, getifaddrs interface detection) was written based on API documentation and
marked with MACOS-UNVERIFIED comments. The GitHub Actions macOS runner (real Apple
hardware) is the primary compile-test environment for this code.

### Process attribution: lookup-at-capture-time limitation

All three platform process resolvers (Linux /proc/net/tcp, macOS proc_pidinfo, Windows
GetExtendedTcpTable) share the same fundamental timing characteristic: the lookup
happens at the moment the packet is captured. Short-lived child processes may have
already exited by then, causing the connection to resolve to the parent process. This
is why `curl` spawned from PowerShell shows as `powershell.exe` rather than `curl.exe`.
This is documented as expected behavior, not a bug. In practice it does not affect real
AI tool detection since Cursor, Claude Code, Python scripts, and similar tools own their
sockets directly and have longer lifetimes.

---

## Data Model Decisions

### Agent-side dedup via connection_id

**Problem:** A single connection to an AI provider produces duplicate events. On all
platforms, both the DNS response (UDP port 53) and the TLS ClientHello (TCP port 443
with SNI) are captured and classified independently. On Windows specifically, the
SIO_RCVALL path (IPv4 packets) and the ETW DNS-Client path run in parallel, so a
single `curl https://api.anthropic.com` fires two events through two completely
independent capture pipelines.

**Why agent-side, not backend:** Custom webhook sink users receive raw events directly.
If dedup only happened in the backend, webhook consumers would get duplicates with no
clean way to suppress them. The agent is the only place where all output paths converge.

**Why connection_id hash, not a simple time window:** A naive "suppress same hostname
within N seconds" approach would collapse genuinely distinct rapid connections to the
same provider (e.g. parallel API calls from a script). The connection_id hash of
`(src_ip, provider_host, timestamp_ms / 2000)` groups events that represent the same
logical connection attempt while allowing distinct connections in different 2-second
buckets to pass through independently. The hash is included in the event payload so
downstream consumers can also use it for their own dedup or correlation.

**Why 2-second buckets:** DNS resolution and the subsequent TLS ClientHello typically
fire within milliseconds of each other. The worst case is Windows ETW DNS-Client, which
has 1-3 seconds of buffering latency. 2-second buckets handle this without the
heavy-handed feel of 5-second collapsing. The boundary-crossing case (DNS at T=1.999s,
SNI at T=2.001s producing different bucket IDs) is handled by the 5-second cache TTL -
both IDs are in the cache simultaneously and the first-seen wins.

**Why 5-second cache TTL:** Independent of the bucket size. The TTL controls how long
expired entries linger before being swept. 5 seconds is generous enough that even in
the boundary-crossing scenario, both the "early bucket" and "late bucket" entries
coexist in the cache. Sweep happens inline on every `is_duplicate()` call via
`HashMap::retain` - no background thread needed.

**ETW DNS src_ip caveat:** ETW DNS-Client events have no source IP (the event contains
only the queried hostname and PID). The connection_id for ETW events uses an empty
src_ip, which means it will not collide with the SIO_RCVALL SNI event for the same
connection (which has a real src_ip). Cross-pipeline dedup on Windows therefore relies
on the dispatch-loop cache seeing both events within the TTL window - whichever arrives
first wins, the second is suppressed because it has the same (host, bucket) pair after
accounting for the src_ip difference. This is acceptable: in the worst case, an ETW and
SNI event for the same connection both pass through (the ETW one with empty src_ip, the
SNI one with a real IP). This only happens if ETW and SNI produce different connection_ids,
which requires them to land in different 2-second buckets - a narrow edge case.

### bytes_sent and bytes_received: removed

These fields were in the initial design and removed during Phase 1. The reason: the
agent only captures the TLS ClientHello handshake packet. Whatever bytes_sent and
bytes_received would contain is the handshake overhead (a few hundred bytes), not the
actual API traffic. Reporting those numbers would be actively misleading - a dashboard
user would see "128 bytes sent to Anthropic" when in reality they sent a 50,000 token
prompt. Accurate byte counting requires tracking the full TCP session lifecycle, which
is a meaningful increase in complexity. This was deferred to a future phase rather than
shipping misleading numbers. The ClickHouse schema retains the columns (migrations are
expensive) but the agent does not populate them.

### model_hint: deferred from Phase 1 to Phase 5

model_hint was originally marked Phase 1 with the intent to derive a coarse model
family from the provider hostname. After implementation it became clear that no useful
hint can be derived from a hostname or DNS query alone - "api.openai.com" does not
reveal whether the caller is using GPT-4 or GPT-3.5. The actual model name lives in
the HTTP request body, which requires MITM mode (Phase 5) to inspect. The field
remains in AiConnectionEvent (always None) as a Phase 5 placeholder.

### duration_ms: deferred

Same reasoning as bytes_sent/bytes_received. The agent captures a single TLS
ClientHello packet per connection - it does not track the full TCP session
lifecycle. Populating duration_ms would require connection state tracking across
multiple packets, which is a meaningful increase in complexity. The field remains
defined in AiConnectionEvent (always None) so the data model is forward-compatible.
Deferred to a future phase alongside byte counting.

### IP range matching as a third detection method

Three detection methods exist: SNI (primary, most reliable), DNS (fallback, catches
cached-DNS misses), and IpRange (last resort for providers with dedicated IP space).

IpRange was added specifically because Anthropic owns dedicated IP ranges
(160.79.104.0/23 for IPv4, 2607:6bc0::/48 for IPv6) that are not shared CDN space.
When ECH hides the SNI hostname and DoH hides the DNS query, the destination IP is still
visible in the packet IP header and can be matched against Anthropic's CIDR blocks.

This only works for providers with dedicated IP space. OpenAI, Cursor, Copilot, and
Gemini are all behind Cloudflare or other shared CDNs - their IPs are shared with
millions of other websites and cannot be used for matching without catastrophic false
positives. The ip_ranges field in providers.toml is intentionally only set for Anthropic.

### Phase 5 fields in the struct

MITM mode fields (model_exact, token_count_input, token_count_output, latency_ttfb_ms,
payload_ref, content_available) are defined in AiConnectionEvent now but always
default/null. They are excluded from JSON output via skip_serializing_if. Defining them
now means the data model is forward-compatible with Phase 5 without a breaking struct
change. They will reappear in output automatically once populated.

### enrollment_token not persisted in AgentConfig

The architecture originally included enrollment_token in AgentConfig. The code
correctly omitted it. After audit, the docs were updated to match the code. The
reasoning: the token is consumed during enrollment and invalidated. Storing an
invalidated credential after it has been used is a security anti-pattern. The agent
authenticates all subsequent requests with agent_id, not the original token.

---

## Community and Open Source Decisions

### Pure open source, no commercial tier

The project is Apache-2.0 throughout. An open source / commercial split (e.g. BUSL
for the backend) was considered and rejected. The decision: pure community tool, no
held-back features, no commercial angle. This simplifies everything - one license,
one repo, no contributor concerns about contributing to a commercially held codebase.

### providers.toml as the community contribution magnet

The provider registry is a standalone TOML file in the repo root that any contributor
can update without writing code. CI validates the schema on every PR. This was
intentionally designed to be the easiest possible contribution path - adding a new AI
provider is a one-minute edit. The community maintaining this file is more valuable than
any single engineer maintaining it.

### Monorepo

Agent, gateway, workers, dashboard, providers, and docs all live in one repo. The
alternative (separate repos per component) would have been cleaner in some ways but
creates a contributor tax - issues, PRs, and context are split across multiple places.
For a community project trying to build momentum, one repo with one issue tracker wins.

---

## ECH and DoH Detection Limitations

### The ECH + DoH ceiling

ECH (Encrypted Client Hello) and DoH (DNS over HTTPS) are general TLS and DNS privacy
features that any application can implement. Browsers (Chrome, Firefox, Edge, Brave) are
the primary current deployers of both:
- ECH: the real SNI hostname is encrypted, the outer ClientHello contains a dummy hostname
- DoH: DNS queries go to dns.google:443 or cloudflare-dns.com:443 over HTTPS, bypassing
  UDP port 53

These eliminate both detection signals the agent relies on for traffic from applications
that implement them. This is not a bug - it is these privacy features working as designed.
The agent reliably detects CLI tools, SDKs, and desktop AI apps (Cursor, Claude Code,
Copilot) because those do not currently implement ECH.

The research document BROWSER-DETECTION-OPTIONS.md covers all investigated approaches.
The honest conclusion: passive network-level detection has a hard ceiling with applications
that deploy ECH+DoH. MITM mode (Phase 5) is the complete answer for organizations that
need visibility into ECH-protected traffic. ETW DNS-Client on Windows provides partial
coverage for applications that use the system DNS resolver rather than their own internal
DoH.

---

## Deferred Decisions

### WASM plugin sink

Planned but not yet implemented. The EventSink trait is designed to accommodate a
WasmPluginSink that loads a .wasm file for custom event transformation logic. Using
wazero (Go-native WASM runtime) rather than wasmtime. Deferred until community demand
justifies the complexity.

### Ollama port-based detection

providers.toml includes ports = [11434] and tls = false for Ollama. The ProviderEntry
struct does not implement these fields - they are parsed but silently ignored. Ollama
runs locally without TLS, so SNI extraction does not apply. Proper detection would
require a TCP heuristic matching connections to localhost:11434. Deferred.

### Linux eBPF getaddrinfo uprobe

The equivalent of Windows ETW DNS-Client for Linux: eBPF uprobes on getaddrinfo() in
libc, intercepting DNS resolutions at the application level regardless of transport.
This was identified in BROWSER-DETECTION-OPTIONS.md as Option 2 for Linux. Not yet
implemented. Would be the Linux counterpart to the Windows ETW DNS-Client path.

### macOS DNS interception

NEDNSProxyProvider (Network Extension framework) would be the macOS equivalent of
Windows ETW DNS-Client. It requires a System Extension, Apple Developer notarization,
and explicit user approval in System Preferences. Significantly more complex than the
Windows or Linux equivalents. Deferred to a later phase.

### ETW DNS process name resolution for short-lived processes

The ETW DNS-Client path provides a PID with each event, but by the time the ETW callback
fires (1-3 seconds of buffering latency), short-lived processes like curl have already
exited. Three approaches were attempted and rejected:

1. **PID→name cache** populated by the SIO_RCVALL path: does not help for IPv6-only
   connections because SIO_RCVALL never sees them, so the cache is never populated.
2. **CreateToolhelp32Snapshot fallback**: takes a point-in-time snapshot of all processes
   when the ETW callback fires. Still too late - curl has already exited by then.
3. **Background process list refresh thread**: continuously snapshots the process list
   every N seconds. Rejected as over-engineering - adds a background thread, memory
   overhead, and complexity for a marginal improvement on short-lived CLI tools.

The decision: accept `"unknown"` for the process name when the process has exited before
the name can be resolved. The PID in `process_pid` is always accurate regardless. The
process name resolves correctly when the process is still alive, which covers all real
AI tools (Cursor, Claude Code, Copilot, Python scripts). Only sub-second CLI
invocations like `curl` are affected, and those are testing scenarios, not real-world
AI tool usage. Using `"unknown"` rather than `"pid:N"` keeps the output clean - the
PID is already in its own field and does not need to be repeated in the name.

### Windows ETW NDIS-PacketCapture

Investigated as the IPv6 fix for Windows and rejected in favor of ETW DNS-Client.
Documented here for completeness: NDIS-PacketCapture requires private undocumented
IOCTLs that netsh activates internally. The provider cannot be started programmatically
without shelling out to netsh. This creates a system-wide session conflict and other
operational problems. ETW DNS-Client solves the actual problem (hostname visibility
across all protocols) more cleanly.

### os_type field on AiConnectionEvent

`os_type` was added to AiConnectionEvent to enable dashboard segmentation by operating
system - whether AI tool adoption differs between Linux, macOS, and Windows developers.
`std::env::consts::OS` was chosen as the source because it is a compile-time constant
that returns clean lowercase strings ("linux", "macos", "windows") with no runtime
detection needed. The value is computed once at startup and passed into every event
construction path.

The field belongs on AiConnectionEvent rather than only in AgentConfig because events
should be self-describing. Webhook consumers who receive raw events do not have access
to agent enrollment metadata. Embedding os_type in each event means any downstream
system can filter or aggregate by OS without a join to the agents table.

---

## Pre-Phase 2 Maturity Pass

A codebase-wide audit was conducted before Phase 2 to bring the agent to enterprise-grade
quality. The audit covered magic numbers, configuration values, error handling, naming,
dead code, and documentation. Three decisions from this pass are worth recording.

### Constants organization: near usage, not a global file

Named constants were organized close to where they are used rather than in a single
top-level `constants.rs` file. Protocol constants shared across capture modules went into
`capture/constants.rs` because pcap.rs, sni.rs, and dns.rs all reference the same
protocol values (TLS record types, DNS flags, Ethernet ethertypes). Operational constants
(drain intervals, batch sizes, timeouts) went at the top of the file that owns them -
drain timing in main.rs, HTTP batch size in http.rs, dedup bucket width in dedup.rs.

A global constants file was considered and rejected. It creates artificial coupling between
unrelated modules - changing a DNS constant should not require touching the same file as
a change to the HTTP batch size. This is inconsistent with how the Rust standard library
and major crates organize their constants, and it produces a file that grows without bound
as the project adds features. The rule: a constant lives in the module that gives it
meaning.

### Tuneable operational values exposed via config.toml, not environment variables

Values that admins may want to tune without recompiling (drain interval, batch sizes,
fetch timeout) were exposed as optional fields in the `[agent]` section of config.toml.
The named constants remain as compile-time fallback defaults; config values take
precedence when present via `Option<u64>` fields and `.unwrap_or(CONSTANT)`.

Environment variables were considered and rejected. They are a common pattern in
twelve-factor Python and Go services but non-idiomatic for Rust CLI tools. The agent
already has config.toml as its established configuration entry point, and adding a parallel
env-var configuration path would create two ways to set the same value with unclear
precedence. One configuration mechanism is better than two.

### All .unwrap() and .expect() calls are intentional

The audit reviewed every `.unwrap()`, `.expect()`, and `panic!()` call outside test code.
All were found to be correct:

- `providers.rs:118` `.expect("bundled providers.toml must be valid")` - the bundled file
  is compiled in via `include_str!` and validated by the `bundled_toml_parses` test. If
  this panics, the binary itself is corrupt and cannot operate. Panic is correct.
- `providers.rs:133,159` `.expect("providers not initialized")` - these are programming
  errors (forgetting to call `init()` before `classify()`). This is an invariant violation,
  not a recoverable runtime condition. Panic is correct.
- `main.rs:150` `sinks.into_iter().next().unwrap()` - guarded by `sinks.len() == 1` on
  the same line. Structurally unreachable.
- `pcap.rs:480` `CString::new(...).unwrap()` - format string `/dev/bpf{i}` cannot contain
  null bytes. Structurally unreachable.

No changes were made to error handling. The codebase does not have hidden panic paths.

---

## Pre-Phase 2 Code Quality Refactor

A structural refactor was done to bring the codebase to maintainable shape before Phase 2
adds backend components. Four decisions are worth recording.

### Business logic extracted from main.rs

main.rs was 455 lines and mixed wiring (config loading, channel setup, task spawning) with
business logic (packet classification, process resolution, event construction, buffer
draining, enrollment). A new contributor reading main.rs had to mentally separate "how
things are connected" from "what things do." The fix was to extract each concern into its
own file:

- `pipeline.rs` - packet-to-event transformation (classify hostname/IP, resolve process,
  compute connection_id, construct event). This is the core detection pipeline.
- `buffer/drain.rs` - background drain loop with exponential backoff. Owns the drain
  constants (interval, batch size, backoff multiplier).
- `identity/enroll.rs` - enrollment flow (create AgentConfig, save, exit) and identity
  loading. Owns the enrollment CLI interaction.
- `output/mod.rs` - `build_sinks()` moved here since it constructs output sinks from config.
- `classifier/mod.rs` - `fetch_providers_url()` moved here since it fetches provider data.

main.rs dropped to 180 lines of pure wiring. Every line is either a constant, a CLI struct
field, or a call to a function defined elsewhere. No business logic remains.

### AiConnectionEvent derives Clone instead of manual clone_event()

The HttpSink had a 26-line `clone_event()` function that manually copied every field of
AiConnectionEvent. This function had to be updated every time a field was added (as
happened with `os_type`). The justification for not deriving Clone was "to keep it
lightweight in the common case" - but Clone on a struct of Strings is zero-cost when not
called. The derive only generates code; it does not change the struct's size, layout, or
runtime behavior in paths that do not clone. The manual function was deleted and replaced
with `event.clone()`.

### AiConnectionEvent::new() constructor centralizes Phase 5 defaults

Event construction was duplicated in two places: the main capture callback in main.rs and
the ETW DNS callback in etw_dns.rs. Both constructed the full 20-field struct with
identical Phase 5 defaults (6 fields always set to None/false). Every new field required
editing both locations. The constructor accepts only the 14 fields that genuinely vary and
fills Phase 5 defaults internally. This means adding a Phase 5 field is a one-line change
in the constructor, and adding a non-Phase-5 field is a compiler error at every call site
(which is the correct behavior - the caller must provide a value).

### pub(crate) instead of pub for internal items

The agent is a binary crate, not a library. The Rust compiler does not enforce the
pub/pub(crate) distinction in binaries - both compile identically. The distinction was
made anyway because it communicates intent to contributors: `pub(crate)` means "used by
other modules in this crate but not part of a public interface." When Phase 2 adds the
proto-generated types as a separate crate, items that are truly public will need to be
`pub` while internal implementation stays `pub(crate)`. Making the distinction now avoids
a retroactive audit later. Constants in capture/constants.rs, batch size defaults in
output sinks, the dedup bucket constant, and internal types like ProviderRegistry were
all narrowed from `pub` to `pub(crate)`.

---

## Phase 2 Backend Decisions

### ORMs for Postgres, plain SQL for ClickHouse

All Postgres access uses ORMs. The Python gateway uses SQLAlchemy 2.0 async with Alembic
for versioned migrations. The Go workers use GORM with struct tags for schema definition.
Raw SQL against Postgres is not permitted except in Alembic migration files and GORM model
definitions.

Raw SQL was rejected for Postgres because schema changes become untracked and error-prone
across environments. With Alembic, every schema change is a versioned migration file that
runs automatically on startup. Contributors never need to manually apply SQL or wonder
whether their local database matches production. GORM was chosen for Go because it is the
idiomatic Go ORM and supports auto-migration for development.

ClickHouse is the intentional exception. No mature ORM exists for ClickHouse in Go. The
ClickHouse schema is append-only and rarely changes. The `clickhouse-go` driver with plain
SQL and named query constants is the correct approach. The schema is defined in
`docker/clickhouse/init.sql` and loaded on container startup.

The SQLAlchemy models in `gateway/models/orm.py` are the source of truth for the Postgres
schema. The GORM structs in `workers/internal/models/models.go` must mirror them exactly.

### bytes_sent and bytes_received removed from ClickHouse schema

The original DECISIONS.md entry for bytes_sent/bytes_received said "The ClickHouse schema
retains the columns (migrations are expensive)." That reasoning applied to a hypothetical
already-deployed schema. Since the ClickHouse schema is being created for the first time
in Phase 2, there is no migration cost. The columns were removed from the initial schema.
When TCP session tracking is implemented in a future phase, adding columns back is a
trivial ALTER TABLE.

### ClickHouse detection_method enum corrected

The ClickHouse detection_method enum was `Enum8('sni'=1, 'dns'=2, 'tcp'=3)` which did not
match the actual detection methods in the agent (Sni, Dns, IpRange, TcpHeuristic). The
agent has used IpRange since Phase 1 and TcpHeuristic is reserved for future Ollama
detection. The enum was corrected to `Enum8('sni'=1, 'dns'=2, 'ip_range'=3,
'tcp_heuristic'=4)` to match reality.

---

## Considered but Deferred

### eBPF for Linux packet capture

eBPF was considered as a replacement for AF_PACKET raw sockets on Linux. Advantages:
kernel-space filtering without full packet copies to userspace, potential to reduce
privilege requirements via CAP_BPF rather than full root, better performance at high
packet rates. Rejected for now because: AF_PACKET already works correctly and performs
well for this workload, eBPF would add a third capture backend (alongside classic BPF
for macOS and SIO_RCVALL for Windows) increasing maintenance surface, and the kernel
version requirement (4.18+ minimum, 5.x+ for portable binaries via CO-RE) would exclude
some deployment targets. Revisit if Linux performance or privilege requirements become
a real pain point in production.

---

## Phase 2 Hardening Decisions

### pydantic-settings for Python configuration management

The gateway uses `pydantic-settings` with a `Settings` class in `gateway/config.py`
for all environment variable loading. Alternatives considered:

- **Scattered `os.environ.get()` calls**: rejected because they are not type-checked,
  validation happens at call time (not startup), and the same default value gets
  duplicated across files.
- **python-decouple or environs**: rejected because pydantic-settings integrates
  naturally with FastAPI's dependency injection, provides type coercion and validation
  at startup, and the gateway already depends on pydantic for request models.
- **A plain dataclass**: rejected because it would require manual validation code
  that pydantic-settings provides for free.

The Go workers use a `Config` struct in `workers/internal/config/config.go` loaded
via `config.Load()` at startup. The struct is passed to all components. No
`os.Getenv` calls exist outside the config package. This is the idiomatic Go approach
— explicit dependency passing rather than global lookups.

### k8s-ready design decisions

All backend services were designed to be Kubernetes-compatible from the start, even
though k8s manifests are not yet provided. The specific decisions:

- **Stateless application pods**: no local state, no session affinity required.
  Gateway, ingest-worker, and api-server can be scaled horizontally.
- **All config from environment variables**: maps naturally to k8s ConfigMaps and
  Secrets. No config files need to be mounted at runtime.
- **Health endpoints**: `GET /health` on gateway (8080) and api-server (8081) for
  readiness and liveness probes. Returns 200 with no auth.
- **Graceful SIGTERM handling**: all services drain in-flight work within a
  configurable timeout (`SHUTDOWN_TIMEOUT_SECS`, default 30s).
- **No host filesystem dependencies**: Docker images are self-contained.

These decisions add zero complexity to the Docker Compose setup but make the
eventual k8s migration straightforward.

### .env.example committed, .env gitignored

`.env.example` is committed to the repo with safe local development defaults and
documentation for every variable. `.env` (the actual file with real credentials)
is in `.gitignore` and must never be committed.

This is the standard approach for open source projects with credentials:
contributors can `cp .env.example .env` and immediately have a working local
environment, while production credentials are never exposed in version control.
Docker Compose loads `.env` automatically via the `env_file` directive.

### Separate Dockerfiles per Go service

Each Go service (`cmd/ingest/` and `cmd/api/`) has its own Dockerfile rather than a
single shared Dockerfile with multi-target stages. The build context is the repo root
in both cases so shared `internal/` packages and `proto/gen/go` are accessible.

This was chosen over the shared multi-target approach because:
- Each service can be built, tagged, and pushed independently in CI.
- Each image contains exactly one binary — no ambiguity about which binary runs.
- In Kubernetes, each service maps to a separate Deployment with its own image,
  scaling, and resource limits. Separate images are a prerequisite for this.
- The shared `Dockerfile.dev` with CompileDaemon handles the dev hot-reload case
  where both services need the Go toolchain.

### Two-layer integration test strategy

Integration tests use two layers: real agent tests first (highest confidence), with
synthetic event tests as a reliable fallback for environments where raw socket capture
is unavailable.

**Real agent tests** start the actual compiled agent binary, trigger real network
traffic (curl to api.openai.com), and verify events flow through the full pipeline
to ClickHouse. These require root for raw socket capture and are automatically skipped
when not running as root. In CI (GitHub Actions Linux runners are root by default),
these run on every push.

**Synthetic event tests** send protobuf EventBatch messages directly to the gateway
HTTP endpoint. They test the full pipeline from HTTP ingest through RabbitMQ to
ClickHouse without requiring a real agent, root access, or network traffic. These
run on all environments and provide coverage when real agent tests cannot run.

**Windows integration tests** were initially deferred but later implemented. The
Windows CI job (`integration-tests-windows`) runs the standalone agent tests
(`captures_sni` and `stdout_mode`) on a `windows-latest` GitHub Actions runner
with Administrator privileges. These tests validate the Windows detection path
(SIO_RCVALL + ETW DNS) by starting the agent, triggering real AI provider traffic,
and verifying JSON events are captured. The enrollment test is excluded because
GitHub Actions Windows runners cannot run Linux containers (no Docker Compose
for the backend stack). Backend-dependent tests run on the Linux CI runner.

The test file was made cross-platform: `is_root()` uses
`ctypes.windll.shell32.IsUserAnAdmin()` on Windows, process termination uses
`proc.terminate()` instead of SIGTERM, and AI provider traffic is triggered via
`urllib` instead of shelling out to `curl`. The job is marked
`continue-on-error: true` until proven stable on Windows runners.

---

## Phase 2 Architecture Fixes

### Gateway ingest route: last_seen_at write removed

An architecture audit found that `gateway/routers/ingest.py` was updating
`agent.last_seen_at` in Postgres before returning 200. This violated the core
architecture rule: the gateway does no database writes on ingest. Its only
responsibilities are token verification, protobuf deserialization, RabbitMQ
publishing, and returning 200.

The `last_seen_at` update was moved exclusively to the Go ingest worker, which
already owns all post-queue processing. The gateway ingest route now has zero
database imports and zero database writes. The `datetime`, `AsyncSession`, and
`get_db` imports were removed entirely.

### Decoupling ClickHouse and Postgres writes in the ingest worker

The original Go ingest worker ran the Postgres `last_seen_at` update inside
`clickhouse.go` after `insertClickHouseEvents()` succeeded. If the ClickHouse
write failed, the function returned early and the Postgres update never executed.
This meant a ClickHouse outage would silently prevent agent activity tracking
even though the agent was clearly sending data.

The fix separated the two concerns into independent writer structs:
`ClickHouseWriter` (events) and `PostgresWriter` (agent metadata). The consumer
in `rabbitmq.go` calls both independently. A ClickHouse failure logs an error and
nacks the message but does not prevent the Postgres update. A Postgres failure
logs an error but does not affect event processing or message acknowledgment.
`workers/internal/writer/postgres.go` was promoted from a placeholder stub to a
full implementation with its own struct, constructor, and `UpdateAgentLastSeen`
method.

---

## Phase 3 Dashboard Decisions

### Environment-aware authentication: JWT with dev bypass

JWT was chosen over OAuth/OIDC and session cookies for dashboard authentication.
The reasoning:

- **No external dependency.** OAuth/OIDC requires an identity provider (Auth0,
  Keycloak, Okta). For a self-hosted internal ops tool, adding an IdP dependency
  defeats the simplicity of `docker compose up`. JWT is self-contained: the Go
  API server signs and validates tokens using a secret from an environment variable.
- **Self-hosted organizations control their own credentials.** There is no SaaS
  account, no third-party token exchange, and no network call during authentication.
- **Stateless API.** JWT tokens are validated without a database lookup on every
  request. The Go middleware decodes the token, checks the signature and expiry,
  and passes through. No session table needed.

Auth is **bypassed entirely in development** (`ENVIRONMENT=development`). The Go
auth middleware is a no-op that passes all requests through. The dashboard renders
without a login screen. This preserves the local developer experience unchanged:
`make dev` starts the full stack with no credentials, no login page, and no tokens
to manage. The dev environment must remain zero-friction.

In production (`ENVIRONMENT=production`), every Go Query API request except
`/health` and `/v1/auth/*` requires a valid `Authorization: Bearer <jwt>` header.
The dashboard shows a login page and stores the JWT in memory (not localStorage)
with a refresh token flow.

### Single admin user instead of user management

A full user management system with multiple accounts, invite flows, RBAC, and
password resets is out of scope for Phase 3. The reasoning:

- **AI Ranger is an internal ops tool.** The dashboard is used by a small number
  of admins who manage agent enrollment and review AI usage data. It is not a
  multi-tenant SaaS product with per-user access control.
- **Premature complexity.** Building a users table, Alembic migration, invite
  emails, password reset flow, and role system for a tool that currently has one
  class of user (admin) is over-engineering.
- **Environment variables are the right primitive.** `ADMIN_EMAIL` and
  `ADMIN_PASSWORD` (plaintext) are set once during deployment. The password
  is hashed via bcrypt once at startup and the plaintext is not retained in
  the config struct. The login endpoint compares against the in-memory hash.
  No database query, no user table, no migration.
- **Plaintext password in the environment, not a pre-generated hash.** bcrypt
  hashing at login time is intentionally expensive and only appropriate for
  the initial verification. Subsequent requests use JWT validation which is
  cheap. Requiring admins to pre-hash their password adds unnecessary friction
  and tooling requirements (htpasswd, bcrypt CLI) with no security benefit
  since transport is HTTPS and the environment variable is already a secret.
- **Reversible.** If real demand emerges for multi-user access (e.g. read-only
  viewers, per-team scoping), a users table can be added in a future phase. The
  JWT infrastructure built now carries forward unchanged: the only change is
  where the credential check happens (database lookup instead of env var
  comparison).

### nginx as single ingress point

All external traffic routes through a single nginx reverse proxy. The
alternatives considered:

- **Direct port exposure for each service**: simpler in development but
  creates operational complexity in production (multiple ports to manage,
  TLS on each service, CORS between dashboard and API).
- **Traefik or Caddy**: more features but heavier dependencies. nginx is
  universally understood, available as a 5MB Alpine image, and handles the
  three routing rules required without any dynamic configuration.
- **API gateway (Kong, Ambassador)**: over-engineered for an internal tool
  with three routes.

nginx provides: single TLS termination point, CORS elimination (dashboard and
API share the same origin), clean URL structure (`/api/` for the query API,
`/ingest/` for agent traffic), and SPA routing fallback. In development, direct
ports remain exposed on each service for debugging and integration tests.

### shadcn/ui with shadcn charts over Tremor

Tremor was the original charting library choice but was rejected because it
requires Tailwind v3 and has not been updated for v4. shadcn/ui provides a
complete component system (buttons, cards, tables, inputs, sidebar) and a
charts module built on Recharts, all native to Tailwind v4. Using one
ecosystem (shadcn) instead of two (shadcn + Tremor) reduces dependency
conflicts and maintenance surface.

### Two-page KISS dashboard design

The dashboard has two primary pages: Dashboard (analytics) and Events (raw
search). Fleet management and token management are in an Admin section accessible
from a secondary link at the bottom of the sidebar.

The original plan had six separate pages: Overview, Fleet, Providers, Users,
Events, and Tokens. This was reduced to two because:

- **The primary user opens the dashboard, understands the picture in thirty
  seconds, and closes it.** Six pages with distinct navigation creates a
  "hunt for the right page" experience. A single analytics Dashboard with
  stat cards, a timeseries chart, and ranked lists gives the full picture
  at a glance.
- **Provider and user breakdowns are not separate pages.** They are ranked
  lists on the Dashboard that respond to time range and provider filter
  selections. Clicking a provider in the chart legend filters both lists.
  This "deep dive" interaction replaces three separate pages.
- **Events is the power-user view.** Raw event search with full-text search
  across all fields, sortable columns, pagination, and expandable row detail.
  This serves the "show me exactly what happened" use case.
- **Fleet and token management are operational utilities, not analytics.**
  They are used during setup and troubleshooting, not daily. Moving them to
  Admin keeps the primary navigation minimal and focused.

### src_ip added to ClickHouse schema in Phase 3

The `src_ip` column was present in the protobuf `AiConnectionEvent` schema and
written by the agent, but was missing from the ClickHouse `ai_events` table
created in Phase 2. The Go ingest writer did not include it in the INSERT
statement, and the column did not exist in `docker/clickhouse/init.sql`.

This was discovered when building the Events page, which needs to display the
source IP in the expanded row detail and support search across it. The column
was added to `init.sql`, the writer, the events query, and the dashboard.

ClickHouse does not use Alembic or any migration framework. The schema is loaded
from `init.sql` on first container start only. Applying schema changes requires
destroying the ClickHouse volume and recreating it: `make dev-reset`. This is
acceptable for development. In production, the equivalent is `ALTER TABLE
ai_events ADD COLUMN src_ip String` run manually or via a deployment script.
The `init.sql` file is the source of truth for the ClickHouse schema.

### Ollama "localhost" hostname removed to fix false positives

The Ollama provider entry in `providers.toml` had `hostnames = ["localhost"]`.
Because the classifier matches hostnames without considering port, every
connection to localhost — including the agent's own HTTP traffic to the gateway
ingest endpoint — was classified as an Ollama event. This produced a steady
stream of false positives whenever the agent was running with a backend.

The fix was to clear the hostname list (`hostnames = []`) while keeping the
Ollama entry and its `ports = [11434]` field. Ollama detection will become
functional once port-based matching is implemented in the classifier. Until
then, Ollama connections will not be detected, which is preferable to
misclassifying all localhost traffic.

---

## Phase 3 Hardening (Pre-Release Audit)

### ClickHouse queries switched from string interpolation to parameterized queries

The ClickHouse store (`workers/internal/store/clickhouse.go`) originally used
`fmt.Sprintf` with a custom `escapeSingleQuote()` helper to build queries with
user-supplied search terms and provider filters. This was a SQL injection risk —
the escape function only handled single quotes and could miss edge cases.

All queries were rewritten to use `clickhouse-go`'s native `?` parameter binding.
The `escapeSingleQuote` function was deleted entirely. Table names and validated
sort column names still use `fmt.Sprintf` (they are constants, not user input).
LIMIT clauses were added to `GetProviders` (50) and `GetTrafficTimeseries` (1000)
to prevent unbounded result sets.

### RabbitMQ credentials template approach

RabbitMQ credentials were previously defined in two places that had to be manually
synchronized: `definitions.json` (hardcoded `guest:guest`) and `.env` env vars
(`RABBITMQ_DEFAULT_USER`, `RABBITMQ_DEFAULT_PASS`). When `load_definitions` is set
in `rabbitmq.conf`, RabbitMQ ignores the `RABBITMQ_DEFAULT_USER/PASS` env vars entirely.
A deployer who changed only the env vars would believe they had secured RabbitMQ but
the broker would still use the hardcoded credentials.

The fix: `definitions.json.template` with `${RABBITMQ_DEFAULT_USER}` and
`${RABBITMQ_DEFAULT_PASS}` placeholders, a custom entrypoint script that runs
`envsubst` to generate `definitions.json` at startup, and mounting this in
docker-compose. The old `definitions.json` with hardcoded credentials was deleted.
Credentials are now controlled entirely by the env vars in `.env`.

### Event batching tuned for sub-second dashboard latency

The original batching defaults were designed for a hypothetical high-volume
enterprise fleet: 30-second drain interval, 100-event HTTP batch size. In practice,
a developer generating a handful of events during local testing had to wait up to
30 seconds to see anything in the dashboard. This made the development loop feel
broken even though the pipeline was working correctly.

New defaults: the SQLite buffer drains every 1 second (configurable via
`drain_interval_secs` in config.toml), the HTTP batch flushes at 10 events instead
of 100, and a 500ms background flush timer sends any events sitting in the batch
buffer regardless of count. Under typical developer load, events appear in the
dashboard within 1 second of capture.

The tradeoff is more frequent HTTP requests to the gateway. A developer machine
generating 10-50 events per minute produces at most one request per second, which
the gateway handles trivially. For high-volume deployments that want larger batches
and less frequent uploads, the drain interval and batch size are configurable.

### Tag-based releases over continuous releases

Releases are triggered by pushing a semantic version tag (`v*.*.*`), not by
merging to main. Merging to main only runs CI. A release is a deliberate,
version-tagged event.

The alternative was continuous releases (build and publish on every merge to
main). This was rejected because: not every merge is release-worthy (doc fixes,
test additions, and internal refactors do not need a new binary), continuous
releases make it hard to communicate "this is a stable version you should
upgrade to," and the release cadence should be controlled by the maintainer
rather than the merge frequency.

The `make release` target enforces the workflow: clean working tree, on main,
valid semver tag, push to trigger the build. The release workflow builds all 5
platform binaries, strips them, packages as archives, generates SHA256
checksums, and creates a GitHub release with all assets attached.

---

## Phase 4 Decisions

### Service installers: systemd / launchd / Windows Service

Each platform gets a one-command install script that downloads the binary,
enrolls with the backend, and registers a background service. The alternatives
considered:

- **Manual binary management** (download, place, write service file by hand):
  rejected because it is error-prone and users forget to start the agent, it
  does not survive reboots, and enrollment is a separate manual step. The
  installer script makes deployment a single command.
- **Package managers** (apt/brew/choco): rejected because they add packaging
  maintenance overhead (deb/rpm spec files, Homebrew formula, Chocolatey
  nuspec) for a project that already publishes standalone binaries. Package
  manager support can be added later without changing the install scripts.
- **Container-based agent**: rejected because the agent needs raw socket access
  to the host network stack, which is awkward from inside a container and
  defeats the purpose of lightweight host-level observability.

systemd was chosen for Linux because it is the standard service manager on all
modern distributions. launchd was chosen for macOS because it is the only
supported daemon management system. Windows Service was chosen because it
provides proper lifecycle management (start, stop, restart, status) through
the Service Control Manager. The `windows-service` Rust crate adds native
SCM integration with minimal code.

### Update by reinstall, not auto-update

The update scripts stop the service, download the latest binary, verify the
checksum, replace the binary, and restart the service. The agent does not
auto-update.

Auto-update was rejected because: it requires a polling mechanism that conflicts
with the zero-call-home principle, organizations need to control when updates
happen (change windows, compliance), and silent binary replacement is a supply
chain risk. Explicit update scripts are transparent and auditable. The admin
runs the script when they choose to update.

### Webhook-only alerting (not email, Slack, or PagerDuty)

When a new AI provider is detected for the first time in an organization, the
ingest worker fires a webhook POST to a configured URL. Direct integrations
with email, Slack, PagerDuty, and other notification services were considered
and rejected.

- **Email**: requires SMTP configuration, template management, and delivery
  reliability handling. Not worth the complexity for a self-hosted tool.
- **Slack/PagerDuty/OpsGenie**: requires API tokens, OAuth flows, and
  per-service integration code. Each service has its own API contract.
- **Webhooks**: universal. They work with Slack (incoming webhooks),
  PagerDuty (events API), Datadog, OpsGenie, Zapier, and any custom
  system. One integration point covers all downstream services.

The webhook approach pushes the integration problem to the receiving end,
which already has tooling for it. A Slack incoming webhook URL, a PagerDuty
events API endpoint, or a custom Lambda function all accept HTTP POSTs.

### ON CONFLICT DO NOTHING for known_providers race safety

Two concurrent ingest workers processing batches that contain the same new
provider could both attempt to insert into known_providers simultaneously.
Using `INSERT ... ON CONFLICT (org_id, provider) DO NOTHING` ensures only one
row is created. Only the inserter that wins the race fires the webhook. The
loser sees `RowsAffected == 0` and skips silently. This avoids duplicate
webhook deliveries without requiring external locking or advisory locks.

### Windows Service via windows-service crate

The agent binary must behave as a native Windows Service to be registered with
`New-Service` and respond correctly to `Stop-Service`. The alternatives:

- **NSSM** (Non-Sucking Service Manager): rejected because it is an external
  dependency. The agent is supposed to be a standalone binary with zero
  external dependencies.
- **sc.exe without SCM protocol**: rejected because the agent would not respond
  to stop signals, making `Stop-Service` fail and requiring `taskkill`.
- **windows-service crate**: adds approximately 40 lines of boilerplate to
  implement the SCM dispatcher, start/stop handling, and status reporting.
  The crate is well-maintained and maps SCM stop requests to process exit,
  which matches the existing Ctrl+C shutdown path. The module is compiled
  only on Windows via `#[cfg(windows)]`.
