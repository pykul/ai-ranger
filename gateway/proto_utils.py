"""Protobuf deserialization helpers.

Uses the generated Python types from proto/gen/python/.
"""

import sys
import os

# In Docker, proto files are copied to /app/proto/ by the Dockerfile.
# Locally, they live at ../proto/gen/python/ relative to the gateway directory.
_gateway_dir = os.path.dirname(os.path.abspath(__file__))
_docker_proto = os.path.join(_gateway_dir, "proto")
_local_proto = os.path.normpath(os.path.join(_gateway_dir, "..", "proto", "gen", "python"))

for _candidate in (_docker_proto, _local_proto):
    if os.path.isdir(_candidate) and _candidate not in sys.path:
        sys.path.insert(0, _candidate)
        break

from ranger.v1 import events_pb2  # noqa: E402


def deserialize_event_batch(data: bytes) -> events_pb2.EventBatch:
    """Deserialize a protobuf EventBatch from raw bytes.

    Raises:
        google.protobuf.message.DecodeError: If the payload is not a valid EventBatch.
    """
    batch = events_pb2.EventBatch()
    batch.ParseFromString(data)
    return batch
