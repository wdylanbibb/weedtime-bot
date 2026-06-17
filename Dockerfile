# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder

WORKDIR /app

COPY weedtime-db/Cargo.toml weedtime-db/Cargo.lock ./weedtime-db/
COPY weedtime-bot/Cargo.toml weedtime-bot/Cargo.lock ./weedtime-bot/
COPY weedtime-db/src ./weedtime-db/src
COPY weedtime-bot/src ./weedtime-bot/src

RUN cargo build --manifest-path weedtime-bot/Cargo.toml --release --locked

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --system weedtime \
    && useradd --system --gid weedtime --home-dir /app --shell /usr/sbin/nologin weedtime

WORKDIR /app

COPY --from=builder /app/weedtime-bot/target/release/weedtime-bot /usr/local/bin/weedtime-bot
COPY weedtime-bot/assets ./assets

RUN mkdir -p /app/data \
    && chown -R weedtime:weedtime /app/data

ENV WEEDTIME_USER_DB_PATH=/app/data/user-stats.db \
    WEEDTIME_GUILD_DB_PATH=/app/data/guild-stats.db

VOLUME ["/app/data"]

USER weedtime

ENTRYPOINT ["/usr/local/bin/weedtime-bot"]
