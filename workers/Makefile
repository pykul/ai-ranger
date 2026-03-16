INGEST_BIN  := bin/ingest-worker
API_BIN     := bin/api-server

.PHONY: build run-ingest run-api test lint clean swag

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
