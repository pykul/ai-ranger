"""Synthetic protobuf event builders for integration tests."""

import sys
import os
import time
import uuid

# Add proto generated code to path
_repo_root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
_proto_dir = os.path.join(_repo_root, "proto", "gen", "python")
if _proto_dir not in sys.path:
    sys.path.insert(0, _proto_dir)

from ranger.v1 import events_pb2  # noqa: E402


def make_test_event(
    agent_id: str,
    provider: str = "openai",
    provider_host: str = "api.openai.com",
    detection_method: int = 0,  # SNI
    process_name: str = "test-process",
) -> events_pb2.AiConnectionEvent:
    """Build a deterministic test event with all required fields populated."""
    return events_pb2.AiConnectionEvent(
        agent_id=agent_id,
        machine_hostname="integration-test-host",
        os_username="test-user",
        os_type="linux",
        timestamp_ms=int(time.time() * 1000),
        provider=provider,
        provider_host=provider_host,
        process_name=process_name,
        process_pid=12345,
        connection_id=uuid.uuid4().hex[:16],
        detection_method=detection_method,
        capture_mode=0,  # DNS_SNI
        src_ip="10.0.0.1",
    )


def make_test_batch(agent_id: str, events: list) -> events_pb2.EventBatch:
    """Wrap events in an EventBatch."""
    return events_pb2.EventBatch(
        agent_id=agent_id,
        sent_at_ms=int(time.time() * 1000),
        events=events,
    )


def encode_batch(batch: events_pb2.EventBatch) -> bytes:
    """Serialize to protobuf bytes."""
    return batch.SerializeToString()
