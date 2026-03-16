"""Health check endpoint tests."""


def test_gateway_health(gateway_client):
    """GET /health on gateway returns 200 with correct body."""
    resp = gateway_client.get("/health")
    assert resp.status_code == 200
    body = resp.json()
    assert body["status"] == "ok"
    assert body["service"] == "gateway"


def test_api_health(api_client):
    """GET /health on API server returns 200 with correct body."""
    resp = api_client.get("/health")
    assert resp.status_code == 200
    body = resp.json()
    assert body["status"] == "ok"
    assert body["service"] == "api"
