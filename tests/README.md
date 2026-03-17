# AI Ranger Tests

## Test Layers

### Unit Tests

Run per-component unit tests with no external dependencies:

```bash
make test    # all components
cargo test   # agent only (49 tests)
```

### Integration Tests

Full pipeline tests against the Docker Compose stack. Two categories:

- **Synthetic event tests**: Send protobuf batches directly to the gateway HTTP endpoint.
  Test the full pipeline from HTTP ingest through RabbitMQ to ClickHouse without requiring
  a real agent or raw socket access. Run on all environments.

- **Real agent tests**: Run the actual compiled agent binary, trigger real network traffic,
  and verify events flow through the full pipeline. Require root/Administrator for raw
  socket capture. Automatically skipped when not running with elevated privileges.

### Running Integration Tests

The simplest way to run everything:

```bash
make test-integration
```

This single command:
1. Ensures `.env` exists (copies from `.env.example` if missing)
2. Builds the agent binary in release mode
3. Starts the Docker Compose stack with `--build --wait` (uses built-in health checks)
4. Installs Python test dependencies
5. Runs all integration tests with sudo (including real agent tests)

Platform-specific scripts live in `tests/integration/scripts/`:

| Script | Platform | What it runs |
|--------|----------|--------------|
| `run-linux.sh` | Linux / WSL | Full suite with sudo |
| `run-macos.sh` | macOS | Full suite with sudo (BPF access) |
| `run-windows.ps1` | Windows | Real agent tests only (Administrator required) |

The entry point `tests/run-integration.sh` detects the OS and dispatches to the right script.

On Windows, run the PowerShell script directly as Administrator:

```powershell
tests\integration\scripts\run-windows.ps1
```

### Adding New Tests

- Place integration tests in `tests/integration/`
- Use typed API clients, not raw HTTP calls:
  - `gateway_api` fixture — `GatewayAPI` with methods: `enroll()`, `ingest()`, `get_providers()`, `health()`
  - `api_server` fixture — `APIServer` with methods: `fleet()`, `overview()`, `health()`, `create_token()`, etc.
  - Raw `gateway_client` / `api_client` are available for edge cases (bad auth, malformed requests)
- Use `enrolled_agent` fixture for tests that need a registered agent
- Use helpers from `tests/integration/helpers/`:
  - `gateway_api.py`: Typed gateway API client
  - `api_server.py`: Typed Go API server client
  - `proto.py`: Build synthetic protobuf events
  - `wait.py`: Poll-with-timeout (never use `time.sleep()`)
  - `agent.py`: Manage real agent subprocess
- Tests requiring root/Administrator: mark with `@pytest.mark.skipif(not is_root(), ...)`
- Tests requiring internet: mark with `@pytest.mark.network`
- Every test must be independent and clean up after itself
