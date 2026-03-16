"""GET /v1/agents/providers - serve the latest providers.toml.

No authentication required. The agent fetches this on startup to load
the provider registry without needing a compiled-in copy.
"""

import os

from fastapi import APIRouter, HTTPException, status
from fastapi.responses import PlainTextResponse

from constants import ROUTE_PROVIDERS, PROVIDERS_TOML_PATH

router = APIRouter()

# Resolve path relative to the repo root (one level up from gateway/).
_PROVIDERS_FILE = os.path.join(os.path.dirname(__file__), "..", "..", PROVIDERS_TOML_PATH)


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
    path = os.path.normpath(_PROVIDERS_FILE)
    if not os.path.isfile(path):
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="providers.toml not found.",
        )
    with open(path) as f:
        content = f.read()
    return PlainTextResponse(content)
