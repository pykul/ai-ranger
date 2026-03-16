INGEST_BIN  := bin/ingest-worker
API_BIN     := bin/api-server

.PHONY: build run-ingest run-api test lint clean swag docker-build docker-build-ingest docker-build-api

swag:              ## Generate Swagger docs from annotations
	swag init -g cmd/api/main.go -o cmd/api/docs --parseDependency

build: swag        ## Build all Go binaries (includes Swagger generation)
	go build -o $(INGEST_BIN) ./cmd/ingest
	go build -o $(API_BIN) ./cmd/api

run-ingest:        ## Run ingest worker locally
	go run ./cmd/ingest

run-api:           ## Run query API server locally
	go run ./cmd/api

test:              ## Run all tests
	go test ./... -v

lint:              ## Run golangci-lint
	golangci-lint run ./...

clean:             ## Remove build artifacts
	rm -rf bin/

docker-build-ingest:  ## Build ingest worker Docker image
	docker build -f cmd/ingest/Dockerfile -t ai-ranger/ingest-worker ..

docker-build-api:     ## Build API server Docker image
	docker build -f cmd/api/Dockerfile -t ai-ranger/api-server ..

docker-build: docker-build-ingest docker-build-api  ## Build all Docker images
