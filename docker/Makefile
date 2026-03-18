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
