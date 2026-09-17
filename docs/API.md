# GyID API 参考文档

> 版本: v1.0 | 更新: 2026-04-09 | 状态: ✅ 完善

---

## 目录

1. [快速开始](#快速开始)
2. [核心类型](#核心类型)
3. [GyIdGenerator 生成器](#gyidgenerator-生成器)
4. [GyIdValidator 验证器](#gyidvalidator-验证器)
5. [地理位置 API](#地理位置-api)
6. [硬件指纹 API](#硬件指纹-api)
7. [头像处理 API](#头像处理-api)
8. [链上锚定 API](#链上锚定-api)
9. [设备关联 API](#设备关联-api)
10. [错误处理](#错误处理)

---

## 快速开始

### 安装依赖

```toml
# Cargo.toml
[dependencies]
gyid-core = { version = "0.1", features = ["native", "chain-signing", "aptos-signing"] }
```

### 基本使用

```rust
use gyid_core::{GyIdGenerator, GyIdValidator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 生成 GyID
    let generator = GyIdGenerator::default();
    let gyid = generator.generate(None).await?;
    println!("GyID: {}", gyid.id);

    // 验证 GyID
    let validator = GyIdValidator::default();
    assert!(validator.validate(&gyid.id));
    Ok(())
}
```

---

## 核心类型

### GyId

GyID 身份对象，包含生成的所有信息。

```rust
pub struct GyId {
    /// GyID 字符串 (格式: GyIDxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx)
    pub id: String,
    /// 原始 BLAKE3 哈希 (64字符 hex)
    pub hash: String,
    /// 时间戳 (UTC毫秒 << 16 | 16bit随机数)
    pub timestamp: u64,
    /// 创建时间 (可选)
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}
```

### GeneratorConfig

生成器配置。

```rust
pub struct GeneratorConfig {
    /// 地理位置精度等级 (默认: City)
    pub geo_level: GeoPrecisionLevel,
    /// 权重配置 (默认: 硬件35% 地理25% 时间15% 头像25%)
    pub weights: GyIdWeights,
    /// 是否采集头像 (默认: true)
    pub with_avatar: bool,
    /// 是否采集地理位置 (默认: true)
    pub with_geo: bool,
}
```

### GeoPrecisionLevel

地理位置精度等级。

```rust
pub enum GeoPrecisionLevel {
    /// L1: 城市级 - IP 定位
    City,
    /// L2: 区域级 - WiFi BSSID 定位
    District,
    /// L3: 精确级 - GPS + WiFi + IP 并行采集
    Exact,
    /// 手动输入坐标
    Manual,
    /// 不使用位置
    None,
}
```

### GyIdWeights

权重配置。

```rust
pub struct GyIdWeights {
    pub hardware: f64,  // 硬件指纹权重 (默认: 0.35)
    pub geo: f64,       // 地理位置权重 (默认: 0.25)
    pub time: f64,      // 时间戳权重 (默认: 0.15)
    pub avatar: f64,     // 头像权重 (默认: 0.25)
}
```

---

## GyIdGenerator 生成器

### 创建生成器

```rust
use gyid_core::{GyIdGenerator, GeneratorConfig, GeoPrecisionLevel};

// 使用默认配置
let generator = GyIdGenerator::default();

// 使用自定义配置
let config = GeneratorConfig {
    geo_level: GeoPrecisionLevel::District,
    weights: GyIdWeights::default(),
    with_avatar: true,
    with_geo: true,
};
let generator = GyIdGenerator::new(config);
```

### 生成 GyID

```rust
// 生成 (不包含头像)
let gyid = generator.generate(None).await?;

// 生成 (包含头像)
let gyid = generator.generate(Some("/path/to/avatar.png")).await?;

// 使用手动坐标生成
let gyid = generator
    .generate_with_manual_geo(39.9042, 116.4074, None)
    .await?;
```

### 采集组件

```rust
// 采集硬件指纹
let fingerprint = generator.collect_fingerprint().await?;

// 采集地理位置
let geo = generator.collect_geo().await?;

// 设置手动坐标 (用于 Manual 模式)
let geo = generator.set_manual_geo(39.9042, 116.4074)?;
```

---

## GyIdValidator 验证器

### 基本验证

```rust
use gyid_core::GyIdValidator;

let validator = GyIdValidator::default();

// 验证格式
assert!(validator.validate("GyIDxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"));

// 验证并获取信息
if let Some(info) = validator.parse("GyID...") {
    println!("版本: {}", info.version);
}
```

---

## 地理位置 API

### IpLocator (IP 定位)

```rust
use gyid_core::geo::IpLocator;

// 获取 IP 位置
let geo = IpLocator::locate().await?;
println!("国家: {}", geo.country);
println!("城市: {}", geo.city);
println!("纬度: {}", geo.latitude);
println!("经度: {}", geo.longitude);
```

### WifiLocator (WiFi BSSID 定位)

```rust
use gyid_core::geo::WifiLocator;

// 扫描周围 WiFi
let bssids = WifiLocator::scan()?;
println!("找到 {} 个 WiFi", bssids.len());

// 查询 MLS 位置
let geo = WifiLocator::query_mls(&bssids).await?;
```

### GpsLocator (GPS 定位)

```rust
use gyid_core::geo::{GpsLocator, GpsAvailability};

// 检查 GPS 可用性
let availability = GpsLocator::check_availability().await;
match availability {
    GpsAvailability::Available => println!("GPS 可用"),
    GpsAvailability::PermissionDenied => println!("需要权限"),
    GpsAvailability::NotAvailable => println!("无 GPS 设备"),
}

// 获取 GPS 位置
let geo = GpsLocator::locate().await?;
```

### H3 网格系统

```rust
use gyid_core::geo::h3::{H3Grid, H3Cell};

// 获取 H3 单元格 ID
if let Some(h3_cell) = &geo.h3_cell {
    println!("H3 ID: {}", h3_cell.cell_id());
    println!("分辨率: {}", h3_cell.resolution());

    // 获取相邻单元格
    let neighbors = h3_cell.neighbors();
    println!("邻居数量: {}", neighbors.len());

    // 获取父级/子级单元格
    let parent = h3_cell.parent(5); // 分辨率 5
    let children = h3_cell.children(9); // 分辨率 9
}
```

---

## 硬件指纹 API

### 采集硬件指纹

```rust
use gyid_core::fingerprint::{
    MacCollector, CpuCollector, BoardCollector, DiskCollector
};

// 采集各类型硬件信息
let mac = MacCollector::collect()?;
let cpu = CpuCollector::collect()?;
let board = BoardCollector::collect()?;
let disk = DiskCollector::collect()?;

println!("MAC: {}", mac);
println!("CPU ID: {}", cpu);
println!("主板序列号: {}", board);
println!("磁盘序列号: {}", disk);
```

---

## 头像处理 API

### 加载头像

```rust
use gyid_core::avatar::AvatarLoader;

// 加载头像
let avatar = AvatarLoader::load("/path/to/avatar.png")?;

// 获取哈希值
println!("哈希: {}", avatar.hash());
```

---

## 链上锚定 API

### ChainAnchor

> 需要启用 `chain-signing` feature

```rust
use gyid_core::{
    ChainAnchor, GyIdAnchor, Blockchain
};

// 创建锚定数据
let anchor = GyIdAnchor::new(gyid.hash.clone(), metadata_hash, 0);

// 锚定到 Polygon Mainnet
let tx_hash = ChainAnchor::anchor(&anchor).await?;
println!("交易哈希: {}", tx_hash);

// 锚定到指定链
let tx_hash = ChainAnchor::anchor_to(&anchor, Blockchain::PolygonAmoy).await?;
let tx_hash = ChainAnchor::anchor_to(&anchor, Blockchain::Aptos).await?;

// 查询交易状态
let status = ChainAnchor::get_tx_status_on(&tx_hash, Blockchain::Polygon).await?;
println!("已确认: {}", status.confirmed);
println!("成功: {}", status.success);

// 验证锚定
let valid = ChainAnchor::verify_by_tx(&gyid.hash, &tx_hash, Blockchain::Polygon).await?;
```

### 支持的区块链

```rust
pub enum Blockchain {
    Polygon,          // Polygon Mainnet
    PolygonAmoy,      // Polygon Amoy 测试网
    Aptos,            // Aptos Mainnet
    AptosTestnet,     // Aptos Testnet
}
```

---

## 设备关联 API

### 关联设备

```rust
use gyid_core::device::{DeviceLink, LinkedDevice};

// 创建关联码
let auth_code = DeviceLink::generate_auth_code()?;
println!("关联码: {}", auth_code);

// 创建设备关联
let link = DeviceLink::create_link(&auth_code, &master_gyid)?;
println!("关联 ID: {}", link.device_id);

// 验证关联
let is_valid = DeviceLink::verify_link(&link)?;
```

---

## 错误处理

### GyIdError

```rust
use gyid_core::GyIdError;

match result {
    Ok(gyid) => println!("生成成功: {}", gyid.id),
    Err(e) => match e {
        GyIdError::FingerprintError(msg) => eprintln!("指纹采集失败: {}", msg),
        GyIdError::GeoError(msg) => eprintln!("位置获取失败: {}", msg),
        GyIdError::AvatarError(msg) => eprintln!("头像处理失败: {}", msg),
        GyIdError::CryptoError(msg) => eprintln!("加密错误: {}", msg),
        GyIdError::StorageError(msg) => eprintln!("存储错误: {}", msg),
        GyIdError::DeviceLinkError(msg) => eprintln!("设备关联错误: {}", msg),
        GyIdError::InvalidParam(msg) => eprintln!("参数错误: {}", msg),
        GyIdError::PermissionDenied(msg) => eprintln!("权限不足: {}", msg),
    },
}
```

---

## Feature Flags

| Feature | 说明 | 默认 |
|---------|------|------|
| `native` | 完整功能 (SQLite, sysinfo, reqwest) | ✅ |
| `wasm` | WASM 平台支持 | ❌ |
| `chain-signing` | Polygon 链上签名 (k256) | ✅ |
| `aptos-signing` | Aptos 链上签名 (ED25519) | ❌ |
| `sync` | 同步阻塞 API | ❌ |

### 编译示例

```bash
# 默认 (native + chain-signing)
cargo build

# 启用 Aptos 签名
cargo build --features "native,chain-signing,aptos-signing"

# WASM 平台
cargo build --target wasm32-unknown-unknown --features wasm
```

---

## 环境变量

### 链上锚定配置

```bash
# Polygon 配置
export GYID_CHAIN_RPC="https://polygon-rpc.com,https://rpc.ankr.com/polygon"
export GYID_CHAIN_KEY="your_private_key_hex"
export GYID_CHAIN_ADDR="0x..."

# Aptos 配置
export GYID_APTOS_KEY="your_ed25519_private_key_hex"
export GYID_APTOS_ADDR="0x..."
```

---

## 示例代码

### 完整生成流程

```rust
use gyid_core::{
    GyIdGenerator, GyIdValidator, GeneratorConfig,
    GeoPrecisionLevel, ChainAnchor, GyIdAnchor, Blockchain,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 配置生成器
    let config = GeneratorConfig {
        geo_level: GeoPrecisionLevel::Exact,
        ..Default::default()
    };
    let generator = GyIdGenerator::new(config);

    // 2. 生成 GyID
    let gyid = generator.generate(Some("avatar.png")).await?;
    println!("生成 GyID: {}", gyid.id);
    println!("哈希: {}", gyid.hash);

    // 3. 验证
    let validator = GyIdValidator::default();
    assert!(validator.validate(&gyid.id));

    // 4. 链上锚定 (可选)
    let anchor = GyIdAnchor::new(gyid.hash.clone(), "metadata_hash".to_string(), 0);
    let tx_hash = ChainAnchor::anchor_to(&anchor, Blockchain::PolygonAmoy).await?;
    println!("链上锚定: {}", tx_hash);

    Ok(())
}
```

---

*文档版本: v1.0*
*最后更新: 2026-04-09*
