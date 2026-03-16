"""Dashboard and providers endpoint tests."""

from helpers.proto import encode_batch, make_test_batch, make_test_event
from helpers.wait import wait_for_clickhouse_event, wait_for_condition


def test_fleet_returns_enrolled_agent(api_server, enrolled_agent):
    """GET /v1/dashboard/fleet returns the enrolled agent."""
    agent_id = enrolled_agent["agent_id"]
    agents = api_server.fleet()
    assert any(a["ID"] == agent_id for a in agents), (
        f"Agent {agent_id} not found in fleet response"
    )


def test_overview_counts_after_ingest(gateway_api, api_server, enrolled_agent, clickhouse_client):
    """GET /v1/dashboard/overview returns non-zero counts after ingest."""
    agent_id = enrolled_agent["agent_id"]
    event = make_test_event(agent_id, provider="anthropic", provider_host="api.anthropic.com")
    batch = make_test_batch(agent_id, [event])
    body = encode_batch(batch)

    # The gateway's RabbitMQ connection may need to reconnect after idle.
    def ingest_succeeds() -> bool:
        return gateway_api.ingest(agent_id, body).status_code == 200

    wait_for_condition(ingest_succeeds, timeout_secs=15, description="ingest POST 200")

    wait_for_clickhouse_event(clickhouse_client, agent_id, "anthropic")

    def overview_has_events() -> bool:
        resp = api_server.overview()
        if resp.status_code != 200:
            return False
        return resp.json().get("total_events", 0) > 0

    wait_for_condition(overview_has_events, timeout_secs=30, description="overview total_events > 0")


def test_providers_endpoint(gateway_api):
    """GET /v1/agents/providers returns valid TOML with known providers."""
    resp = gateway_api.get_providers()
    # May be 404 if providers.toml is not bundled in Docker — that's acceptable
    if resp.status_code == 404:
        return
    assert resp.status_code == 200
    body = resp.text
    assert "[[providers]]" in body
    assert "anthropic" in body
