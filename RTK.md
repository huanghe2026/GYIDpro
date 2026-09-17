# 北斗 RTK 集成计划

> **状态**：规划中，尚未启动开发
> **创建时间**：2026-07-19
> **负责模块**：gyid-core（主）、geoyuan-core（可选联动）

## 1. 背景与目标

当前 GeoYuan / GyID 项目的定位体系基于 **IP / WiFi / 单点 GPS** 三路竞速，精度分级为 `City / District / Exact`，最优精度约 3-10 米（单点 GPS）。为支撑高精度场景（资产锚定、物理位置证明 PoL、地理围栏确权等），需引入 **北斗 RTK（实时动态差分定位）**，将室外定位精度提升至 **1-3 厘米**。

### 核心目标

- 接入支持北斗（BDS）的 RTK 接收机，获取厘米级固定解
- 通过 NTRIP 协议拉取 CORS 差分数据，实现实时差分
- RTK 作为**最高精度优先源**，失败时自动降级到现有 GPS / WiFi / IP
- 不破坏现有定位链路，向后兼容

### 非目标

- 不替换现有 IP / WiFi / GPS 定位（室内场景仍需依赖）
- 不在本阶段实现自建基准站（仅接入第三方 CORS）

---

## 2. RTK 基础概念

| 概念 | 说明 |
|------|------|
| **RTK** | Real-Time Kinematic，实时动态差分定位。利用基准站已知精确坐标与流动站观测值的差分修正，达到厘米级精度 |
| **流动站** | Rover，即用户端的 RTK 接收机（本项目接入的设备） |
| **基准站** | Base，位置已知且固定的接收机，向流动站发送差分修正数据 |
| **CORS** | Continuously Operating Reference Station，连续运行基准站网络，由服务商运营，用户通过 NTRIP 协议拉取差分流 |
| **NTRIP** | Networked Transport of RTCM via Internet Protocol，基于 HTTP 的差分数据传输协议 |
| **RTCM 3.x** | 差分数据标准格式，由基准站编码、流动站解码应用 |
| **NMEA 0183** | 接收机输出的标准语句格式（GGA / GST / RMC 等），含位置、精度、质量标识 |
| **固定解 / 浮点解** | GGA 质量标识 `4`=固定解（厘米级）、`5`=浮点解（分米级）、`1`=单点、`2`=DGPS |

---

## 3. 硬件准备

### 3.1 RTK 接收机选型

需支持 **北斗（BDS）+ GPS + GLONASS + Galileo** 多频多系统：

| 型号 | 厂商 | 参考价 | 特点 |
|------|------|--------|------|
| **ZED-F9P** | u-blox | ¥800-1500 | 开源生态最好，u-center 工具链完善，社区支持多 |
| **UM980** | 华大北斗 | ¥600-1200 | 国产，北斗支持完整，性价比高 |
| **UM4B** | 和芯星通 | ¥1000-2000 | 国产高端，多频全系统 |

**推荐**：原型阶段用 u-blox ZED-F9P（生态成熟，调试方便）；量产可切华大北斗。

### 3.2 接入方式

- **USB**：即插即用，识别为虚拟串口（Windows `COMx`，Linux `/dev/ttyACM0`）
- **串口**：TTL/RS232，需电平转换，适合嵌入式集成
- **蓝牙**：部分接收机支持，适合移动场景

### 3.3 差分数据源

| 类型 | 提供方 | 费用 | 覆盖 | 备注 |
|------|--------|------|------|------|
| **商业 CORS** | 千寻位置 | ~¥0.01/次 或包月 | 全国 | 推荐首选，稳定性好 |
| **商业 CORS** | 六分科技 | 类似 | 全国 | 备选 |
| **运营商** | 中国移动高精度定位 | 包月 | 全国 | 覆盖广 |
| **自建基准站** | 自有 | 一次性硬件成本 | 局部 | 本阶段不实施 |
| **免费 CORS** | 部分省份测试账号 | 免费 | 局部 | 稳定性不保证，仅测试用 |

---

## 4. 新增 Rust 依赖

```toml
# gyid-core/Cargo.toml [dependencies]
serialport = "4"        # 串口/USB 读取 NMEA 流
nmea-parser = "0.10"    # 解析 GGA/GST/RMC（含北斗 GBxx 语句）
ntrip-rs = "0.4"        # NTRIP 客户端拉取 RTCM 差分（备选：自行用 reqwest + tokio 实现）
```

> 依赖项版本在实施时以 crates.io 最新稳定版为准。

---

## 5. 新增模块设计

### 5.1 文件结构

