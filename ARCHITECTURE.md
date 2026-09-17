# GyID SDK 架构文档

## 概述

GyID（GeoYuan Identity）是一个去中心化身份系统，基于多维度数据融合生成唯一账号：

```
GyID = Base58(BLAKE3(W₁·hardware ⊕ W₂·geo ⊕ W₃·timestamp ⊕ W₄·avatar ⊕ salt))
```

## 项目结构

```
GYID/
├── gyid-core/          # 核心算法库（Rust）
│   ├── src/
│   │   ├── fingerprint/    # 硬件指纹采集 (MAC/CPU/Board/Disk)
│   │   ├── geo/            # 地理位置 (IP/WiFi/GPS)
│   │   ├── avatar/         # 头像处理 (图片哈希)
│   │   ├── crypto/         # 加密算法 (BLAKE3/Base58)
│   │   ├── identity/       # GyID 生成器和验证器
│   │   ├── device/         # 设备关联 (主从设备树)
│   │   └── storage/        # 本地 SQLite 存储 (仅 native)
│   └── Cargo.toml
│
├── gyid-sdk/           # 跨平台 SDK 层
│   ├── src/
│   │   ├── api.rs          # 统一 API (GyIdSdk)
│   │   ├── models.rs       # 跨平台数据模型
│   │   ├── error.rs        # 错误类型
│   │   ├── platform/       # 平台特定实现
│   │   │   ├── fingerprint.rs  # 平台指纹抽象
│   │   │   ├── native.rs       # Windows/macOS/Linux 特有
│   │   │   └── wasm_compat.rs  # WASM 时间/随机数兼容
│   │   └── ffi/            # 语言绑定
│   │       ├── wasm.rs         # WebAssembly (wasm-bindgen)
│   │       ├── android.rs      # Android JNI
│   │       └── c_ffi.rs        # C ABI (iOS/C/Python/Unity)
│   ├── android/
│   │   └── GyIdSdk.kt      # Android Kotlin 封装
│   ├── ios/
│   │   └── GyIdSDK.swift   # iOS Swift 封装
│   ├── gyid_sdk.h          # C 头文件
│   ├── build-wasm.ps1      # WASM 构建脚本
│   ├── build-android.ps1   # Android NDK 构建脚本
│   ├── build-ios.sh        # iOS XCFramework 构建脚本
│   └── Package.swift       # Swift Package Manager 清单
│
├── gyid-cli/           # 命令行工具
│   └── src/
│       └── commands/       # generate/verify/link/devices/export
│
└── DEVELOPMENT_PLAN.md # 开发计划（8阶段）
```

## Feature Flags

### gyid-core

| Feature        | 说明                                         | 默认 |
|---------------|----------------------------------------------|------|
| `native`      | 完整功能：SQLite、reqwest、sysinfo、tokio     | ✅    |
| `wasm`        | WASM 平台：跳过所有 native 依赖              | ❌    |
| `chain-signing` | 链上签名：k256 + tiny-keccak               | ❌    |
| `sync`        | 同步阻塞 API                                 | ❌    |

### gyid-sdk

| Feature        | 说明                                         | 默认 |
|---------------|----------------------------------------------|------|
| `native`      | 原生平台支持（包含 gyid-core/native）         | ✅    |
| `wasm`        | WebAssembly 支持（wasm-bindgen + JS API）     | ❌    |
| `android`     | Android JNI 绑定                             | ❌    |
| `uniffi-bindings` | UniFFI 多语言绑定                        | ❌    |
| `chain-signing` | 链上签名                                  | ❌    |

## 跨平台构建指南

### Web (WASM)

```powershell
# 安装依赖
cargo install wasm-pack
rustup target add wasm32-unknown-unknown

# 构建
cd gyid-sdk
wasm-pack build --target web --release --out-dir pkg-web --out-name gyid_sdk -- --no-default-features --features wasm

# 或使用脚本
.\build-wasm.ps1 -Release
```

