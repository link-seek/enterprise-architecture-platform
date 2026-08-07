#!/bin/bash
set -euo pipefail

# EAP 后端部署脚本 — 在 ECS self-hosted runner 上执行
# 环境变量由 deploy-pipeline.yml 注入:
#   IMAGE_TAG, ACR_REGISTRY, ACR_NAMESPACE, ACR_REPO
#   APP_SEED_ADMIN_EMAIL, APP_SEED_ADMIN_PASSWORD (from GitHub Secrets)
#   APP_SEED_EDITOR_EMAIL, APP_SEED_EDITOR_PASSWORD (from GitHub Secrets)
#   APP_SEED_STRANGER_EMAIL, APP_SEED_STRANGER_PASSWORD (from GitHub Secrets)

CONTAINER_NAME="eap-backend"
IMAGE="${ACR_REGISTRY}/${ACR_NAMESPACE}/${ACR_REPO}:${IMAGE_TAG}"
SERVICE_FILE="/etc/systemd/system/eap-backend.service"

echo "=== Deploying ${IMAGE} ==="

podman pull "$IMAGE"

# Stop existing service
systemctl stop eap-backend 2>/dev/null || true
podman rm -f "$CONTAINER_NAME" 2>/dev/null || true

# Create systemd service that runs podman in foreground
# This avoids conmon dying and leaving the container unresponsive
cat > "$SERVICE_FILE" << EOF
[Unit]
Description=EAP Backend (Podman)
After=network.target
Wants=network-online.target

[Service]
Type=simple
ExecStartPre=-/usr/bin/podman rm -f ${CONTAINER_NAME}
ExecStart=/usr/bin/podman run --name ${CONTAINER_NAME} --network=host -v /opt/eap/data:/app/data -e APP_ENV=production -e APP_DATABASE__URL=sqlite:///app/data/platform.db?mode=rwc -e APP_SEED_ADMIN_EMAIL=${APP_SEED_ADMIN_EMAIL} -e APP_SEED_ADMIN_PASSWORD=${APP_SEED_ADMIN_PASSWORD} -e APP_SEED_EDITOR_EMAIL=${APP_SEED_EDITOR_EMAIL} -e APP_SEED_EDITOR_PASSWORD=${APP_SEED_EDITOR_PASSWORD} -e APP_SEED_STRANGER_EMAIL=${APP_SEED_STRANGER_EMAIL} -e APP_SEED_STRANGER_PASSWORD=${APP_SEED_STRANGER_PASSWORD} -e RUST_LOG=info,sqlx::pool=warn ${IMAGE}
ExecStop=/usr/bin/podman stop ${CONTAINER_NAME}
Restart=always
RestartSec=5
TimeoutStartSec=60

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable eap-backend
systemctl restart eap-backend

# Wait for health check
echo "=== Waiting for backend to start ==="
for i in $(seq 1 15); do
  if curl -sf http://localhost:8080/health 2>/dev/null; then
    echo ""
    echo "Backend healthy!"
    exit 0
  fi
  echo "Waiting... ($i/15)"
  sleep 2
done
echo "ERROR: Health check failed after 30s"
exit 1
