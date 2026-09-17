# GyID SDK - Android AAR 构建脚本
# 使用方式: .\build-android.ps1 [-NdkPath <path>] [-AbiList <abi,...>]
#
# 构建 Android JNI 动态库，支持 armeabi-v7a / arm64-v8a / x86_64

param(
    [string]$NdkPath = $env:ANDROID_NDK_HOME,
    [string[]]$AbiList = @("arm64-v8a", "armeabi-v7a", "x86_64"),
    [switch]$Release = $false
)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

Write-Host "=== GyID SDK Android 构建 ===" -ForegroundColor Cyan

# 检查 NDK
if (-not $NdkPath -or -not (Test-Path $NdkPath)) {
    Write-Host "[错误] ANDROID_NDK_HOME 未设置或路径不存在: $NdkPath" -ForegroundColor Red
    Write-Host "请安装 Android NDK 并设置环境变量: `$env:ANDROID_NDK_HOME = 'C:\Android\ndk\<version>'" -ForegroundColor Yellow
    exit 1
}

Write-Host "NDK 路径: $NdkPath" -ForegroundColor Green

# Rust target → ABI 映射
$abiToTarget = @{
    "arm64-v8a"    = "aarch64-linux-android"
    "armeabi-v7a"  = "armv7-linux-androideabi"
    "x86_64"       = "x86_64-linux-android"
    "x86"          = "i686-linux-android"
}

$profile = if ($Release) { "release" } else { "debug" }
$outputBase = "target/android-libs"
New-Item -ItemType Directory -Force -Path $outputBase | Out-Null

foreach ($abi in $AbiList) {
    $target = $abiToTarget[$abi]
    if (-not $target) {
        Write-Host "[警告] 未知 ABI: $abi，跳过" -ForegroundColor Yellow
        continue
    }

    Write-Host ""
    Write-Host "[构建] ABI: $abi → Rust target: $target" -ForegroundColor Yellow

    # 确保 Rust target 已安装
    $installed = rustup target list --installed
    if ($installed -notcontains $target) {
        Write-Host "  安装 Rust target: $target..." -ForegroundColor Yellow
        rustup target add $target
    }

    # 设置 NDK 工具链环境变量
    $toolchainDir = "$NdkPath\toolchains\llvm\prebuilt\windows-x86_64\bin"
    $env:CC_aarch64_linux_android    = "$toolchainDir\aarch64-linux-android21-clang.cmd"
    $env:CC_armv7_linux_androideabi  = "$toolchainDir\armv7a-linux-androideabi21-clang.cmd"
    $env:CC_x86_64_linux_android     = "$toolchainDir\x86_64-linux-android21-clang.cmd"
    $env:AR_aarch64_linux_android    = "$toolchainDir\llvm-ar.exe"
    $env:AR_armv7_linux_androideabi  = "$toolchainDir\llvm-ar.exe"
    $env:AR_x86_64_linux_android     = "$toolchainDir\llvm-ar.exe"
    $env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = "$toolchainDir\aarch64-linux-android21-clang.cmd"
    $env:CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER = "$toolchainDir\armv7a-linux-androideabi21-clang.cmd"
    $env:CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER = "$toolchainDir\x86_64-linux-android21-clang.cmd"

    # 编译
    $profileFlag = if ($Release) { "--release" } else { "" }
    cargo build --target $target $profileFlag --features android 2>&1 | Write-Host

    if ($LASTEXITCODE -ne 0) {
        Write-Host "[错误] 构建 $abi 失败" -ForegroundColor Red
        exit 1
    }

    # 复制 .so 文件
    $soFile = "target\$target\$profile\libgyid_sdk.so"
    $destDir = "$outputBase\$abi"
    New-Item -ItemType Directory -Force -Path $destDir | Out-Null
    Copy-Item $soFile $destDir -Force
    Write-Host "  复制 libgyid_sdk.so → $destDir" -ForegroundColor Green
}

Write-Host ""
Write-Host "=== Android 构建完成 ===" -ForegroundColor Green
Write-Host "输出目录: $PSScriptRoot/$outputBase" -ForegroundColor Green
Write-Host ""
Write-Host "在 Android 项目中使用:" -ForegroundColor Cyan
Write-Host "  将 libgyid_sdk.so 复制到 app/src/main/jniLibs/<abi>/" -ForegroundColor White
