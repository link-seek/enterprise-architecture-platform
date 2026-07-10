# === Stage 1: cargo-chef planner ===
FROM rust:1.85-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

# === Stage 2: analyze dependencies ===
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# === Stage 3: build dependencies + app ===
FROM chef AS builder
# Use USTC mirror for faster downloads in China
RUN mkdir -p /usr/local/cargo && \
    echo '[source.crates-io]\nreplace-with = "ustc"\n[source.ustc]\nregistry = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"' \
    > /usr/local/cargo/config.toml
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --recipe-path recipe.json --release
COPY . .
RUN cargo build --release

# === Stage 4: minimal runtime ===
FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/server /app/server
COPY --from=builder /app/config /app/config
# Create data directory for SQLite
RUN mkdir -p /app/data
EXPOSE 8080
CMD ["/app/server"]
