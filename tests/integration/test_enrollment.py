"""Agent enrollment endpoint tests."""

import uuid

from conftest import SEED_TOKEN


def test_enroll_valid_token(gateway_client):
    """Valid token returns 200 with agent_id and org_id."""
    agent_id = str(uuid.uuid4())
    resp = gateway_client.post(
        "/v1/agents/enroll",
        json={
            "token": SEED_TOKEN,
            "agent_id": agent_id,
            "hostname": "enroll-test-host",
            "os_username": "enroll-test-user",
            "os": "linux",
            "agent_version": "0.1.0-test",
        },
    )
    assert resp.status_code == 200
    data = resp.json()
    assert data["agent_id"] == agent_id
    assert data["org_id"]  # non-empty


def test_enroll_invalid_token(gateway_client):
    """Bad token returns 401."""
    resp = gateway_client.post(
        "/v1/agents/enroll",
        json={
            "token": "tok_does_not_exist",
            "agent_id": str(uuid.uuid4()),
            "hostname": "bad-token-host",
            "os_username": "user",
            "os": "linux",
            "agent_version": "0.1.0",
        },
    )
    assert resp.status_code == 401
