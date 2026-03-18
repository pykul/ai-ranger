.PHONY: all build test lint clean proto dev dev-reset down logs help test-integration

help:              ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

proto:             ## Compile .proto files and regenerate code for Python and Go
	$(MAKE) -C proto

build:             ## Build all components
	$(MAKE) -C agent build
	$(MAKE) -C gateway build
	$(MAKE) -C workers build
	$(MAKE) -C dashboard build

test:              ## Run all tests
	$(MAKE) -C agent test
	$(MAKE) -C gateway test
	$(MAKE) -C workers test

lint:              ## Lint all components
	$(MAKE) -C agent lint
	$(MAKE) -C gateway lint
	$(MAKE) -C workers lint
	$(MAKE) -C dashboard lint

dev:               ## Start dev environment (builds, waits for all 8 services healthy, then returns)
	$(MAKE) -C docker dev

dev-reset:         ## Wipe all volumes (data lost!) and restart dev (use after schema changes)
	$(MAKE) -C docker dev-reset

down:              ## Stop all Docker Compose services
	$(MAKE) -C docker down

logs:              ## Tail logs from all Docker Compose services
	$(MAKE) -C docker logs

test-integration:  ## Build agent, start Docker stack, run 23 integration tests (requires sudo)
	@bash tests/run-integration.sh

clean:             ## Clean all build artifacts
	$(MAKE) -C agent clean
	$(MAKE) -C gateway clean
	$(MAKE) -C workers clean
	$(MAKE) -C dashboard clean
	$(MAKE) -C proto clean
