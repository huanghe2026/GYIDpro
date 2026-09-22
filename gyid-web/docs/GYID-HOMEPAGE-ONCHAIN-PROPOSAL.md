# GyID 品牌改版与上链落地方案建议

> 日期：2026-09-22
> 状态：网站侧改动已实施并通过验证；合约部署与服务端接线为待办
> 范围：gyid-web 门户/控制台、contracts（GeoTITRegistry）、trip-server 上链接线、底层链选型

---

## 目录

1. [背景与目标](#1-背景与目标)
2. [品牌与首页改版方案](#2-品牌与首页改版方案)
3. [上链方案](#3-上链方案)
4. [底层链技术栈选型](#4-底层链技术栈选型)
5. [实施清单](#5-实施清单)
6. [非目标与风险](#6-非目标与风险)

---

## 1. 背景与目标

GyID 是基于 IETF TRIP（draft-ayerbe-trip-protocol-04，RATS WG 个人草案）的轨迹身份证明实现。原网站首页以研究者视角叙事（协议机制、莱维飞行、CBOR、测试统计），与目标用户（需要"证明自己是真人"的普通用户）不匹配。

**本方案目标：**

1. 首页突出品牌 **Geoyuan ID（简称 GyID）**，用用户语言讲清"它是什么、对我有什么用、怎么获得"；
2. 突出 Geoyuan ID 的**特点**与**获取路径**，技术细节下沉到 /protocol、/architecture；
3. 完善**上链**：让用户能在产品中看到自己身份的链上登记状态，并明确剩余的端到端闭环工作；
4. 回答"是否要从零构建一条存储 GyID 的区块链"以及技术栈/许可证选型问题。

**设计原则（沿用 GYIP-0003 §3）：**

- 链上只锚**存在性**（DID 公钥、epoch Merkle 根、handle），不锚轨迹；
- 原始 GPS 坐标、照片、网格明细永不上链、永不出端；
- 平台代付 gas，普通用户零持币；
- 技术标识（passphrase、H3、CBOR、Ed25519、nonce、PoH、TIT、DID）保持英文不译。

---

## 2. 品牌与首页改版方案

### 2.1 信息架构

首页由"技术介绍页"重构为 6 个用户向区块：

| # | 区块 | 内容 | 用户获得的信息 |
|---|------|------|----------------|
| 1 | Hero | 渐变大字 **Geoyuan ID** + "简称 GyID" + 主张"用你真实走过的路，证明你是真人" + 4 个利益标签 | 这是什么、对我意味着什么 |
| 2 | 四个特点 | 只有真人能拥有 / 位置永远不上传 / 没法买卖代持 / 不绑姓名不用注册 | 为什么值得信任 |
| 3 | 获取四步 | 创建身份 → 积累轨迹 → 主动验证 → 领证书并上链，每步直达控制台页面 | 我现在该做什么 |
| 4 | 上链锚定 | 身份登记 / 周期锚定 / 专属 Handle 三卡 + 隐私承诺 + 测试网状态胶囊 | 链上存了什么、隐私边界 |
| 5 | 使用入口 | 网页版 / Android / 命令行 | 哪里能用 |
| 6 | 最终 CTA | "下一次证明我是真人，不用再掏出证件" | 行动号召 |

原首页的"三道裂缝、从传感器到 PoH 的 5 步技术流程、137 workspace 测试、10 个 Verifier 端点"等研究者内容从首页移除，保留在 `/protocol` 与 `/architecture` 深页。

### 2.2 获取路径与控制台页面的映射

| 步骤 | 用户动作 | 直达页面 |
|------|----------|----------|
| 01 创建身份 | 浏览器本地生成密钥，passphrase 加密存 localStorage | `/console/identity` |
| 02 积累轨迹 | 实时采集，或导入 GPX / 带 EXIF 的 JPEG | `/console/collect`、`/console/import` |
| 03 主动验证 | WebSocket 实时挑战 + 签名应答 + 临界性评估 | `/console/verify` |
| 04 领证书并上链 | 查收 PoH 证书，身份存在性由 Verifier 中继上链 | `/console/certificates` |

### 2.3 中英文案

- 文案统一维护在 `src/i18n/zh.ts` / `en.ts`，中文为源语言，英文 `satisfies Dict` 做结构校验；
- 首次访问按浏览器语言自动选择（`navigator.languages` 含 `zh` → 中文，否则英文），右上角可手动切换并记忆于 `localStorage("gyid.locale")`；
- 浏览器标题同步改为品牌标题：
  - 中：`Geoyuan ID（GyID）— 用真实轨迹证明你是真人`
  - 英：`Geoyuan ID (GyID) — Prove You're Human With Your Real Trajectory`

### 2.4 已改动文件

- `src/i18n/zh.ts`、`src/i18n/en.ts`：`home.*` 整段重写；`meta.title` 更新；`identity.onchain.*` 新增
- `src/pages/site/Home.tsx`：6 区块重写
- `index.html`：静态标题与早期语言脚本中的标题同步

---

## 3. 上链方案

### 3.1 合约现状（已就绪）

合约 `contracts/src/GeoTITRegistry.sol`（**MIT 许可**，与父仓库 GeoYuan 专有 LICENSE 不冲突）已实现并通过 12 个用例测试：

| 函数 | 选择器 | 语义 |
|------|--------|------|
| `register(pubkey, didSuffix)` | `0xcf2d31fb` | Ed25519 公钥 → DID 绑定，一次写定不可改 |
| `anchorEpoch(pubkey, epochNo, merkleRoot, uniqueCells)` | `0x4f65a1cc` | epoch 从 0 起严格连续，存 Merkle 根 + 唯一 cell 数 |
| `claimHandle(pubkey, name, breadcrumbs, trustX100)` | `0xa5188f93` | n ≥ 100 且 T ≥ 20，handle 仅可声明一次 |
| `identityOf(pubkey)` | `0x4fc9c91a` | 只读：registered/registeredAt/epochCount/lastEpoch/lastUniqueCells/handleClaimedAt |
| `handleOf(pubkey)` | `0x0c1a880a` | 只读：handle 字符串 |

三个写函数全部由 `verifier` 地址门控（平台中继代付 gas），`owner` 可轮换 verifier；无代理、无自毁、无发币。

Rust 侧原语已具备：`trip-core::anchor`（ABI 编码 + EIP-1559 签名，含三方交叉验证黄金向量）、CLI `gyid anchor {deploy,register,epoch,handle,epoch-root,status,networks}`。

### 3.2 网页端只读集成（已实施）

- 新增 `src/lib/registry.ts`：**零第三方依赖**，直接 JSON-RPC `eth_call` 读取 GeoTITRegistry；
  - 选择器由 contracts 侧 ethers 计算，写函数选择器与 contracts/README.md 记录交叉核对一致；
  - ABI tuple/动态 string 解码经构造数据验证；
  - 对非合约地址的 `eth_call` 返回空 `0x`，按全零（未登记）处理，避免误报；
- `src/pages/Identity.tsx` 解锁面板新增"区块链登记（GeoTITRegistry）"卡片：
  - 已登记：登记时间、已锚定周期数、最近唯一网格、Handle + Basescan 链接；
  - 未登记：引导文案"完成首次主动验证后由 Verifier 自动登记"；
  - RPC 不可达：降级为错误提示，不影响其他功能；
- 构建期环境变量（`vite.config.ts` define 注入）：

| 变量 | 缺省值 | 作用 |
|------|--------|------|
| `VITE_REGISTRY_ADDRESS` | 空 | 合约地址；**未配置时链上卡片整体不渲染**（功能静默关闭） |
| `VITE_RPC_URL` | `https://sepolia.base.org` | 只读 JSON-RPC 端点 |

### 3.3 端到端上链闭环（实施状态）

1. **部署合约到 Base Sepolia**（待执行，需要部署者私钥）：
   ```bash
   cd contracts
   # 先编译产出 artifact（CLI 部署读取 contracts/artifacts/GeoTITRegistry.json）
   npm run compile
   # 方式一：部署脚本（.env 配 PRIVATE_KEY / VERIFIER，脚本校验 chainId=84532）
   PRIVATE_KEY=0x<部署者私钥> VERIFIER=0x<Verifier 付费地址> npm run deploy
   # 方式二：CLI（verifier 缺省 = 部署者）
   cargo run -p trip-cli -- anchor deploy \
     --rpc https://sepolia.base.org \
     --artifact contracts/artifacts/GeoTITRegistry.json \
     --verifier 0x<中继地址>
   ```
2. **trip-server 写链中继（已实施，2026-09-22）**：新增 `trip-server/src/chain.rs`，
   独立 actor 串行发交易（无 nonce 竞争），复用 `trip-core::anchor` 编码/签名：
   - PoH 首次签发成功 → 自动 `register`（先查 `identityOf`，幂等）；
   - 证据链落库后每满 `TRIP_EPOCH_SIZE`（默认 100）条 → 自动 `anchorEpoch`，
     epoch 序号按链上 `epochCount` 推进，重启不重放；
   - 所有链上失败仅 WARN，不阻塞验证流程；`estimateGas` 失败回退 20 万 gas；
   - 仅在 `TRIP_ANCHOR` + `EVM_PRIVATE_KEY` 齐备时启用，否则完全静默关闭。
3. **前端配置合约地址并重新构建**：
   ```bash
   VITE_REGISTRY_ADDRESS=0x<部署得到的地址> pnpm build
   ```
   Identity 页链上卡片自动激活。
4. **主网迁移前置条件**：合约安全审计、owner 改多签、付费私钥进 KMS/HSM。

### 3.4 已验证项

- `pnpm typecheck`（tsc --noEmit）零错误；`pnpm build` 通过；
- 浏览器实测（占位 EOA 地址）：中英双语文案、未登记态展示、刷新重查、空返回不误判、无 JS 报错；
- Base Sepolia 公共 RPC 实测：对无合约地址 `eth_call` 返回 `0x`（验证降级路径）。

---

## 4. 底层链技术栈选型

### 4.1 结论

**不建议为 GyID 从零新建区块链。** 链上负载极小（每用户 1 次登记 + 偶发 Merkle 根 + 1 次 handle；全链年写入量预计仅数万笔量级），且合约已按 EVM 写好。自建链的验证人运维、升级治理、钱包/浏览器/SDK 生态成本换不来产品收益。

**推荐路径：**

1. 现阶段（已对齐）：GeoTITRegistry 部署 **Base Sepolia** 测试网；
2. 主网：**Base** 主网（gas 极低、大众用户与合规生态、现有代码零改动），备选 Optimism / Polygon；若追求最强公信力可 Ethereum L1（写入极少，成本可接受）；
3. 远期若确有"Geoyuan Chain"产品化需求：**OP Stack（MIT）** 自有 L2 为首选，**Cosmos SDK（Apache-2.0）** 主权链为次选。

### 4.2 方案对比

| 方案 | 技术栈 | 许可证 / 成本 | 对现有合约 | 适用阶段 |
|------|--------|---------------|------------|----------|
| **A. 现有 EVM 公链（推荐）** | Solidity + JSON-RPC + Etherscan 系浏览器 | Base/OP/Polygon 生态工具均开源；无授权费 | 原样部署 | 现在 → 主网 |
| **B. OP Stack 自有 L2** | op-geth（EVM 等价）+ batcher/proposer；可 Conduit/Caldera 托管 sequencer | **MIT**；不入 Superchain 无收入分成，加入才分成 | 原样部署 | 远期产品化 |
| **C. Cosmos SDK 主权 L1** | Go 写 `x/gyid` 模块 + CometBFT BFT + IBC + Keplr 生态 | **Apache-2.0**（Cosmos SDK/CometBFT），主权链框架中最宽松 | 需用 Go 重写逻辑；需自建验证人集/浏览器/faucet | 强主权需求 |
| **D. Arbitrum Orbit/Nitro** | Nitro 全栈 | **BSL 1.1**：结转到 Arbitrum One/Nova 的 L3 免费；**独立 L2 须缴 10% 净协议收入**（8% DAO + 2% Guild） | EVM 兼容但许可证不适合专有商业项目 | 规避 |
| E. Polkadot SDK（Substrate） | Rust + FRAME + libp2p + WASM runtime | 早期 GPL-3.0（强 copyleft，与 GeoYuan 专有 LICENSE 冲突）；2024 年底起多数 crate 转 **Apache-2.0**，但 fork 前必须逐 crate 核对 LICENSE | 不兼容，需用 FRAME/pallet 重写 | 不推荐 |
| F. Avalanche Subnet | Go/定制 VM | BSD-3 | 需定制 EVM 子网配置 | 备选 |

### 4.3 若坚持从零造 L1：完整技术栈清单

- **P2P 网络**：libp2p（MIT），节点发现 / gossipsub 广播 / Kademlia DHT
- **共识**：直接用 CometBFT（Apache-2.0）BFT 共识；自写 PoW/PoS/BFT 不现实
- **执行层**：revm（Rust，MIT）嵌入 EVM，或自研 GyID 原生状态机（register/anchor/handle 三个操作）
- **状态存储**：RocksDB（Apache-2.0）或 Pebble（BSD-3）+ Merkle 状态树
- **密码学**：secp256k1（付费/签名密钥）、Ed25519（身份公钥）、keccak256
- **对外接口**：JSON-RPC / WebSocket、索引器（block/events → 查询 API）、区块浏览器、钱包适配、faucet（测试网）、多语言 SDK
- **运维与治理**：创世配置、运行时升级/硬分叉机制、验证人集与质押（若 PoS）、监控告警、种子节点/快照、审计
- **粗估成本**：1–2 名区块链工程师 3–6 个月到测试网，外加安全审计；对"只存存在性证明"的场景属纯负担。

### 4.4 许可证核查要点（版权冲突规避）

- GeoTITRegistry 合约本体 SPDX 为 **MIT**，可随 GeoYuan 专有产品自由使用，保留版权声明即可；
- OP Stack 核心仓库为 **MIT**（2026 年现状；Base 已基于其分叉出自研代码，印证可自由带走）；
- Arbitrum Nitro 为 **BSL**，到期转 Apache-2.0 但独立 L2 的 10% 分成条款独立适用，不可误当 MIT；
- 任何框架 fork 前执行：`license-checker` / 逐 crate LICENSE 清点，区分代码许可与商标权（"Arbitrum""Polygon"等名称/标识不随开源授权）。

---

## 5. 实施清单

### 已完成（2026-09-22）

- [x] 首页品牌改版（6 区块，中英双语，技术内容下沉）
- [x] meta.title / index.html 标题同步
- [x] `src/lib/registry.ts` 零依赖只读客户端（含空返回降级）
- [x] Identity 页"区块链登记"卡片 + env 开关
- [x] typecheck / build / 浏览器中英切换与未登记态验证
- [x] **trip-server 链上中继**：`src/chain.rs` actor 自动 `register` / `anchorEpoch`
      （幂等查链、串行 nonce、失败只 WARN、缺省关闭），config/README/接线完成，
      clippy 零警告，11 项测试通过

### 待办（端到端闭环）

- [ ] 部署 GeoTITRegistry 到 Base Sepolia（`cd contracts && npm run compile &&
      RPC_URL=https://sepolia.base.org PRIVATE_KEY=0x… VERIFIER=0x… npm run deploy`），
      记录合约地址
- [ ] 启动带中继的 trip-server：`TRIP_ANCHOR=eip155:84532:<地址>
      EVM_PRIVATE_KEY=0x<verifier 付费私钥>`（地址须与部署时 VERIFIER 一致）
- [ ] 生产构建注入 `VITE_REGISTRY_ADDRESS`，跑一次完整三方验证 + 100 条面包屑，
      核对已登记/已锚定/handle 三种真实态与 Basescan 交易
- [ ] handle 声明：达到 n≥100 且 T≥20 后用 `gyid anchor handle` 手动执行（暂不自动）
- [ ] 主网前：合约审计、owner 多签、服务端付费密钥 KMS/HSM 托管
- [ ] 远期评估（按需，不提前投入）：OP Stack L2 或 Cosmos SDK 主权链

---

## 6. 非目标与风险

**非目标：**

- 不在链上存任何 GPS 坐标、照片、H3 cell 明细、原始轨迹；
- 不发币、不做 gas 代币经济（用户侧零持币）；
- 本期不实现证书吊销注册表（GYIP-0003 远期项）；
- 不改动 TRIP 协议语义与临界性引擎，仅做产品叙事与锚定接线；
- 不新建底层区块链。

**风险与对策：**

| 风险 | 影响 | 对策 |
|------|------|------|
| 合约尚未审计、owner 单签 | 主网资金/登记被单点控制 | 主网前审计 + 多签；测试网期不承载真实承诺 |
| verifier 中心化写权限 | 平台可影响登记可用性（无法伪造内容，因端上密码学验证） | 文档如实披露；远期多 verifier / 去中心化中继 |
| RPC 端点不稳定或限流 | 前端链上卡片不可用 | 已做静默降级；生产用自建/付费 RPC 或多端点回退 |
| handle 不可改、不可吊销 | 用户误声明无法撤回 | UI 声明前强确认；语义在首页与合约注释中明示 |
| TRIP 仍是 Internet-Draft | 协议可能随上游变更 | 首页不承诺 RFC 状态；版本号随 draft 更新 |
| BSL 类框架误用 | 商业分成/版权冲突 | 选型锁定 MIT/Apache-2.0/BSD，fork 前逐文件清点许可证 |
