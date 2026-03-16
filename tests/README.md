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
  and verify events flow through the full pipeline. Require root for raw socket capture.
  Automatically skipped when not running as root.

### Running Integration Tests

```bash
# 1. Start the backend
cp .env.example .env
make dev

# 2. Install test dependencies
pip install -r tests/integration/requirements.txt

# 3. Run all integration tests (skips real agent tests if not root)
pytest tests/integration/ -v

# 4. Run real agent tests (requires root)
sudo pytest tests/integration/test_ingest_real_agent.py -v

# 5. Run with network tests (requires internet access)
pytest tests/integration/ -v -m "network"
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
- Tests requiring root: mark with `@pytest.mark.skipif(not is_root(), ...)`
- Tests requiring internet: mark with `@pytest.mark.network`
- Every test must be independent and clean up after itself
