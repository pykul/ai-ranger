"""POST /v1/agents/enroll - enroll a new agent with an enrollment token.

Flow: validate token hash -> check expiry and usage -> create agent record -> return org_id.
"""

import uuid
from datetime import datetime, timezone

from fastapi import APIRouter, Depends, HTTPException, status
from pydantic import BaseModel
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from constants import ROUTE_ENROLL
from dependencies import get_db
from models.agent import Agent
from models.token import EnrollmentToken

router = APIRouter()


class EnrollmentRequest(BaseModel):
    """Request body for agent enrollment (mirrors protobuf EnrollmentRequest)."""

    token: str
    agent_id: str
    hostname: str
    os_username: str
    os: str
    agent_version: str


class EnrollmentResponse(BaseModel):
    """Response body after successful enrollment."""

    org_id: str
    agent_id: str


@router.post(
    ROUTE_ENROLL,
    response_model=EnrollmentResponse,
    status_code=status.HTTP_200_OK,
    summary="Enroll a new agent using an enrollment token",
    description="Validates the enrollment token, creates an agent record in Postgres, "
    "and returns the org_id. The token's used_count is incremented.",
    responses={
        200: {"description": "Agent enrolled successfully."},
        401: {"description": "Invalid, expired, or exhausted enrollment token."},
    },
)
async def enroll(
    req: EnrollmentRequest,
    db: AsyncSession = Depends(get_db),
) -> EnrollmentResponse:
    """Validate token, create agent, return org_id."""
    token_hash = EnrollmentToken.hash_token(req.token)

    result = await db.execute(
        select(EnrollmentToken)
        .where(EnrollmentToken.token_hash == token_hash)
        .with_for_update()
    )
    token_record = result.scalar_one_or_none()

    if token_record is None:
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid token.")

    # Check expiry.
    if token_record.expires_at is not None and token_record.expires_at < datetime.now(timezone.utc):
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Token expired.")

    # Check usage limit.
    if token_record.used_count >= token_record.max_uses:
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Token exhausted.")

    # Create agent record.
    agent = Agent(
        id=uuid.UUID(req.agent_id),
        org_id=token_record.org_id,
        hostname=req.hostname,
        os_username=req.os_username,
        os=req.os,
        agent_version=req.agent_version,
    )
    db.add(agent)

    # Increment token usage.
    token_record.used_count += 1
    db.add(token_record)

    await db.commit()

    return EnrollmentResponse(
        org_id=str(token_record.org_id),
        agent_id=req.agent_id,
    )
