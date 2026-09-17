#!/bin/bash
# GEOYUAN Button Deploy Script
# Deploys tools/webbutton/z to server
# Usage: ./button.sh [options]
# Options:
#   --server IP       Server IP (set via --server or DEPLOY_SERVER env var)
#   --user USER       SSH user (default: root)
#   --path PATH       Remote path (default: /var/www/geoyuan/tools/webbutton/z)
#   --key FILE        SSH key file

set -e

# Defaults (server IP must be provided via --server or DEPLOY_SERVER env var)
SERVER="${DEPLOY_SERVER:-}"
USER="root"
REMOTE_PATH="/var/www/html/geoyuan.com/dist/tools/webbutton/z"
SSH_KEY="$HOME/.ssh/id_rsa"

# Parse args
while [[ "$#" -gt 0 ]]; do
    case $1 in
        --server) SERVER="$2"; shift ;;
        --user) USER="$2"; shift ;;
        --path) REMOTE_PATH="$2"; shift ;;
        --key) SSH_KEY="$2"; shift ;;
        *) echo "Unknown parameter: $1"; exit 1 ;;
    esac
    shift
done

# Script is in the z folder itself
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE_DIR="$SCRIPT_DIR"

# Validate server address
if [ -z "$SERVER" ]; then
    echo -e "${RED}Error: Server IP not set. Use --server <IP> or set DEPLOY_SERVER env var.${NC}"
    exit 1
fi

# Colors
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo ""
echo -e "${CYAN}================================================${NC}"
echo -e "${CYAN}  GEOYUAN BUTTON DEPLOY SCRIPT${NC}"
echo -e "${CYAN}  Source: $SOURCE_DIR${NC}"
echo -e "${CYAN}================================================${NC}"
echo ""

# Check source directory
if [ ! -d "$SOURCE_DIR" ]; then
    echo -e "${RED}Error: Source directory not found: $SOURCE_DIR${NC}"
    exit 1
fi

# List files to deploy
echo -e "${YELLOW}[1/3] Checking source files...${NC}"
FILE_COUNT=$(find "$SOURCE_DIR" -type f | wc -l)
echo -e "    Found $FILE_COUNT files to deploy"
find "$SOURCE_DIR" -type f -name "*.html" -o -name "*.css" -o -name "*.js" | head -10 | while read f; do
    echo -e "    ${GREEN}✓${NC} ${f#$SOURCE_DIR/}"
done
echo ""

# SSH args
SSH_OPTS="-o StrictHostKeyChecking=no -o ConnectTimeout=30"
if [ -f "$SSH_KEY" ]; then
    SSH_OPTS="$SSH_OPTS -i $SSH_KEY"
fi

echo -e "${YELLOW}[2/3] Uploading to server $SERVER...${NC}"
ssh $SSH_OPTS "$USER@$SERVER" "mkdir -p $REMOTE_PATH"

# Use rsync if available, fallback to scp
if command -v rsync &> /dev/null; then
    echo "    Using rsync..."
    RSYNC_SSH="ssh $SSH_OPTS"
    rsync -az --delete -e "$RSYNC_SSH" "$SOURCE_DIR/" "$USER@$SERVER:$REMOTE_PATH/"
else
    echo "    Using scp..."
    scp $SSH_OPTS -r "$SOURCE_DIR/"* "$USER@$SERVER:$REMOTE_PATH/"
fi
echo -e "${GREEN}    Upload complete.${NC}"

echo -e "${YELLOW}[3/3] Setting permissions...${NC}"
ssh $SSH_OPTS "$USER@$SERVER" "chmod -R 644 $REMOTE_PATH && find $REMOTE_PATH -type d -exec chmod 755 {} \;"

# Set up web directory in nginx config if needed
echo -e "${YELLOW}    Configuring NGINX...${NC}"
ssh $SSH_OPTS "$USER@$SERVER" << 'ENDSSH'
REMOTE_PATH="/var/www/html/geoyuan.com/dist/tools/webbutton/z"
# Create or update nginx config snippet
NGINX_CONF="/etc/nginx/conf.d/geoyuan-button.conf"
cat > "$NGINX_CONF" << 'CONF'
location /tools/webbutton/ {
    alias /var/www/html/geoyuan.com/dist/tools/webbutton/;
    index index.html;
    try_files $uri $uri/ =404;
}
CONF
nginx -t && systemctl reload nginx 2>/dev/null || nginx -t
echo "    NGINX configured."
ENDSSH
echo -e "${GREEN}    Permissions set.${NC}"

echo ""
echo -e "${CYAN}================================================${NC}"
echo -e "${GREEN}  DEPLOY COMPLETE!${NC}"
echo -e "${CYAN}  URL: https://www.geoyuan.com/tools/webbutton/z/${NC}"
echo -e "${CYAN}  Claim: https://www.geoyuan.com/tools/webbutton/z/claim.html${NC}"
echo -e "${CYAN}================================================${NC}"
echo ""
