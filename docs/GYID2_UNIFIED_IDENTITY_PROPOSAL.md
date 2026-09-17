# GyID 2.0 统一身份方案建议

> 副标题：让 GYIDpro（Rust 节点/SDK）与 geoyuan.com（地理元 diliy 线上产品）统一为
> **「基于地理位置 + 个人数据的、不可撤销的、真实可信的去中心化永久网络 ID」**
>
> 撰写日期：2026-09-16 · 状态：讨论稿 v2（§8 三个闸门已确认）· 适用仓库：GYIDpro + diliy

---

## 0. 一页纸结论

1. **现状**：GYIDpro 里其实有两套互相重复的身份实现（`gyid-core` 硬件指纹派与 `geoyuan-core` GPS 照片派）；线上产品 geoyuan.com（`~/myweb/diliy`）已经在 Base 主网做 ERC-721 卡片锚定 + IPFS + payload_hash 存证，但**两者在代码层目前零集成**，只有品牌同源。

2. **必须先纠正一个技术现实**：今天的 GyID 字符串**不是永久 ID**。生成算法里含有随机盐和毫秒时间戳，每生成一次就变一个；GPS/EXIF/硬件指纹也都可被伪造。因此"不可撤销、完全真实可信、永久"这三个目标，靠现有一次性哈希方案**在数学上不成立**，必须换成 **W3C DID（永久身份根）+ 可验证凭证（真实性靠证据累积）+ 双链锚定（永久性）** 的标准模型。

3. **统一目标架构 GyID 2.0**：
   - 身份根 = 用户自持的 **Ed25519 密钥对**（`geoyuan-core` 已有），标识为 `did:geoyuan:<base58>`，注册记录锚定 Base 主网；
   - 每张地理元卡片、每次 GPS 拍照、每台设备、每次社交共址、每个账户实名，都是挂在 DID 下的一张**签名凭证（VC）**；
   - "真实可信"用**凭证密度 + 多源交叉验证**做可信度分级，而不是宣称单一信号不可伪造；
   - "永久"靠 Base 链上注册 + IPFS 元数据 + 用户自持密钥 + P2P 节点个人副本共同保证；
   - "不可撤销"限定为**身份根不可删除**，凭证仍可作废、账户仍可解绑、内容仍可删除（这是《个人信息保护法》第 47 条的硬要求，见 §7）。

4. **整合成本低**：diliy 已有 anchor-worker / payload_hash / IPFS / 平台代持 / 微信登录 / E2EE / geo-proxy / 足迹系统的完整管道；GYIDpro 已有 WASM SDK、Ed25519、H3、PoL、设备树。新增的核心只有三块：**浏览器端 DID 密钥生成与注册、卡片 payload 增加 `owner_did` + 签名、一个 DID 注册合约（或注册表）**。

5. **全球坐标**：这个赛道里 World ID（虹膜）、Humanity Protocol（掌纹）解决"唯一真人"但无地理与个人数据；FOAM/XYO 解决位置但无身份层；最接近的学术方案是 **IETF 正在制定的 TRIP 协议（轨迹→持久假名身份 TIT，2026-05 第 04 版草案）**。**"地理存在史 + 个人数据卡片 + 社交共址"三者累积成 DID，并与一个已上线合规产品结合，目前世界范围内没有看到同构项目**——这是 GeoYuan 的空位。

---

## 1. 现状盘点：你手里到底有什么

### 1.1 GYIDpro（Rust workspace，CodeBuddy 时期产物）

