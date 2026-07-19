FROM rust:1.94-slim AS builder
RUN apt-get update && apt-get install -y curl pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN cargo build --release -p server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y sqlite3 ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/server ./server-bin
COPY --from=builder /app/config ./config
COPY --from=builder /app/migration ./migration
EXPOSE 8080
CMD ["./server-bin"]
