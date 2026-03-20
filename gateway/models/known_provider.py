"""Known providers table for new-provider-first-seen alerting."""

import uuid
from datetime import datetime, timezone

from sqlalchemy import Text, DateTime, ForeignKey, Uuid, UniqueConstraint
from sqlalchemy.orm import Mapped, mapped_column

from models.base import Base


class KnownProvider(Base):
    """Tracks which providers have been seen for each organization.

    When a provider appears for the first time for an org, a row is inserted
    and an optional webhook fires. The unique constraint on (org_id, provider)
    ensures only one row per org-provider pair.
    """

    __tablename__ = "known_providers"
    __table_args__ = (
        UniqueConstraint("org_id", "provider", name="idx_known_providers_org_provider"),
    )

    id: Mapped[uuid.UUID] = mapped_column(Uuid, primary_key=True, default=uuid.uuid4)
    org_id: Mapped[uuid.UUID] = mapped_column(
        Uuid, ForeignKey("organizations.id"), nullable=False
    )
    provider: Mapped[str] = mapped_column(Text, nullable=False)
    first_seen_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        default=lambda: datetime.now(timezone.utc),
        nullable=False,
    )
