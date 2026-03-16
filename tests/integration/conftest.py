"""Integration test fixtures for the AI Ranger backend pipeline.

All fixtures use sync httpx clients to avoid async event loop conflicts
between session-scoped and function-scoped fixtures in pytest-asyncio.

Tests should use the typed API clients (gateway_api, api_server) rather
than raw httpx clients. Raw clients are available for edge cases like
testing malformed requests or missing auth.
"""

import os
import subprocess
import time
import uuid

import clickhouse_connect
import httpx
import pytest

from helpers.api_server import APIServer
from helpers.gateway_api import GatewayAPI

# -- Environment defaults (match .env.example for local dev) -------------------

GATEWAY_URL = os.environ.get("GATEWAY_URL", "http://localhost:8080")
API_URL = os.environ.get("API_URL", "http://localhost:8081")
RABBITMQ_MGMT_URL = os.environ.get("RABBITMQ_MGMT_URL", "http://localhost:15672")
RABBITMQ_USER = os.environ.get("RABBITMQ_DEFAULT_USER", "guest")
RABBITMQ_PASS = os.environ.get("RABBITMQ_DEFAULT_PASS", "guest")
SEED_TOKEN = os.environ.get("SEED_TOKEN", "tok_test_dev")
CLICKHOUSE_HOST = os.environ.get("CLICKHOUSE_HOST", "localhost")
CLICKHOUSE_PORT = int(os.environ.get("CLICKHOUSE_HTTP_PORT", "8123"))


def pytest_configure(config):
    """Register custom markers."""
    config.addinivalue_line("markers", "network: test requires external network access")


# -- Docker Compose stack management ------------------------------------------

def _is_stack_healthy() -> bool:
    try:
        gw = httpx.get(f"{GATEWAY_URL}/health", timeout=3)
        api = httpx.get(f"{API_URL}/health", timeout=3)
        return gw.status_code == 200 and api.status_code == 200
    except Exception:
        return False


@pytest.fixture(scope="session")
def docker_stack():
    """Ensure Docker Compose stack is running."""
    if _is_stack_healthy():
        yield "already_running"
        return

    repo_root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    compose_dir = os.path.join(repo_root, "docker")
    env_file = os.path.join(repo_root, ".env")

    subprocess.run(
        ["docker", "compose", "--env-file", env_file, "up", "-d", "--build"],
        cwd=compose_dir, check=True, capture_output=True,
    )

    deadline = time.monotonic() + 90
    while time.monotonic() < deadline:
        if _is_stack_healthy():
            break
        time.sleep(2)
    else:
        pytest.fail("Docker Compose stack did not become healthy within 90 seconds")

    yield "started_by_fixture"

    subprocess.run(
        ["docker", "compose", "--env-file", env_file, "down", "-v"],
        cwd=compose_dir, capture_output=True,
    )


# -- Raw HTTP clients ---------------------------------------------------------

@pytest.fixture
def gateway_client(docker_stack):
    """Raw sync HTTP client for the gateway. Prefer gateway_api for most tests."""
    with httpx.Client(base_url=GATEWAY_URL, timeout=10) as client:
        yield client


@pytest.fixture
def api_client(docker_stack):
    """Raw sync HTTP client for the API server. Prefer api_server for most tests."""
    with httpx.Client(base_url=API_URL, timeout=10) as client:
        yield client


# -- Typed API clients --------------------------------------------------------

@pytest.fixture
def gateway_api(gateway_client) -> GatewayAPI:
    """Typed API client for the gateway. Use this for most tests."""
    return GatewayAPI(gateway_client)


@pytest.fixture
def api_server(api_client) -> APIServer:
    """Typed API client for the Go API server. Use this for most tests."""
    return APIServer(api_client)


# -- Database ------------------------------------------------------------------

@pytest.fixture
def clickhouse_client(docker_stack):
    """ClickHouse HTTP client for verification queries."""
    client = clickhouse_connect.get_client(host=CLICKHOUSE_HOST, port=CLICKHOUSE_PORT)
    yield client
    client.close()


# -- Agent enrollment ----------------------------------------------------------

@pytest.fixture
def enrolled_agent(gateway_api):
    """Enroll a fresh test agent. Returns dict with agent_id and org_id."""
    agent_id = str(uuid.uuid4())
    data = gateway_api.enroll(token=SEED_TOKEN, agent_id=agent_id)
    yield {"agent_id": agent_id, "org_id": data["org_id"]}


# -- Agent binary --------------------------------------------------------------

@pytest.fixture(scope="session")
def agent_binary():
    """Build or locate the agent binary. Returns path or skips."""
    binary = os.environ.get("AGENT_BINARY")
    if binary and os.path.isfile(binary):
        return binary

    repo_root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    for c in [
        os.path.join(repo_root, "target", "release", "ai-ranger"),
        os.path.join(repo_root, "target", "debug", "ai-ranger"),
    ]:
        if os.path.isfile(c):
            return c

    result = subprocess.run(
        ["cargo", "build", "--release", "--manifest-path",
         os.path.join(repo_root, "agent", "Cargo.toml")],
        capture_output=True, text=True,
    )
    if result.returncode == 0:
        release = os.path.join(repo_root, "target", "release", "ai-ranger")
        if os.path.isfile(release):
            return release

    pytest.skip(f"Agent binary not available: {result.stderr[:200]}")
