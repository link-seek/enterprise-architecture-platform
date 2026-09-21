FROM rust:1.94-slim AS builder
RUN apt-get update && apt-get install -y curl pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN test -d ./templates/pilot-consumer
RUN cargo build --release -p server

# runtime 需要 git：provisionPilotConsumer 的 push_rendered 走 git CLI
# （init/add/commit/remote/push）。原 distroless 无 shell/git，会在 push 步
# 报 `git init ... No such file or directory` 并触发 fail-closed 删仓。
# debian:13-slim 与 builder（rust:1.94-slim / glibc 2.39）同代，避免 glibc 不匹配。
FROM debian:13-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends git ca-certificates \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/server ./server-bin
COPY --from=builder /app/config ./config
COPY --from=builder /app/migration ./migration
COPY --from=builder /app/templates ./templates
# 构建期自检：生成器依赖的运行时物料缺失时直接失败，而不是等到生产 mutation 才炸。
RUN test -x ./server-bin \
 && test -d ./config \
 && test -d ./migration \
 && test -d ./templates/pilot-consumer \
 && git --version
EXPOSE 8080
CMD ["./server-bin"]
