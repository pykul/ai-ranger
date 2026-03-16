"""GET /v1/agents/providers - serve the latest providers.toml.

No authentication required. The agent fetches this on startup to load
the provider registry without needing a compiled-in copy.
"""

import os

from fastapi import APIRouter, HTTPException, status
from fastapi.responses import PlainTextResponse

from config import get_settings
from constants import ROUTE_PROVIDERS

router = APIRouter()


@router.get(
    ROUTE_PROVIDERS,
    response_class=PlainTextResponse,
    summary="Fetch the latest providers.toml",
    description="Returns the community-maintained provider registry as plain text TOML. "
    "No authentication required. The agent fetches this on every startup.",
    responses={
        200: {"description": "providers.toml content as text/plain."},
        404: {"description": "providers.toml not found on the server."},
    },
)
async def get_providers() -> PlainTextResponse:
    """Serve providers.toml as plain text."""
    settings = get_settings()
    # Resolve relative to the gateway working directory, then try repo root.
    candidates = [
        settings.providers_toml_path,
        os.path.join(os.path.dirname(__file__), "..", "..", settings.providers_toml_path),
    ]
    for candidate in candidates:
        path = os.path.normpath(candidate)
        if os.path.isfile(path):
            with open(path) as f:
                return PlainTextResponse(f.read())

    raise HTTPException(
        status_code=status.HTTP_404_NOT_FOUND,
        detail="providers.toml not found.",
    )