```
gyid-core/src/geo/
├── mod.rs          # 改动：GeoLocation 字段扩展 + 精度级新增
├── gps.rs          # 现有，不动
├── wifi.rs         # 现有，不动
├── ip.rs           # 现有，不动
├── h3.rs           # 现有，不动
├── nominatim.rs    # 现有，不动
└── rtk.rs          # 新增：RTK 定位模块
```

### 5.2 `rtk.rs` 三大组件

| 组件 | 职责 | 关键点 |
|------|------|--------|
| **串口读取** | `serialport` 打开设备，按行读 NMEA 语句 | 跨平台设备名探测（`COM3` / `/dev/ttyACM0`） |
| **NTRIP 客户端** | 连接 CORS 挂载点，拉 RTCM 3.x 差分流，回灌接收机 | 鉴权（Basic Auth）、挂载点选择、断线重连 |
| **NMEA 解析** | `nmea-parser` 解析 GGA/GST/RMC | 重点判断 GGA 第 6 字段质量标识 |

### 5.3 定位质量判断

GGA 语句第 6 字段（Fix Quality）：

| 值 | 含义 | 精度 | 是否可用 |
|----|------|------|----------|
| `0` | 无定位 | - | 否 |
| `1` | 单点定位 | 3-10m | 降级用 |
| `2` | DGPS | 0.5-3m | 降级用 |
| `4` | **RTK 固定解** | **1-3cm** | **目标精度** |
| `5` | RTK 浮点解 | 0.2-1m | 可用但非最优 |

**策略**：优先等待 `4`（固定解），超时后接受 `5`（浮点解），再超时降级到现有 GPS。

---

## 6. 数据结构改动

### 6.1 `GeoLocation` 字段扩展

当前 `gyid-core/src/geo/mod.rs:99` 的 `GeoLocation` **缺时间戳、缺 accuracy 数值、缺定位质量**，需补齐：

```rust
pub struct GeoLocation {
    pub level: GeoPrecisionLevel,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    // ─── 新增字段 ───
    pub altitude: Option<f64>,              // RTK 有高程
    pub accuracy: Option<f64>,              // 精度（米），RTK 可达 0.02m
    pub fix_quality: Option<GpsFixQuality>, // 定位质量枚举
    pub timestamp: Option<u64>,             // Unix 毫秒
    pub source: Option<String>,             // "rtk" / "gps" / "wifi" / "ip"
    // ─── 现有字段保留 ───
    pub city: Option<String>,
    pub country: Option<String>,
    pub wifi_bssids: Option<Vec<String>>,
    pub h3_cell: Option<String>,
    pub h3_resolution: Option<u8>,
}

pub enum GpsFixQuality {
    Single,      // 单点
    Dgps,        // 差分 GPS
    RtkFixed,    // RTK 固定解（厘米级）
    RtkFloat,    // RTK 浮点解（分米级）
    Unknown,
}
```

### 6.2 精度级扩展

```rust
pub enum GeoPrecisionLevel {
    City,
    District,
    Exact,        // 现有：单点 GPS 级（米级）
    Centimeter,   // 新增：RTK 固定解（厘米级）
    Manual,
    None,
}
```

---

## 7. 编排层改动

### 7.1 新增 `RtkLocator`

```rust
// gyid-core/src/geo/rtk.rs
pub struct RtkLocator {
    port: String,           // "COM3" / "/dev/ttyACM0"
    ntrip: NtripConfig,     // CORS 连接配置
    timeout: Duration,
}

impl RtkLocator {
    pub fn locate(&self) -> Result<GeoLocation>;          // 阻塞直到固定解或超时
    pub fn locate_fast(&self) -> Result<GeoLocation>;      // 短超时，接受浮点解
    pub fn check_availability(&self) -> RtkAvailability;   // 硬件+差分流检测
}
```

### 7.2 `collect_geo()` 改动

当前 `IdentityGenerator::collect_geo()`（`identity/generator.rs:146`）按 `config.geo_level` 编排 IP/WiFi/GPS。改动：

- 新增 RTK 分支：**优先尝试 RTK，失败/超时降级到现有 GPS / WiFi / IP**
- `collect_geo_parallel()` 加入 RTK 作为竞速源，RTK 固定解到达即短路返回
- 新增 `config.use_rtk: bool` 配置项，默认 false（不影响现有行为）

### 7.3 降级链

```
RTK 固定解 (4)  →  RTK 浮点解 (5)  →  单点 GPS  →  WiFi MLS  →  IP
   厘米级            分米级           米级        10-50m      城市/区域级
```

