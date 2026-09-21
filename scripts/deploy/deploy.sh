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
ENV_FILE="/opt/eap/eap-backend.env"
# Task3 试点 App 私钥落盘位置 + podman 启动包装脚本
PILOT_KEY_FILE="/opt/eap/pilot-app-key.pem"
RUNNER_FILE="/opt/eap/run-backend.sh"

echo "=== Deploying ${IMAGE} ==="

podman pull "$IMAGE"

# Stop existing service
systemctl stop eap-backend 2>/dev/null || true
podman rm -f "$CONTAINER_NAME" 2>/dev/null || true

# Generate a restricted-permission env file so that seed passwords are not
# exposed in the systemd unit file or in the process list (ps / /proc).
# Optional seed vars (editor/stranger) are only written when non-empty: the
# backend treats unset env (std::env::var returns Err) as "skip seeding", but
# an empty string would be treated as a valid (too-short) password and bail.
# Fall back to E2E_* env vars so the backend seeds with the same credentials
# the tests use, even when APP_SEED_* secrets are not separately configured.
mkdir -p "$(dirname "$ENV_FILE")"
(
  umask 077
  {
    echo "APP_ENV=production"
    echo "APP_DATABASE__URL=sqlite:///app/data/platform.db?mode=rwc"
    printf 'APP_SEED_ADMIN_EMAIL=%s\n' "$APP_SEED_ADMIN_EMAIL"
    printf 'APP_SEED_ADMIN_PASSWORD=%s\n' "$APP_SEED_ADMIN_PASSWORD"
    SEED_EDITOR_EMAIL="${APP_SEED_EDITOR_EMAIL:-${E2E_EDITOR_EMAIL:-}}"
    SEED_EDITOR_PASSWORD="${APP_SEED_EDITOR_PASSWORD:-${E2E_EDITOR_PASSWORD:-}}"
    if [[ -n "${SEED_EDITOR_EMAIL:-}" && -n "${SEED_EDITOR_PASSWORD:-}" ]]; then
      printf 'APP_SEED_EDITOR_EMAIL=%s\n' "$SEED_EDITOR_EMAIL"
      printf 'APP_SEED_EDITOR_PASSWORD=%s\n' "$SEED_EDITOR_PASSWORD"
    fi
    SEED_STRANGER_EMAIL="${APP_SEED_STRANGER_EMAIL:-${E2E_STRANGER_EMAIL:-}}"
    SEED_STRANGER_PASSWORD="${APP_SEED_STRANGER_PASSWORD:-${E2E_STRANGER_PASSWORD:-}}"
    if [[ -n "${SEED_STRANGER_EMAIL:-}" && -n "${SEED_STRANGER_PASSWORD:-}" ]]; then
      printf 'APP_SEED_STRANGER_EMAIL=%s\n' "$SEED_STRANGER_EMAIL"
      printf 'APP_SEED_STRANGER_PASSWORD=%s\n' "$SEED_STRANGER_PASSWORD"
    fi
    if [[ -n "${PILOT_GITHUB_APP_ID:-}" ]]; then
      printf 'PILOT_GITHUB_APP_ID=%s\n' "$PILOT_GITHUB_APP_ID"
    fi
    if [[ -n "${PILOT_GITHUB_ORG:-}" ]]; then
      printf 'PILOT_GITHUB_ORG=%s\n' "$PILOT_GITHUB_ORG"
    fi
    # PILOT_GITHUB_APP_KEY 是多行 PEM，**不能**写进 EnvironmentFile：
    # systemd 只取首行（`-----BEGIN RSA PRIVATE KEY-----`），后端拿到残值报
    # InvalidKeyFormat。改为落盘 0600 文件，由启动包装脚本注入容器环境
    # （容器 env 值可以包含换行）。
    # Task3 试点模板占位输入：空值不写（后端用各自默认值）。
    if [[ -n "${PILOT_REPO_NAME:-}" ]]; then
      printf 'PILOT_REPO_NAME=%s\n' "$PILOT_REPO_NAME"
    fi
    if [[ -n "${PILOT_RUNNER:-}" ]]; then
      printf 'PILOT_RUNNER=%s\n' "$PILOT_RUNNER"
    fi
    if [[ -n "${PILOT_FRONTEND_URL:-}" ]]; then
      printf 'PILOT_FRONTEND_URL=%s\n' "$PILOT_FRONTEND_URL"
    fi
    if [[ -n "${PILOT_API_URL:-}" ]]; then
      printf 'PILOT_API_URL=%s\n' "$PILOT_API_URL"
    fi
    # JWT 秘钥走 0600 env 文件，不再依赖 L1 的 sed 注入（ExecStart 已改为包装脚本，
    # sed 会破坏命令行）；service 文件里保留 APP_JWT__SECRET 字样让 L1 步骤短路。
    if [[ -n "${APP_JWT__SECRET:-}" ]]; then
      printf 'APP_JWT__SECRET=%s\n' "$APP_JWT__SECRET"
    fi
    echo "RUST_LOG=info,sqlx::pool=warn"
  } > "$ENV_FILE"
)
chmod 600 "$ENV_FILE"

# Task3 试点 App 私钥：写 0600 文件（多行 PEM 原样保留），空值则清掉避免残留旧钥。
if [[ -n "${PILOT_GITHUB_APP_KEY:-}" ]]; then
  (
    umask 077
    printf '%s\n' "$PILOT_GITHUB_APP_KEY" > "$PILOT_KEY_FILE"
  )
  chmod 600 "$PILOT_KEY_FILE"
else
  rm -f "$PILOT_KEY_FILE"
fi

# 启动包装脚本：把 PEM 以单条 argv 注入容器 env（多行合法），
# 绕开 systemd EnvironmentFile 的单行限制。
cat > "$RUNNER_FILE" << EOF
#!/bin/bash
set -euo pipefail
EXTRA=()
if [[ -s ${PILOT_KEY_FILE} ]]; then
  EXTRA+=(-e "PILOT_GITHUB_APP_KEY=\$(cat ${PILOT_KEY_FILE})")
fi
exec /usr/bin/podman run --name ${CONTAINER_NAME} --network=host \\
  -v /opt/eap/data:/app/data --env-file ${ENV_FILE} \${EXTRA[@]+"\${EXTRA[@]}"} ${IMAGE}
EOF
chmod 700 "$RUNNER_FILE"

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
# APP_JWT__SECRET 由 ${ENV_FILE} 注入（此处保留字样以兼容 L1 注入步骤探测）
ExecStart=${RUNNER_FILE}
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
