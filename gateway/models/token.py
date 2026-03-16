"""Enrollment token model."""

import hashlib
import uuid
from datetime import datetime, timezone

from sqlalchemy import ForeignKey, Integer, String, DateTime
from sqlalchemy.orm import Mapped, mapped_column

from constants import TOKEN_HASH_ALGORITHM
from models.base import Base


class EnrollmentToken(Base):
    """A single-use or multi-use token for enrolling agents.

    Tokens are stored as SHA256 hashes -- the plaintext is never persisted.
    """

    __tablename__ = "enrollment_tokens"

    id: Mapped[uuid.UUID] = mapped_column(primary_key=True, default=uuid.uuid4)
    org_id: Mapped[uuid.UUID] = mapped_column(ForeignKey("organizations.id"), nullable=False)
    token_hash: Mapped[str] = mapped_column(String, unique=True, nullable=False)
    label: Mapped[str | None] = mapped_column(String, nullable=True)
    created_by: Mapped[uuid.UUID | None] = mapped_column(nullable=True)
    expires_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)
    max_uses: Mapped[int] = mapped_column(Integer, default=1)
    used_count: Mapped[int] = mapped_column(Integer, default=0)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=lambda: datetime.now(timezone.utc)
    )

    @staticmethod
    def hash_token(plaintext: str) -> str:
        """Hash a plaintext token for storage or lookup."""
        return hashlib.new(TOKEN_HASH_ALGORITHM, plaintext.encode()).hexdigest()
