.PHONY: all build test lint clean proto dev down logs help test-integration

help:              ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

proto:             ## Compile .proto files and regenerate code for Python and Go
	$(MAKE) -C proto

build:             ## Build all components
	$(MAKE) -C agent build
	$(MAKE) -C gateway build
	$(MAKE) -C workers build

test:              ## Run all tests
	$(MAKE) -C agent test
	$(MAKE) -C gateway test
	$(MAKE) -C workers test

lint:              ## Lint all components
	$(MAKE) -C agent lint
	$(MAKE) -C gateway lint
	$(MAKE) -C workers lint

dev:               ## Start full local dev environment
	$(MAKE) -C docker dev

down:              ## Stop local dev environment
	$(MAKE) -C docker down

logs:              ## Tail logs from all services
	$(MAKE) -C docker logs

test-integration:  ## Build agent, start backend, run all integration tests (requires sudo)
	@bash tests/run-integration.sh

clean:             ## Clean all build artifacts
	$(MAKE) -C agent clean
	$(MAKE) -C gateway clean
	$(MAKE) -C workers clean
	$(MAKE) -C proto clean

# Phase 3+ targets (dashboard not yet created):
# dashboard-build: ## Build dashboard
#	$(MAKE) -C dashboard build
# dashboard-lint:  ## Lint dashboard
#	$(MAKE) -C dashboard lint
