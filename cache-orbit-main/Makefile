SHELL := bash
.PHONY: help dev dev-stop dev-logs test lint fmt

help:
	@echo "╔══════════════════════════════════════════╗"
	@echo "║        CACHE ORBIT — LOCAL VERIFY       ║"
	@echo "╚══════════════════════════════════════════╝"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

dev:
	@echo "🚀 Lancement de Cache Orbit..."
	docker compose up --build -d
	@echo "✅ Services prêts : http://localhost:3000 (UI) | http://localhost:8080 (API)"

dev-stop: ## Arrête l'environnement
	docker compose down -v

dev-logs: ## Affiche les logs
	docker compose logs -f

test: ## Exécute les tests Rust
	cd src/cache-engine && cargo test --all

lint: ## Lint Rust
	cd src/cache-engine && cargo clippy -- -D warnings

fmt: ## Format Rust
	cd src/cache-engine && cargo fmt
