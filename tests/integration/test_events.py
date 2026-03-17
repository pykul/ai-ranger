"""Events endpoint tests. Verifies search, pagination, and sorting."""

from helpers.proto import encode_batch, make_test_batch, make_test_event
from helpers.wait import wait_for_clickhouse_event, wait_for_condition


def _ingest_test_events(gateway_api, agent_id):
    """Ingest a batch with multiple providers so search and sort are testable."""
    events = [
        make_test_event(agent_id, provider="openai", provider_host="api.openai.com"),
        make_test_event(agent_id, provider="anthropic", provider_host="api.anthropic.com"),
        make_test_event(agent_id, provider="openai", provider_host="api.openai.com"),
    ]
    batch = make_test_batch(agent_id, events)
    body = encode_batch(batch)

    def ingest_succeeds():
        return gateway_api.ingest(agent_id, body).status_code == 200

    wait_for_condition(ingest_succeeds, timeout_secs=15, description="ingest POST 200")


def test_events_returns_paginated_response(
    gateway_api, enrolled_agent, api_server, clickhouse_client
):
    """GET /v1/events returns events with total, page, limit fields."""
    agent_id = enrolled_agent["agent_id"]
    _ingest_test_events(gateway_api, agent_id)
    wait_for_clickhouse_event(clickhouse_client, agent_id, "openai")

    def events_available():
        resp = api_server.events(days=7, page=1, limit=25)
        if resp.status_code != 200:
            return False
        return resp.json().get("total", 0) > 0

    wait_for_condition(events_available, timeout_secs=15, description="events available")

    resp = api_server.events(days=7, page=1, limit=25)
    assert resp.status_code == 200
    data = resp.json()
    assert "events" in data
    assert "total" in data
    assert "page" in data
    assert "limit" in data
    assert data["total"] > 0
    assert len(data["events"]) > 0
    event = data["events"][0]
    assert "timestamp" in event
    assert "provider" in event
    assert "os_username" in event
    assert "src_ip" in event


def test_events_search_filters_results(
    gateway_api, enrolled_agent, api_server, clickhouse_client
):
    """GET /v1/events?q=openai returns only events matching the search term."""
    agent_id = enrolled_agent["agent_id"]
    _ingest_test_events(gateway_api, agent_id)
    wait_for_clickhouse_event(clickhouse_client, agent_id, "openai")

    def search_returns_results():
        resp = api_server.events(q="openai", days=7)
        if resp.status_code != 200:
            return False
        return resp.json().get("total", 0) > 0

    wait_for_condition(search_returns_results, timeout_secs=15, description="search results")

    resp = api_server.events(q="openai", days=7)
    data = resp.json()
    assert data["total"] > 0
    for event in data["events"]:
        fields = [
            event.get("provider", ""),
            event.get("provider_host", ""),
            event.get("process_name", ""),
            event.get("machine_hostname", ""),
            event.get("os_username", ""),
        ]
        assert any("openai" in f.lower() for f in fields), (
            f"Event does not match search 'openai': {event}"
        )


def test_events_pagination(
    gateway_api, enrolled_agent, api_server, clickhouse_client
):
    """GET /v1/events?page=1&limit=2 respects pagination parameters."""
    agent_id = enrolled_agent["agent_id"]
    _ingest_test_events(gateway_api, agent_id)
    wait_for_clickhouse_event(clickhouse_client, agent_id, "openai")

    def has_events():
        resp = api_server.events(days=7, limit=1)
        return resp.status_code == 200 and resp.json().get("total", 0) > 0

    wait_for_condition(has_events, timeout_secs=15, description="events ingested")

    resp = api_server.events(days=7, page=1, limit=2)
    data = resp.json()
    assert data["page"] == 1
    assert data["limit"] == 2
    assert len(data["events"]) <= 2


def test_events_sort_ascending(
    gateway_api, enrolled_agent, api_server, clickhouse_client
):
    """GET /v1/events?sort=timestamp&order=asc returns events in ascending order."""
    agent_id = enrolled_agent["agent_id"]
    _ingest_test_events(gateway_api, agent_id)
    wait_for_clickhouse_event(clickhouse_client, agent_id, "openai")

    def has_multiple():
        resp = api_server.events(days=7)
        return resp.status_code == 200 and resp.json().get("total", 0) > 1

    wait_for_condition(has_multiple, timeout_secs=15, description="multiple events")

    resp = api_server.events(days=7, sort="timestamp", order="asc", limit=25)
    data = resp.json()
    if len(data["events"]) >= 2:
        timestamps = [e["timestamp"] for e in data["events"]]
        assert timestamps == sorted(timestamps), "Events not in ascending order"
