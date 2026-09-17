# GyID

> 去中心化身份账号系统 - 基于多维度动态账号生成

## 项目简介

**GyID** 是一个基于 Rust 语言开发的去中心化身份标识系统，通过多维度信息融合生成用户的动态唯一账号。

### 核心特点

- 🚀 **去中心化**: 无需第三方机构，用户自主生成和管理身份
- 🔒 **隐私保护**: 不存储原始硬件数据，只保留不可逆的特征值
- 🎯 **动态性**: 账号随时间、设备、地点等因素动态变化
- 🌐 **跨平台**: 支持 Windows、macOS、Linux、Web (WASM)、Android、iOS
- 🔗 **链上锚定**: 可选 Polygon PoS / Aptos 链上验证

## 账号生成维度

| 维度 | 数据源 | 说明 |
|------|--------|------|
| 硬件指纹 | MAC、CPU ID、主板序列号 | 设备唯一性 |
| 地理位置 | IP/WiFi/GPS | 三级精度可选 |
| 时间戳 | UTC 毫秒 + 随机数 | 防碰撞 |
| 头像 | 用户头像 BLAKE3 哈希 | 个人标识（可选）|

## 技术栈

- **语言**: Rust
- **哈希**: BLAKE3 + SHA-256
- **存储**: SQLite + 区块链锚定
- **GUI**: Dioxus (Rust Web 前端)
- **SDK**: WASM / Android (Kotlin) / iOS (Swift) / C FFI

## 快速开始

### 编译

```bash
# 编译所有模块
cargo build --release

# 仅编译 CLI
cargo build --release -p gyid-cli

# 仅编译 GUI（需要安装 Dioxus CLI）
cargo build --release -p gyid-gui
```

### 运行 CLI

```bash
# 生成 GyID（城市级精度）
cargo run --release -p gyid-cli -- generate

# 指定头像
cargo run --release -p gyid-cli -- generate --avatar /path/to/avatar.png

# 指定精度
cargo run --release -p gyid-cli -- generate --level exact

# 验证 GyID
cargo run --release -p gyid-cli -- verify GyIDxxxxx

# 设备关联
cargo run --release -p gyid-cli -- link --auth-code GEN

# 链上锚定
cargo run --release -p gyid-cli -- chain anchor

# H3 网格操作
cargo run --release -p gyid-cli -- h3 encode 39.9042 116.4074
```

### 运行 GUI

```bash
# 安装 Dioxus CLI
cargo install dioxus-cli

# 启动 GUI
cd gyid-gui && dx serve
```

### 运行测试

```bash
cargo test --workspace
```

## CLI 命令

```
gyid generate          # 生成新 GyID
gyid verify <GYID>     # 验证 GyID 格式
gyid devices            # 列出关联设备
gyid link              # 设备关联
gyid chain anchor      # 链上锚定
gyid chain status <tx> # 查询交易状态
gyid chain verify      # 验证链上锚定
gyid chain networks    # 显示支持的链
gyid gps-check         # 检查 GPS 可用性
gyid geo-manual        # 手动输入坐标
gyid h3 encode         # H3 网格编码
gyid h3 decode         # H3 网格解码
gyid h3 neighbors      # 获取相邻单元格
gyid config show       # 显示配置
gyid export            # 导出 GyID
gyid --help            # 显示帮助
```

## 项目结构

