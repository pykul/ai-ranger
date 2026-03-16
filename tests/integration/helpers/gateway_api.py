"""Typed API client for the AI Ranger gateway.

Encapsulates route paths, auth headers, and content-types so test code
never constructs raw HTTP calls against the gateway.
"""

import httpx


class GatewayAPI:
    """Typed wrapper around the gateway HTTP endpoints."""

    def __init__(self, client: httpx.Client):
        self._client = client

    def health(self) -> dict:
        """GET /health — returns {"status": "ok", "service": "gateway"}."""
        resp = self._client.get("/health")
        resp.raise_for_status()
        return resp.json()

    def enroll(
        self,
        token: str,
        agent_id: str,
        hostname: str = "test-host",
        os_username: str = "test-user",
        os: str = "linux",
        agent_version: str = "0.1.0-test",
    ) -> dict:
        """POST /v1/agents/enroll — returns {"agent_id": ..., "org_id": ...}."""
        resp = self._client.post(
            "/v1/agents/enroll",
            json={
                "token": token,
                "agent_id": agent_id,
                "hostname": hostname,
                "os_username": os_username,
                "os": os,
                "agent_version": agent_version,
            },
        )
        resp.raise_for_status()
        return resp.json()

    def enroll_raw(self, **kwargs) -> httpx.Response:
        """POST /v1/agents/enroll — returns raw response (for error testing)."""
        return self._client.post("/v1/agents/enroll", json=kwargs)

    def ingest(self, agent_id: str, protobuf_bytes: bytes) -> httpx.Response:
        """POST /v1/ingest — sends protobuf EventBatch with Bearer auth."""
        return self._client.post(
            "/v1/ingest",
            content=protobuf_bytes,
            headers={
                "Content-Type": "application/x-protobuf",
                "Authorization": f"Bearer {agent_id}",
            },
        )

    def ingest_raw(self, content: bytes, headers: dict | None = None) -> httpx.Response:
        """POST /v1/ingest — raw request (for auth/error testing)."""
        h = headers or {}
        return self._client.post("/v1/ingest", content=content, headers=h)

    def get_providers(self) -> httpx.Response:
        """GET /v1/agents/providers — returns providers.toml as plain text."""
        return self._client.get("/v1/agents/providers")
