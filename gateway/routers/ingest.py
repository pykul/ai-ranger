"""POST /v1/ingest - receive protobuf EventBatch from agents.

Flow: verify Bearer token -> deserialize protobuf -> publish to RabbitMQ -> 200.
No processing logic, no database writes, no business logic.
"""

from datetime import datetime, timezone

from fastapi import APIRouter, Depends, Request, Response, status
from sqlalchemy.ext.asyncio import AsyncSession

from constants import CONTENT_TYPE_PROTOBUF, ROUTE_INGEST
from dependencies import get_db, verify_bearer_token
from models.agent import Agent
from proto_utils import deserialize_event_batch
from rabbitmq import publish_event_batch

router = APIRouter()


@router.post(
    ROUTE_INGEST,
    status_code=status.HTTP_200_OK,
    summary="Receive protobuf EventBatch from an enrolled agent",
    description="Verifies the agent Bearer token, deserializes the protobuf payload, "
    "publishes raw bytes to RabbitMQ, and returns 200. No processing logic.",
    responses={
        200: {"description": "Batch accepted and queued."},
        401: {"description": "Invalid or missing Bearer token."},
        400: {"description": "Invalid protobuf payload."},
    },
)
async def ingest(
    request: Request,
    agent: Agent = Depends(verify_bearer_token),
    db: AsyncSession = Depends(get_db),
) -> Response:
    """Accept a protobuf EventBatch, validate, enqueue to RabbitMQ."""
    body = await request.body()

    # Validate the protobuf is parseable (raises on invalid data).
    deserialize_event_batch(body)

    # Publish raw bytes to RabbitMQ. Workers handle deserialization and storage.
    publish_event_batch(body)

    # Update agent last_seen_at.
    agent.last_seen_at = datetime.now(timezone.utc)
    db.add(agent)
    await db.commit()

    return Response(status_code=status.HTTP_200_OK)
