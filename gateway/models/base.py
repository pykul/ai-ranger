"""SQLAlchemy declarative base shared across all models."""

from sqlalchemy.orm import DeclarativeBase


class Base(DeclarativeBase):
    """Base class for all ORM models. Alembic uses this to detect schema changes."""

    pass
