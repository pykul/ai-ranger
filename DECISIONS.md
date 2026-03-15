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
genuinely valuable in that context. The author also wanted to learn Rust, and building
a real project is the best way to do that.

The Go argument was not wrong - it would have produced more contributors. The Rust
decision was made with eyes open to that tradeoff.

### Backend: Python (Flask) + Go, not Rust throughout

The backend follows a pattern from SentinelOne where the author previously worked: a
thin Python/Flask gateway handles agent-facing ingest (auth, deserialize, enqueue) and
Go workers handle everything else (async processing, storage writes, dashboard API).

Python was chosen for the gateway because it is battle-tested for this thin-gateway
pattern and familiar to ops teams. Go was chosen for workers because goroutines handle
the concurrent workload well and the ClickHouse and RabbitMQ client libraries are
mature. The rule: no processing logic ever lives in Flask. The moment a database write
or business logic appears in a Flask route, it belongs in Go.

### Wire format: Protobuf

JSON was considered and rejected for agent-to-gateway communication. Protobuf was
chosen because: it is smaller on the wire (matters for agents sending batches every 30
seconds), the schema is enforced across all three languages (Rust agent, Python gateway,
Go workers), and adding a field to AiConnectionEvent is a single .proto change that
propagates everywhere. The generated code for all three languages is committed to the
repo so contributors do not need protoc installed.

Note: the HttpSink currently sends JSON with a comment marking it for protobuf in
Phase 2. The protobuf infrastructure is in place but the HTTP transport has not been
switched yet.

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
directly without SNI parsing, and PID attribution comes for free. The only gap: browsers
that use their own internal DoH resolver (Chrome, Firefox, Edge, Brave) bypass the
Windows DNS client entirely, so ETW DNS-Client events do not fire for those connections.
This is a browser-category limitation, not a Windows limitation.

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

## Browser Detection Limitations

### The ECH + DoH ceiling

Modern browsers (Chrome, Firefox, Edge, Brave) deploy two privacy features that defeat
passive network observation simultaneously:
- ECH (Encrypted Client Hello): the real SNI hostname is encrypted, the outer
  ClientHello contains a dummy hostname
- DoH (DNS over HTTPS): DNS queries go to dns.google:443 or cloudflare-dns.com:443
  over HTTPS, bypassing UDP port 53

These eliminate both detection signals the agent relies on for browser traffic. This is
not a bug - it is browsers working as designed. The agent reliably detects CLI tools,
SDKs, and desktop AI apps (Cursor, Claude Code, Copilot) because those do not use ECH.

The research document BROWSER-DETECTION-OPTIONS.md covers all investigated approaches.
The honest conclusion: passive network-level detection has a hard ceiling with modern
browsers on unmanaged machines. MITM mode (Phase 5) is the complete answer for
organizations that need browser-level visibility. ETW DNS-Client on Windows provides
partial browser coverage for browsers that use the system DNS resolver rather than
their own internal DoH.

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
