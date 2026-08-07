#!/bin/bash
set -euo pipefail

# EAP 部署回退 — 在 ECS self-hosted runner 上执行
# 恢复: SQLite DB + systemd service 文件 + 重启旧镜像

BACKUP_DIR="/opt/eap/backups"
DB_PATH="/opt/eap/data/platform.db"
SERVICE_FILE="/etc/systemd/system/eap-backend.service"

echo "=== Rollback started ==="

# 1. 找到最新备份
if [ ! -f "$BACKUP_DIR/latest-backup.txt" ]; then
  echo "ERROR: No backup found, cannot rollback"
  exit 1
fi

TIMESTAMP=$(cat "$BACKUP_DIR/latest-backup.txt")
DB_BACKUP="$BACKUP_DIR/platform.db.bak.$TIMESTAMP"
SVC_BACKUP="$BACKUP_DIR/eap-backend.service.bak.$TIMESTAMP"

echo "Rolling back to backup: $TIMESTAMP"

# 2. 停止当前服务
echo "Stopping current service..."
systemctl stop eap-backend 2>/dev/null || true
podman rm -f eap-backend 2>/dev/null || true

# 3. 恢复 SQLite DB
if [ -f "$DB_BACKUP" ]; then
  cp "$DB_BACKUP" "$DB_PATH"
  echo "DB restored from: $DB_BACKUP"
else
  echo "WARNING: DB backup not found, keeping current DB"
fi

# 4. 恢复 service 文件（含旧镜像 tag）
if [ -f "$SVC_BACKUP" ]; then
  cp "$SVC_BACKUP" "$SERVICE_FILE"
  echo "Service file restored from: $SVC_BACKUP"

  OLD_IMAGE=$(grep '^ExecStart=' "$SERVICE_FILE" | grep -oP '\S+:\S+$' || true)
  if [ -n "$OLD_IMAGE" ]; then
    echo "Rolling back to image: $OLD_IMAGE"
    # 预拉旧镜像（确保本地有）
    podman pull "$OLD_IMAGE" 2>/dev/null || echo "WARNING: Could not pre-pull old image, will rely on podman run"
  fi
else
  echo "ERROR: Service backup not found, cannot restore service file"
  exit 1
fi

# 5. 重启服务
systemctl daemon-reload
systemctl restart eap-backend

# 6. 等待 health check
echo "=== Waiting for backend to start ==="
for i in $(seq 1 30); do
  if curl -sf http://localhost:8080/health 2>/dev/null; then
    echo ""
    echo "Rollback successful! Backend healthy."
    exit 0
  fi
  echo "Waiting... ($i/30)"
  sleep 3
done

echo "ERROR: Health check failed after 90s"
echo "=== Systemd status ==="
systemctl status eap-backend --no-pager 2>/dev/null || true
echo "=== Journal (last 30 lines) ==="
journalctl -u eap-backend --no-pager -n 30 2>/dev/null || true
exit 1
