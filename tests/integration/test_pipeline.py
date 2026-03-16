"""Pipeline verification tests — RabbitMQ queue state after ingest."""

import httpx

from conftest import RABBITMQ_MGMT_URL, RABBITMQ_USER, RABBITMQ_PASS
from helpers.proto import encode_batch, make_test_batch, make_test_event
from helpers.wait import wait_for_condition


def test_rabbitmq_queue_drains(gateway_client, enrolled_agent):
    """After ingest, ranger.ingest queue depth returns to 0."""
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

    def queue_is_drained() -> bool:
        r = httpx.get(
            f"{RABBITMQ_MGMT_URL}/api/queues/%2F/ranger.ingest",
            auth=(RABBITMQ_USER, RABBITMQ_PASS),
            timeout=5,
        )
        if r.status_code != 200:
            return False
        return r.json().get("messages", -1) == 0

    wait_for_condition(queue_is_drained, timeout_secs=15, description="ranger.ingest queue drain")


def test_dead_letter_queue_empty(gateway_client, enrolled_agent):
    """After successful ingest, ranger.dlq has zero messages."""
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

    # Wait for processing to complete
    def queue_processed() -> bool:
        r = httpx.get(
            f"{RABBITMQ_MGMT_URL}/api/queues/%2F/ranger.ingest",
            auth=(RABBITMQ_USER, RABBITMQ_PASS),
            timeout=5,
        )
        return r.status_code == 200 and r.json().get("messages", -1) == 0

    wait_for_condition(queue_processed, timeout_secs=15, description="ingest queue processed")

    r = httpx.get(
        f"{RABBITMQ_MGMT_URL}/api/queues/%2F/ranger.dlq",
        auth=(RABBITMQ_USER, RABBITMQ_PASS),
        timeout=5,
    )
    assert r.status_code == 200
    assert r.json().get("messages", -1) == 0, "Dead letter queue is not empty"
