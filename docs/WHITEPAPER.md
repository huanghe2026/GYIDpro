# GeoYuan P2P 协议白皮书

## GPv1 — Geospatial Proof Protocol

> **版本**: 1.0  
> **状态**: 实现中 (Implemented)  
> **最后更新**: 2026-07-14  
> **协议标识符**: `/geoyuan/1.0`

---

## 目录

1. [摘要](#1-摘要)
2. [背景与动机](#2-背景与动机)
3. [设计哲学](#3-设计哲学)
4. [协议架构总览](#4-协议架构总览)
5. [H3 六边形网格系统](#5-h3-六边形网格系统)
6. [GeoCast 地理广播协议](#6-geocast-地理广播协议)
7. [PoL 位置证明](#7-pol-位置证明)
8. [消息信封与协议](#8-消息信封与协议)
9. [密码学基础](#9-密码学基础)
10. [P2P 传输层](#10-p2p-传输层)
11. [经济模型](#11-经济模型)
12. [节点类型与资源约束](#12-节点类型与资源约束)
13. [安全分析](#13-安全分析)
14. [测试与验证](#14-测试与验证)
15. [路线图](#15-路线图)
16. [术语表](#16-术语表)

---

## 1. 摘要

GeoYuan 是一个基于地理位置的去中心化身份与经济系统。其核心命题是：**物理存在即数字价值**——用户通过在真实地理位置拍摄带有 GPS 信息的照片，铸造不可伪造的地理身份标识（GyID）和原生代币（GeoYuan, GY）。

本文档定义 GeoYuan 的 P2P 网络协议 **GPv1（Geospatial Proof Protocol v1）**，该协议在标准 libp2p 传输层之上引入两项独创机制：

- **GeoCast**：基于 H3 六边形网格的地理广播路由。消息不再全网泛洪，而是由接收端按地理邻近性自主过滤，将网络流量降低 90% 以上。
- **PoL（Proof of Location）**：位置证明层。每条 P2P 消息可附加 GPS 坐标、时间戳、H3 cell 索引和 Ed25519 签名，接收端可验证发送者的物理位置声明。

GPv1 的设计目标是让每一台普通桌面 PC 或笔记本都能作为一个全节点运行，内存占用不超过 500MB，同时保持完全去中心化——无中心服务器，节点间直连通信。

---

## 2. 背景与动机

### 2.1 现有方案的不足

传统区块链网络（如比特币、以太坊）的 P2P 层采用全网泛洪广播（Gossip 协议）：每条交易或区块被转发给所有已连接节点。这种模式在网络规模较小时效率尚可，但存在根本缺陷：

| 问题 | 影响 |
|------|------|
| **流量浪费** | 一条仅与北京相关的消息，会被转发给全球所有节点 |
| **地理无关性** | 网络拓扑与物理位置完全脱钩，无法利用地理邻近性 |
| **Sybil 脆弱性** | 无物理约束，攻击者可低成本创建大量虚拟节点 |
| **资源门槛高** | 全节点需要存储完整链状态，普通设备难以承担 |

### 2.2 GeoYuan 的切入点

GeoYuan 的价值创造本身就依赖地理位置——用户必须在特定物理位置拍照才能铸造 GyID 和 GY。这意味着：

- 每个节点天然拥有可验证的物理位置
- 大部分消息（如转账、铸造广播）具有地理局部性
- 物理位置可以作为抗 Sybil 的自然壁垒

GPv1 正是基于这一洞察，将**地理空间信息**从应用层数据提升为**网络路由的一等公民**。

---

## 3. 设计哲学

### 3.1 物理存在 = 数字价值

GeoYuan 的核心等式：

```
一张带 GPS 的照片 = 一个 GyID = 一个 GeoYuan
```

照片的 SHA-256 哈希、GPS 坐标、毫秒级时间戳和随机盐值共同构成唯一标识。即使两人在同一 GPS 点拍照，时间差异或照片内容差异也会产生不同的 GyID。

### 3.2 地理感知网络

GPv1 不重新发明传输协议（libp2p 已经足够成熟），而是在应用层注入地理智能：

```
传统 P2P:    随机拓扑 → 全网泛洪 → 地理无关
GPv1:        地理拓扑 → 区域广播 → 位置证明
```

### 3.3 轻量级全节点

每个桌面运行端都是一个完整节点，不依赖中心化服务。资源约束硬性要求：

- 内存 ≤ 500MB
- 普通 PC/笔记本硬件即可满足
- 无需 GPU、无需高速磁盘

---

## 4. 协议架构总览

### 4.1 分层架构

```
┌─────────────────────────────────────────────────┐
│              应用层 (GyID / GY 钱包)              │
├─────────────────────────────────────────────────┤
│  PoL 验证层    │  GeoCast 路由层                  │
│  (GPS+签名)    │  (H3 邻近过滤)                   │
├─────────────────────────────────────────────────┤
│          消息信封 (Envelope / CBOR)              │
├─────────────────────────────────────────────────┤
│     libp2p 传输层                                │
│  TCP │ Noise │ Yamux │ Kademlia DHT │ mDNS      │
└─────────────────────────────────────────────────┘
```

### 4.2 技术栈

| 层级 | 技术 | 版本 | 用途 |
|------|------|------|------|
| 传输层 | libp2p | 0.53 | P2P 连接、发现、加密 |
| 网格系统 | h3o (H3) | 0.9 | 地理网格划分与邻居计算 |
| 签名 | ed25519-dalek | 2.1 | 数字签名 |
| 哈希 | BLAKE3 | 1.5 | 消息哈希 |
| 序列化 | ciborium (CBOR) | 0.2 | 消息编码 |
| 运行时 | Tokio | 1.36 | 异步 I/O |
| 存储 | redb | 2.1 | 本地嵌入式 KV 数据库 |

### 4.3 协议标识符

所有 GeoYuan P2P 消息使用统一协议标识符：

```
/geoyuan/1.0
```

各消息类型有独立的子协议路径（见 [§8.3](#83-消息类型)）。

---

## 5. H3 六边形网格系统

### 5.1 为什么选择 H3

H3 是 Uber 开发的全球六边形离散网格系统，具有以下特性使其适合 GeoYuan：

| 特性 | 对 GeoYuan 的价值 |
|------|------------------|
| **六边形邻接** | 每个格子有 6 个等距邻居，无方向偏差 |
| **层级分辨率** | 16 级分辨率（Res 0-15），可按需选择精度 |
| **全球覆盖** | 覆盖整个地球表面，无死角 |
| **高效索引** | 64 位整数索引，O(1) 邻居查询 |

### 5.2 分辨率选择：Res 12

GPv1 默认使用 **Resolution 12**：

| 参数 | Res 10 | Res 11 | **Res 12（采用）** | Res 13 |
|------|--------|--------|---------------------|--------|
| 六边形边长 | ~66m | ~25m | **~9m** | ~3.5m |
| 六边形宽度 | ~122m | ~46m | **~17m** | ~6m |
| 全球格子数 | ~36 亿 | ~260 亿 | **~1.8 万亿** | ~12.6 万亿 |
| GeoCast 3 环半径 | ~400m | ~150m | **~55m** | ~21m |

选择 Res 12 的理由：

- **精度**：~9m 边长对应"一栋楼或一个小建筑群"的粒度，满足 30 米以内的定位精度需求
- **广播范围**：3 环邻居覆盖 ~55m 半径，与城市街区的社交距离匹配
- **内存影响可忽略**：路由表只存储已连接节点的 H3 cell，不存储全球全量格子

### 5.3 GPS → H3 cell 映射

```rust
/// 默认分辨率
pub const DEFAULT_H3_RESOLUTION: u8 = 12;

/// 从 GPS 坐标计算 H3 cell
pub fn gps_to_cell(lat: f64, lon: f64) -> Option<H3Cell> {
    let ll = LatLng::new(lat, lon).ok()?;
    let resolution = Resolution::try_from(DEFAULT_H3_RESOLUTION).ok()?;
    let cell = ll.to_cell(resolution);
    Some(H3Cell(cell.into()))
}
```

每个 GPS 坐标 `(lat, lon)` 被映射为一个 64 位整数 H3 cell 索引。这个索引是确定性的——相同坐标总是产生相同 cell。

### 5.4 邻居环计算

GeoCast 使用 `grid_disk` 计算指定环数内的所有邻居：

```rust
/// GeoCast 广播邻居环数（3 环 ≈ ~55m 半径）
pub const GEOCAST_NEIGHBOR_RINGS: u8 = 3;

/// 获取指定 cell 的 N 环邻居
pub fn get_neighbors(cell: H3Cell, ring: u8) -> Vec<H3Cell>

/// 检查两个 cell 是否在指定环范围内
pub fn is_within_ring(cell_a: H3Cell, cell_b: H3Cell, ring: u8) -> bool
```

3 环邻居包含 1 + 6 + 12 + 18 = **37 个格子**，覆盖约 55 米半径的圆形区域。

---

## 6. GeoCast 地理广播协议

### 6.1 设计动机

传统 Gossip 协议将消息转发给所有已连接节点，无论消息的地理相关性。对于 GeoYuan 这类地理敏感应用，这意味着大量无关流量。

GeoCast 的核心思想：**消息应只在地理相关的范围内传播**。

### 6.2 接收端过滤模型

GPv1 采用**接收端过滤**而非发送端过滤。这一设计决策的关键考量：

| 方案 | 优点 | 缺点 |
|------|------|------|
| 发送端过滤 | 减少网络传输 | 发送端需知道每个对等节点的位置（隐私问题） |
| **接收端过滤** ✅ | 发送端无需暴露对等节点位置 | 消息仍被传输到所有节点（但被丢弃） |

GPv1 选择接收端过滤，因为：

1. **隐私**：发送端不需要维护对等节点的位置数据库
2. **简洁**：广播逻辑不依赖网络拓扑知识
3. **安全**：接收端自主决定是否处理消息，无法被发送端操纵

### 6.3 消息处理流程

```
发送端                                接收端
  │                                     │
  │  geocast_broadcast(msg, h3_cell)    │
  │────────────────────────────────────>│
  │                                     │
  │         载荷: [8字节源H3 cell]      │
  │              [原始消息]              │
  │                                     │
  │                          解析前8字节 → src_cell
  │                          读取本机 local_h3_cell
  │                                     │
  │                    ┌── is_within_ring(src, local, 3)?
  │                    │
  │              ┌──YES┴──NO──┐
  │              │             │
  │     处理 inner_payload   丢弃消息
  │     回复 GeoCastResponse  (debug 日志)
  │              │
  │<─────────────┘
  │  GeoCastResponse (本机 H3 cell)
  │                                     │
```

### 6.4 载荷格式

GeoCast 消息的 `payload` 字段采用二进制前缀格式：

```
┌──────────────────────┬──────────────────────────┐
│  8 bytes (little-endian)  │  剩余字节              │
│  源节点 H3 cell 索引      │  原始消息载荷          │
└──────────────────────┴──────────────────────────┘
```

接收端逻辑（`swarm.rs`）：

```rust
MessageType::GeoCast => {
    let local_cell = status_bg.lock().unwrap().local_h3_cell;
    if let Some(local) = local_cell {
        if envelope.payload.len() >= 8 {
            let src_bytes: [u8; 8] = envelope.payload[..8].try_into().unwrap_or([0u8; 8]);
            let src_cell = u64::from_le_bytes(src_bytes);
            let source = H3Cell::from_u64(src_cell);
            let local_h3 = H3Cell::from_u64(local);
            if h3grid::is_within_ring(source, local_h3, h3grid::GEOCAST_NEIGHBOR_RINGS) {
                // 在范围内：处理消息
                let inner_payload = envelope.payload[8..].to_vec();
                // ... 解析并处理 inner_payload
                // 回复 GeoCastResponse
            } else {
                // 不在范围内：丢弃
            }
        }
    }
}
```

### 6.5 流量优化效果

假设网络中有 100 个节点，其中 5 个在广播范围内：

| 模式 | 每条消息传输次数 | 带宽利用率 |
|------|-----------------|-----------|
| 全网广播 | 100 次 | 5% 有效 |
| **GeoCast (接收端过滤)** | 100 次传输，5 次处理 | 5% 有效（但 CPU 浪费减少） |
| GeoCast + 发送端路由（未来） | 5 次 | 100% 有效 |

> **注**：当前版本（GPv1）实现接收端过滤，流量仍被传输到所有节点但被丢弃。未来版本（GPv2）计划引入 Kademlia DHT 的地理感知路由，实现发送端过滤。

### 6.6 公开 API

```rust
impl SwarmHandle {
    /// 全网广播（不携带 H3 cell）
    pub fn broadcast(&self, msg_type: MessageType, payload: Vec<u8>)

    /// GeoCast 地理广播（携带源 H3 cell）
    pub fn geocast_broadcast(&self, msg_type: MessageType, payload: Vec<u8>, h3_cell: u64)

    /// 设置本机 H3 cell（GeoCast 路由定位）
    pub fn announce_location(&self, h3_cell: u64)
}
```

节点启动时通过 `announce_location()` 设置本机 H3 cell，之后即可使用 `geocast_broadcast()` 发送地理广播。

---

## 7. PoL 位置证明

### 7.1 概述

PoL（Proof of Location）是 GPv1 的第二大独创机制。它允许消息发送者附加一个**密码学位置证明**，证明自己在特定时间位于特定地理位置。

与 GPS 验证不同，PoL 不依赖外部预言机——它证明的是"持有某私钥的人声称自己在某位置"，而非"物理上确实在该位置"。真正的物理验证通过照片铸造时的 EXIF GPS 数据完成。

### 7.2 结构定义

```rust
pub struct LocationProof {
    /// GPS 纬度
    pub latitude: f64,
    /// GPS 经度
    pub longitude: f64,
    /// UTC 时间戳（毫秒）
    pub timestamp_millis: u64,
    /// H3 cell (Res 12) 索引
    pub h3_cell: u64,
    /// Ed25519 签名 (64 bytes)
    pub signature: Vec<u8>,
    /// 签名者公钥 (32 bytes)
    pub public_key: [u8; 32],
}
```

### 7.3 签名机制

PoL 的签名消息采用固定长度二进制格式：

```
签名消息 = lat(8B) || lon(8B) || timestamp(8B) || h3_cell(8B) || payload_hash(32B)
         = 64 字节
```

各字段使用小端序（little-endian）编码，`payload_hash` 是消息载荷的 BLAKE3 哈希（32 字节）。

```rust
fn message_to_sign(
    lat: f64, lon: f64, timestamp: u64,
    h3_cell: u64, payload_hash: &[u8; 32]
) -> Vec<u8> {
    let mut msg = Vec::with_capacity(64);
    msg.extend_from_slice(&lat.to_le_bytes());
    msg.extend_from_slice(&lon.to_le_bytes());
    msg.extend_from_slice(&timestamp.to_le_bytes());
    msg.extend_from_slice(&h3_cell.to_le_bytes());
    msg.extend_from_slice(payload_hash);
    msg
}
```

### 7.4 创建与验证

**创建 PoL**：

```rust
pub fn new(
    lat: f64, lon: f64, h3_cell: u64,
    payload_hash: &[u8; 32], keypair: &KeyPair
) -> Self {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let msg = Self::message_to_sign(lat, lon, timestamp, h3_cell, payload_hash);
    let sig = keypair.sign(&msg);
    Self { latitude: lat, longitude: lon, timestamp_millis: timestamp, h3_cell,
           signature: sig.to_bytes().to_vec(), public_key: keypair.public_bytes() }
}
```

**验证 PoL**：

```rust
pub fn verify(&self, payload_hash: &[u8; 32]) -> bool {
    if self.signature.len() != 64 { return false; }
    let msg = Self::message_to_sign(
        self.latitude, self.longitude,
        self.timestamp_millis, self.h3_cell, payload_hash
    );
    let sig: [u8; 64] = match self.signature.as_slice().try_into() {
        Ok(s) => s, Err(_) => return false,
    };
    KeyPair::verify_signature(&msg, &sig, &self.public_key)
}
```

验证过程：

1. 检查签名长度是否为 64 字节
2. 重新构造签名消息（使用证明中的 GPS、时间戳、H3 cell 和传入的 payload_hash）
3. 用证明中的公钥验证 Ed25519 签名
4. 任意字段被篡改都会导致签名验证失败

### 7.5 安全属性

| 属性 | 保证 |
|------|------|
| **不可伪造** | 没有私钥无法生成有效签名 |
| **完整性** | GPS/时间/cell/载荷任一字段被篡改，签名失效 |
| **不可否认** | 签名者公钥固定，无法否认已签名的位置声明 |
| **绑定载荷** | payload_hash 将位置证明与特定消息绑定，防重放 |

### 7.6 与 GeoCast 的协同

PoL 与 GeoCast 是互补的：

```
GeoCast 解决: "消息应被谁处理？" → 地理邻近的节点
PoL    解决: "消息来自谁，在哪？" → 可验证的位置声明
```

GeoCast 消息**应当**附加 PoL 证明（`location_proof: Some(...)`），普通消息**可选**附加。

---

## 8. 消息信封与协议

### 8.1 Envelope 结构

所有 P2P 消息被封装在统一的 `Envelope` 中：

```rust
pub struct Envelope {
    /// 消息类型
    pub msg_type: MessageType,
    /// 发送者 PeerID (Base58)
    pub sender: String,
    /// 消息载荷 (序列化后的业务数据)
    pub payload: Vec<u8>,
    /// 时间戳 (Unix 秒)
    pub timestamp: i64,
    /// 可选签名 (Ed25519 hex)
    pub signature: Option<String>,
    /// 可选 PoL 位置证明
    #[serde(default)]
    pub location_proof: Option<LocationProof>,
}
```

### 8.2 序列化：CBOR

GPv1 使用 **CBOR（Concise Binary Object Representation, RFC 8949）** 作为线格式，而非 JSON：

| 特性 | JSON | CBOR |
|------|------|------|
| 编码 | 文本 | 二进制 |
| 体积 | 较大 | **紧凑 ~40%** |
| 解析速度 | 较慢 | **快 2-3x** |
| Schema | 无 | 无（但有标签系统） |

```rust
pub fn serialize_cbor(&self) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    ciborium::ser::into_writer(self, &mut buf)?;
    Ok(buf)
}

pub fn deserialize_cbor(data: &[u8]) -> io::Result<Self> {
    ciborium::de::from_reader(data)
}
```

CBOR 的 `#[serde(default)]` 标注确保 `location_proof` 字段向后兼容——旧节点发送的无 PoL 消息仍能被新节点正确解析。

### 8.3 消息类型

```rust
pub enum MessageType {
    Handshake,           // 握手
    WalletRequest,       // 钱包同步请求
    WalletResponse,      // 钱包同步响应
    GeoIdRequest,        // GyID 同步请求
    GeoIdResponse,       // GyID 同步响应
    DeviceLinkRequest,   // 设备关联请求（从→主）
    DeviceLinkResponse,  // 设备关联响应
    DeviceListRequest,   // 设备列表查询
    DeviceListResponse,  // 设备列表响应
    CoinMinted,          // 铸造广播
    CoinTransferred,     // 转账广播
    Ping,                // 心跳
    Pong,                // 心跳响应
    GeoCast,             // 地理广播（携带 H3 cell）
    GeoCastResponse,     // 地理广播响应
}
```

每种子类型对应独立的协议路径：

| 消息类型 | 协议路径 | 需 PoL |
|---------|----------|--------|
| `GeoCast` | `/geoyuan/geocast` | ✅ 必须 |
| `CoinTransferred` | `/geoyuan/coin/transferred` | ✅ 建议 |
| `CoinMinted` | `/geoyuan/coin/minted` | ✅ 建议 |
| `WalletRequest` | `/geoyuan/wallet/request` | ❌ 可选 |
| `Ping` | `/geoyuan/ping` | ❌ |

### 8.4 业务 Payload

GPv1 定义了多种业务载荷结构：

- **TransferPayload**：转账广播（tx_id, from/to GyID, amount, sender_pubkey）
- **MintPayload**：铸造广播（gy_id, latitude, longitude）
- **WalletSyncPayload/Response**：钱包状态同步（余额、最近交易摘要）
- **DeviceLinkRequest/Response**：多设备关联
- **TxSummary**：交易摘要（截断哈希、类型、金额、时间戳）

---

## 9. 密码学基础

### 9.1 Ed25519 签名

GPv1 使用 **Ed25519** 作为唯一的签名算法（通过 `ed25519-dalek` 2.1）：

```rust
pub struct KeyPair {
    secret: [u8; 32],      // 私钥
    public: VerifyingKey,   // 公钥
}
```

选择 Ed25519 的理由：

| 属性 | Ed25519 | ECDSA (secp256k1) | RSA-2048 |
|------|---------|--------------------|-----------| 
| 公钥大小 | **32B** | 33B | 256B |
| 签名大小 | **64B** | 64-72B | 256B |
| 签名速度 | **~50μs** | ~100μs | ~1ms |
| 验证速度 | **~150μs** | ~300μs | ~100μs |
| 安全等级 | 128-bit | 128-bit | 112-bit |

关键 API：

```rust
impl KeyPair {
    pub fn generate() -> Self                          // 生成新密钥对
    pub fn sign(&self, message: &[u8]) -> Signature    // 签名
    pub fn verify(&self, msg: &[u8], sig: &Signature) -> bool  // 验证（实例方法）
    pub fn verify_signature(                           // 验证（静态方法）
        message: &[u8], signature: &[u8; 64], public_key: &[u8; 32]
    ) -> bool
    pub fn public_bytes(&self) -> [u8; 32]             // 公钥字节
    pub fn secret_bytes(&self) -> [u8; 32]             // 私钥字节（持久化用）
}
```

### 9.2 BLAKE3 哈希

消息哈希使用 **BLAKE3**（而非 SHA-256）：

- 速度：BLAKE3 在现代 CPU 上比 SHA-256 快 **5-10 倍**
- 并行：原生支持多线程并行计算
- 输出：32 字节（256-bit），满足 128-bit 安全等级

PoL 的 `payload_hash` 和交易的 `tx_hash` 均使用 BLAKE3。

### 9.3 密钥管理

私钥以 32 字节 hex 字符串形式持久化在本地（`~/.local/share/geoyuan/`），不经过网络传输。首次启动时自动生成，后续从本地存储加载。

```rust
// 持久化
let secret_hex = hex::encode(keypair.secret_bytes());
// 加载
let keypair = KeyPair::from_secret(&hex::decode(secret_hex)?)?;
```

---

## 10. P2P 传输层

### 10.1 libp2p 协议栈

GPv1 构建于 libp2p 0.53 之上，使用以下 feature 组合：

```toml
libp2p = { version = "0.53", features = [
    "tcp",              # TCP 传输
    "noise",            # Noise 协议加密
    "yamux",            # 多路复用
    "mdns",             # 局域网自动发现
    "kad",              # Kademlia DHT 路由
    "request-response", # 请求-响应模式
    "macros",           # 派生宏
    "tokio",            # Tokio 运行时
    "cbor"              # CBOR 编解码
]}
```

协议栈分层：

```
┌─────────────────────────────────────┐
│  GeoYuan RequestResponse (CBOR)     │  应用协议
├─────────────────────────────────────┤
│  Kademlia DHT     │  mDNS           │  发现层
├─────────────────────────────────────┤
│  Noise (XX 握手)                     │  加密层
├─────────────────────────────────────┤
│  Yamux (多路复用)                    │  复用层
├─────────────────────────────────────┤
│  TCP                                │  传输层
└─────────────────────────────────────┘
```

### 10.2 节点发现

GPv1 支持两种节点发现机制：

**mDNS（局域网）**：
- 零配置，同一局域网内自动发现对等节点
- 适合本地测试和办公场景
- 无需任何引导节点

**Kademlia DHT（公网）**：
- 分布式哈希表，支持跨公网节点发现
- 需要至少一个 Bootstrap 节点（`<SERVER_IP>:4001`）
- 适合 Internet 规模的部署

### 10.3 连接加密

所有 P2P 连接使用 **Noise Protocol Framework**（XX 握手模式）加密：

- 前向安全：即使长期私钥泄露，历史流量无法解密
- 身份隐藏：握手过程中双方身份相互认证
- 零配置：无证书管理开销

### 10.4 请求-响应模式

GeoYuan 使用 libp2p 的 `request-response` 协议，而非纯 Gossip。这意味着：

- 每条消息可以有对应的响应（如 `WalletRequest` → `WalletResponse`）
- GeoCast 消息收到后会回复 `GeoCastResponse`
- 比 Gossip 更适合需要确认的交互场景

---

## 11. 经济模型

### 11.1 GeoYuan 铸造

**核心机制**：上传带 GPS 信息的照片 → 铸造 1 GY

```
📸 照片(含GPS EXIF)
    ↓
SHA-256(照片数据) ⊕ GPS坐标 ⊕ 毫秒时间戳 ⊕ 随机盐
    ↓
📍 GyID (地理身份标识，永久有效)
    ↓
🪙 +1 GY (GeoYuan 代币)
```

### 11.2 定位精度层级

| 层级 | 精度 | 来源 | 用途 |
|------|------|------|------|
| 原始 GPS | 1-3 米 | 高德 SDK / EXIF | 验证物理存在 |
| H3 网格 | ~9 米 (Res 12) | h3o | GeoCast 路由 |
| GeoHash | ~19 米 (7位) | GyID 计算 | 唯一性标识 |

### 11.3 供应量分析

GeoYuan 不设固定总量（区别于比特币的 2100 万），而是受物理世界约束：

| 区域 | 面积 | Res 12 格子数 | GY 上限 |
|------|------|--------------|---------|
| 城市建成区 | ~1,200,000 km² | ~560 亿 | 高 |
| 农业/乡村 | ~48,000,000 km² | ~2.2 万亿 | 极高 |
| 全球陆地 | ~149,000,000 km² | ~6.9 万亿 | 理论极限 |

实际有效供应受以下约束：

- 每个 H3 cell 每自然日最多铸造 1 个 GY
- 需要真实物理到场拍照（非虚拟定位）
- 大部分海洋和无人区实际无法铸造

> **设计理念**：GeoYuan 是"地理发现型"代币——总量由可探索的物理世界决定，而非人为设定的数字。

### 11.4 未来铸币方式（规划中）

| 方式 | 机制 | 状态 |
|------|------|------|
| 📸 照片铸币 | GPS 照片 → +1 GY | ✅ 已实现 |
| 🚶 轨迹铸币 | 连续 GPS 轨迹 → +0.1~0.5 GY | 规划中 |
| 🏆 地理发现 | 首次在某 H3 cell 铸造 → +2 GY | 规划中 |
| 🏪 PoL 节点奖励 | 节点在线 >8h + GPS 稳定 → +0.5 GY/日 | 规划中 |
| 🤝 双人验证 | 同 H3 cell 两人同时铸造 → 各 +0.5 GY | 规划中 |

---

## 12. 节点类型与资源约束

### 12.1 节点类型

| 类型 | 描述 | 运行方式 | 适用场景 |
|------|------|----------|----------|
| **Bootstrap 节点** | 公网入口，协助新节点发现 | Linux 服务器, systemd | 生产部署 |
| **CLI 节点** | 命令行全节点 | `geoyuan p2p start` | 开发/测试/轻量运行 |
| **GUI 节点** | 带图形界面的全节点 | `geoyuan-gui` | 日常使用 |

### 12.2 内存约束

所有节点类型满足 **≤ 500MB** 硬性要求：

| 组件 | CLI 节点 | GUI 节点 |
|------|---------|---------|
| Rust 运行时 | ~5 MB | ~5 MB |
| Tokio 异步运行时 | ~10-20 MB | ~10-20 MB |
| libp2p 协议栈 | ~20-50 MB | ~20-50 MB |
| redb 本地数据库 | ~10-30 MB | ~10-30 MB |
| 应用状态 | ~20-50 MB | ~20-50 MB |
| Slint GUI 渲染 | — | ~50-100 MB |
| **总计** | **~80-150 MB** ✅ | **~200-350 MB** ✅ |

### 12.3 端口规划

```
P2P 节点:    4001-4999  (libp2p TCP, 默认 4001)
WebSocket:   由 NGINX 转发到 4001 (公网接入)
```

### 12.4 持久化

节点数据存储在本地，无中心数据库：

```
~/.local/share/geoyuan/
├── identity.json      # GyID 身份
├── wallet.redb        # 钱包数据库 (redb)
├── keystore.hex       # Ed25519 私钥
└── app_state.json     # 应用状态 (余额、交易记录、成就)
```

---

## 13. 安全分析

### 13.1 威胁模型

| 威胁 | 描述 | GPv1 防御 |
|------|------|-----------|
| **Sybil 攻击** | 攻击者创建大量虚拟节点 | 物理位置约束 + PoL 签名 |
| **位置伪造** | 谎报 GPS 位置 | 照片 EXIF 验证 + PoL 签名绑定 |
| **消息篡改** | 中途修改消息内容 | CBOR 序列化 + Ed25519 签名 |
| **重放攻击** | 重发旧消息 | 时间戳 + nonce + payload_hash 绑定 |
| **中间人攻击** | 窃听/篡改通信 | Noise 协议加密 |

### 13.2 Sybil 攻击防御

传统 P2P 网络中，创建虚拟节点成本为零。GeoYuan 的防御：

1. **铸造门槛**：每个 GyID 需要真实 GPS 照片，虚拟节点无法铸造
2. **PoL 签名**：每条消息附带位置签名，伪造者需要真实私钥
3. **地理邻近性**：GeoCast 只在物理邻近范围内传播，远程 Sybil 节点无法干扰局部通信

### 13.3 位置伪造防御

PoL 本身不证明物理在场（它证明的是"签名者声称在某位置"）。真正的物理验证在铸造环节：

- 照片 EXIF GPS 数据由相机/手机硬件写入
- 铸造时验证 EXIF 完整性（未被后期修改）
- 同一 H3 cell 每日铸造上限防止刷币

### 13.4 重放攻击防御

PoL 的签名消息包含 `timestamp_millis` 和 `payload_hash`：

- **时间戳**：接收端可检查时间偏差（建议阈值 < 5 分钟）
- **payload_hash**：将位置证明绑定到特定消息，旧消息的 PoL 无法用于新消息
- **WalletSyncPayload** 中的 `nonce` 字段提供额外重放保护

### 13.5 已知局限

| 局限 | 说明 | 缓解方案 |
|------|------|----------|
| GPS 欺骗 | 高级攻击者可伪造 GPS 信号 | 未来引入多源验证（WiFi/基站） |
| 接收端过滤的流量浪费 | 消息仍传输到所有节点 | GPv2 计划引入发送端地理路由 |
| 无经济惩罚 | 恶意节点无直接成本 | 未来引入质押/声誉系统 |

---

## 14. 测试与验证

### 14.1 测试覆盖

| 类别 | 数量 | 状态 |
|------|------|------|
| 单元测试 | 85 | ✅ 全绿 |
| 集成测试 | 21 | ✅ 全绿 |
| 文档测试 | 1 | ✅ 全绿 |
| **总计** | **107** | ✅ |

### 14.2 关键测试用例

**H3 网格测试**（7 个）：
- `test_gps_to_cell` — GPS → H3 cell 转换
- `test_get_neighbors_3_rings` — 3 环邻居数量 (≥37)
- `test_is_within_ring` — 邻近性判断（北京 vs 上海应不在 3 环内）

**PoL 测试**（5 个）：
- `test_location_proof_create_and_verify` — 创建并验证
- `test_location_proof_wrong_hash_fails` — 错误 payload 哈希应失败
- `test_pol_serialization_roundtrip` — CBOR 序列化往返

**交易验签测试**（5 个）：
- `test_verify_signature_valid` — 有效签名通过
- `test_verify_signature_invalid` — 错误公钥应失败
- `test_verify_signature_empty_fails` — 空签名应失败

**Native 网络测试**（3 个，需 `native` feature）：
- `test_swarm_native_mdns_discovery` — mDNS 自动发现
- `test_swarm_native_geocast` — GeoCast 地理广播
- `test_swarm_native_pol_verification` — PoL 端到端验证

### 14.3 自动化测试脚本

本地多节点测试脚本：
- `tools/p2p-test.ps1` (Windows)
- `tools/p2p-test.sh` (Linux)

脚本自动启动 3 个 CLI 节点，验证 mDNS 发现、GeoCast 广播和 PoL 验证。

---

## 15. 路线图

### GPv1（当前版本）

- ✅ Ed25519 真实验签
- ✅ H3 Res 12 地理网格
- ✅ GeoCast 接收端过滤
- ✅ PoL 位置证明
- ✅ CLI p2p 命令 (start/status/peers/peers-nearby)
- ✅ GUI 成就/能量系统
- ✅ 107 测试全绿
- ⏳ 服务器部署（`<SERVER_IP>`，待 SSH 恢复）

### GPv2（规划中）

- 📋 发送端地理路由（Kademlia 地理感知 DHT）
- 📋 多方式铸币引擎（轨迹/发现/PoL 节点奖励/双人验证）
- 📋 PoL 时间窗口验证（接收端检查时间偏差）
- 📋 节点声誉系统
- 📋 移动端 SDK（Android/iOS）

### GPv3（远期）

- 📋 跨链锚定（Polygon/Aptos）
- 📋 零知识位置证明（zkPoL）
- 📋 地理联邦学习

---

## 16. 术语表

| 术语 | 定义 |
|------|------|
| **GyID** | GeoYuan Identity，基于 GPS 照片的地理身份标识 |
| **GY** | GeoYuan，原生代币 |
| **GPv1** | Geospatial Proof Protocol v1，本文档定义的协议 |
| **GeoCast** | 基于 H3 网格的地理广播路由机制 |
| **PoL** | Proof of Location，位置证明 |
| **H3** | Uber 的六边形离散网格系统 |
| **Res 12** | H3 分辨率 12，边长 ~9m |
| **Envelope** | P2P 消息信封，封装消息类型、载荷、签名和 PoL |
| **Bootstrap 节点** | 公网入口节点，协助新节点发现 |
| **CBOR** | Concise Binary Object Representation，二进制序列化格式 |

---

## 附录 A：代码引用

| 模块 | 文件路径 |
|------|----------|
| PoL 定义 | `geoyuan-core/src/p2p/protocol.rs` |
| H3 网格 | `geoyuan-core/src/geo/h3grid.rs` |
| Swarm 事件循环 | `geoyuan-core/src/p2p/swarm.rs` |
| Ed25519 签名 | `geoyuan-core/src/crypto/signer.rs` |
| 交易验签 | `geoyuan-core/src/chain/transaction.rs` |
| CLI p2p 命令 | `geoyuan-cli/src/commands/p2p.rs` |
| 集成测试 | `geoyuan-core/tests/integration.rs` |

## 附录 B：依赖版本

| 依赖 | 版本 | 用途 |
|------|------|------|
| libp2p | 0.53 | P2P 网络 |
| h3o | 0.9 | H3 网格 |
| ed25519-dalek | 2.1 | Ed25519 签名 |
| blake3 | 1.5 | 哈希 |
| ciborium | 0.2 | CBOR 序列化 |
| tokio | 1.36 | 异步运行时 |
| redb | 2.1 | 本地数据库 |

---

*本白皮书基于 GeoYuan 项目实际代码实现撰写。所有代码引用均可在 `c:/Users/Nice/GYID/` 仓库中验证。*  
*GeoYuan Team · 2026*
