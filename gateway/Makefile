.PHONY: build run run-dev test lint install clean migrate migrate-create

install:           ## Install Python dependencies
	pip install -r requirements.txt

build: install     ## Install deps and verify the app loads cleanly
	python -c "from main import app"

migrate:           ## Run Alembic migrations (creates/updates Postgres schema)
	alembic upgrade head

migrate-create:    ## Create a new migration: make migrate-create name=add_foo_column
	alembic revision --autogenerate -m "$(name)"

run: migrate       ## Run gateway with uvicorn (migrates DB first)
	uvicorn main:app --host 0.0.0.0 --port 8080

run-dev: migrate   ## Run in dev mode with auto-reload (migrates DB first)
	uvicorn main:app --host 0.0.0.0 --port 8080 --reload

test:              ## Run tests
	pytest tests/ -v

lint:              ## Lint with ruff and type-check with mypy
	ruff check .
	mypy . --ignore-missing-imports

clean:             ## Remove cached files
	find . -type d -name __pycache__ -exec rm -rf {} +
	find . -name "*.pyc" -delete
