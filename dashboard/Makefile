.PHONY: build dev lint clean install

install:           ## Install dependencies
	npm ci

build: install     ## Build for production
	npm run build

dev:               ## Start dev server with hot reload
	npm run dev

lint:              ## Lint TypeScript
	npm run lint

clean:             ## Remove build artifacts
	rm -rf dist node_modules
