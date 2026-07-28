#!/bin/bash
set -euo pipefail

# EAP 后端部署脚本 — 在 ECS self-hosted runner 上执行
# 环境变量由 deploy-pipeline.yml 注入:
#   IMAGE_TAG, ACR_REGISTRY, ACR_NAMESPACE, ACR_REPO
#   APP_SEED_ADMIN_EMAIL, APP_SEED_ADMIN_PASSWORD (from GitHub Secrets)

CONTAINER_NAME="eap-backend"
IMAGE="${ACR_REGISTRY}/${ACR_NAMESPACE}/${ACR_REPO}:${IMAGE_TAG}"

echo "=== Deploying ${IMAGE} ==="

podman pull "$IMAGE"

podman run -d \
  --name "$CONTAINER_NAME" \
  --replace \
  --restart=unless-stopped \
  -p 8080:8080 \
  -v /opt/eap/data:/app/data \
  -e APP_ENV=production \
  -e APP_DATABASE__URL=sqlite:///app/data/platform.db?mode=rwc \
  -e APP_LLM__BACKEND=llm \
  -e APP_SEED_ADMIN_EMAIL="${APP_SEED_ADMIN_EMAIL}" \
  -e APP_SEED_ADMIN_PASSWORD="${APP_SEED_ADMIN_PASSWORD}" \
  -e RUST_LOG=info,sqlx::pool=warn \
  "$IMAGE"
