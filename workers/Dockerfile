FROM golang:1.26-alpine AS builder

WORKDIR /app

# Copy proto generated code first so the replace directive in go.mod resolves.
COPY proto/gen/go /proto/gen/go

# Copy workers source.
COPY workers/go.mod workers/go.sum ./
RUN go mod download
COPY workers/ .

RUN go build -o /bin/ingest-worker ./cmd/ingest
RUN go build -o /bin/api-server ./cmd/api

# -- Dev target (hot reload via CompileDaemon)
FROM golang:1.26-alpine AS dev
RUN go install github.com/githubnemo/CompileDaemon@latest
WORKDIR /app
COPY proto/gen/go /proto/gen/go
COPY workers/go.mod workers/go.sum ./
RUN go mod download
COPY workers/ .

# -- Ingest worker target
FROM alpine:3.20 AS ingest
COPY --from=builder /bin/ingest-worker /bin/ingest-worker
CMD ["/bin/ingest-worker"]

# -- API server target
FROM alpine:3.20 AS api
COPY --from=builder /bin/api-server /bin/api-server
EXPOSE 8081
CMD ["/bin/api-server"]
