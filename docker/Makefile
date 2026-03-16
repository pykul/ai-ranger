COMPOSE         := docker compose --env-file ../.env -f docker-compose.yml
COMPOSE_DEV     := docker compose --env-file ../.env -f docker-compose.yml -f docker-compose.dev.yml

.PHONY: up dev down logs reset ps build

up:                ## Start full stack (production-like)
	$(COMPOSE) up -d
	@echo ""
	@echo "Gateway Swagger UI:  http://localhost:8080/docs"
	@echo "Go API Swagger UI:   http://localhost:8081/docs"
	@echo "RabbitMQ Management: http://localhost:15672 (credentials from .env)"

dev:               ## Start full stack with dev overrides (source mounts, hot reload)
	$(COMPOSE_DEV) up
	@echo ""
	@echo "Gateway Swagger UI:  http://localhost:8080/docs"
	@echo "Go API Swagger UI:   http://localhost:8081/docs"
	@echo "RabbitMQ Management: http://localhost:15672 (credentials from .env)"

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
