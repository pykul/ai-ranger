"""Typed API client for the AI Ranger Go API server.

Encapsulates route paths so test code never constructs raw HTTP calls
against the API server.
"""

import httpx


class APIServer:
    """Typed wrapper around the Go API server HTTP endpoints."""

    def __init__(self, client: httpx.Client):
        self._client = client

    def health(self) -> dict:
        """GET /health — returns {"status": "ok", "service": "api"}."""
        resp = self._client.get("/health")
        resp.raise_for_status()
        return resp.json()

    def fleet(self) -> list[dict]:
        """GET /v1/dashboard/fleet — returns list of enrolled agents."""
        resp = self._client.get("/v1/dashboard/fleet")
        resp.raise_for_status()
        return resp.json()

    def overview(self) -> httpx.Response:
        """GET /v1/dashboard/overview — returns raw response (may 500 during ClickHouse settle)."""
        return self._client.get("/v1/dashboard/overview")

    def providers(self) -> list[dict]:
        """GET /v1/dashboard/providers — returns provider breakdown."""
        resp = self._client.get("/v1/dashboard/providers")
        resp.raise_for_status()
        return resp.json()

    def users(self) -> list[dict]:
        """GET /v1/dashboard/users — returns per-user activity."""
        resp = self._client.get("/v1/dashboard/users")
        resp.raise_for_status()
        return resp.json()

    def traffic(self) -> list[dict]:
        """GET /v1/dashboard/traffic/timeseries -- returns hourly traffic."""
        resp = self._client.get("/v1/dashboard/traffic/timeseries")
        resp.raise_for_status()
        return resp.json()

    def machines(self, **params: str | int) -> httpx.Response:
        """GET /v1/dashboard/machines -- returns per-machine activity."""
        return self._client.get("/v1/dashboard/machines", params=params)

    def events(self, **params: str | int) -> httpx.Response:
        """GET /v1/events — returns raw response for flexible assertions."""
        return self._client.get("/v1/events", params=params)

    def create_token(self, org_id: str, label: str, max_uses: int) -> dict:
        """POST /v1/admin/tokens — creates enrollment token."""
        resp = self._client.post(
            "/v1/admin/tokens",
            json={"org_id": org_id, "label": label, "max_uses": max_uses},
        )
        resp.raise_for_status()
        return resp.json()

    def delete_token(self, token_id: str) -> None:
        """DELETE /v1/admin/tokens/:id — revokes token."""
        resp = self._client.delete(f"/v1/admin/tokens/{token_id}")
        resp.raise_for_status()

    def revoke_agent(self, agent_id: str) -> None:
        """DELETE /v1/admin/agents/:id — revokes agent."""
        resp = self._client.delete(f"/v1/admin/agents/{agent_id}")
        resp.raise_for_status()

    def get_settings(self) -> httpx.Response:
        """GET /v1/admin/settings - returns org settings (masked webhook URL)."""
        return self._client.get("/v1/admin/settings")

    def update_settings(self, org_id: str, webhook_url: str | None) -> httpx.Response:
        """PUT /v1/admin/settings - updates webhook URL."""
        return self._client.put(
            "/v1/admin/settings",
            json={"org_id": org_id, "webhook_url": webhook_url},
        )

    def test_webhook(self) -> httpx.Response:
        """POST /v1/admin/settings/test - fires a test webhook."""
        return self._client.post("/v1/admin/settings/test")
