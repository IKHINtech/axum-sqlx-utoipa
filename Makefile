SHELL := /bin/sh
APP_NAME := axum-ecommerce-api
ENV_FILE ?= .env

.PHONY: setup dev check fmt lint test migrate seed docker-build compose-up compose-down compose-logs

setup:
	@test -f $(ENV_FILE) || cp .env.example $(ENV_FILE)
	@echo "Env ready at $(ENV_FILE)"

dev:
	cargo run

check:
	cargo check --all-features --all-targets

fmt:
	cargo fmt

lint:
	cargo clippy --all-features --all-targets -- -D warnings

test:
	cargo test --all-features

migrate:
	cargo run --bin migrate

seed:
	cargo run --bin seed

docker-build:
	docker build -t $(APP_NAME):latest .

compose-up:
	docker compose up -d --build

compose-down:
	docker compose down

compose-logs:
	docker compose logs -f
