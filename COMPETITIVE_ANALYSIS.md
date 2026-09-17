# GeoYuan 竞品分析

> **最后更新**: 2026-07-14  
> **研究范围**: 国际上与 GeoYuan 类似的位置证明、地理铸币、去中心化地图项目

---

## 目录

1. [概述](#概述)
2. [位置证明 (Proof of Location) 类](#一位置证明-proof-of-location-类)
3. [去中心化地理数据 / DePIN 类](#二去中心化地理数据--depin-类)
4. [照片 / 位置铸币类](#三照片--位置铸币类最接近-geoyuan)
5. [地理位置游戏](#四地理位置游戏)
6. [GeoYuan 独创性分析](#geoyuan-独创性分析)
7. [结论](#结论)

---

## 概述

GeoYuan 是一个基于 GPS 照片的去中心化身份与经济系统，核心特征包括：

- **GPS 照片铸币**：一张带 GPS 的照片 = 一个 GyID = 一个 GeoYuan (GY)
- **GeoCast 地理路由**：基于 H3 六边形网格的 P2P 地理广播
- **PoL 位置证明**：每条消息可附加 GPS + Ed25519 签名的位置证明
- **完全去中心化**：libp2p 直连，无中心服务器，不依赖任何区块链

经调研，国际上存在多个相关项目，但**没有一个同时覆盖以上四个维度**。

---

## 一、位置证明 (Proof of Location) 类

### 1. FOAM

| 属性 | 详情 |
|------|------|
| **官网** | foam.space |
| **代币** | FOAM (ERC-20) |
| **链** | Ethereum |
| **状态** | 活跃 (2018 至今) |

**核心机制**：
- Crypto-Spatial Coordinates (CSC) — 以太坊上的开放地理位置标准
- Proof of Location — 通过地面无线电信标网络验证物理位置
- Spatial Index — 区块链上的空间数据可视化浏览器
- Token Curated Registry — 社区策划的地理兴趣点 (POI) 列表

**与 GeoYuan 的关系**：
- ✅ 相似：都有 Proof of Location 概念
- ❌ 不同：FOAM 用 RF 无线电信标，GeoYuan 用 GPS + Ed25519 签名
- ❌ 不同：FOAM 依赖以太坊，GeoYuan 是纯 P2P
- ❌ 不同：FOAM 无照片铸币，无独立身份系统

### 2. XYO Network

| 属性 | 详情 |
|------|------|
| **官网** | xyo.network |
| **代币** | XYO (ERC-20) |
| **链** | Ethereum / XYO Layer One |
| **状态** | 活跃 (2018 至今)，全球节点超百万 |

**核心机制**：
- Proof of Origin — 数据来源验证
- Proof of Location — 蓝牙节点位置签名
- DePIN (去中心化物理基础设施网络) 龙头项目
- 2025 年收购 GEO (集奥聚合)，深化加密定位技术

**与 GeoYuan 的关系**：
- ✅ 相似：都关注去中心化位置验证
- ❌ 不同：XYO 依赖专用蓝牙硬件节点，GeoYuan 用普通 PC
- ❌ 不同：XYO 聚焦位置数据市场，GeoYuan 聚焦身份 + 经济系统
- ❌ 不同：XYO 无照片铸币，无 H3 地理路由

### 3. Spacecoin (STI)

| 属性 | 详情 |
|------|------|
| **官网** | spacecoin.io |
| **代币** | — |
| **状态** | 开发中 (2025) |

**核心机制**：
- 专利区块链 PoL 系统
- PING-PONG 方法：节点间 RF 信号往返时间测距
- ECHO 信号共享测距结果
- "光球" (lightspheres) 三角定位，无需同步时钟

**与 GeoYuan 的关系**：
- ✅ 相似：都使用 Proof of Location
- ❌ 不同：Spacecoin 用 RF 信号三角定位，GeoYuan 用 GPS + 签名
- ❌ 不同：Spacecoin 无经济模型，GeoYuan 有照片铸币

---

## 二、去中心化地理数据 / DePIN 类

### 4. Helium

| 属性 | 详情 |
|------|------|
| **官网** | helium.com |
| **代币** | HNT / IOT / MOBILE (Solana SPL) |
| **链** | Solana (2023 年从自有链迁移) |
| **状态** | 活跃 (2019 至今)，日均用户超 230 万 |

**核心机制**：
- Proof of Coverage (PoC) — 热点间 RF 信号挑战验证覆盖
- Burn-and-Mint — HNT 销毁生成 Data Credits (USD 锚定)
- 三代币体系：HNT (主币) / IOT (物联网奖励) / MOBILE (5G 奖励)
- 2025 年第三次减半，年发行量从 1500 万降至 750 万 HNT

**与 GeoYuan 的关系**：
- ✅ 相似：都是去中心化网络，都有代币激励
- ❌ 不同：Helium 聚焦无线覆盖，GeoYuan 聚焦地理身份 + 铸币
- ❌ 不同：Helium 依赖 Solana 链，GeoYuan 是纯 P2P
- ❌ 不同：Helium 需要专用热点硬件 ($300-500)，GeoYuan 用普通 PC

### 5. Hivemapper

| 属性 | 详情 |
|------|------|
| **官网** | hivemapper.com |
| **代币** | HONEY (Solana SPL) |
| **链** | Solana |
| **状态** | 活跃 (2022 至今)，已覆盖 7 亿+ 公里，100+ 国家 |

**核心机制**：
- 行车记录仪 (Bee dashcam) 采集街景图像
- AI 处理图像生成地图数据（限速、路标、施工区）
- HONEY 代币奖励贡献者
- Burn-and-Mint 模式：客户购买 Map Credits，销毁 HONEY
- 隐私优先：边缘端自动模糊人脸/车牌

**与 GeoYuan 的关系**：
- ✅ 相似：最接近的"物理贡献换币"模型
- ❌ 不同：Hivemapper 用行车记录仪，GeoYuan 用 GPS 照片
- ❌ 不同：Hivemapper 聚焦地图数据，GeoYuan 聚焦身份 + 经济
- ❌ 不同：Hivemapper 依赖专用硬件 + Solana 链

---

## 三、照片 / 位置铸币类（最接近 GeoYuan）

### 6. SNPIT

| 属性 | 详情 |
|------|------|
| **官网** | snpit.com |
| **代币** | SNPT / STP |
| **链** | — |
| **状态** | 活跃 (日本，2024 至今) |

**核心机制**：
- Snap-to-Earn — 用相机 NFT 拍照赚取代币
- 相机 NFT 可升级，拍出更高品质照片赚更多
- 照片可参加"战斗"赢得 SNPIT Tokens
- "世界资料库"项目：带时间戳和位置的照片数据库
- Game-Fi + 生活方式应用结合

**与 GeoYuan 的关系**：
- ✅ **最接近的"照片铸币"项目**
- ❌ 不同：SNPIT 绑定相机 NFT，不验证 GPS 真实性
- ❌ 不同：SNPIT 无去中心化身份系统
- ❌ 不同：SNPIT 无 P2P 地理路由
- ❌ 不同：SNPIT 是 Game-Fi 娱乐导向，GeoYuan 是基础设施

### 7. Lost Worlds

| 属性 | 详情 |
|------|------|
| **官网** | lostworlds.io |
| **代币** | — |
| **链** | Polygon |
| **状态** | 活跃 (2022 至今) |

**核心机制**：
- 位置 NFT — 必须到指定 GPS 位置才能铸造
- Proof of Presence — GPS 验证物理到场
- 地理围栏 (geofencing) 限定铸造区域
- 应用：旅游活动、Web3 活动奖励、品牌地理投放

**与 GeoYuan 的关系**：
- ✅ 相似：都要求物理到场才能铸造
- ❌ 不同：Lost Worlds 铸造 NFT，GeoYuan 铸造代币 + 身份
- ❌ 不同：Lost Worlds 无 P2P 网络，依赖 Polygon 链
- ❌ 不同：Lost Worlds 无 H3 地理路由，无 PoL 签名

### 8. TravelMintNFT

| 属性 | 详情 |
|------|------|
| **官网** | github.com/Atmacxa/TravelMintNFT |
| **代币** | — (ETH 支付) |
| **链** | Ethereum / Base |
| **状态** | 开源项目 (2025) |

**核心机制**：
- 旅行照片 → NFT，锚定地理位置
- 地理位置永久不可变（即使所有权变更）
- 地图界面查看 NFT 位置
- 去中心化市场交易

**与 GeoYuan 的关系**：
- ✅ 相似：照片 + 地理位置锚定
- ❌ 不同：TravelMintNFT 铸造 NFT，无独立代币经济
- ❌ 不同：无 P2P 网络，无身份系统，无地理路由

### 9. StepN

| 属性 | 详情 |
|------|------|
| **官网** | stepn.com |
| **代币** | GMT / GST |
| **链** | Solana |
| **状态** | 活跃 (2022 至今) |

**核心机制**：
- Move-to-Earn — 步行/跑步 GPS 轨迹赚币
- 运动鞋 NFT 决定收益倍率
- GPS 验证物理运动
- 双代币：GMT (治理) + GST (游戏内)

**与 GeoYuan 的关系**：
- ✅ 相似：GPS 验证 + 代币奖励（类似 GeoYuan 规划中的"轨迹铸币"）
- ❌ 不同：StepN 聚焦运动，GeoYuan 聚焦地理身份 + 照片
- ❌ 不同：StepN 依赖 Solana 链 + NFT 经济

---

## 四、地理位置游戏

### 10. Upland

| 属性 | 详情 |
|------|------|
| **官网** | upland.me |
| **代币** | UPX (平台币) |
| **链** | EOS |
| **状态** | 活跃 (2020 至今) |

**核心机制**：
- 虚拟经济映射真实地址
- 购买/交易虚拟地产
- 地理位置游戏化

**与 GeoYuan 的关系**：
- ❌ 纯虚拟经济，无物理验证，无去中心化身份

### 11. GeoNFT

| 属性 | 详情 |
|------|------|
| **核心机制** | AR 寻宝 + 地理 NFT 收集 |

**与 GeoYuan 的关系**：
- ❌ 娱乐导向，无去中心化网络，无经济模型

---

## GeoYuan 独创性分析

### 独创性矩阵

| 项目 | PoL 位置证明 | 照片铸币 | 地理路由 (H3) | 去中心化身份 |
|------|:----------:|:-------:|:----------:|:---------:|
| FOAM | ✅ | ❌ | ❌ | ❌ |
| XYO Network | ✅ | ❌ | ❌ | ❌ |
| Spacecoin | ✅ | ❌ | ❌ | ❌ |
| Helium | ❌ (PoC) | ❌ | ❌ | ❌ |
| Hivemapper | ❌ | ❌ (行车仪) | ❌ | ❌ |
| SNPIT | ❌ | ✅ | ❌ | ❌ |
| Lost Worlds | ❌ | ❌ (位置 NFT) | ❌ | ❌ |
| TravelMintNFT | ❌ | ✅ | ❌ | ❌ |
| StepN | ❌ | ❌ (运动) | ❌ | ❌ |
| Upland | ❌ | ❌ | ❌ | ❌ |
| **GeoYuan** | **✅** | **✅** | **✅** | **✅** |

> GeoYuan 是唯一同时覆盖四个维度的项目。

### GeoYuan 独有的 4 项创新

| # | 创新点 | 说明 | 竞品现状 |
|---|--------|------|---------|
| 1 | **GeoCast H3 地理路由** | P2P 消息按 H3 六边形网格邻近性过滤，非全网广播 | 无项目使用 H3 地理路由 |
| 2 | **GPS 照片 + Ed25519 铸币** | 照片 EXIF GPS + 时间戳 + Ed25519 签名 → 不可伪造的 GY | SNPIT 最接近但无 GPS 绑定 |
| 3 | **完全去中心化 P2P** | libp2p 直连，无中心服务器，不依赖任何区块链 | FOAM/Helium 依赖以太坊/Solana |
| 4 | **GyID 地理身份系统** | 硬件指纹 + GPS + 时间戳 + 头像哈希的多维身份 | 无项目有独立身份层 |

### 关键维度对比

| 维度 | GeoYuan | 最接近的竞品 |
|------|---------|-------------|
| **铸币触发** | GPS 照片 (EXIF + 签名) | SNPIT: 相机拍照 (无 GPS 验证) |
| **位置验证** | Ed25519 GPS 签名 (PoL) | FOAM: RF 无线电信标 |
| **网络架构** | libp2p 纯 P2P (无链) | Helium: Solana 链上 |
| **硬件要求** | 普通 PC (≤500MB) | XYO: 专用蓝牙节点; Hivemapper: 专用行车仪 |
| **身份层** | GyID 独立系统 | 无项目有独立身份层 |
| **路由协议** | GeoCast (H3 Res 12) | 无项目使用地理路由 |
| **数据存储** | redb 本地数据库 | FOAM: 以太坊链上; Hivemapper: Solana |

---

## 结论

GeoYuan 在"照片 GPS 铸币 + 地理感知 P2P 路由 + 去中心化身份"三者的结合上是**国际上独有的**。

最接近的单项竞品：

| 维度 | 最接近竞品 | GeoYuan 的差异 |
|------|-----------|---------------|
| 照片铸币 | SNPIT (日本) | GeoYuan 绑定 GPS + Ed25519 签名，SNPIT 无 GPS 验证 |
| 位置证明 | FOAM (以太坊) | GeoYuan 用 GPS 签名，FOAM 用 RF 信标；GeoYuan 纯 P2P，FOAM 依赖以太坊 |
| 地理贡献换币 | Hivemapper (Solana) | GeoYuan 用照片，Hivemapper 用行车仪；GeoYuan 纯 P2P，Hivemapper 依赖 Solana |
| 运动赚币 | StepN (Solana) | GeoYuan 规划中的轨迹铸币与此类似，但 GeoYuan 还有照片铸币 + 身份系统 |

**没有任何一个国际项目同时具备**：
1. GPS 照片铸币
2. H3 地理感知 P2P 路由 (GeoCast)
3. Ed25519 位置证明 (PoL)
4. 独立去中心化身份 (GyID)

这正是 GeoYuan 的核心竞争壁垒。

---

*调研日期: 2026-07-14*  
*数据来源: 项目官网、白皮书、WebSearch 公开信息*