```
GYID/
├── gyid-core/          # 核心库
│   └── src/
│       ├── fingerprint/  # 硬件指纹采集 (MAC/CPU/Board/Disk)
│       ├── geo/          # 地理位置 (IP/WiFi/GPS/H3)
│       ├── avatar/      # 头像处理
│       ├── crypto/      # 加密模块 (BLAKE3/Base58)
│       ├── identity/     # 身份生成核心
│       ├── device/       # 设备关联
│       └── storage/      # 存储模块 (SQLite/Chain)
├── gyid-cli/           # 命令行工具
│   └── src/commands/    # generate/verify/link/chain/gps/h3
├── gyid-gui/           # Dioxus GUI 应用
│   └── src/
│       ├── pages/        # home/generate/devices/settings/import/history
│       └── components/   # sidebar/status_bar/gyid_card/qr_export
├── gyid-sdk/           # 跨平台 SDK
│   ├── src/
│   │   ├── ffi/         # WASM/Android/C FFI
│   │   └── api.rs        # 统一 API
│   ├── android/         # Android Kotlin 封装
│   ├── ios/             # iOS Swift 封装
│   └── gyid-sdk.udl     # UniFFI 定义
├── tests/               # 集成测试
└── docs/                # 文档
```

## SDK 多平台支持

### Web (WASM)

```javascript
import init, { generate_gyid, verify_gyid } from '@gyid/sdk-web';

await init();
const result = await generate_gyid(JSON.stringify({ geo_level: "city" }));
console.log(result.id); // "GyID..."
```

### Android (Kotlin)

```kotlin
// GyIdSdk.kt 已提供完整封装
val result = GyIdSdk.generateDefault()
println("GyID: ${result.id}")
```

### iOS (Swift)

```swift
// GyIdSDK.swift 已提供完整封装
let result = try await GyIdSDK.shared.generate()
print("GyID: \(result.id)")
```

### C / Unity

```c
#include "gyid_sdk.h"

GyIdCResult result = gyid_generate_default();
if (result.status == GYID_OK) {
    printf("GyID: %s\n", result.data);
}
gyid_free_result(result);
```

## 链上锚定

### 支持的区块链

| 链 | 网络 | 特性 |
|----|------|------|
| Polygon PoS | Mainnet | EIP-155 签名 |
| Polygon Amoy | 测试网 | 免费领取 MATIC |
| Aptos | Mainnet | ED25519 签名 |
| Aptos Testnet | 测试网 | 免费领取 APT |

### 配置

```bash
# Polygon 配置
export GYID_CHAIN_RPC="https://polygon-rpc.com"
export GYID_CHAIN_KEY="your_private_key_hex"
export GYID_CHAIN_ADDR="0x..."

# Aptos 配置
export GYID_APTOS_KEY="your_ed25519_private_key_hex"
export GYID_APTOS_ADDR="0x..."
```

## 版本

当前版本: **v0.4.1** (2026-04-07)

- ✅ gyid-core 核心库（零 Clippy 警告）
- ✅ gyid-cli 命令行工具
- ✅ gyid-gui Dioxus GUI（主页/生成页/设备页/设置页/导入页/历史页）
- ✅ gyid-sdk 跨平台 SDK (WASM/Android/iOS/C FFI)
- ✅ 链上锚定 (Polygon PoS + Aptos)
- ✅ H3 六边形网格集成
- ✅ 全量测试覆盖（39 单元测试 + 8 集成测试）

## 编译环境

项目使用 `stable-x86_64-pc-windows-gnu` 工具链（MinGW-w64）：

```bash
# 确保已安装 stable-gnu 工具链
rustup install stable-x86_64-pc-windows-gnu

# 确保 MinGW-w64 GCC 在 PATH 中
# Windows: 安装 MSYS2，添加 C:\msys64\mingw64\bin 到 PATH

# 使用 rust-toolchain.toml 自动选择工具链
cargo build
```


## 文档

- [ARCHITECTURE.md](ARCHITECTURE.md) - 架构设计
- [DEVELOPMENT_PLAN.md](DEVELOPMENT_PLAN.md) - 开发计划
- [docs/API.md](docs/API.md) - API 参考文档
- [docs/SECURITY.md](docs/SECURITY.md) - 安全设计
- [docs/HARNESS_ENGINEERING.md](docs/HARNESS_ENGINEERING.md) - 编译规则

## 许可证

MIT OR Apache-2.0
