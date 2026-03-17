COMPOSE         := docker compose --env-file ../.env -f docker-compose.yml
COMPOSE_DEV     := docker compose --env-file ../.env -f docker-compose.yml -f docker-compose.dev.yml

.PHONY: up dev dev-reset down logs reset ps build

up:                ## Start full stack (production-like)
	$(COMPOSE) up -d
	@echo ""
	@echo "Gateway Swagger UI:  http://localhost:8080/docs"
	@echo "Go API Swagger UI:   http://localhost:8081/docs"
	@echo "RabbitMQ Management: http://localhost:15672 (credentials from .env)"

dev:               ## Start full stack (returns when all services are healthy)
	DOCKER_BUILDKIT=0 $(COMPOSE_DEV) up -d --build --wait
	@echo ""
	@echo "All services healthy. Dashboard: http://localhost:8000"
	@echo "Gateway Swagger:  http://localhost:8080/docs"
	@echo "API Swagger:      http://localhost:8081/docs"
	@echo "RabbitMQ:         http://localhost:15672"
	@echo ""
	@echo "Run 'make logs' to view service logs."

dev-reset:         ## Tear down everything including volumes, then start dev (use after schema changes)
	$(COMPOSE) down -v
	DOCKER_BUILDKIT=0 $(COMPOSE_DEV) up -d --build --wait
	@echo ""
	@echo "All services healthy. Dashboard: http://localhost:8000"

down:              ## Stop all services
	$(COMPOSE) down

logs:              ## Tail logs from all services
	$(COMPOSE) logs -f

ps:                ## Show status of all services
	$(COMPOSE) ps

build:             ## Rebuild all Docker images
	$(COMPOSE) build

reset:             ## Tear down everything including volumes (destructive, migrations run fresh)
	$(COMPOSE) down -v
	$(COMPOSE) up -d
