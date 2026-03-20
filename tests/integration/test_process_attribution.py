"""Process attribution integration tests.

Verifies that os_username flows correctly through the pipeline for both
normal events and fallback cases (PID 0 / DNS events).
"""

from helpers.proto import encode_batch, make_test_batch, make_test_event
from helpers.wait import wait_for_clickhouse_event, wait_for_condition


def _make_event_with_username(agent_id, username, pid=12345, provider="openai"):
    """Build a test event with a specific os_username and process_pid."""
    from helpers.proto import _repo_root  # noqa: F401
    import sys, os  # noqa: E401

    _proto_dir = os.path.join(
        os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))),
        "proto", "gen", "python",
    )
    if _proto_dir not in sys.path:
        sys.path.insert(0, _proto_dir)

    import time
    import uuid
    from ranger.v1 import events_pb2

    return events_pb2.AiConnectionEvent(
        agent_id=agent_id,
        machine_hostname="integration-test-host",
        os_username=username,
        os_type="linux",
        timestamp_ms=int(time.time() * 1000),
        provider=provider,
        provider_host=f"api.{provider}.com",
        process_name="test-process",
        process_pid=pid,
        connection_id=uuid.uuid4().hex[:16],
        detection_method=0,
        capture_mode=0,
        src_ip="10.0.0.1",
    )


def test_event_with_root_fallback(gateway_api, enrolled_agent, clickhouse_client):
    """An event with os_username='root' and pid=0 (DNS fallback) flows through correctly."""
    agent_id = enrolled_agent["agent_id"]
    event = _make_event_with_username(agent_id, "root", pid=0, provider="test_root_fallback")
    batch = make_test_batch(agent_id, [event])
    body = encode_batch(batch)

    resp = gateway_api.ingest(agent_id, body)
    assert resp.status_code == 200

    row = wait_for_clickhouse_event(clickhouse_client, agent_id, "test_root_fallback")
    assert row["os_username"] == "root"


def test_event_with_developer_username(gateway_api, enrolled_agent, clickhouse_client):
    """An event with a realistic developer username flows through and is searchable."""
    agent_id = enrolled_agent["agent_id"]
    event = _make_event_with_username(agent_id, "developer", provider="test_dev_user")
    batch = make_test_batch(agent_id, [event])
    body = encode_batch(batch)

    resp = gateway_api.ingest(agent_id, body)
    assert resp.status_code == 200

    row = wait_for_clickhouse_event(clickhouse_client, agent_id, "test_dev_user")
    assert row["os_username"] == "developer"


def test_search_events_by_username(gateway_api, enrolled_agent, api_server, clickhouse_client):
    """Searching events by os_username returns matching results."""
    agent_id = enrolled_agent["agent_id"]
    unique_user = "searchable_user_xyz"
    event = _make_event_with_username(agent_id, unique_user, provider="test_search_user")
    batch = make_test_batch(agent_id, [event])
    body = encode_batch(batch)

    resp = gateway_api.ingest(agent_id, body)
    assert resp.status_code == 200

    wait_for_clickhouse_event(clickhouse_client, agent_id, "test_search_user")

    def search_returns_results():
        resp = api_server.events(q=unique_user, days=7)
        if resp.status_code != 200:
            return False
        data = resp.json()
        return data.get("total", 0) > 0

    wait_for_condition(search_returns_results, timeout_secs=15, description="username search")

    resp = api_server.events(q=unique_user, days=7)
    data = resp.json()
    assert data["total"] > 0
    for event_row in data["events"]:
        assert unique_user in event_row.get("os_username", ""), (
            f"Expected os_username to contain '{unique_user}', got: {event_row}"
        )
