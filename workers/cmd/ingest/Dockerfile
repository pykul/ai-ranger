FROM golang:1.26-alpine AS builder

WORKDIR /app

# Copy proto generated code so the replace directive in go.mod resolves.
COPY proto/gen/go /proto/gen/go

# Copy workers source and build.
COPY workers/go.mod workers/go.sum ./
RUN go mod download
COPY workers/ .

RUN go build -o /bin/ingest-worker ./cmd/ingest

# -- Runtime image: single binary, minimal surface
FROM alpine:3.20
COPY --from=builder /bin/ingest-worker /bin/ingest-worker
CMD ["/bin/ingest-worker"]
