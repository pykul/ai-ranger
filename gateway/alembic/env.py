"""Alembic environment configuration for async SQLAlchemy.

Imports all ORM models so Alembic can autogenerate migrations.
Database URL comes from the Settings class in config.py.
"""

import asyncio
import os
import sys
from logging.config import fileConfig

from alembic import context
from sqlalchemy import pool
from sqlalchemy.ext.asyncio import create_async_engine

# Ensure the gateway root is on sys.path so models and config are importable
# when alembic is invoked from a different working directory.
_gateway_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if _gateway_dir not in sys.path:
    sys.path.insert(0, _gateway_dir)

# Import all models so Alembic sees them for autogenerate.
from models.base import Base
from models.org import Organization  # noqa: F401
from models.token import EnrollmentToken  # noqa: F401
from models.agent import Agent  # noqa: F401

from config import get_settings

alembic_config = context.config

if alembic_config.config_file_name is not None:
    fileConfig(alembic_config.config_file_name)

target_metadata = Base.metadata

_settings = get_settings()


def run_migrations_offline() -> None:
    """Run migrations in 'offline' mode -- emit SQL to stdout."""
    context.configure(
        url=_settings.database_url,
        target_metadata=target_metadata,
        literal_binds=True,
        dialect_opts={"paramstyle": "named"},
    )
    with context.begin_transaction():
        context.run_migrations()


def do_run_migrations(connection) -> None:
    """Run migrations against a live connection."""
    context.configure(connection=connection, target_metadata=target_metadata)
    with context.begin_transaction():
        context.run_migrations()


async def run_migrations_online() -> None:
    """Run migrations in 'online' mode with an async engine."""
    connectable = create_async_engine(_settings.database_url, poolclass=pool.NullPool)

    async with connectable.connect() as connection:
        await connection.run_sync(do_run_migrations)

    await connectable.dispose()


if context.is_offline_mode():
    run_migrations_offline()
else:
    asyncio.run(run_migrations_online())
