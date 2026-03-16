"""Seed development data: test organization and enrollment token.

Revision ID: 0002
Revises: 0001
Create Date: 2026-03-16

Only inserts data when ENVIRONMENT=development AND SEED_TOKEN is set.
The SEED_TOKEN value is hashed with SHA256 before storage.
If SEED_TOKEN is not set, the migration completes without inserting anything.

For local development, .env.example sets SEED_TOKEN=tok_test_dev.
Production environments must NOT set SEED_TOKEN.
"""
import hashlib
import os
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa

revision: str = "0002"
down_revision: Union[str, None] = "0001"
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None

ORG_ID = "00000000-0000-0000-0000-000000000001"
TOKEN_ID = "00000000-0000-0000-0000-000000000002"


def upgrade() -> None:
    # Note: os.environ is used intentionally here. Alembic migrations run in a
    # separate process context where FastAPI's Settings class is not available.
    # This is an accepted exception to the "no os.environ outside config" rule.
    if os.environ.get("ENVIRONMENT") != "development":
        return

    seed_token = os.environ.get("SEED_TOKEN")
    if not seed_token:
        return

    token_hash = hashlib.sha256(seed_token.encode()).hexdigest()

    op.execute(
        sa.text(
            "INSERT INTO organizations (id, name, slug) "
            "VALUES (CAST(:org_id AS uuid), :name, :slug) "
            "ON CONFLICT (id) DO NOTHING"
        ).bindparams(org_id=ORG_ID, name="Dev Organization", slug="dev-org")
    )

    op.execute(
        sa.text(
            "INSERT INTO enrollment_tokens (id, org_id, token_hash, label, max_uses, used_count) "
            "VALUES (CAST(:tok_id AS uuid), CAST(:org_id AS uuid), :token_hash, :label, :max_uses, :used_count) "
            "ON CONFLICT (id) DO NOTHING"
        ).bindparams(
            tok_id=TOKEN_ID,
            org_id=ORG_ID,
            token_hash=token_hash,
            label="Development token (unlimited uses)",
            max_uses=2147483647,  # UNLIMITED_USES — max 32-bit signed int
            used_count=0,
        )
    )


def downgrade() -> None:
    op.execute(sa.text("DELETE FROM enrollment_tokens WHERE id = CAST(:tok_id AS uuid)").bindparams(tok_id=TOKEN_ID))
    op.execute(sa.text("DELETE FROM organizations WHERE id = CAST(:org_id AS uuid)").bindparams(org_id=ORG_ID))
