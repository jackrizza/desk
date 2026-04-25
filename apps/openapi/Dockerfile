FROM rust:1.88-slim-bookworm AS chef

RUN apt-get update \
    && apt-get install -y --no-install-recommends protobuf-compiler pkg-config libssl-dev python3 python3-dev python3-venv \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef --locked

WORKDIR /app

FROM chef AS planner

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build -p openapi --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl python3 \
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