| 模块 | 身份观 | 输入因子 | 持久化 | 成熟度 |
|---|---|---|---|---|
| [gyid-core](file:///home/huanghe/myweb/GYIDpro/gyid-core/src/identity/generator.rs) | **硬件+环境派** | MAC/CPU/主板/磁盘指纹 + IP/WiFi/GPS(H3) + 毫秒时间戳+随机数 + 头像哈希 | 本地 SQLite；Polygon/Aptos 锚定哈希 | v0.4，SDK 覆盖 WASM/Android/iOS/C |
| [geoyuan-core](file:///home/huanghe/myweb/GYIDpro/geoyuan-core/src/identity/generator.rs) | **GPS 照片派** | 照片 SHA-256（含 EXIF GPS）+ 经纬度 + 拍摄时间 + 盐，高德纠偏 | 本地 redb；libp2p P2P 同步 | 136 测试，含钱包/铸币/PoI/PoL |

配套资产：

- **GPv1 P2P 协议**（[WHITEPAPER.md](file:///home/huanghe/myweb/GYIDpro/docs/WHITEPAPER.md)）：libp2p + H3 Res12 的 GeoCast 地理广播 + Ed25519 签名的 PoL 位置证明；
- **多方式铸币引擎**：照片/轨迹/地理发现/节点在线/双人验证/签到链；
- **gyid-sdk**：WASM（可直接跑在浏览器）、Android、iOS、C FFI；
- **WebButton**（[tools/webbutton](file:///home/huanghe/myweb/GYIDpro/tools/webbutton/z/index.html)）：可嵌入任意网页的 GyID 生成弹窗，已按 GEOYUAN 视觉规范实现；
- **官网** gyid.geoyuan.com（Astro）。

### 1.2 geoyuan.com = 地理元（`~/myweb/diliy`，线上在运营）

- 产品：绑定地理坐标的卡片社交（"Card Your Life"），Astro + React + Supabase，部署于 `8.136.127.182`（[README](file:///home/huanghe/myweb/diliy/README.md)）。
- 上链：Base 主网 ERC-721 [DiliyCard.sol](file:///home/huanghe/myweb/diliy/contracts/DiliyCard.sol)，平台钱包代持（国内数字藏品合规模式），`tokenId = keccak256(card.id)`，tokenURI 走 IPFS。
- 锚定管道：`anchor-card` 入队 → pg_cron `anchor-worker` 逐张 mint → `verify-anchor` 三方比对；卡片核心载荷已做哈希承诺：
  `payload_hash = keccak256(card_id, card_number, owner_id, nickname, school, gps, created_at, media_hash)`
  （[anchor-worker/index.ts#L100-L122](file:///home/huanghe/myweb/diliy/supabase/functions/anchor-worker/index.ts#L100-L122)）
- 已有周边能力：EXIF 铸卡、高德 geo-proxy、足迹打卡（连续签到/地点去重，[footprint.ts](file:///home/huanghe/myweb/diliy/src/lib/footprint.ts)）、E2EE 聊天、钱包绑定、卡片配对（双人社交关系）、ProofSeal/ChainFeed 链上时间线、x402 支付试点。
- GYU = 微信充值的**站内法币积分**（1 元=1 GYU，不发币、不提现，已有明确合规决策）。
- 既有战略文档 [上链完备性_去中心化_盈利方案.md](file:///home/huanghe/myweb/diliy/.trae/documents/上链完备性_去中心化_盈利方案.md) 已确定：Base + 合规联盟链双轨、IPFS、资产导出包、自助验证工具。

### 1.3 两者关系

品牌同源（gyid.geoyuan.com 子域），**代码层无任何集成**：diliy 全仓没有 DID/GyID 身份字段（已 grep 确认），Rust 节点也不认识 diliy 账户。两套 Rust core 之间同样互不引用。

---

## 2. 问题诊断：为什么现在的 GyID 还不是"永久可信 ID"

诚实地面对这六个问题，是统一方案成立的前提。

| # | 问题 | 证据 | 后果 |
|---|---|---|---|
| P1 | **GyID 每次生成都不一样** | [gyid-core generator#L497-L503](file:///home/huanghe/myweb/GYIDpro/gyid-core/src/identity/generator.rs#L497-L503) 与 [geoyuan-core generator#L130-L136](file:///home/huanghe/myweb/GYIDpro/geoyuan-core/src/identity/generator.rs#L130-L136) 都把"时间戳+随机盐"喂进哈希 | 它是**一次性实例编号**，不是可复现、可登录、可累积信用的永久身份 |
| P2 | **硬件指纹不稳定也不唯一** | MAC 可改、虚拟机可伪造、设备可更换；WASM 端只能拿 UA/Canvas 指纹 | 不能单独充当"真实性根基"，只能作为弱凭证 |
| P3 | **GPS 与 EXIF 可伪造** | 白皮书 §13.5 自己承认 PoL 仅证明"持私钥者声称在该处"；EXIF 任意可写、虚拟定位软件普遍 | "照片证明人在那里"在没有传感器可信度与交叉验证时不成立 |
| P4 | **永久性没有持久层兜底** | geoyuan-core 只写本机 redb、P2P 节点互不信任、无共识最终性；gyid-core 只往 Polygon 发一笔 hash 交易；diliy NFT 挂在**平台钱包**名下 | 关站/换机/重装即可能丢；平台理论上可处分代持资产 |
| P5 | **两套算法、两套编码、两套世界观** | 一个 Base58 截 32 字符、一个截 22 字符；一个硬件权重模型、一个照片模型；CLI 包名经历 gyid-cli→geoyuan-cli 混乱（见 [TODO_2026-07-15.md](file:///home/huanghe/myweb/GYIDpro/TODO_2026-07-15.md)） | 无法互操作，无法形成标准，第三方无法接入 |
| P6 | **"不可注销"与法律冲突** | 《个人信息保护法》第 47 条、《网络数据安全管理条例》均赋予注销/删除权 | 永久的只能是"身份根的存在性事实"，不能是平台上的账户与内容 |

**结论**：目标不变，但实现哲学要换——

> 永久，靠"密钥 + 公链锚定"；真实，靠"多源签名凭证的累积"；不可撤销，靠"身份根与账户/凭证分层"。

---

## 3. 目标架构：GyID 2.0 = did:geoyuan + 凭证累积 + 双链锚定

### 3.1 分层模型

```
┌───────────────────────────────────────────────────────────────┐
│  应用层    geoyuan.com 卡片社交 │ Rust GUI 全节点 │ WebButton  │
├───────────────────────────────────────────────────────────────┤
│  凭证层 VC 设备凭证 │ 地理存在凭证(照片/H3) │ 账户凭证(手机/微信) │
│            社交共址凭证 │ 轨迹凭证 │ 真人凭证(可选 World ID 等) │
├───────────────────────────────────────────────────────────────┤
│  身份层    did:geoyuan:<base58>  +  DID Document（公钥集/服务端）│
│            Ed25519（已有 signer.rs）│ 设备树/密钥轮换/社交恢复    │
├───────────────────────────────────────────────────────────────┤
│  锚定层    Base 主网（DID 注册 + 卡片 NFT/IPFS）               │
│            联盟链存证（星火/BSN，二期，合规）                   │
│            IPFS（照片+完整元数据）                             │
├───────────────────────────────────────────────────────────────┤
│  传输层    GPv1 libp2p（GeoCast/PoL）＝ 个人数据副本与凭证交换  │
│            HTTPS/Supabase（线上产品的日常通道，渐进去中心化）    │
└───────────────────────────────────────────────────────────────┘
```

### 3.2 身份根：从"算出来的字符串"变成"持有的密钥"

- 用户首次使用时（网页/APP/桌面端）生成 **Ed25519 密钥对**：
  - 网页端直接复用 [gyid-sdk 的 WASM 目标](file:///home/huanghe/myweb/GYIDpro/gyid-sdk/src/ffi/wasm.rs)；
  - 桌面端复用 [geoyuan-core 的 crypto/signer.rs](file:///home/huanghe/myweb/GYIDpro/geoyuan-core/src/crypto/signer.rs)；
  - 私钥永不出设备，助记词/多设备关联（复用现有 [device/master.rs](file:///home/huanghe/myweb/GYIDpro/gyid-core/src/device/master.rs) 授权码协议）负责恢复。
- 标识：`did:geoyuan:z8ia…`（Base58 公钥指纹；兼容 W3C DID Core，未来可解析为标准 DID Document）。
- 现有 `GyID…` 字符串**保留为品牌名和展示名**，但语义上变成"对 DID 的可读展示 + 初代兼容"，不再承担唯一性与永久性职责。
- DID Document（公钥列表、认证方法、P2P 服务端点、IPFS 资料指针）首次注册时锚定链上；换设备=**加新公钥**而非换新身份。

### 3.3 真实可信 = 凭证矩阵 + 可信度分级

不再声称某个信号"不可伪造"，而是让每个信号成为一张**由发布者签名、可独立验证、可累积**的凭证：

| 凭证 | 签发者 | 内容（哈希上链/IPFS） | 抗伪强度 | 复用的现有代码 |
|---|---|---|---|---|
| 设备凭证 | 本机 | 硬件指纹 + 设备公钥签名，绑定到 DID | ★★ | gyid-core fingerprint |
| 地理存在凭证 | 本人签名 + 平台/节点佐证 | 照片哈希 + EXIF GPS + 拍摄时间 + H3 cell + PoL | ★★★（叠加佐证） | geoyuan-core photo/PoL、diliy submitMint |
| 轨迹凭证 | 本机 | GPS 序列 + H3 链式签名（每日一结） | ★★★ | mint_engine TrackMinter、footprint |
| 共址凭证 | 两用户互签 | 同 H3 cell、5 分钟内互签 | ★★★★ | DualVerificationMinter、diliy pairing |
| 账户凭证 | geoyuan.com | 手机号/微信/学校邮箱实名等级（平台签名，不公开明文） | ★★★ | Supabase Auth、schoolDomain |
| 真人凭证（可选） | 外部 | 接入 World ID / Humanity Protocol 的 ZK 证明，只存"通过"结论 | ★★★★★ | 未来集成，不依赖 |

可信度分级建议：`L0 仅密钥` → `L1 +账户凭证` → `L2 +N 张地理凭证` → `L3 +共址/轨迹一致性` → `L4 +外部真人凭证`。
**反作弊靠"凭证密度与一致性"**：H3 历史网格去重（diliy 已有 ~110m 网格 key）、EXIF 完整性、传感器来源标识、高德服务端定位交叉（已有 geo-proxy Edge Function）、双人共址互证；造假单点容易、长期伪造一整套高相关地理人生的成本极高。这与 IETF TRIP 草案用"轨迹物理统计特征（PSD 1/f 噪声、Hamiltonian 行为评分）"识别人类移动是同一思想（见 §5.3）。

### 3.4 永久性设计：四处冗余，关站不丢

1. **Base 主网**：DID 注册记录 + 卡片 NFT（已在跑）——存在性永久、全球可查；
2. **IPFS**：照片与完整元数据 JSON（diliy 方案中 P0 待办，应尽快做）；
3. **用户自持**：私钥/助记词 + **资产导出包**（卡片 JSON、照片、payload_hash、tx_hash，diliy 方案已规划）；
4. **P2P 个人副本**：Rust 节点（redb）保存本人凭证与加密个人数据，GPv1 负责节点间交换与备份，GeoCast/PoL 原样保留——但**把它从"链"重新定位为"凭证交换与个人数据网络"**，不要求它承担共识最终性（PoI 自研链/VRF/联邦分片等宏大设计降为远期研究，避免过早投入，见 §6）。

### 3.5 "不可撤销"的正确切分（合规可落地）

| 对象 | 是否可撤销 | 理由 |
|---|---|---|
| DID 身份根（链上注册记录） | **不可删除**，公钥可轮换 | "永久 ID"的核心承诺 |
| 单张凭证 | 可声明作废（revocation） | 设备丢失、误发 |
| geoyuan.com 账户与内容 | **必须可注销/可删除** | PIPL 第 47 条；注销=账户凭证解绑，链上只留"曾存在"的哈希事实 |
| 代持 NFT | 平台不得任意处分，规则透明；中期开放用户自持 | 数字藏品监管 + 资产安全 |

---

## 4. 与 geoyuan.com 的具体整合方案（工程级）

目标：**用户在 geoyuan.com 铸第一张卡时，顺手诞生一个永久 GyID（DID）；此后所有卡片都挂在该 DID 下，并反哺桌面节点。**

### 4.1 新用户铸卡流程（统一后）

```
浏览器（WASM gyid-sdk，经 WebButton 承载）
  ① 生成 Ed25519 密钥 → DID；私钥存 IndexedDB/助记词备份
  ② 选照片 → 本地算 照片BLAKE3/SHA256 + 读 EXIF GPS + H3 cell
  ③ 用 DID 私钥对 canonical payload 签名
  ④ 提交 {card 字段, owner_did, did_pubkey, owner_sig} 到 Supabase
anchor-card / anchor-worker（复用现有管道）
  ⑤ payload_hash 规范中加入 owner_did（见 4.2），先验 owner_sig
  ⑥ Base：注册/更新 DID（首卡时）+ mint DiliyCard（tokenURI=ipfs://…）
  ⑦ 回写 tx_hash / did；卡片成为该 DID 下第一张「地理存在凭证」
用户侧
  ⑧ 桌面 Rust 节点用同一 DID 扫码/签名登录（SIWG），拉取自己的链上凭证与 IPFS 数据
```

### 4.2 代码改动清单（按仓库）

**diliy（geoyuan.com）**

1. `cards` 表迁移：新增 `owner_did text`、`did_sig text`、`did_registered_at`、`credential_json jsonb`（索引 `owner_did`）；存量卡回填 `owner_did = null`（保持旧 payload_hash 不变，沿用 media_hash 同款兼容策略）。
2. [anchor-worker buildCanonicalPayload](file:///home/huanghe/myweb/diliy/supabase/functions/anchor-worker/index.ts#L100-L117)：payload 增加 `owner_did`（null 不影响旧卡重算）；mint 前用 did_pubkey 验 `owner_sig`。
3. 新增 **DID 注册合约**（建议极简：`mapping(did=>DIDRecord{pubkey,created_at,updated_tx})`，或干脆复用一个不可升级的 `GeoyuanID` 注册表，不要给 owner 留注销/篡改接口；合约由平台钱包部署，与 [DiliyCard.sol](file:///home/huanghe/myweb/diliy/contracts/DiliyCard.sol) 同一套 Key 管理）。**不需要发币**，注册调用由 anchor-worker 代付 gas。
4. 前端：
   - 引入 `@gyid/sdk-web`（现成 WASM）做密钥生成/签名，UI 直接收编 [WebButton](file:///home/huanghe/myweb/GYIDpro/tools/webbutton/z/index.html) 弹窗；
   - [MintPage](file:///home/huanghe/myweb/diliy/src/components/mint/MintPage.tsx) 提交时携带 DID 与签名；
   - 新增"DID 资料卡"页：公钥、凭证列表、可信度等级、链上凭证链接、资产导出包下载（自助验证单文件 HTML 托管 IPFS，diliy 既有规划）；
   - [ProofSeal](file:///home/huanghe/myweb/diliy/src/components/ProofSeal.tsx) 增加 DID 与验签展示。
5. 登录：新增 **Sign-In With Geoyuan（SIWG）**——DID 挑战码签名登录（[eip1193.ts](file:///home/huanghe/myweb/diliy/src/lib/eip1193.ts) 的钱包模式可直接借鉴），让桌面节点/第三方用 GyID 登录网站，微信登录并存。
6. IPFS：铸卡即 pin 图片与完整 metadata（P0 缺口，不做则"永久"不成立）。

**GYIDpro（Rust）**

1. **合并两套 core**：以 `geoyuan-core` 为主体（已有 Ed25519/H3/photo/p2p），把 `gyid-core` 的 fingerprint / device tree / SDK 平台层并入，废弃带随机盐的旧式"身份哈希"，改为确定性的 **DID = f(公钥)**；照片/硬件只进入凭证。
2. 新增 `identity/did.rs`：DID 生成/解析、DID Document（CBOR/JSON）、注册/解析（读 Base RPC）、`verify_owner_sig`。
3. 新增 `credentials/` 模块：DeviceCredential / GeoPresenceCredential / CoLocationCredential 结构、签发、验证、撤销；PoL 升级为 GeoPresence 凭证的签名格式（现有字段几乎可直接复用）。
4. geoyuan-cli/gui：`geoyuan did new/resolve/export`、`geoyuan login geoyuan.com`（SIWG）、`geoyuan vc list/verify`。
5. SDK：WASM API 对齐网站流程（generate_did / sign_payload / export_mnemonic），Android/iOS 后续跟进（也是 DEVELOPMENT_STATUS 中明确缺口）。

### 4.3 GY 与 GYU 的关系（必须讲清，避免合规事故）

- **GYU**：维持现状——法币 1:1 站内积分，不上链、不发币（已有决策，继续遵守）。
- **GY（P2P 节点里的"币"）**：**不要按代币对外宣发**。在统一架构里把它重新定位为**链下"脚印（Foot mark）"**——用户在真实地点留下的可验证足迹记录（与可信度等级、成就系统挂钩，类似 diliy 已有的签到积分/能量 UI），不与法币兑换、不公开发行。未来若要代币化，需要在有明确合规路径（海外主体+当地牌照）时单独立项，与国内产品物理隔离。

---

## 5. 世界范围内的类似方案（2026 年 9 月调研）

### 5.1 去中心化身份标准与基础设施

| 方案 | 做什么 | 与 GyID 2.0 的关系 |
|---|---|---|
| **W3C DID Core + Verifiable Credentials 2.0** | 全球身份互操作标准：DID 标识 + 可验证凭证 + 展示证明 | **直接遵循**，did:geoyuan 按此设计，保证未来可被任意钱包/验证方解析 |
| **DIF（去中心化身份基金会）/ Sidetree / ION** | Layer-2 式 DID 锚定（ION 锚定比特币，无需发币） | 注册表设计可借鉴；ION 证明了"不发币也能做公链 DID" |
| **did:key / did:pkh / did:web** | 纯公钥型 / 公钥哈希型 / 域名锚定型 DID | did:geoyuan 的编码与解析可对齐这些成熟 method |
| **Privado ID（原 Polygon ID）** | 基于 ZK 的链上身份与选择性披露凭证 | 未来"证明在某城市而不暴露坐标"的 zkPoL 可参考 |
| **Ceramic / ComposeDB** | 去中心化数据流与可组合身份（2022–2024 明星项目，之后商业化明显收缩、生态沉寂） | **教训**：身份层不能押注单一初创公司的网络，锚定要选成熟公链+标准 |
| ENS | 人读名称层 | GyID 展示名可远期接 ENS 式命名，非核心 |

### 5.2 "证明你是真人"（Proof of Personhood）赛道

| 方案 | 唯一性根基 | 规模（2026） | 局限 |
|---|---|---|---|
| **World ID 4.0**（[world.org](https://world.org/blog/announcements/world-id-full-stack-proof-of-human)，2026-04 升级） | Orb 虹膜硬件 + AMPC/OPRF(TACEO) + ZK，一次性 nullifier 防关联 | 160 国、约 1800 万 Orb 验证用户；主打 AI Agent 时代"真人验证"，已接 Zoom/DocuSign | 专用硬件、生物识别隐私争议（多国监管动作）、**无地理位置、无个人数据沉淀** |
| **Humanity Protocol** | 手机端本地掌纹/掌脉 + ZK，Polygon CDK zkEVM L2，发 DID+VC | 900 万+ Human ID，与 Mastercard 合作 | 同样是纯"真人唯一性"，**无地理维度** |
| **Proof of Humanity** | 视频提交 + 链上押金 + 社交担保 | 小而老 | 女巫成本低、增长停滞 |
| **BrightID / Circles** | 社交图见面验证 / 信任网 UBI | 社区型 | 无地理凭证，体验重 |
| **Gitcoin Passport（Passport XYZ）** | 多源凭证（Web2+Web3）聚合评分 | 空投防女巫事实标准 | **"凭证累积评分"思路与本方案最接近**，但它无位置、无照片、无永久身份承诺 |

参考：World 白皮书 [whitepaper.world.org](https://whitepaper.world.org/achieving-proof-of-human)（2026-03 更新）明确论述"唯一性"是 PoH 的核心难题。

### 5.3 地理位置即身份/即信用：**与本方案最接近的一类**

| 方案 | 机制 | 差距 |
|---|---|---|
| **IETF TRIP 草案**（[draft-ayerbe-trip-protocol-04](https://datatracker.ietf.org/doc/draft-ayerbe-trip-protocol/)，2026-05） | 签名的空间量化位置证明"面包屑"串成追加日志→聚合为 **Trajectory Identity Token（TIT）持久假名身份**；用功率谱密度 1/f 特征与 Hamiltonian 能量函数区分人类/合成轨迹 | **思想上几乎就是 GyID 2.0 的学术表亲**：轨迹→身份、物理真实性统计判别。但它无照片、无个人数据卡片、无社交层、无产品、无私钥恢复体系——GeoYuan 可考虑对齐其术语甚至参与标准 |
| **OGC Testbed-20 IPT 报告**（[docs.ogc.org](https://docs.ogc.org/per/24-033.html)，2025-04） | 地理空间数据的完整性/来源/信任，明确提出 VC/VP 兼容的 SSI 路线 | 标准化风向：地理界正在走向"地理数据+W3C 凭证"，早对齐可占位 |
| **FOAM / XYO** | 信标网络/蓝牙节点的 Proof of Location | 依赖专用硬件与以太坊，无身份层（仓库 [COMPETITIVE_ANALYSIS.md](file:///home/huanghe/myweb/GYIDpro/COMPETITIVE_ANALYSIS.md) 已详析） |
| **Hivemapper / Helium** | 行车仪/热点物理贡献换币（Solana） | 地图/通信基础设施，无身份 |
| **Lost Worlds / SNPIT / StepN / TravelMintNFT** | 到场铸 NFT / 拍照赚币 / 运动赚币 | 娱乐或单点证明，无永久身份与凭证体系 |

### 5.4 国家级数字身份（合规参照）

- **欧盟 eIDAS 2.0 / EUDI 钱包**：2026 年起成员国必须提供，DID/VC 进入政府级应用（SSI 市场 2025→2026 预计近翻倍至约 66 亿美元）；
- **爱沙尼亚 e-Residency**：国家背书的数字居留；
- **国内**：**星火链网 BID 区块链分布式标识**（对接 ITU-T X.revid 国际标准）、BSN 分布式身份、CTID 网络身份凭证——二期做联盟链存证与境内合规时的对接对象。

### 5.5 差异化定位矩阵

| 方案 | 唯一性根基 | 地理位置 | 个人数据/照片 | 社交共证 | 永久自持 | 中国合规可落地 |
|---|---|---|---|---|---|---|
| World ID / Humanity | 生物特征（强） | ✗ | ✗ | ✗ | ✔ | △ 生物数据高敏 |
| Gitcoin Passport | 凭证聚合 | ✗ | ✗ | △ | △ | △ |
| FOAM / XYO | 硬件/RF 位置 | ✔ | ✗ | ✗ | 依赖公链 | ✗ |
| IETF TRIP | 轨迹统计真实性 | ✔✔ | ✗ | ✗ | 设计中 | ✔（标准友好） |
| 传统数字藏品平台 | 平台账户 | △ | △ | ✗ | ✗（平台代持） | ✔ |
| **GyID 2.0（本方案）** | **自持密钥 + 凭证密度（可叠加生物凭证）** | **✔✔（H3+照片+轨迹）** | **✔✔（卡片即凭证）** | **✔（双人共址）** | **✔（Base+IPFS+自持）** | **✔（双轨+积分不发币）** |

**一句话定位**：World ID 证明"你是唯一的真人"，TRIP 证明"你走过真实的轨迹"，**GyID 2.0 证明"这是一个在真实地点持续生活、留下过真实数据与关系的、永久可验证的人"**——并且这个证明已经有 geoyuan.com 这个真实产品在持续产生素材。

---

## 6. 路线图（建议）

| 阶段 | 周期（粗估） | 内容 | 产出 |
|---|---|---|---|
| **R0 概念对齐与冻结** | 1–2 周 | 确认本文档；冻结术语（GyID=DID 品牌、GYU=积分、GY=脚印 Foot mark）；冻结双轨合规边界 | GYIP-0002 DID 规范草案 |
| **R1 DID 最小闭环（网页优先）** | 4–6 周 | WASM 生成 DID；cards 表迁移；payload 加 owner_did+签名；极简 GeoyuanID 注册合约上 Base（先测试网后主干）；首卡即注册；ProofSeal 展示 | geoyuan.com 上出现"永久 GyID" |
| **R2 永久性补齐** | 3–4 周 | 图片/metadata 强制 IPFS；资产导出包；IPFS 自助验证页；助记词与设备管理 UI | 关站可验证、可恢复 |
| **R3 Rust 侧统一** | 4–8 周 | 合并 gyid/geoyuan 两套 core；did/credential 模块；CLI/GUI 支持 SIWG 登录 geoyuan.com、拉取本人链上凭证 | 一个全功能桌面节点 |
| **R4 凭证体系与可信度** | 持续 | 设备/地理/共址/轨迹凭证；可信度分级接入配对、铸卡风控与稀有度；双人共址互签上线 | 真实性从口号变成分数 |
| **R5 去中心化增强** | 远期 | GPv1 凭证交换/个人加密备份；联盟链存证（星火/BSN）；zkPoL（证明城市不暴露坐标）；评估对接 World ID/TRIP 标准 | 平台依赖持续下降 |
| **暂停/不做** | — | 自研 PoI 链、VRF 出块、联邦分片、GY 代币化、移动端 SDK（R3 后再排） | 控制风险，先证身份再谈链 |

排序原则：**先把"永久"做实（R1+R2，全在已上线管道上增量），再把"节点"做通（R3），最后才谈"分布式共识"**。这与 diliy 既有"凭证型去中心化"决策完全同向，只是把凭证主体从"平台账户"升级为"用户自持 DID"。

---

## 7. 风险与必须诚实的边界

1. **"完全真实可信"不可达，只能逼近**。GPS 欺骗、EXIF 伪造、设备指纹篡改都真实存在；方案靠多源凭证、轨迹一致性、共址互证和历史沉没成本提高造假门槛，**对外文案应使用"高可信/可验证/持续累积"，不要承诺"无法伪造"**（仓库白皮书当前措辞建议同步收敛）。
2. **"不可撤销"受 PIPL/数安条例约束**：永久仅限链上身份根的存在性；账户、凭证、内容必须支持注销、解绑、删除与违法处置。
3. **平台代持是当前最大的中心化风险**：NFT 在平台钱包名下。短期靠规则透明+导出包，中期在合规允许范围内提供用户钱包自持选项（SIWG + safeTransferFrom 白名单）。
4. **私钥即身份，丢失即灭失**：上线前必须有助记词、多设备关联（已有协议）、可选社交恢复；网页端 IndexedDB 清除是高频事故，要有强制备份引导。
5. **位置数据是最高敏感级个人信息**：精确坐标默认不上链明文（链上只放哈希/H3 粗网格），明文仅在 IPFS 加密包或本人节点；zkPoL 是正确的长期方向。
6. **不要发币、不要与 GYU 混用**：境内产品严格维持数字藏品+法币积分模式，区块链信息服务备案等资质按 diliy 既有合规清单推进。
7. **技术收敛风险**：统一意味着废弃旧 GyID 哈希语义，需要双格式兼容窗口（旧字符串映射到新 DID 的早期凭证），避免老用户与旧文档失效。

---

## 8. 附：给决策的三个最小问题（已确认）

1. ✅ 接受 GyID 本质从"多因子哈希值"改为**自持密钥 DID + 凭证**。凭证指 W3C 可验证凭证（VC），含设备凭证、地理存在凭证、轨迹凭证、共址凭证、账户凭证、真人凭证六类（见 §3.3），每张由签发者签名、可独立验证、可累积、可吊销，挂在用户 DID 名下。
2. ✅ R1 先在 geoyuan.com 铸卡流上做试点（Base 测试网 → 主网），桌面节点随后跟进。
3. ✅ 接受境内产品中降级表述为**脚印（Foot mark）**，代币化无限期搁置。

---

*参考文档：项目内 [WHITEPAPER.md](file:///home/huanghe/myweb/GYIDpro/docs/WHITEPAPER.md)、[GYIP-0001](file:///home/huanghe/myweb/GYIDpro/docs/GYIP-0001-GeoYuan-Protocol-Spec.md)、[COMPETITIVE_ANALYSIS.md](file:///home/huanghe/myweb/GYIDpro/COMPETITIVE_ANALYSIS.md)、[GEOYUAN_DISTRIBUTED_ARCHITECTURE_REPORT.md](file:///home/huanghe/myweb/GYIDpro/docs/GEOYUAN_DISTRIBUTED_ARCHITECTURE_REPORT.md)；diliy [上链完备性_去中心化_盈利方案](file:///home/huanghe/myweb/diliy/.trae/documents/上链完备性_去中心化_盈利方案.md)。外部来源见 §5 内联链接。*
