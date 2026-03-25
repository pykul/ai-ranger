FROM golang:1.26-alpine AS builder

WORKDIR /app

# Copy proto generated code so the replace directive in go.mod resolves.
COPY proto/gen/go /proto/gen/go

# Copy workers source and build.
COPY workers/go.mod workers/go.sum ./
RUN go mod download
COPY workers/ .

# Generate Swagger docs from annotations, then build.
RUN go install github.com/swaggo/swag/cmd/swag@v1.8.1 && \
    swag init -g cmd/api/main.go -o cmd/api/docs --parseDependency
RUN go build -o /bin/api-server ./cmd/api

# -- Runtime image: single binary, minimal surface
FROM alpine:3.20
COPY --from=builder /bin/api-server /bin/api-server
EXPOSE 8081
CMD ["/bin/api-server"]
