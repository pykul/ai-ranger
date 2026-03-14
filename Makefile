.PHONY: build test lint clean help

help:             ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

build:            ## Build all components
	$(MAKE) -C agent build

test:             ## Run all tests
	$(MAKE) -C agent test

lint:             ## Lint all components
	$(MAKE) -C agent lint

clean:            ## Clean all build artifacts
	$(MAKE) -C agent clean

# Phase 2+ targets (directories not yet created):
# proto:          ## Compile .proto files and regenerate code for all languages
#	$(MAKE) -C proto
# dev:            ## Start full local dev environment
#	$(MAKE) -C docker dev
# down:           ## Stop local dev environment
#	$(MAKE) -C docker down
# logs:           ## Tail logs from all services
#	$(MAKE) -C docker logs
