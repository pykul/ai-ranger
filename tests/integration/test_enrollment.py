"""Agent enrollment endpoint tests."""

import uuid

from conftest import SEED_TOKEN


def test_enroll_valid_token(gateway_api):
    """Valid token returns agent_id and org_id."""
    agent_id = str(uuid.uuid4())
    data = gateway_api.enroll(token=SEED_TOKEN, agent_id=agent_id)
    assert data["agent_id"] == agent_id
    assert data["org_id"]  # non-empty


def test_enroll_invalid_token(gateway_api):
    """Bad token returns 401."""
    resp = gateway_api.enroll_raw(
        token="tok_does_not_exist",
        agent_id=str(uuid.uuid4()),
        hostname="bad-token-host",
        os_username="user",
        os="linux",
        agent_version="0.1.0",
    )
    assert resp.status_code == 401
