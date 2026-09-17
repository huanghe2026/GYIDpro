@echo off
REM GYID WebButton 部署脚本 (Windows)
REM 用法: deploy.bat

setlocal enabledelayedexpansion

set ENV=production
set DEPLOY_DIR=\\server\geoyuan.com\tools\webbutton\z
set BACKUP_DIR=\\server\geoyuan.com\tools\webbutton\z.backup

echo ========================================
echo GYID WebButton 部署脚本
echo 环境: %ENV%
echo ========================================

echo [1/4] 准备文件...
if not exist "%DEPLOY_DIR%" (
    mkdir "%DEPLOY_DIR%"
)

echo [2/4] 复制文件...
xcopy /E /I /Y /Q "css" "%DEPLOY_DIR%\css\"
xcopy /E /I /Y /Q "js" "%DEPLOY_DIR%\js\"
xcopy /E /I /Y /Q "assets" "%DEPLOY_DIR%\assets\"
xcopy /Y /Q "index.html" "%DEPLOY_DIR%\"
xcopy /Y /Q "embed.html" "%DEPLOY_DIR%\"
xcopy /Y /Q "README.md" "%DEPLOY_DIR%\"

echo [3/4] 验证文件...
if exist "%DEPLOY_DIR%\index.html" (
    echo [OK] index.html
)
if exist "%DEPLOY_DIR%\js\gyid-webbutton.js" (
    echo [OK] gyid-webbutton.js
)

echo [4/4] 完成!
echo ========================================
echo 部署完成!
echo 请手动上传到服务器或配置 CI/CD
echo ========================================

pause
