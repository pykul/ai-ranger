"""Per-organization settings (webhook URL, etc.)."""

import uuid
from datetime import datetime, timezone

from sqlalchemy import Text, DateTime, ForeignKey, Uuid
from sqlalchemy.orm import Mapped, mapped_column

from models.base import Base


class OrgSettings(Base):
    """Per-org settings including the alerting webhook URL."""

    __tablename__ = "org_settings"

    id: Mapped[uuid.UUID] = mapped_column(Uuid, primary_key=True, default=uuid.uuid4)
    org_id: Mapped[uuid.UUID] = mapped_column(
        Uuid, ForeignKey("organizations.id"), nullable=False, unique=True
    )
    webhook_url: Mapped[str | None] = mapped_column(Text, nullable=True)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=lambda: datetime.now(timezone.utc)
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        default=lambda: datetime.now(timezone.utc),
        onupdate=lambda: datetime.now(timezone.utc),
    )
