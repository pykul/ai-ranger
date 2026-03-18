# AI Ranger

**Discover which AI tools your team is using, without reading a single message.**

Claude, Cursor, Copilot, ChatGPT, local models. All of it, in one place.

AI Ranger is an open source agent that runs on your machines and tells you which AI
providers are being called and by which tools.
Visibility into AI usage across your organization, with no proxy on your network and
nothing intercepted.

No content inspection. No proxies. No certificate installation. Just network metadata,
handled transparently, from source code you can read yourself.

---

## How it works (and why it is not spying on you)

When your machine connects to an AI provider like `api.anthropic.com`, it sends a small
plaintext greeting before any encryption begins. That greeting contains the destination
hostname. AI Ranger reads that hostname and nothing else.

Think of it like a postal worker reading the address on the envelope. The letter inside
is sealed. We never open it, never read it, never see it. We only see where it went.

This is fundamentally different from tools that decrypt your traffic. There is no
man-in-the-middle proxy. There is no custom certificate installed on your machine.
There is no interception. AI Ranger is a passive observer of metadata that is already
visible to every router and firewall between you and the destination. The encrypted
content of your prompts and responses stays encrypted, always.

**What AI Ranger sees:**
- Which AI provider was contacted (`api.anthropic.com`, `api.openai.com`, etc.)
- Which application made the call (Cursor, Claude Code, a Python script, etc.)
- When it happened

**What AI Ranger never sees:**
- The content of any prompt or message
- Any response from an AI provider
- Anything inside an encrypted connection

The agent is fully open source. Every line of network-touching code is public and
auditable. That is not a marketing claim, it is the architecture. If you are not
sure what the agent does, you can read it.

**A note on root access.** Capturing raw network packets requires elevated privileges
on all major operating systems. The agent runs as root on Linux and macOS, or as
Administrator on Windows. This is standard for any
tool that observes network-layer metadata. The same requirement applies to Wireshark,
tcpdump, and endpoint security agents. What root access gives the agent is the ability
to see packet headers. It does not change what the agent reads from those packets, which
is only the destination hostname. The source code is there to verify this.

The agent binary is fully standalone with no external dependencies. No drivers, no
additional software, no npcap. Download it and run it.

---

## Why this exists

Developers today use more AI tools than any organization can easily track. Cursor on
one machine, Claude Code on another, Copilot in the IDE, ChatGPT in the browser,
a local Ollama instance running overnight. Most engineering leads have no idea which
tools their team is actually using, let alone how heavily.

AI Ranger gives you that visibility without requiring you to route traffic through a
proxy, install certificates, or touch your existing tooling. It works alongside whatever
your developers are already using, without getting in the way.

---

## What you get

- **Per-user, per-tool breakdown:** see which developer is using which AI provider
  and which application they are calling it from
- **Fleet management:** enroll machines with a single command, manage them from a
  dashboard
- **Self-hostable:** your data never leaves your infrastructure
- **Open source:** Apache-2.0, community-maintained, no vendor lock-in

---

## Prerequisites

To build and develop AI Ranger, you need the following tools:

| Tool | Minimum Version | Purpose |
|------|----------------|---------|
| Docker + Docker Compose | Docker 24+ | Running the full platform stack |
| Node.js | 22+ | Building and running the dashboard |
| Rust (via rustup) | 1.75+ (see `agent/Cargo.toml`) | Building the agent |
| Go | 1.22+ (see `workers/go.mod`) | Building the workers |
| Python | 3.12+ | Running the gateway |
| protoc | 3.0+ | Regenerating protobuf code (only when changing `.proto` files) |

Run the install script for your OS:

```bash
# macOS
bash scripts/install-deps/macos.sh

# Linux (Debian/Ubuntu or Fedora/RHEL)
bash scripts/install-deps/linux.sh

# Windows (run as Administrator)
powershell -ExecutionPolicy Bypass -File scripts/install-deps/windows.ps1
```

### Verify your setup

```bash
docker --version          # Docker version 24+
docker compose version    # Docker Compose version 2+
node --version            # v22+
rustc --version           # rustc 1.75+
go version                # go1.22+
python3 --version         # Python 3.12+
protoc --version          # libprotoc 3+
```

---

## Quick start

### 1. Start the platform

```bash
git clone https://github.com/pykul/ai-ranger
cd ai-ranger
cp .env.example .env
make dev
```

The first run downloads Docker images and builds all services, which takes a few
minutes. Subsequent runs are faster. The command waits for every service to report
healthy before returning. When it finishes you should see `All services healthy`
and all 8 services are running.

If you see stale data from a previous run, use `make dev-reset` instead. It wipes
all database volumes and starts fresh.

