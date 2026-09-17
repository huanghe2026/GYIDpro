#!/usr/bin/env bash
# GyID SDK - iOS XCFramework 构建脚本
# 使用方式: ./build-ios.sh [--release]
#
# 构建 iOS + macOS Universal XCFramework，支持 Swift Package Manager
# 需要在 macOS 上运行，并安装 Xcode Command Line Tools

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

PROFILE="${1:-debug}"
if [ "$PROFILE" = "--release" ]; then
    PROFILE="release"
    CARGO_FLAGS="--release"
else
    CARGO_FLAGS=""
fi

HEADER_NAME="gyid_sdk.h"
XCFRAMEWORK_NAME="GyIdSDK.xcframework"
OUT_DIR="target/ios-xcframework"

echo "=== GyID SDK iOS 构建 (${PROFILE}) ==="

# 检查环境
if [[ "$(uname)" != "Darwin" ]]; then
    echo "[错误] iOS 构建需要 macOS 环境"
    exit 1
fi

# iOS targets
TARGETS=(
    "aarch64-apple-ios"          # iPhone 真机 (arm64)
    "aarch64-apple-ios-sim"      # iOS 模拟器 (arm64, Apple Silicon Mac)
    "x86_64-apple-ios"           # iOS 模拟器 (x86_64, Intel Mac)
)

# macOS target (for Catalyst / macOS)
MACOS_TARGET="aarch64-apple-darwin"
MACOS_TARGET_X86="x86_64-apple-darwin"

# 安装 targets
echo "[1/4] 安装 Rust targets..."
for target in "${TARGETS[@]}" "$MACOS_TARGET" "$MACOS_TARGET_X86"; do
    rustup target add "$target" 2>/dev/null || true
done

# 编译各 target
echo "[2/4] 编译..."
for target in "${TARGETS[@]}"; do
    echo "  编译 $target..."
    cargo build --target "$target" $CARGO_FLAGS
done

# 生成 C 头文件（使用 cbindgen）
echo "[3/4] 生成 C 头文件..."
if command -v cbindgen &>/dev/null; then
    cbindgen --crate gyid-sdk --output "target/$HEADER_NAME" --lang c
else
    echo "[警告] cbindgen 未安装，跳过头文件生成（cargo install cbindgen）"
    # 使用预置头文件
fi

# 创建 XCFramework
echo "[4/4] 打包 XCFramework..."
mkdir -p "$OUT_DIR"

DEVICE_LIB="target/aarch64-apple-ios/${PROFILE}/libgyid_sdk.a"
SIM_ARM_LIB="target/aarch64-apple-ios-sim/${PROFILE}/libgyid_sdk.a"
SIM_X86_LIB="target/x86_64-apple-ios/${PROFILE}/libgyid_sdk.a"

# 合并模拟器 fat library
SIM_FAT_LIB="$OUT_DIR/sim-fat/libgyid_sdk.a"
mkdir -p "$OUT_DIR/sim-fat"
lipo -create "$SIM_ARM_LIB" "$SIM_X86_LIB" -output "$SIM_FAT_LIB"

# 创建 XCFramework
xcodebuild -create-xcframework \
    -library "$DEVICE_LIB" -headers "target/" \
    -library "$SIM_FAT_LIB" -headers "target/" \
    -output "$OUT_DIR/$XCFRAMEWORK_NAME"

echo ""
echo "=== iOS 构建完成 ==="
echo "XCFramework: $SCRIPT_DIR/$OUT_DIR/$XCFRAMEWORK_NAME"
echo ""
echo "Swift Package Manager 使用方式:"
echo "  .binaryTarget("
echo "      name: \"GyIdSDK\","
echo "      path: \"$XCFRAMEWORK_NAME\""
echo "  )"
