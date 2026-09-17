# GeoYuan 开发状态总览

> **最后更新**: 2026-07-29  
> **基于**: `DEVELOPMENT_PLAN.md` 8 阶段计划 + 实际代码盘点

---

## ✅ 已完成

| 模块 | 内容 | 测试 |
|------|------|------|
| **geoyuan-core** | 9 个模块（identity / crypto / geo / photo / wallet / storage / consensus / p2p / chain）| 136 全绿 |
| **geoyuan-cli** | 5 组命令（identity / wallet / consensus / storage / p2p）| 编译通过 |
| **geoyuan-gui** | Slint GUI + 成就 / 能量 / 转账 / 设备关联 | 编译通过 |
| **GPv1 协议** | Ed25519 验签 + H3 GeoCast + PoL 位置证明 | 15 个新测试 |
| **P2P 测试与部署** | 自动化脚本 + systemd + NGINX + 运维文档 | 3 个 native 测试 |
| **多方式铸币引擎** | 轨迹铸币 / 地理发现 / PoL 节点 / 双人验证 / 签到链 | 24 个新测试 |
| **文档** | 白皮书 (`docs/WHITEPAPER.md`) + 运维指南 + 官网 | — |

### 已完成详情

#### geoyuan-core (136 测试全绿)
- **identity**: GeoID 生成器、验证器
- **crypto**: Ed25519 签名、BLAKE3 哈希、Base58 编码
- **geo**: 高德 SDK、Geohash、H3 六边形网格 (Res 12)、逆地理编码
- **photo**: EXIF 解析、SHA256 哈希
- **wallet**: GeoCoin 结构、铸造、转账、**多方式铸币引擎**
- **storage**: redb 本地数据库
- **consensus**: PoI 共识、VRF
- **p2p**: libp2p Swarm、mDNS + Kademlia、GeoCast 地理广播、PoL 位置证明
- **chain**: 交易结构、Ed25519 验签

#### 多方式铸币引擎 (`wallet/mint_engine.rs`)
| 方式 | 结构体 | 机制 | 奖励 |
|------|--------|------|------|
| 📸 照片铸币 | `CoinMinter` | GPS 照片 → 铸造 | +1 GY（✅ 已有）|
| 🚶 轨迹铸币 | `TrackMinter` | 连续 GPS 轨迹（步行 >1km / 骑行 >3km）| +0.1~0.5 GY |
| 🏆 地理发现 | `DiscoveryMinter` | 首次在某 H3 cell (Res 12) 铸造 | +2 GY 创世奖励 |
| 🏪 PoL 节点奖励 | `PolNodeMinter` | 节点在线 >8h + GPS 稳定 | +0.5 GY / 日 |
| 🤝 双人验证 | `DualVerificationMinter` | 同 H3 cell 两人同时铸造（5 分钟内）| 各 +0.5 GY |
| 🔄 签到链 | `CheckinChainMinter` | 连续 7 天在同一城市签到 | +1 GY |

**Geoyuan 结构扩展**:
- 新增 `mint_type: MintType` 字段（6 种铸币类型）
- 新增 `value_milli_gy: u64` 字段（毫 GY 单位，1000 = 1 GY）
- `Wallet` 新增 `total_value_gy()` / `count_by_type()` 方法
- 向后兼容：`#[serde(default)]` 确保旧数据可正常反序列化

#### geoyuan-cli
- `identity` — GyID 身份操作
- `wallet` — 钱包与 GeoYuan 操作
- `consensus` — PoI 共识状态
- `storage` — 本地存储操作
- `p2p` — P2P 节点管理（start / status / peers / peers-nearby）

#### geoyuan-gui (Slint)
- 主页、生成页、设备页、设置页、导入页、历史记录页
- 转账页（真实签名 + P2P 广播）
- 钱包页（余额 + 交易历史）
- 成就 / 能量系统（铸造 / 转账时触发）
- 设备关联页（授权码 + 关联列表）

#### GPv1 协议 (Geospatial Proof Protocol v1)
- **Ed25519 真实验签**: `Transaction::verify_signature()` 使用 Ed25519
- **H3 GeoCast 地理广播**: Res 12 (~9m 边长)，接收端 H3 邻近过滤
- **PoL 位置证明**: GPS + 时间戳 + H3 cell + Ed25519 签名

#### P2P 测试与部署
- `tools/p2p-test.ps1` + `tools/p2p-test.sh` — 自动化多节点测试
- `deploy/geoyuan-p2p.service` — systemd 服务配置
- `deploy/nginx-geoyuan-p2p.conf` — NGINX WSS 反向代理
- `geoyuan-web/deploy-p2p.sh` — 一键部署脚本
- `docs/P2P_OPERATIONS.md` — 运维指南

---

## ❌ 未完成

### 🔴 高优先级 — 核心功能缺失

| # | 任务 | 说明 | 对应阶段 |
|---|------|------|---------|
| 1 | **服务器部署** | 脚本就绪，待 `<SERVER_IP>` SSH 恢复后执行 `deploy-p2p.sh` | P2P |
| 2 | **IPFS 锚定** | `DEVELOPMENT_PLAN.md` 3.5 提到，geoyuan-core 无 ipfs 模块 | 第三阶段 |
| 3 | **铸币引擎 CLI 集成** | 将多方式铸币引擎接入 `geoyuan-cli wallet` 命令 | 新增 |

