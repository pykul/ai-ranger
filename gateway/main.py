"""AI Ranger Gateway - FastAPI application.

Thin agent-facing gateway. Three responsibilities per request:
1. Verify Bearer token
2. Deserialize protobuf payload
3. Publish to RabbitMQ

No processing logic, no business logic. If it does more than auth + deserialize + enqueue,
it belongs in the Go workers.

Swagger UI: http://localhost:8080/docs
OpenAPI spec: http://localhost:8080/openapi.json
"""

from fastapi import FastAPI

from routers import health, ingest, enroll, providers

app = FastAPI(
    title="AI Ranger Gateway",
    description="Agent-facing ingest gateway. Receives protobuf event batches, "
    "validates enrollment tokens, and publishes to RabbitMQ.",
    version="0.1.0",
    docs_url="/docs",
    openapi_url="/openapi.json",
    root_path="/ingest",
)

app.include_router(health.router, tags=["Health"])
app.include_router(ingest.router, tags=["Ingest"])
app.include_router(enroll.router, tags=["Enrollment"])
app.include_router(providers.router, tags=["Providers"])
