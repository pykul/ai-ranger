"""FastAPI dependency functions for injection into route handlers."""

import uuid
from typing import AsyncGenerator

from fastapi import Depends, Header, HTTPException, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from constants import AGENT_STATUS_ACTIVE, AUTH_SCHEME
from database import async_session
from models.agent import Agent


async def get_db() -> AsyncGenerator[AsyncSession, None]:
    """Yield an async database session, closing it after the request."""
    async with async_session() as session:
        yield session


async def verify_bearer_token(
    authorization: str = Header(..., alias="Authorization"),
    db: AsyncSession = Depends(get_db),
) -> Agent:
    """Validate the Authorization Bearer token against enrolled agents.

    The token value is the agent_id (UUID). Returns the Agent record
    or raises 401 if not found or revoked.
    """
    if not authorization.startswith(f"{AUTH_SCHEME} "):
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid authorization scheme. Expected: Bearer <agent_id>",
        )

    token_value = authorization[len(AUTH_SCHEME) + 1 :]

    try:
        agent_id = uuid.UUID(token_value)
    except ValueError:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid agent_id format.",
        )

    result = await db.execute(select(Agent).where(Agent.id == agent_id))
    agent = result.scalar_one_or_none()

    if agent is None:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Unknown agent_id.",
        )

    if agent.status != AGENT_STATUS_ACTIVE:
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Agent has been revoked.",
        )

    return agent
