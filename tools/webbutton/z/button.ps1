# GEOYUAN Button Deploy Script (PowerShell)
# Deploys tools/webbutton/z to server
# Usage: .\button.ps1 [-Server <IP>] [-User <User>] [-RemotePath <Path>] [-KeyFile <File>]

param(
    [string]$Server = $env:DEPLOY_SERVER,
    [string]$User = "root",
    [string]$RemotePath = "/var/www/html/geoyuan.com/dist/tools/webbutton/z",
    [string]$KeyFile = "$HOME\.ssh\id_rsa"
)

$ErrorActionPreference = "Stop"

# Get script directory (script is in the z folder itself)
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$SourceDir = $ScriptDir  # Source is the z folder where script resides

# Colors
$CYAN = "`e[0;36m"
$GREEN = "`e[0;32m"
$YELLOW = "`e[1;33m"
$RED = "`e[0;31m"
$NC = "`e[0m"

Write-Host ""
Write-Host "${CYAN}================================================${NC}"
Write-Host "${CYAN}  GEOYUAN BUTTON DEPLOY SCRIPT${NC}"
Write-Host "${CYAN}  Source: $SourceDir${NC}"
Write-Host "${CYAN}================================================${NC}"
Write-Host ""

# Check source directory
if (-not (Test-Path $SourceDir)) {
    Write-Host "${RED}Error: Source directory not found: $SourceDir${NC}" -ForegroundColor Red
    exit 1
}

# List files to deploy
Write-Host "${YELLOW}[1/3] Checking source files...${NC}" -ForegroundColor Yellow
$Files = Get-ChildItem -Path $SourceDir -File -Recurse
$FileCount = $Files.Count
Write-Host "    Found $FileCount files to deploy"
$Files | Where-Object { $_.Extension -match '\.(html|css|js)$' } | Select-Object -First 10 | ForEach-Object {
    $RelPath = $_.FullName.Replace($SourceDir, "").TrimStart("\")
    Write-Host "    ${GREEN}+${NC} $RelPath"
}
Write-Host ""

# Build SSH options
$SSHOpts = "-o StrictHostKeyChecking=no -o ConnectTimeout=30"
if (Test-Path $KeyFile) {
    $SSHOpts = "$SSHOpts -i `"$KeyFile`""
}

Write-Host "${YELLOW}[2/3] Uploading to server $Server...${NC}" -ForegroundColor Yellow

# Create remote directory
ssh $SSHOpts "$User@$Server" "mkdir -p $RemotePath"

# Try rsync first, fallback to scp
$HasRsync = Get-Command rsync -ErrorAction SilentlyContinue
if ($HasRsync) {
    Write-Host "    Using rsync..."
    $RSYNC_SSH = "ssh $SSHOpts"
    rsync -az --delete -e "$RSYNC_SSH" "$SourceDir/" "$User@$Server`:$RemotePath/"
} else {
    Write-Host "    Using scp..."
    Get-ChildItem -Path $SourceDir -Recurse -File | ForEach-Object {
        $RelPath = $_.FullName.Replace($SourceDir, "").TrimStart("\")
        $RemoteFile = "$RemotePath/$RelPath"
        $RemoteDir = Split-Path -Parent $RemoteFile
        ssh $SSHOpts "$User@$Server" "mkdir -p `"$RemoteDir`""
        scp $SSHOpts $_.FullName "$User@$Server`:`"$RemoteFile`""
    }
}
Write-Host "    Upload complete." -ForegroundColor Green

Write-Host "${YELLOW}[3/3] Setting permissions...${NC}" -ForegroundColor Yellow

# Set permissions and configure NGINX
ssh $SSHOpts "$User@$Server" @"
mkdir -p $RemotePath
chmod -R 644 $RemotePath && find $RemotePath -type d -exec chmod 755 {} \;
cat > /etc/nginx/conf.d/geoyuan-button.conf << 'NGINXCONF'
location /tools/webbutton/ {
    alias /var/www/html/geoyuan.com/dist/tools/webbutton/;
    index index.html;
    try_files `$uri `$uri/ =404;
}
NGINXCONF
nginx -t 2>/dev/null && systemctl reload nginx 2>/dev/null || nginx -t
echo "NGINX configured."
"@

Write-Host "    Permissions set." -ForegroundColor Green

Write-Host ""
Write-Host "${CYAN}================================================${NC}"
Write-Host "${GREEN}  DEPLOY COMPLETE!${NC}" -ForegroundColor Green
Write-Host "${CYAN}  URL: https://www.geoyuan.com/tools/webbutton/z/${NC}"
Write-Host "${CYAN}  Claim: https://www.geoyuan.com/tools/webbutton/z/claim.html${NC}"
Write-Host "${CYAN}================================================${NC}"
Write-Host ""
