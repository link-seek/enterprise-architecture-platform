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

# Remove old container — handle stuck conmon scenarios
podman rm -f "$CONTAINER_NAME" 2>/dev/null || true
if podman container exists "$CONTAINER_NAME" 2>/dev/null; then
  echo "Container stuck, cleaning up storage..."
  CID=$(podman inspect "$CONTAINER_NAME" --format '{{.Id}}' 2>/dev/null || true)
  podman system prune -f --external 2>/dev/null || true
  if [ -n "$CID" ] && [ -d "/var/lib/containers/storage/overlay-containers/${CID}" ]; then
    rm -rf "/var/lib/containers/storage/overlay-containers/${CID}" 2>/dev/null || true
  fi
  podman rm -f "$CONTAINER_NAME" 2>/dev/null || true
fi

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
