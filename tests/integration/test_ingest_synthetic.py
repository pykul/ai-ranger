"""Synthetic protobuf ingest tests. Runs on all environments — no agent binary needed."""

import uuid

from helpers.proto import encode_batch, make_test_batch, make_test_event
from helpers.wait import wait_for_clickhouse_event


def test_ingest_single_event(gateway_client, enrolled_agent, clickhouse_client):
    """One event -> 200, appears in ClickHouse within timeout."""
    agent_id = enrolled_agent["agent_id"]
    event = make_test_event(agent_id, provider="openai", provider_host="api.openai.com")
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

    row = wait_for_clickhouse_event(clickhouse_client, agent_id, "openai")
    assert row["provider"] == "openai"
    assert row["provider_host"] == "api.openai.com"


def test_ingest_batch_of_five(gateway_client, enrolled_agent, clickhouse_client):
    """Five events -> all appear in ClickHouse."""
    agent_id = enrolled_agent["agent_id"]
    providers = [
        ("openai", "api.openai.com"),
        ("anthropic", "api.anthropic.com"),
        ("cursor", "api2.cursor.sh"),
        ("mistral", "api.mistral.ai"),
        ("deepseek", "api.deepseek.com"),
    ]
    events = [make_test_event(agent_id, p, h) for p, h in providers]
    batch = make_test_batch(agent_id, events)
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

    for provider, _ in providers:
        row = wait_for_clickhouse_event(clickhouse_client, agent_id, provider)
        assert row["provider"] == provider


def test_ingest_no_auth(gateway_client):
    """No Bearer header -> 401 or 422."""
    resp = gateway_client.post(
        "/v1/ingest",
        content=b"dummy",
        headers={"Content-Type": "application/x-protobuf"},
    )
    assert resp.status_code in (401, 422)


def test_ingest_bad_auth(gateway_client):
    """Wrong agent_id -> 401."""
    fake_id = str(uuid.uuid4())
    resp = gateway_client.post(
        "/v1/ingest",
        content=b"dummy",
        headers={
            "Content-Type": "application/x-protobuf",
            "Authorization": f"Bearer {fake_id}",
        },
    )
    assert resp.status_code == 401


def test_ingest_invalid_protobuf(gateway_client, enrolled_agent):
    """Garbage bytes -> 400 or 500."""
    agent_id = enrolled_agent["agent_id"]
    resp = gateway_client.post(
        "/v1/ingest",
        content=b"\xff\xfe\x00\x01garbage",
        headers={
            "Content-Type": "application/x-protobuf",
            "Authorization": f"Bearer {agent_id}",
        },
    )
    assert resp.status_code in (400, 500)


def test_ingest_updates_last_seen(gateway_client, enrolled_agent):
    """agent.last_seen_at in Postgres is updated after ingest."""
    agent_id = enrolled_agent["agent_id"]

    event = make_test_event(agent_id)
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

    # Verify via the fleet endpoint that last_seen_at is set
    import httpx
    from conftest import API_URL
    fleet_resp = httpx.get(f"{API_URL}/v1/dashboard/fleet", timeout=5)
    agents = fleet_resp.json()
    agent = next((a for a in agents if a["ID"] == agent_id), None)
    assert agent is not None
    assert agent["LastSeenAt"] is not None


def test_all_detection_methods(gateway_client, enrolled_agent, clickhouse_client):
    """One event per detection method — all appear with correct values."""
    agent_id = enrolled_agent["agent_id"]
    methods = [
        (0, "sni"),      # SNI
        (1, "dns"),      # DNS
        (2, "ip_range"), # IP_RANGE
    ]
    events = [
        make_test_event(
            agent_id,
            provider=f"test_{name}",
            provider_host=f"{name}.test.com",
            detection_method=dm,
        )
        for dm, name in methods
    ]
    batch = make_test_batch(agent_id, events)
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

    for _, name in methods:
        row = wait_for_clickhouse_event(clickhouse_client, agent_id, f"test_{name}")
        assert row["detection_method"] == name
