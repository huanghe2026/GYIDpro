# GeoYuan / TRIP 开发状态总览

> **最后更新**: 2026-09-20
> **基于**: `docs/GYIP-0003-TRIP-GeoYuan-Dev-Plan.md` + 实际代码盘点
>
> **当前阶段**: W8 已闭合，W7/W9-W12 待开始

---

## ✅ 已完成（W1–W8）

| 模块 | 内容 | 测试 |
|------|------|------|
| **trip-core** | 纯协议核心：确定性 CBOR、Ed25519、面包屑哈希链、Epoch/Merkle、Liveness、PoH(15字段)、DID/DID Document、TIT、`engine/`(psd/levy/behavior/hamiltonian/trust/sim)、**anchor**(ABI/EIP-1559/选择器) | **127 测试全绿** |
| **trip-cli** | CLI（二进制名 `gyid`）：`did`/`tit`/`anchor`(deploy/register/epoch/handle/status) 命令 | 编译通过 |
| **trip-server** | Verifier HTTP/WS 服务（`/v1/evidence`、`/v1/verify`、`WS /v1/challenge`、`/v1/poh`、`/v1/identity/:hex`） | 编译通过 |
| **contracts/GeoTITRegistry** | 链上存在性登记簿：register/anchorEpoch/claimHandle，verifier 门控，零外部依赖测试 | **12 用例全过**（内存 EVM） |
| **gyid-shared** | 三端共享业务层（identity 加密、collect_breadcrumb、Chain、liveness、poh、verifier_client） | — |
| **gyid-wasm** | 浏览器 Attester 桥（tsify 生成 d.ts） | — |
| **gyid-web** | SolidJS 演示页（Identity/Collect/Import/Verify/Certificates/Explorer） | — |
| **gyid-android-rs / gyid-android** | UniFFI 安卓绑定雏形 | — |

### W8 新增详情：链上锚定（GYIP-0003 §5.3）

#### trip-core::anchor（Rust 侧）
- **ABI 编码**：`encode_register`/`encode_anchor_epoch`/`encode_claim_handle`（纯计算，可单测）
- **EIP-1559 签名**：`sign_eip1559`（k256 secp256k1，含 low-S、yParity 规范化）
- **选择器计算**：`selector_of` 与合约 solc 输出交叉验证
- **epochKey 一致性**：`keccak256(abi.encode(pubkey, epochNo))` 三方验证（Rust/Python/solc）
- **黄金向量**固化在 `trip-core/src/anchor.rs` 测试中

#### trip-cli anchor 子命令
| 命令 | 功能 |
|------|------|
| `gyid anchor networks` | 列出已知 EVM 网络（Base Sepolia 等） |
| `gyid anchor deploy` | 部署 GeoTITRegistry 到指定链 |
| `gyid anchor register` | 登记 Ed25519 公钥 → DID 后缀绑定 |
| `gyid anchor epoch` | 锚定 epoch Merkle 根（序号从 0 严格连续） |
| `gyid anchor handle` | 声明 handle（n≥100 且 T≥20） |
| `gyid anchor status` | 查交易回执 |
| `gyid anchor epoch-root` | 链上读 epoch 根（eth_call） |

#### 合约 GeoTITRegistry.sol
- **设计原则**：不发币、不锚轨迹、平台代付 gas（verifier 门控）、可升级性最小化
- **写函数**：`register`（一次写定）、`anchorEpoch`（序号连续）、`claimHandle`（门槛+唯一）
- **治理**：`owner` 可轮换 `verifier`；无代理无自毁
- **测试**：12 个用例覆盖登记/锚定/handle/权限/构造函数（零 forge-std 依赖）

#### 合约工具链
| 工具 | 功能 |
|------|------|
| `tools/compile.mjs` | solc-js 编译 → artifacts + 选择器打印 |
| `tools/test-inproc.mjs` | 内存 EVM (@ethereumjs/vm) 跑测试 |
| `tools/deploy.mjs` | 部署到 Base Sepolia / 任意 EVM |
| `tools/decode-calldata.mjs` | 解码 calldata 调试用 |

---

## 📋 待开始（W7 / W9–W12）

| 阶段 | 内容 | 状态 |
|------|------|------|
| **W7** | GeoLife/MDC 标定（地理生活日志 / 多设备一致性） | ❌ 未开始 |
| **W9** | NeuroCriticality（神经临界性指标） | ❌ 未开始 |
| **W10** | TRIP-Arena（多方验证竞技场） | ❌ 未开始 |
| **W11** | diliy 整合（日历/提醒/社交） | ❌ 未开始 |
| **W12** | 主网部署 + 正式发布 | ❌ 未开始 |

---

## 工程质量

| 检查项 | 状态 |
|--------|------|
| `cargo test --workspace` | ✅ 127 测试全绿 |
| `cargo clippy --workspace -D warnings` | ✅ 零警告 |
| `cargo fmt --all --check` | ✅ 通过 |
| 合约内存 EVM 测试 | ✅ 12/12 通过 |
| `unsafe` 使用 | ✅ 零处（测试除外） |

---

*GeoYuan / TRIP Team · 2026-09-20*
