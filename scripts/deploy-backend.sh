#!/bin/bash
set -euo pipefail

# EAP 后端部署脚本 - 在 ECS 上执行
# 用法: deploy-backend.sh <image-tag>
# 例如: deploy-backend.sh latest

ACR_REGISTRY="crpi-fswclfmtgfktdkq7.cn-guangzhou.personal.cr.aliyuncs.com"
ACR_NAMESPACE="eap-app"
IMAGE_NAME="backend"
CONTAINER_NAME="eap-backend"
TAG="${1:-latest}"
FULL_IMAGE="${ACR_REGISTRY}/${ACR_NAMESPACE}/${IMAGE_NAME}:${TAG}"

echo "=== EAP Backend Deploy ==="
echo "Image: ${FULL_IMAGE}"

# 1. 登录 ACR
echo "[1/5] Login ACR..."
echo "${ACR_PASSWORD}" | podman login -u "${ACR_USERNAME}" --password-stdin "${ACR_REGISTRY}"

# 2. 拉取最新镜像
echo "[2/5] Pull image..."
podman pull "${FULL_IMAGE}"

# 3. 停掉旧容器
echo "[3/5] Stop old container..."
if podman ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
  podman stop "${CONTAINER_NAME}" 2>/dev/null || true
  podman rm "${CONTAINER_NAME}" 2>/dev/null || true
fi

# 4. 启动新容器
echo "[4/5] Start new container..."
podman run -d \
  --name "${CONTAINER_NAME}" \
  --restart=always \
  -p 8080:8080 \
  -v /opt/eap/data:/app/data \
  -e RUST_LOG=info \
  "${FULL_IMAGE}"

# 5. 健康检查
echo "[5/5] Health check..."
sleep 3
for i in $(seq 1 10); do
  if curl -sf http://localhost:8080/health > /dev/null 2>&1; then
    echo "✅ Backend is healthy!"
    curl -s http://localhost:8080/health
    echo ""
    podman ps --filter "name=${CONTAINER_NAME}"
    exit 0
  fi
  echo "  Waiting... (${i}/10)"
  sleep 2
done

echo "❌ Backend health check failed!"
podman logs "${CONTAINER_NAME}" --tail 20
exit 1
