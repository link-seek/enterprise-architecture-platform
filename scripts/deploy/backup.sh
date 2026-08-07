#!/bin/bash
set -euo pipefail

# EAP 部署前备份 — 在 ECS self-hosted runner 上执行
# 备份: SQLite DB + systemd service 文件 + 旧镜像 tag

TIMESTAMP=$(date +%Y%m%d%H%M%S)
DB_PATH="/opt/eap/data/platform.db"
SERVICE_FILE="/etc/systemd/system/eap-backend.service"
BACKUP_DIR="/opt/eap/backups"

mkdir -p "$BACKUP_DIR"

echo "=== Backup started at $TIMESTAMP ==="

# 1. 备份 SQLite DB
if [ -f "$DB_PATH" ]; then
  DB_BACKUP="$BACKUP_DIR/platform.db.bak.$TIMESTAMP"
  cp "$DB_PATH" "$DB_BACKUP"
  echo "DB backed up: $DB_BACKUP ($(du -h "$DB_BACKUP" | cut -f1))"
else
  echo "WARNING: DB file not found at $DB_PATH, skipping DB backup"
fi

# 2. 备份 systemd service 文件
if [ -f "$SERVICE_FILE" ]; then
  SVC_BACKUP="$BACKUP_DIR/eap-backend.service.bak.$TIMESTAMP"
  cp "$SERVICE_FILE" "$SVC_BACKUP"
  echo "Service file backed up: $SVC_BACKUP"

  # 3. 提取旧镜像 tag（从 service 文件的 ExecStart 行）
  OLD_IMAGE=$(grep '^ExecStart=' "$SERVICE_FILE" | grep -oP '\S+:\S+$' || true)
  if [ -n "$OLD_IMAGE" ]; then
    echo "$OLD_IMAGE" > "$BACKUP_DIR/previous-image.txt"
    echo "Previous image: $OLD_IMAGE"
  else
    echo "WARNING: Could not extract previous image from service file"
  fi
else
  echo "WARNING: Service file not found at $SERVICE_FILE, skipping service backup"
fi

# 4. 记录备份时间戳供 rollback 使用
echo "$TIMESTAMP" > "$BACKUP_DIR/latest-backup.txt"

echo "=== Backup completed ==="
