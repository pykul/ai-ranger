COMPOSE         := docker compose --env-file ../.env -f docker-compose.yml
COMPOSE_DEV     := docker compose --env-file ../.env -f docker-compose.yml -f docker-compose.dev.yml

.PHONY: up dev dev-reset down logs reset ps build

up:                ## Start base stack without dev overlays (no hot reload)
	$(COMPOSE) up -d
	@echo ""
	@echo "Gateway Swagger UI:  http://localhost:8080/docs"
	@echo "Go API Swagger UI:   http://localhost:8081/docs"
	@echo "RabbitMQ Management: http://localhost:15672 (credentials from .env)"

dev:               ## Build images, start dev stack, wait for all 8 services healthy
	DOCKER_BUILDKIT=0 $(COMPOSE_DEV) up -d --build --wait
	@echo ""
	@echo "All services healthy. Dashboard: http://localhost:8000"
	@echo "Gateway Swagger:  http://localhost:8080/docs"
	@echo "API Swagger:      http://localhost:8081/docs"
	@echo "RabbitMQ:         http://localhost:15672"
	@echo ""
	@echo "Run 'make logs' to view service logs."

dev-reset:         ## Wipe ALL volumes (data lost!), rebuild, wait for healthy (use after schema changes)
	$(COMPOSE) down -v
	@# Clear agent config so it re-enrolls against the fresh database.
	@# The agent runs as root/Administrator, so the config dir may need elevation.
	@# Covers all three platforms: Linux, macOS, and Windows (via APPDATA).
	@if [ "$$(uname -s)" = "Darwin" ]; then \
		dir="$$HOME/Library/Application Support/ai-ranger"; \
	else \
		dir="$${XDG_CONFIG_HOME:-$$HOME/.config}/ai-ranger"; \
	fi; \
	if [ -d "$$dir" ]; then \
		rm -rf "$$dir" 2>/dev/null || sudo rm -rf "$$dir"; \
		echo "Cleared agent config at $$dir (will re-enroll on next run)"; \
	fi; \
	windir="$${APPDATA:-}"; \
	if [ -n "$$windir" ] && [ -d "$$windir/ai-ranger" ]; then \
		rm -rf "$$windir/ai-ranger"; \
		echo "Cleared Windows agent config at $$windir/ai-ranger (will re-enroll on next run)"; \
	fi
	DOCKER_BUILDKIT=0 $(COMPOSE_DEV) up -d --build --wait
	@echo ""
	@echo "All services healthy. Dashboard: http://localhost:8000"

down:              ## Stop all Docker Compose services
	$(COMPOSE) down

logs:              ## Tail logs from all Docker Compose services
	$(COMPOSE) logs -f

ps:                ## Show running status of all services
	$(COMPOSE) ps

build:             ## Rebuild all Docker images without starting
	$(COMPOSE) build

reset:             ## Wipe ALL volumes and restart base stack (destructive, migrations run fresh)
	$(COMPOSE) down -v
	$(COMPOSE) up -d
