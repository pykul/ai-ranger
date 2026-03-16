"""Async SQLAlchemy engine and session factory.

Connection URL comes from the Settings class in config.py.
No os.environ calls here.
"""

from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from config import get_settings

_settings = get_settings()

engine = create_async_engine(_settings.database_url, echo=False, pool_size=5, max_overflow=10)

async_session = async_sessionmaker(engine, class_=AsyncSession, expire_on_commit=False)
