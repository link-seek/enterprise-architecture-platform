#!/bin/bash
set -euo pipefail

# EAP 后端自动部署 agent — 在 ECS 上由 systemd timer 定时调用
# 检查 ACR 镜像是否有更新，有则 pull + restart + health check

ACR_REGISTRY="crpi-fswclfmtgfktdkq7.cn-guangzhou.personal.cr.aliyuncs.com"
ACR_NAMESPACE="eap-app"
IMAGE_NAME="backend"
TAG="latest"
FULL_IMAGE="${ACR_REGISTRY}/${ACR_NAMESPACE}/${IMAGE_NAME}:${TAG}"
CONTAINER_NAME="eap-backend"
DATA_DIR="/opt/eap/data"
LOG_FILE="/var/log/eap-autodeploy.log"

log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"; }

# ACR 凭证从 /opt/eap/acr-creds.env 读取
if [ ! -f /opt/eap/acr-creds.env ]; then
  log "ERROR: /opt/eap/acr-creds.env not found, skipping"
  exit 0
fi
source /opt/eap/acr-creds.env

# 登录 ACR
echo "${ACR_PASSWORD}" | podman login -u "${ACR_USERNAME}" --password-stdin "${ACR_REGISTRY}" >/dev/null 2>&1

# 获取当前本地镜像 ID
OLD_ID=$(podman images --format '{{.ID}}' "${FULL_IMAGE}" 2>/dev/null | head -1 || echo "")

# Pull 最新镜像
if ! podman pull "${FULL_IMAGE}" >/dev/null 2>&1; then
  log "WARN: pull failed, skipping"
  exit 0
fi

NEW_ID=$(podman images --format '{{.ID}}' "${FULL_IMAGE}" 2>/dev/null | head -1 || echo "")

if [ "${OLD_ID}" = "${NEW_ID}" ] && [ -n "${OLD_ID}" ]; then
  # 镜像没变，检查容器是否在跑
  if podman ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    exit 0  # 一切正常，跳过
  fi
  log "Container not running, restarting with same image..."
fi

log "New image detected (or container down), deploying..."
log "  Old: ${OLD_ID:-none}  New: ${NEW_ID}"

# 停掉旧容器
if podman ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
  podman stop "${CONTAINER_NAME}" 2>/dev/null || true
  podman rm "${CONTAINER_NAME}" 2>/dev/null || true
fi

# 启动新容器
mkdir -p "${DATA_DIR}"
podman run -d \
  --name "${CONTAINER_NAME}" \
  --restart=always \
  --network host \
  -v "${DATA_DIR}:/app/data" \
  -e RUST_LOG=info \
  "${FULL_IMAGE}" >/dev/null 2>&1

# 健康检查
sleep 3
for i in $(seq 1 15); do
  if curl -sf http://localhost:8080/health >/dev/null 2>&1; then
    log "OK: Backend healthy (image ${NEW_ID:0:12})"
    exit 0
  fi
  sleep 2
done

log "ERROR: Health check failed after 30s"
podman logs --tail 30 "${CONTAINER_NAME}" 2>&1 | tee -a "$LOG_FILE"
exit 1
