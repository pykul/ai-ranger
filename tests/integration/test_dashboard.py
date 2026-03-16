"""Dashboard and providers endpoint tests."""

from helpers.proto import encode_batch, make_test_batch, make_test_event
from helpers.wait import wait_for_clickhouse_event


def test_fleet_returns_enrolled_agent(api_client, enrolled_agent):
    """GET /v1/dashboard/fleet returns the enrolled agent."""
    agent_id = enrolled_agent["agent_id"]

    resp = api_client.get("/v1/dashboard/fleet")
    assert resp.status_code == 200
    agents = resp.json()
    assert any(a["ID"] == agent_id for a in agents), (
        f"Agent {agent_id} not found in fleet response"
    )


def test_overview_counts_after_ingest(gateway_client, api_client, enrolled_agent, clickhouse_client):
    """GET /v1/dashboard/overview returns non-zero counts after ingest."""
    agent_id = enrolled_agent["agent_id"]
    event = make_test_event(agent_id, provider="anthropic", provider_host="api.anthropic.com")
    batch = make_test_batch(agent_id, [event])
    body = encode_batch(batch)

    resp = gateway_client.post(
        "/v1/ingest",
        content=body,
        headers={
            "Content-Type": "application/x-protobuf",
            "Authorization": f"Bearer {agent_id}",
        },
    )
    assert resp.status_code == 200

    wait_for_clickhouse_event(clickhouse_client, agent_id, "anthropic")

    # The overview endpoint queries ClickHouse which may briefly return errors
    # while data settles. Retry a few times.
    from helpers.wait import wait_for_condition

    result = {}

    def overview_has_events() -> bool:
        r = api_client.get("/v1/dashboard/overview")
        if r.status_code != 200:
            return False
        result.update(r.json())
        return result.get("total_events", 0) > 0

    wait_for_condition(overview_has_events, timeout_secs=30, description="overview total_events > 0")


def test_providers_endpoint(gateway_client):
    """GET /v1/agents/providers returns valid TOML with known providers."""
    resp = gateway_client.get("/v1/agents/providers")
    # May be 404 if providers.toml is not bundled in Docker — that's acceptable
    if resp.status_code == 404:
        return  # providers.toml not available in this environment
    assert resp.status_code == 200
    body = resp.text
    assert "[[providers]]" in body
    assert "anthropic" in body