#### 1. 服务器部署

```bash
# SSH 恢复后一键部署
bash geoyuan-web/deploy-p2p.sh --server-build
```

#### 2. IPFS 锚定

- `DEVELOPMENT_PLAN.md` 任务 3.5 提到 IPFS 元数据锚定
- geoyuan-core 当前无 `ipfs` 模块
- 需实现: 照片哈希上 IPFS、元数据不可篡改锚定

#### 3. 铸币引擎 CLI 集成

`geoyuan-core` 已实现多方式铸币引擎（`wallet/mint_engine.rs`），但 `geoyuan-cli` 尚未接入：
- `wallet mint track` — 轨迹铸币
- `wallet mint discovery` — 地理发现奖励
- `wallet mint pol` — PoL 节点奖励
- `wallet mint dual` — 双人验证
- `wallet mint checkin` — 签到链

---

### 🟡 中优先级 — 平台扩展

| # | 任务 | 说明 | 对应阶段 |
|---|------|------|---------|
| 4 | **geoyuan-sdk** | 跨平台 SDK（WASM / Android / iOS / C FFI）— workspace 中不存在 | 第七阶段 |
| 5 | **Android APP** | Jetpack Compose UI + 高德 SDK + APK | 第四阶段 |
| 6 | **HarmonyOS APP** | ArkUI + 高德鸿蒙 SDK + HAP | 第五阶段 |
| 7 | **iOS APP** | SwiftUI + 高德 iOS SDK + IPA | 第六阶段 |

#### 4. geoyuan-sdk (跨平台 SDK)

当前 workspace 仅有 `gyid-sdk`（GyID 身份系统的 SDK），缺少 `geoyuan-sdk`（GeoYuan 经济系统的 SDK）。

需要实现:
- WASM 绑定（Web 前端集成）
- Android Kotlin 绑定（JNI）
- iOS Swift 绑定（C FFI）
- C ABI 头文件
- SDK 文档

#### 5-7. 移动端 APP

| 平台 | UI 框架 | 地图 SDK | 语言 | 交付物 |
|------|---------|---------|------|--------|
| Android | Jetpack Compose | AMap Location | Kotlin | APK |
| HarmonyOS | ArkUI | 高德定位 SDK | ArkTS | HAP |
| iOS | SwiftUI | AMap iOS SDK | Swift | IPA |

---

### 🟢 低优先级 — 优化与完善

| # | 任务 | 说明 | 对应阶段 |
|---|------|------|---------|
| 8 | **安全审计** | 代码安全审查 | 第八阶段 8.3 |
| 9 | **GPv2 路线图** | 发送端地理路由 / PoL 时间窗口验证 / 节点声誉系统 | 远期 |
| 10 | **GUI dead_code 清理** | geoyuan-gui 有 24 处 `#[allow(dead_code)]` 未接入 UI | 优化 |

#### 8. 安全审计
- 交易签名安全性审查
- P2P 协议抗攻击分析
- 隐私泄露风险评估

#### 9. GPv2 路线图

| 功能 | 说明 |
|------|------|
| 发送端地理路由 | Kademlia 地理感知 DHT，实现发送端过滤 |
| PoL 时间窗口验证 | 接收端检查时间偏差（建议 < 5 分钟）|
| 节点声誉系统 | 基于历史行为的信任评分 |
| 零知识位置证明 (zkPoL) | 不暴露精确位置的位置证明（GPv3）|

#### 10. GUI dead_code 清理

geoyuan-gui 中以下文件有未接入 UI 的代码:
- `photo.rs` (2 处)
- `crypto.rs` (2 处)
- `node.rs` (5 处)
- `app.rs` (8 处)
- `location.rs` (7 处)

---

## 里程碑对照

| 里程碑 | 目标 | 状态 |
|--------|------|------|
| M1 | geoyuan-core 核心库 | ✅ 完成 |
| M2 | geoyuan-gui Windows 应用 | ✅ 完成 |
| M3 | P2P 分布式网络 | ✅ 完成 |
| M4 | Android 应用与 SDK | ❌ 未开始 |
| M5 | HarmonyOS 应用与 SDK | ❌ 未开始 |
| M6 | iOS 应用与 SDK | ❌ 未开始 |
| M7 | 全平台 SDK | ❌ 未开始 |
| M8 | 正式版 1.0 Release | ❌ 未开始 |

---

## 建议开发顺序

1. **铸币引擎 CLI 集成** — 让多方式铸币可通过命令行使用
2. **服务器部署** — SSH 恢复后一键执行
3. **IPFS 锚定** — 数据不可篡改性保障
4. **geoyuan-sdk** — 为移动端做准备
5. **Android APP** — 最大用户群体
6. **安全审计** — 发布前必做
7. **HarmonyOS / iOS** — 平台覆盖

---

*GeoYuan Team · 2026*