**输出**: `gyid-sdk/pkg-web/` 目录，包含 npm 包 `@gyid/sdk-web`

### Android

```powershell
# 需要 Android NDK 环境
$env:ANDROID_NDK_HOME = "C:\Android\ndk\<version>"

cd gyid-sdk
.\build-android.ps1

# 输出: target/android-libs/<abi>/libgyid_sdk.so
```

**使用方式**: 将 `.so` 文件复制到 Android 项目的 `app/src/main/jniLibs/<abi>/` 目录，使用 `GyIdSdk.kt` 封装类

### iOS (需要 macOS)

```bash
cd gyid-sdk
./build-ios.sh --release
# 输出: target/ios-xcframework/GyIdSDK.xcframework
```

**Swift Package Manager**: 参见 `Package.swift`

### Windows / macOS / Linux (Native)

```bash
cargo build -p gyid-sdk --release
# 输出: target/release/gyid_sdk.dll / libgyid_sdk.dylib / libgyid_sdk.so
```

**C 语言头文件**: `gyid-sdk/gyid_sdk.h`

## 核心算法

### GyID 生成流程

1. **硬件指纹** (W₁ = 0.35)
   - Windows: getmac / wmic
   - macOS: ifconfig / ioreg
   - Linux: /sys/class/net / /proc/cpuinfo
   - WASM: navigator.userAgent + screen + CPU cores

2. **地理位置** (W₂ = 0.25)
   - L1 城市级: ip-api.com IP 定位
   - L2 区域级: Mozilla Location Service + WiFi BSSID
   - L3 精确级: GPS + WiFi 混合 (Windows WinRT / macOS CoreLocation / Linux GeoClue2)

3. **时间戳** (W₃ = 0.15)
   - UTC 毫秒时间戳 << 16 | 16bit 随机数
   - 格式: `u64 = (now_ms << 16) | random16`

4. **头像哈希** (W₄ = 0.25)
   - SHA-256 of image pixels
   - 可选输入

5. **最终计算**
   - `hash = BLAKE3(concat(factors, salt))`
   - `id = "GyID" + Base58(hash)[2..34]`

### 设备关联协议

```
主设备                              从设备
   |                                   |
   |  生成授权码(10分钟有效)            |
   |  CODE = PREFIX:TIMESTAMP:HMAC     |
   |                                   |
   |  <-- 二维码/剪贴板 传输 CODE -->   |
   |                                   |
   |           验证 CODE 格式          |
   |           校验 TIMESTAMP 未过期   |
   |           校验 HMAC 签名          |
   |                                   |
   |  <-- 创建 DeviceLink 记录 -->      |
   |                                   |
   |     更新主设备 linked_devices++    |
```

## 数据存储

### 本地 SQLite (native 平台)

位置: `{LocalAppData}/gyid/gyid.db`

```sql
-- GyID 主记录
CREATE TABLE gyids (
    id TEXT PRIMARY KEY,          -- GyID 字符串
    hash TEXT NOT NULL,           -- 原始哈希
    created_at INTEGER NOT NULL,  -- 时间戳 (u64)
    linked_devices INTEGER DEFAULT 0,
    data TEXT                     -- JSON 附加数据
);

-- 设备关联记录
CREATE TABLE device_links (
    link_id TEXT PRIMARY KEY,
    master_id TEXT NOT NULL,
    linked_id TEXT NOT NULL,
    auth_code TEXT NOT NULL,
    linked_at INTEGER NOT NULL,
    active INTEGER DEFAULT 1
);

-- 配置键值对
CREATE TABLE config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

### 链上锚定 (✅ 已实现 v0.4)

- **Polygon PoS**: 已实现 EVM 链上锚定
  - Mainnet: chainId 137 (https://polygon-rpc.com)
  - Amoy 测试网: chainId 80002 (https://rpc-amoy.polygon.technology)
  - 多 RPC 端点支持，自动故障转移
- **EIP-155 签名**: k256 + tiny-keccak 实现
- **锚定数据格式**: `gyid` magic + gyid_hash (32B) + metadata_hash (32B)
- **环境变量配置**:
  - `GYID_CHAIN_KEY`: 私钥 (hex)
  - `GYID_CHAIN_ADDR`: 发送方地址 (0x...)
  - `GYID_CHAIN_RPC`: 自定义 RPC 端点 (可选，逗号分隔多端点)
- **Aptos**: 预留接口，待实现

## API 参考

### Rust SDK

```rust
use gyid_sdk::GyIdSdk;

