# Multi-stage build to produce a small runtime image
FROM rust:1.81 as builder
WORKDIR /app

# System deps for PostgreSQL + OpenSSL
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
WORKDIR /app

COPY --from=builder /app/target/release/axum-ecommerce-api /usr/local/bin/app
COPY migrations ./migrations

ENV APP_HOST=0.0.0.0 \
    APP_PORT=3000 \
    RUST_LOG=info,axum_ecommerce_api=debug

EXPOSE 3000
CMD ["app"]
