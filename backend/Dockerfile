FROM rust:1.94-slim AS builder
RUN apt-get update && apt-get install -y curl pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN test -d ./templates/pilot-consumer
RUN cargo build --release -p server

FROM gcr.io/distroless/cc-debian13
WORKDIR /app
COPY --from=builder /app/target/release/server ./server-bin
COPY --from=builder /app/config ./config
COPY --from=builder /app/migration ./migration
COPY --from=builder /app/templates ./templates
EXPOSE 8080
CMD ["./server-bin"]
