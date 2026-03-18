.PHONY: all build test lint clean proto dev dev-reset down logs help test-integration \
       install ps prod prod-down dev-dashboard

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

install:           ## Install dependencies for all components
	$(MAKE) -C dashboard install
	$(MAKE) -C gateway install
	cd workers && go mod download

dev:               ## Start dev environment (builds, waits for all 8 services healthy, then returns)
	$(MAKE) -C docker dev

dev-reset:         ## Wipe all volumes (data lost!) and restart dev (use after schema changes)
	$(MAKE) -C docker dev-reset

dev-dashboard:     ## Start just the dashboard in hot-reload mode for frontend development
	$(MAKE) -C dashboard dev

down:              ## Stop all Docker Compose services
	$(MAKE) -C docker down

logs:              ## Tail logs from all Docker Compose services
	$(MAKE) -C docker logs

ps:                ## Show running status of all Docker Compose services
	$(MAKE) -C docker ps

prod:              ## Start production stack (requires DOMAIN, JWT_SECRET, ADMIN_EMAIL, ADMIN_PASSWORD in .env)
	@missing=""; \
	for var in DOMAIN JWT_SECRET ADMIN_EMAIL ADMIN_PASSWORD; do \
		val=$$(grep "^$$var=" .env 2>/dev/null | cut -d= -f2-); \
		if [ -z "$$val" ] || echo "$$val" | grep -q '^#'; then \
			missing="$$missing $$var"; \
		fi; \
	done; \
	if [ -n "$$missing" ]; then \
		echo ""; \
		echo "WARNING: The following required production variables are not set in .env:"; \
		echo "  $$missing"; \
		echo ""; \
		echo "Set them before starting production. See README.md § Production deployment."; \
		echo ""; \
		exit 1; \
	fi
	cd docker && docker compose --env-file ../.env \
		-f docker-compose.yml -f docker-compose.prod.yml \
		up -d --build --wait
	@echo ""
	@echo "Production stack running. Dashboard: https://$$(grep '^DOMAIN=' .env | cut -d= -f2)"

prod-down:         ## Stop production stack
	cd docker && docker compose --env-file ../.env \
		-f docker-compose.yml -f docker-compose.prod.yml \
		down

test-integration:  ## Build agent, start Docker stack, run integration tests (requires sudo)
	@bash tests/run-integration.sh

clean:             ## Clean all build artifacts
	$(MAKE) -C agent clean
	$(MAKE) -C gateway clean
	$(MAKE) -C workers clean
	$(MAKE) -C dashboard clean
	$(MAKE) -C proto clean
