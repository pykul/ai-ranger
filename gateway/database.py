"""Async SQLAlchemy engine and session factory.

The DATABASE_URL environment variable configures the connection.
Default is suitable for local development with docker compose.
"""

import os

from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

# Default matches docker-compose.yml postgres service.
DATABASE_URL = os.environ.get(
    "DATABASE_URL",
    "postgresql+asyncpg://ranger:ranger@localhost:5432/ranger",
)

engine = create_async_engine(DATABASE_URL, echo=False, pool_size=5, max_overflow=10)

async_session = async_sessionmaker(engine, class_=AsyncSession, expire_on_commit=False)
