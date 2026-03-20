"""Add known_providers table for new-provider-first-seen alerting.

Revision ID: 0004
Revises: 0003
Create Date: 2026-03-20
"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa

revision: str = "0004"
down_revision: Union[str, None] = "0003"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    op.create_table(
        "known_providers",
        sa.Column("id", sa.Uuid(), primary_key=True, server_default=sa.text("gen_random_uuid()")),
        sa.Column("org_id", sa.Uuid(), sa.ForeignKey("organizations.id"), nullable=False),
        sa.Column("provider", sa.Text(), nullable=False),
        sa.Column("first_seen_at", sa.DateTime(timezone=True), server_default=sa.func.now(), nullable=False),
    )
    # Unique constraint: one entry per (org, provider) pair.
    op.create_index(
        "idx_known_providers_org_provider",
        "known_providers",
        ["org_id", "provider"],
        unique=True,
    )


def downgrade() -> None:
    op.drop_table("known_providers")