// 生成 GyID
let result = GyIdSdk::generate_default().await?;
println!("GyID: {}", result.id);

// 自定义选项
let opts = GenerateOptions {
    geo_level: Some("district".to_string()),
    with_geo: Some(true),
    avatar_path: Some("/path/to/avatar.png".to_string()),
    ..Default::default()
};
let result = GyIdSdk::generate(opts).await?;

// 验证
assert!(GyIdSdk::verify(&result.id));

// 设备关联
let link_result = GyIdSdk::link_device(LinkOptions {
    auth_code: "GEN".to_string(),
    master_gyid: None,
}).await?;
```

### JavaScript / TypeScript (WASM)

```javascript
import init, { generate_gyid, verify_gyid, get_sdk_version } from '@gyid/sdk-web';

await init();

// 生成
const json = await generate_gyid(JSON.stringify({ geo_level: "city" }));
const result = JSON.parse(json);
console.log(result.id);    // "GyID..."
console.log(result.hash);  // "abc123..."

// 验证
const isValid = verify_gyid(result.id);  // true

// 版本
const ver = get_sdk_version();
console.log(JSON.parse(ver).version);
```

### Kotlin (Android)

```kotlin
import io.gyid.sdk.GyIdSdk

// 在协程中调用（避免阻塞主线程）
lifecycleScope.launch(Dispatchers.IO) {
    try {
        val result = GyIdSdk.generateDefault()
        Log.d("GyID", "Generated: ${result.id}")
    } catch (e: GyIdException) {
        Log.e("GyID", "Error: ${e.message}")
    }
}
```

### Swift (iOS)

```swift
import GyIdSDK

let sdk = GyIdSDK.shared

// 异步生成
Task {
    do {
        let result = try await sdk.generateDefault()
        print("GyID: \(result.id)")
    } catch {
        print("Error: \(error.localizedDescription)")
    }
}
```

## 版本规划

| 版本    | 状态 | 内容                                   |
|--------|------|---------------------------------------|
| v0.1   | ✅完成 | 核心算法、CLI、设备关联                 |
| v0.2   | ✅完成 | 跨平台 SDK（native/WASM/Android/iOS）  |
| v0.3   | ✅完成 | Dioxus GUI 应用（头像预览/QR导出）        |
| v0.4   | ✅完成 | 链上锚定（Polygon PoS + Aptos）        |
| v1.0   | 🔜计划 | 正式发布                               |

### v0.4 链上锚定功能 (2026-04-05)
- [x] Polygon PoS 支持（Mainnet + Amoy 测试网）
- [x] EIP-155 交易签名（k256 + tiny-keccak）
- [x] 锚定数据格式（"gyid" magic + gyid_hash + metadata_hash）
- [x] 交易状态查询
- [x] 链上验证功能
- [x] CLI chain 命令（anchor/verify/status/networks）
- [x] Aptos 支持（ED25519 + BCS 编码）

### v0.3 GUI 功能清单 (2026-04-05)
- [x] 主页：GyID 展示、快速生成入口
- [x] 生成页：精度选择、权重调整、头像上传、生成进度
- [x] 设备页：关联设备列表、设备关联流程
- [x] 设置页：地理精度、权重配置、头像设置、深色模式
- [x] 权重验证提示（weights_valid 集成）
- [x] 头像预览（AvatarPreview 组件，128x128 缩略图）
- [x] GyID 导出功能（JSON/QR码 PNG）
