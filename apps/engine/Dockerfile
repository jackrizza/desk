FROM rust:1.88-slim-bookworm AS chef

RUN apt-get update \
    && apt-get install -y --no-install-recommends protobuf-compiler pkg-config libssl-dev \
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
RUN cargo build -p engine --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /usr/sbin/nologin appuser
WORKDIR /app

COPY --from=builder /app/target/release/engine /usr/local/bin/engine

RUN mkdir -p /app/cache_data && chown -R appuser:appuser /app

USER appuser

ENV OPENAPI_BASE_URL=http://openapi:3000/api
ENV ENGINE_POLL_INTERVAL_SECONDS=30

CMD ["engine"]