**Open http://localhost:8000 in your browser to see the dashboard.**

| Service | URL |
|---------|-----|
| Dashboard | http://localhost:8000 |
| Gateway Swagger UI | http://localhost:8000/ingest/docs |
| API Server Swagger UI | http://localhost:8000/api/docs |
| RabbitMQ Management | http://localhost:15672 |

Run `make logs` to view service logs. Direct ports (8080 for gateway, 8081
for API server) are also available for debugging.

### 2. Build, enroll, and run the agent

The dev environment seeds a test enrollment token (`tok_test_dev`) automatically.
Build the agent and start it with `--token` and `--backend`. It enrolls and starts
capturing in one step:

```bash
cargo build --manifest-path agent/Cargo.toml

# Linux / macOS
sudo ./target/debug/ai-ranger --token=tok_test_dev --backend=http://localhost:8000/ingest

# Windows (run as Administrator)
.\target\debug\ai-ranger.exe --token=tok_test_dev --backend=http://localhost:8000/ingest
```

On first run, the agent enrolls with the backend and begins capturing immediately.
On subsequent runs, just run the binary with no flags. The enrollment is saved
to a platform-specific config directory and reused automatically:

- Linux: `~/.config/ai-ranger/config.json`
- macOS: `~/Library/Application Support/ai-ranger/config.json`
- Windows: `%APPDATA%\ai-ranger\config.json`

**If the agent says "already enrolled" but events do not appear in the dashboard,**
the saved config is stale. This happens when the backend database is reset (e.g.
`make dev-reset`) while the agent still has credentials from a previous run. Delete
the config file and re-enroll:

```bash
# Linux
rm ~/.config/ai-ranger/config.json

# macOS
rm ~/Library/Application\ Support/ai-ranger/config.json

# Windows (PowerShell)
Remove-Item "$env:APPDATA\ai-ranger\config.json"
```

Then restart the agent with `--token` and `--backend` as shown above.

### 3. Verify end-to-end

In another terminal, trigger some AI provider traffic:

```bash
curl -s https://api.openai.com > /dev/null
curl -s https://api.anthropic.com > /dev/null
```

Wait a few seconds for events to flow through RabbitMQ into ClickHouse, then
check that they arrived:

```bash
# See your enrolled agent
curl -s http://localhost:8000/api/v1/dashboard/fleet | python3 -m json.tool

# See detected events
curl -s http://localhost:8000/api/v1/dashboard/overview | python3 -m json.tool
```

You should see `total_connections` greater than 0 in the overview response.

---

## Running standalone (no backend)

The agent works completely independently. With no enrollment it prints events to
stdout, which is useful for testing, scripting, or piping into your own tooling:

```bash
# Linux / macOS
sudo ai-ranger

# Windows (run as Administrator)
ai-ranger.exe
```

```json
{"agent_id":"","machine_hostname":"Omri-PC","os_username":"omria","os_type":"windows","connection_id":"a1b2c3d4e5f6","timestamp_ms":1773564763684,"provider":"openai","provider_host":"api.openai.com","process_name":"curl.exe","process_pid":22276,"src_ip":"192.168.1.232","detection_method":"SNI","capture_mode":"DNS_SNI"}
```

Fields like `agent_id` are populated after enrollment. In standalone mode they are empty.
No account. No config. No data sent anywhere.

A default `config.toml` with all available options documented ships at `agent/config.toml`.

---

## Production deployment

### Backend setup

**Prerequisites:**