---

## 8. Trait 抽象建议（可选但推荐）

当前项目**全工作区无 trait 抽象**，各定位器均为具体 struct。建议趁此次扩展引入：

```rust
#[async_trait]
pub trait LocationSource: Send + Sync {
    async fn locate(&self) -> Result<GeoLocation>;
    fn priority(&self) -> u8;              // 数值越小优先级越高
    fn availability(&self) -> SourceAvailability;
}

// 实现
impl LocationSource for RtkLocator { ... }   // priority = 0
impl LocationSource for GpsLocator { ... }   // priority = 1
impl LocationSource for WifiLocator { ... }  // priority = 2
impl LocationSource for IpLocator { ... }    // priority = 3
```

编排逻辑改为 `Vec<Box<dyn LocationSource>>` 驱动，后续新增定位源（北斗短报文、蓝牙信标、UWB 等）无需改编排。

> 本项为可选重构，可在 RTK 模块稳定后单独进行。

---

## 9. 实施步骤

| 阶段 | 内容 | 前置条件 | 产出 |
|------|------|----------|------|
| **P0 硬件验证** | 采购 F9P 开发板，接电脑用 u-center 软件确认能输出 NMEA、能连千寻 CORS 拿固定解 | 采购到货 | 硬件可用性确认 |
| **P1 串口原型** | `rtk.rs` 实现串口读 + NMEA 解析（不接 NTRIP），验证能拿到单点坐标 | P0 | 单点定位可用 |
| **P2 NTRIP 接入** | 实现 NTRIP 客户端，拉差分流回灌接收机，验证 GGA 质量标识变成 `4` | P1 | 固定解可达 |
| **P3 数据结构** | 补 `GeoLocation` 字段 + 加 `Centimeter` 精度级 + `GpsFixQuality` 枚举 | 无（可与 P1 并行） | 类型系统就绪 |
| **P4 编排接入** | `RtkLocator` 接入 `collect_geo()`，RTK 优先 + 降级链 | P2、P3 | 端到端可用 |
| **P5 配置项** | `config.use_rtk` 开关，默认 false，不影响现有行为 | P4 | 向后兼容 |
| **P6 Trait 重构** | （可选）抽 `LocationSource` trait，四个定位器统一接口 | P4 稳定 | 架构改进 |
| **P7 测试** | 单元测试（NMEA 文件回放）+ 集成测试（真实硬件）+ 性能测试 | P4 | 测试全绿 |

---

## 10. 成本与风险

### 10.1 成本

| 项目 | 金额 | 说明 |
|------|------|------|
| RTK 接收机（F9P 开发板） | ¥800-1500 | 一次性 |
| 千寻 CORS 服务 | ~¥0.01/次 或包月 | 按用量 |
| 开发工时 | - | P0-P7 约 2-3 周 |

### 10.2 风险与对策

| 风险 | 影响 | 对策 |
|------|------|------|
| **室内无效** | RTK 需可见卫星，室内完全不可用 | 不替换现有源，RTK 失败自动降级到 WiFi/IP |
| **跨平台设备名** | Windows `COMx` / Linux `/dev/ttyACMx` / macOS `/dev/cu.usbmodem` | 实现自动探测，枚举可用串口 |
| **NTRIP 断线** | 网络波动导致差分中断，精度降级 | 断线重连 + 心跳检测，降级到单点 GPS |
| **固定解收敛慢** | 冷启动到固定解可能需 30-120 秒 | `locate_fast()` 接受浮点解，不阻塞主流程 |
| **隐私影响** | 厘米级坐标进 `to_hash_factor()` 可能导致身份哈希抖动 | RTK 结果降级到 H3 Res 12 再入哈希，避免厘米级抖动影响身份稳定性 |
| **接收机多样性** | 不同厂商 NMEA 语句可能有细微差异 | 用 `nmea-parser` 严格解析，异常语句降级处理 |

---

## 11. 后续可扩展方向

- **自建基准站**：部署固定 RTK 接收机，摆脱 CORS 依赖
- **北斗短报文**：集成短报文通信能力，支持无网络环境位置上报
- **PPP-RTK**：精密单点动态定位，无需附近基准站，全球覆盖（精度亚分米）
- **多源融合**：IMU + RTK + 视觉 SLAM 组合导航，覆盖室内外全场景
- **链上锚定**：RTK 固定解坐标 + 时间戳上链，作为 PoL（位置证明）的高精度证据

---

## 变更记录

| 日期 | 变更 |
|------|------|
| 2026-07-19 | 初始规划文档创建 |
