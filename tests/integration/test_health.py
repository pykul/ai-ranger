"""Health check endpoint tests."""


def test_gateway_health(gateway_api):
    """GET /health on gateway returns correct body."""
    body = gateway_api.health()
    assert body["status"] == "ok"
    assert body["service"] == "gateway"


def test_api_health(api_server):
    """GET /health on API server returns correct body."""
    body = api_server.health()
    assert body["status"] == "ok"
    assert body["service"] == "api"
