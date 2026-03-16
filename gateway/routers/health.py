"""GET /health - health check for readiness and liveness probes.

No authentication required. Returns HTTP 200 with a JSON status body.
Used by Docker Compose health checks and Kubernetes probes.
"""

from fastapi import APIRouter, status
from pydantic import BaseModel

from constants import ROUTE_HEALTH

router = APIRouter()


class HealthResponse(BaseModel):
    """Health check response body."""

    status: str
    service: str


@router.get(
    ROUTE_HEALTH,
    response_model=HealthResponse,
    status_code=status.HTTP_200_OK,
    summary="Health check",
    description="Returns HTTP 200 if the gateway is running. "
    "No authentication required. Used by Docker and Kubernetes probes.",
    responses={200: {"description": "Service is healthy."}},
)
async def health() -> HealthResponse:
    """Return health status."""
    return HealthResponse(status="ok", service="gateway")
