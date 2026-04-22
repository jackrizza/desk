FROM rust:1.86-slim-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY apps/openapi/Cargo.toml apps/openapi/Cargo.toml
COPY crates/cache/Cargo.toml crates/cache/Cargo.toml
COPY crates/database/Cargo.toml crates/database/Cargo.toml
COPY crates/models/Cargo.toml crates/models/Cargo.toml
COPY crates/stock_data/Cargo.toml crates/stock_data/Cargo.toml
RUN mkdir -p apps/openapi/src crates/cache/src crates/database/src crates/models/src crates/stock_data/src
RUN echo "fn main() {}" > apps/openapi/src/main.rs
RUN echo "" > crates/cache/src/lib.rs
RUN echo "" > crates/database/src/lib.rs
RUN echo "" > crates/models/src/lib.rs
RUN echo "" > crates/stock_data/src/lib.rs
RUN cargo build -p openapi --release || true

COPY . .
RUN cargo build -p openapi --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /usr/sbin/nologin appuser
WORKDIR /app

COPY --from=builder /app/target/release/openapi /usr/local/bin/openapi

RUN mkdir -p /app/cache_data && chown -R appuser:appuser /app

USER appuser

ENV API_BIND_ADDRESS=0.0.0.0:3000
ENV CACHE_DIR=/app/cache_data

EXPOSE 3000

CMD ["openapi"]
