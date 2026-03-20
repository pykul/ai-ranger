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
	@# The agent runs as root/Administrator, so we clear both the current
	@# user's and root's config dirs.  Tries all possible locations:
	@# native Linux/macOS paths, WSL-mapped Windows APPDATA, and native
	@# Windows APPDATA (for PowerShell invocation).  Missing dirs are
	@# silently skipped.
	@dirs=""; \
	if [ "$$(uname -s)" = "Darwin" ]; then \
		dirs="$$HOME/Library/Application Support/ai-ranger /var/root/Library/Application Support/ai-ranger"; \
	else \
		dirs="$${XDG_CONFIG_HOME:-$$HOME/.config}/ai-ranger /root/.config/ai-ranger"; \
	fi; \
	if grep -qi microsoft /proc/version 2>/dev/null; then \
		winappdata=$$(cmd.exe /C "echo %APPDATA%" 2>/dev/null | tr -d '\r'); \
		if [ -n "$$winappdata" ]; then \
			dirs="$$dirs $$(wslpath "$$winappdata")/ai-ranger"; \
		fi; \
	fi; \
	if [ -n "$${APPDATA:-}" ]; then \
		dirs="$$dirs $$APPDATA/ai-ranger"; \
	fi; \
	for dir in $$dirs; do \
		if [ -d "$$dir" ] || sudo test -d "$$dir" 2>/dev/null; then \
			rm -rf "$$dir" 2>/dev/null || sudo rm -rf "$$dir"; \
			echo "Cleared agent config at $$dir (will re-enroll on next run)"; \
		fi; \
	done
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
