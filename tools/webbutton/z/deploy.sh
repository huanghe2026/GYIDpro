#!/bin/bash
# GYID WebButton 部署脚本
# 用法: ./deploy.sh [环境]
# 环境: production (默认)

set -e

ENV=${1:-production}
DEPLOY_DIR="/var/www/geoyuan.com/tools/webbutton/z"
BACKUP_DIR="/var/www/geoyuan.com/tools/webbutton/z.backup"

echo "========================================"
echo "GYID WebButton 部署脚本"
echo "环境: $ENV"
echo "========================================"

# 备份现有文件
if [ -d "$DEPLOY_DIR" ]; then
    echo "[1/4] 备份现有文件..."
    if [ -d "$BACKUP_DIR" ]; then
        rm -rf "$BACKUP_DIR"
    fi
    cp -r "$DEPLOY_DIR" "$BACKUP_DIR"
    echo "备份完成: $BACKUP_DIR"
fi

# 同步文件
echo "[2/4] 同步文件..."
rsync -avz --delete \
    --exclude='*.map' \
    --exclude='.git' \
    ./ "$DEPLOY_DIR/"

# 设置权限
echo "[3/4] 设置权限..."
find "$DEPLOY_DIR" -type f -exec chmod 644 {} \;
find "$DEPLOY_DIR" -type d -exec chmod 755 {} \;

# 验证部署
echo "[4/4] 验证部署..."
if curl -sf "https://geoyuan.com/tools/webbutton/z/index.html" > /dev/null; then
    echo "✅ 部署成功!"
else
    echo "⚠️ 文件已上传，但 CDN 缓存可能需要几分钟生效"
fi

echo ""
echo "========================================"
echo "部署完成!"
echo "访问地址: https://geoyuan.com/tools/webbutton/z"
echo "========================================"
