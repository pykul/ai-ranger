"""Centralized configuration via pydantic-settings.

All runtime configuration comes from environment variables. No os.environ calls
elsewhere in the gateway codebase. Constants in constants.py are for application
contract values only (route paths, queue names, header names).

The Settings instance is created once at import time and injected into FastAPI
routes via the get_settings() dependency. If a required variable is missing or
invalid, pydantic-settings raises a ValidationError at startup — the service
fails fast with a clear error message.
"""

from functools import lru_cache

from pydantic import Field
from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    """Typed environment variable configuration for the AI Ranger gateway."""

    # -- Postgres --------------------------------------------------------------

    database_url: str = Field(
        default="postgresql+asyncpg://ranger:ranger@localhost:5432/ranger",
        description="SQLAlchemy async connection URL for Postgres. "
        "In Docker, set to postgresql+asyncpg://<user>:<pass>@postgres:5432/<db>.",
    )

    # -- RabbitMQ --------------------------------------------------------------

    rabbitmq_url: str = Field(
        default="amqp://guest:guest@localhost:5672/",
        description="AMQP connection URL for RabbitMQ. "
        "In Docker, set to amqp://<user>:<pass>@rabbitmq:5672/.",
    )

    # -- Server ----------------------------------------------------------------

    gateway_port: int = Field(
        default=8080,
        description="Port the gateway listens on. "
        "Must match the port exposed in docker-compose.yml.",
    )

    # -- Providers -------------------------------------------------------------

    providers_toml_path: str = Field(
        default="providers/providers.toml",
        description="Path to the community provider registry file. "
        "Relative to the repo root locally, or an absolute path in Docker.",
    )

    # -- Environment -----------------------------------------------------------

    environment: str = Field(
        default="production",
        description="Set to 'development' to enable seed data and debug features. "
        "Any other value disables development-only behavior.",
    )

    # -- Seed data -------------------------------------------------------------

    seed_token: str | None = Field(
        default=None,
        description="Plaintext enrollment token to seed into the database. "
        "Only used when ENVIRONMENT=development. "
        "If not set, the seed migration is skipped entirely.",
    )

    # -- Graceful shutdown -----------------------------------------------------

    shutdown_timeout_secs: int = Field(
        default=30,
        description="Seconds to wait for in-flight requests before force-stopping.",
    )

    model_config = {"env_prefix": "", "case_sensitive": False}


@lru_cache
def get_settings() -> Settings:
    """Return the cached Settings instance. Created once at first call."""
    return Settings()
