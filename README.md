# Axum E‑Commerce API

Axum-based REST API for a lightweight e-commerce backend: products, carts, orders, favorites, and admin workflows with JWT auth, PostgreSQL persistence, and auto-generated OpenAPI docs.

## Stack
- Rust 2024, Axum 0.8, tower-http layers (request-id propagation, tracing, body limits, concurrency cap)
- SQLx (PostgreSQL) migrations + queries, chrono/uuid
- Auth: Argon2 password hashing, JWT (bearer), role-based guards (admin)
- Docs/UI: utoipa + Scalar at `/docs`
- Observability: tracing-subscriber, request IDs, audit logs

## Quickstart
1) Prereq: PostgreSQL running and `DATABASE_URL` reachable; Rust toolchain installed.  
2) Configure `.env` (example):
   ```env
   DATABASE_URL=postgres://postgres:postgres@localhost:5432/axum_ecommerce
   JWT_SECRET=change_me
   APP_HOST=127.0.0.1
   APP_PORT=3000
   ```
3) Run (migrations auto-run on startup):
   ```sh
   cargo run
   ```
4) API docs: open `http://127.0.0.1:3000/docs`.

## Development workflow
- Hot feedback: `bacon` (tasks are defined in `bacon.toml`; default `check`).
- Lint: `cargo clippy --all-features --all-targets -- -D warnings`
- Test: `cargo test --all-features`
- Format: `cargo fmt`
- Make targets: see `Makefile` (`make setup`, `make dev`, `make lint`, `make test`, `make migrate`, `make docker-build`, `make compose-up/down`).
- Seed sample data: `make seed` (creates admin/user accounts + demo products; runs migrations first).

## Endpoints (high level)
- `GET /health` – health check
- Auth: `POST /api/auth/register`, `POST /api/auth/login` (returns bearer token)
- Products: list/search/sort/paginate, CRUD (admin)
- Cart: list, add, remove
- Orders: list by user, checkout, pay, fetch single
- Favorites: list, add, remove
- Admin: list all orders, fetch/update order status, low-stock list + inventory adjustments
- OpenAPI: served at `/docs` (Scalar UI)

All protected routes expect `Authorization: Bearer <token>`; admin-only routes enforce `role == "admin"`.

## Data model (migrations)
- users (email, password_hash, role), products, favorites, cart_items
- orders + order_items (with payment_status, invoice_number, paid_at, updated_at)
- audit_logs for key actions

## Middleware & limits
- ConcurrencyLimitLayer (100 concurrent requests), 1MB request body limit
- Request IDs injected (`x-request-id`) and propagated for tracing

## Project layout
- `src/routes/` – route handlers (auth, products, cart, orders, favorites, admin, docs, health)
- `src/models.rs` – SQLx models
- `src/middleware/auth.rs` – JWT extractor & role checks
- `src/response.rs` – `ApiResponse` + pagination meta
- `migrations/` – SQLx migrations
- `bacon.toml` – dev tasks/watch config
- `Makefile` – common dev shortcuts
- `Dockerfile`, `docker-compose.yml` – containerized app + Postgres
- `.env.example` – starter environment variables

## Running migrations manually (optional)
```sh
cargo sqlx migrate run
```
(Runtime also applies migrations automatically on startup.)

## Tests
- `cargo test` or `make test` (includes a health check test; expand as needed for routes/integration).

## Notes
- Product listing responses are raw arrays (transparent wrapper) under `ApiResponse.data`.
- Tracing level can be tuned via `RUST_LOG` (defaults to `info,axum_ecommerce_api=debug`).***

## Docker & Compose
- Build image: `make docker-build`
- Run with Postgres: `make compose-up` (exposes app on `localhost:3000`, Postgres on `localhost:5432`)
- Stop: `make compose-down`
- Logs: `make compose-logs`

## Using as a template
1) Copy/clone repo.
2) Run `make setup` to create `.env` from `.env.example`.
3) Adjust names (crate/package) if desired, and set `JWT_SECRET`.
4) Start Postgres via `docker compose up` (or your own DB) and run `cargo run` or `make dev`.
5) Update `README`/migrations/models/routes` as you build your own domain.