- A Linux server with Docker and Docker Compose v2 installed
- A domain name with a DNS A record pointing to the server
- Ports 80 and 443 open in the server's firewall
- TLS certificates (Let's Encrypt recommended)

**1. Generate TLS certificates:**

```bash
sudo apt install certbot
sudo certbot certonly --standalone -d ranger.example.com
sudo mkdir -p /etc/letsencrypt/live/default
sudo ln -sf /etc/letsencrypt/live/ranger.example.com/fullchain.pem /etc/letsencrypt/live/default/fullchain.pem
sudo ln -sf /etc/letsencrypt/live/ranger.example.com/privkey.pem /etc/letsencrypt/live/default/privkey.pem
```

**2. Configure environment variables:**

```bash
git clone https://github.com/pykul/ai-ranger
cd ai-ranger
cp .env.example .env
```

Edit `.env` and set the following production values:

| Variable | Value | Notes |
|----------|-------|-------|
| `ENVIRONMENT` | `production` | Enables JWT auth, disables seed data |
| `DOMAIN` | `ranger.example.com` | Your production domain |
| `JWT_SECRET` | Output of `openssl rand -hex 32` | Used to sign dashboard login tokens |
| `ADMIN_EMAIL` | `admin@example.com` | Dashboard login email |
| `ADMIN_PASSWORD` | A strong password | Plaintext here, hashed in memory at startup |
| `POSTGRES_PASSWORD` | A strong password | Postgres superuser password |
| `RABBITMQ_DEFAULT_USER` | `ranger` | Change from default `guest` |
| `RABBITMQ_DEFAULT_PASS` | A strong password | Change from default `guest` |

**3. Start the production stack:**

```bash
cd docker
docker compose --env-file ../.env \
  -f docker-compose.yml -f docker-compose.prod.yml \
  up -d --wait
```

The dashboard is available at `https://ranger.example.com`. Only ports 80
(redirects to 443) and 443 are exposed. All internal services (Postgres,
ClickHouse, RabbitMQ, gateway, workers) are not reachable from outside.

### Pre-built agent binaries

Pre-built binaries for Linux, macOS (Intel and Apple Silicon), and Windows are
attached to every release on the [GitHub Releases page](https://github.com/pykul/ai-ranger/releases).
No Rust toolchain required.

```bash
# macOS (Apple Silicon)
curl -sSL https://github.com/pykul/ai-ranger/releases/latest/download/ai-ranger-aarch64-apple-darwin \
  -o /usr/local/bin/ai-ranger && chmod +x /usr/local/bin/ai-ranger

# macOS (Intel)
curl -sSL https://github.com/pykul/ai-ranger/releases/latest/download/ai-ranger-x86_64-apple-darwin \
  -o /usr/local/bin/ai-ranger && chmod +x /usr/local/bin/ai-ranger

# Linux (x86_64)
curl -sSL https://github.com/pykul/ai-ranger/releases/latest/download/ai-ranger-x86_64-unknown-linux-gnu \
  -o /usr/local/bin/ai-ranger && chmod +x /usr/local/bin/ai-ranger
```

Each release includes SHA256 checksums in `checksums.txt`. Verify before running:

```bash
sha256sum -c checksums.txt --ignore-missing
```

### Enrolling with a production instance

Generate an enrollment token from the admin API, then start the agent:

```bash
# Linux / macOS
ai-ranger --token=tok_your_token --backend=https://your-instance.com/ingest

# Windows (run as Administrator)
ai-ranger.exe --token=tok_your_token --backend=https://your-instance.com/ingest
```

The agent enrolls with the backend on first run and starts capturing immediately.
On subsequent runs, just `ai-ranger` (or `ai-ranger.exe` on Windows). The
enrollment is remembered.

For scripted deployments where enrollment and daemon start are separate steps
(e.g. installer scripts), use `--enroll` to enroll and exit without capturing:

```bash
ai-ranger --enroll --token=tok_your_token --backend=https://your-instance.com/ingest
# then start as daemon separately
```

---

## Supported AI providers

AI Ranger ships with a community-maintained registry of known AI provider hostnames.
It currently covers:

- Anthropic / Claude
- OpenAI / ChatGPT
- Cursor
- GitHub Copilot
- Google Gemini
- Mistral
- Cohere
- Hugging Face
- Replicate
- Together AI
- Perplexity
- DeepSeek
- xAI / Grok
- AI21 Labs
- Amazon Bedrock
- Azure OpenAI
- Stability AI
- Ollama (local models)

Missing a provider? [Open a PR](https://github.com/pykul/ai-ranger/blob/main/providers/CONTRIBUTING.md).
Adding a provider is a one-minute TOML edit, no code required.

---

## Privacy and security

- **Zero call-home by default.** The agent never contacts any URL unless you explicitly
  configure a backend. Running `ai-ranger` with no config produces local stdout output only.
- **No content inspection.** The agent reads SNI hostnames and connection metadata. It
  never reads, buffers, or transmits any part of the encrypted payload.
- **Local-first.** Events are buffered locally on the machine and only uploaded when
  a backend is configured and reachable. Nothing is sent to any third party.
- **Explicit enrollment.** The backend URL and enrollment token must be explicitly
  provided during installation. They are never hardcoded or bundled.
- **Fully auditable.** Every line of code is open source. Read it, fork it, run it
  yourself. The privacy guarantee is structural, not a policy.

**A note on process names.** AI Ranger identifies which application made a connection
by looking up the process that owns the network socket at the moment the connection is
detected. If you run a short-lived command like `curl` from a shell, you may see the
shell (e.g. `bash`, `zsh`, `powershell.exe`) as the process name, or `unknown` if the
command finished before the lookup ran. The process ID is always accurate regardless.
Real AI tools like Cursor, Claude Code, and Copilot keep their connections open and
always show up correctly.

**A note on browser detection.** Some applications, primarily modern browsers, encrypt
the destination hostname using Encrypted Client Hello (ECH), a general TLS privacy
feature, which prevents the agent from reading it via SNI.
For providers with dedicated IP ranges - currently the Anthropic API - the agent falls
back to matching the connection's destination IP against known CIDR ranges. These
connections appear with `detection_method: "IP_RANGE"` in the output.

**Deployment security.** In production, agent-to-platform communication is encrypted
over HTTPS, the dashboard requires JWT authentication, enrollment tokens are hashed
before storage, ClickHouse queries use parameterized inputs, and the dashboard never
exposes internal error details to the browser. Event data stays inside your
infrastructure unless you explicitly configure an outbound webhook sink.

AI Ranger is a visibility tool, not a security boundary. You are responsible for
hardening the host infrastructure: TLS certificates, firewall rules, restricting
database and message queue ports to the Docker network, and storing secrets in a
secrets manager rather than a plain `.env` file. The security of your deployment is
determined by the infrastructure you put around it. See ARCHITECTURE.md for the
production deployment checklist.

---

## Architecture overview

The agent is a single Rust binary. It captures TLS ClientHello packets using OS-native
raw sockets (no libpcap, no external drivers), extracts the destination hostname from
each one, matches it against a provider registry, and routes the resulting events to
one or more output sinks. By default the only sink is stdout. The agent is fully
functional with no other components present.

Output sinks are pluggable. The agent ships with a stdout sink, a file sink, a backend
sink that sends protobuf batches to the AI Ranger gateway, and a webhook sink for custom
destinations. Multiple sinks can be active at once, configured in `config.toml`. This is
how teams with existing observability infrastructure connect AI Ranger to Datadog, Splunk,
or any HTTPS endpoint without running the backend at all.

The platform is self-hosted. It consists of nginx as the single entry point, a
Python/FastAPI gateway that receives agent batches and publishes them to RabbitMQ,
Go workers that consume from the queue and write to storage, and a React dashboard.
Postgres holds identity data (organizations, agents, enrollment tokens) with schema
managed via Alembic migrations. ClickHouse holds the event timeseries. The full
stack starts with `make dev`.

When the backend sink is configured, the agent buffers events locally in SQLite and
uploads them within seconds. Events typically appear in the dashboard under 1 second
after capture. If the backend is unreachable, events accumulate locally and are
delivered when the connection recovers.

For the complete technical design, see [ARCHITECTURE.md](./ARCHITECTURE.md).

---

## Roadmap

The current version of AI Ranger operates in passive SNI capture mode only. This is
intentional. It is the trust-first approach, and it covers the most important use
case: knowing which AI providers your team is talking to, without reading what they
are saying.

**MITM mode (Phase 5, planned, opt-in only)**

A future version (Phase 5) will include an optional MITM (man-in-the-middle) capture
mode for users and organizations that want deeper visibility. When enabled, this mode
will reveal the exact model being called (e.g. `claude-opus-4-5` vs `claude-haiku-3-5`),
token counts, and response latency. Information that is only available inside the
encrypted payload.

This mode will require explicit opt-in: a separate install step, a separate flag, and
an acknowledgment of what it does. It will never be the default. It will also come with
honest caveats. Some tools use certificate pinning and will not work through a local
proxy, and any mode that reads prompt content introduces PII considerations that need
to be handled deliberately.

MITM mode is tracked in the architecture document. Community input on the design is
welcome before implementation begins.

---

## Contributing

AI Ranger is a community tool. Contributions are welcome at every level.

The easiest way to contribute is to add a provider to `providers/providers.toml`.
If you see an AI tool making network calls that AI Ranger is not detecting, open a PR.
The format is simple and documented in `providers/CONTRIBUTING.md`.

For code contributions, see the [Quick start](#quick-start) section to set up your
development environment, then:

```bash
make test       # run all tests
make lint       # lint all components
```

### Running integration tests

Integration tests verify the full pipeline end-to-end: agent binary, gateway,
RabbitMQ, Go workers, ClickHouse, and Postgres. One command does everything:
builds the agent, starts the Docker Compose stack, waits for health checks,
and runs all tests including real agent capture tests with sudo:

```bash
make test-integration
```

On Windows, run the PowerShell script as Administrator:

```powershell
tests\integration\scripts\run-windows.ps1
```

See `tests/README.md` for details on the test layers and how to add new tests.

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

---

## How this was built

AI Ranger was built using a deliberate methodology for managing complex projects
with AI assistants. See [METHODOLOGY.md](./METHODOLOGY.md) for the full writeup.

## License

Apache-2.0. See [LICENSE](./LICENSE).
