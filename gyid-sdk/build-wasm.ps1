# GyID SDK - WASM 构建脚本
# 使用方式: .\build-wasm.ps1 [-Release] [-OutDir <path>]
#
# 此脚本编译 gyid-sdk 为 WebAssembly，并生成可发布的 npm 包 (@gyid/sdk-web)

param(
    [switch]$Release = $false,
    [string]$OutDir = "pkg-web"
)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

Write-Host "=== GyID SDK WASM 构建 ===" -ForegroundColor Cyan

# 检查 wasm-pack
if (-not (Get-Command "wasm-pack" -ErrorAction SilentlyContinue)) {
    Write-Host "[错误] wasm-pack 未安装，请先运行: cargo install wasm-pack" -ForegroundColor Red
    exit 1
}

# 确保 wasm32 target 已安装
Write-Host "[1/3] 检查 wasm32-unknown-unknown target..." -ForegroundColor Yellow
$targets = rustup target list --installed
if ($targets -notcontains "wasm32-unknown-unknown") {
    Write-Host "  安装 wasm32-unknown-unknown..." -ForegroundColor Yellow
    rustup target add wasm32-unknown-unknown
}

# 构建
Write-Host "[2/3] 构建 WASM 包..." -ForegroundColor Yellow
$profile = if ($Release) { "--release" } else { "--dev" }
$target = "web"  # web | bundler | nodejs | no-modules

wasm-pack build `
    --target $target `
    $profile `
    --out-dir $OutDir `
    --out-name gyid_sdk `
    -- --no-default-features --features wasm

if ($LASTEXITCODE -ne 0) {
    Write-Host "[错误] WASM 构建失败" -ForegroundColor Red
    exit 1
}

# 生成增强的 package.json（覆盖 wasm-pack 默认的）
Write-Host "[3/3] 更新 npm 包元信息..." -ForegroundColor Yellow
$pkgJson = @{
    name = "@gyid/sdk-web"
    version = (Get-Content "$PSScriptRoot/../Cargo.toml" | Select-String '^version = "(.+)"').Matches.Groups[1].Value
    description = "GyID decentralized identity SDK for Web (WebAssembly)"
    main = "gyid_sdk.js"
    module = "gyid_sdk.js"
    types = "gyid_sdk.d.ts"
    files = @("gyid_sdk_bg.wasm", "gyid_sdk.js", "gyid_sdk.d.ts")
    keywords = @("gyid", "identity", "wasm", "decentralized")
    license = "MIT"
    repository = @{
        type = "git"
        url = "https://github.com/geoyuan/gyid"
    }
    sideEffects = $false
} | ConvertTo-Json -Depth 4

$pkgJson | Set-Content "$OutDir/package.json" -Encoding UTF8
Write-Host ""
Write-Host "=== 构建成功 ===" -ForegroundColor Green
Write-Host "输出目录: $PSScriptRoot/$OutDir" -ForegroundColor Green
Write-Host ""
Write-Host "发布到 npm:" -ForegroundColor Cyan
Write-Host "  cd $OutDir && npm publish --access public" -ForegroundColor White
Write-Host ""
Write-Host "本地测试:" -ForegroundColor Cyan
Write-Host "  import init, { generate_gyid, verify_gyid } from './$OutDir/gyid_sdk.js'" -ForegroundColor White
