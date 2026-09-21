# GeoYuan / TRIP 开发状态总览

> **最后更新**: 2026-09-21
> **基于**: `docs/GYIP-0003-TRIP-GeoYuan-Dev-Plan.md` + 实际代码盘点
>
> **当前阶段**: W8 已闭合，**W7 进行中**，W9-W12 待开始

---

## ✅ 已完成（W1–W8）

| 模块 | 内容 | 测试 |
|------|------|------|
| **trip-core** | 纯协议核心：确定性 CBOR、Ed25519、面包屑哈希链、Epoch/Merkle、Liveness、PoH(15字段)、DID/DID Document、TIT、`engine/`(psd/levy/behavior/hamiltonian/trust/sim/**calibration**+**calibration_report**)、**anchor**(ABI/EIP-1559/选择器) | **132 测试全绿** |
| **trip-cli** | CLI（二进制名 `gyid`）：`did`/`tit`/`anchor`(deploy/register/epoch/handle/status) 命令 | 编译通过 |
| **trip-server** | Verifier HTTP/WS 服务（`/v1/evidence`、`/v1/verify`、`WS /v1/challenge`、`/v1/poh`、`/v1/identity/:hex`） | 编译通过 |
| **contracts/GeoTITRegistry** | 链上存在性登记簿：register/anchorEpoch/claimHandle，verifier 门控，零外部依赖测试 | **12 用例全过**（内存 EVM） |
| **gyid-shared** | 三端共享业务层（identity 加密、collect_breadcrumb、Chain、liveness、poh、verifier_client） | — |
| **gyid-wasm** | 浏览器 Attester 桥（tsify 生成 d.ts） | — |
| **gyid-web** | SolidJS 演示页（Identity/Collect/Import/Verify/Certificates/Explorer） | — |
| **gyid-android-rs / gyid-android** | UniFFI 安卓绑定雏形 | — |

### W6 已完成详情：GyID 多前端三件套（W6 Phase 10 收尾）

- **trip-server**：CORS（`TRIP_CORS_ORIGINS`）、`GET /v1/identity/:hex`、`GET /v1/pohs?attester=`、`GET /v1/explorer`（全网身份聚合，Explorer 页数据源）
- **gyid-shared**：三端共享业务层，104+ 单测覆盖 identity round-trip / 面包屑签名 / 链校验 / PoH 验证
- **gyid-wasm + gyid-web**：SolidJS SPA（Identity 多账户加密 / Collect 实时采集 / Verify 8 步状态机 / Certificates / Explorer），`pnpm build` + `tsc --noEmit` 全过
- **gyid CLI（`gyid` 二进制）**：`init` / `collect` / `verify` / `poh-list` / `poh-show` 用户命令 + 高级命令分区
- **gyid-android**：Compose 三屏（CollectService 前台采集 FusedLocation+WiFi+IMU、断点续传、VerifyScreen WS→签名→PoH 徽章），arm64-v8a + x86_64 双 ABI
- **端到端 smoke**：wasm 算 H3 cell → CLI 签链 → POST /v1/evidence → identity/explorer 聚合一致（协议规则：连续同 cell 拒、间隔 <300s 拒、<900s 无 exploration 拒）

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

### W7 进行中：GeoLife/MDC 标定（§7.1 人群参数校准）

#### trip-core::engine::calibration（新增）
- **GeoLife PLT 解析器**：支持 Microsoft Research GeoLife 格式（纬度,经度,_,_,日期时间,_）
- **轨迹预处理**：时间排序、5分钟间隔去重、H3 res10 量化
- **标定流水线**：PSD α 分析 + Levy MLE 拟合 + 桥校验
- **人群统计报告**：α/β 分布、P5-P95 分位数、生物区间覆盖率
- **ROC 分析**：多阈值 TPR/FPR、Youden's J 最优阈值
- **边界推荐**：自动检测是否需要调整草案 α∈[0.30, 0.80] 边界

#### trip-core::engine::calibration_report（新增）
- **Markdown 白皮书生成器**：从 JSON 报告生成 IETF 风格数据白皮书草稿
- 包含：执行摘要、数据集描述、α/β 统计表、ROC 曲线数据、方法论、结论与建议

#### 运行方式
```bash
# 下载 GeoLife 数据后运行标定
cargo run --release -p trip-core --example geolife_calibration /path/to/GeoLife_Trajectories_1.3 [output.json]
```

---

## 📋 待开始（W7 / W9–W12）

| 阶段 | 内容 | 状态 |
|------|------|------|
| **W7** | GeoLife/MDC 标定（地理生活日志 / 多设备一致性） | 🔄 进行中：calibration 模块 + 报告生成器已完成，待下载真实数据运行 |
| **W9** | NeuroCriticality（神经临界性指标） | ❌ 未开始 |
| **W10** | TRIP-Arena（多方验证竞技场） | ❌ 未开始 |
| **W11** | diliy 整合（日历/提醒/社交） | ❌ 未开始 |
| **W12** | 主网部署 + 正式发布 | ❌ 未开始 |

---

## 工程质量

| 检查项 | 状态 |
|--------|------|
| `cargo test --workspace` | ✅ 137 测试全绿 |
| `cargo clippy --workspace -D warnings` | ✅ 零警告 |
| `cargo fmt --all --check` | ✅ 通过 |
| 合约内存 EVM 测试 | ✅ 12/12 通过 |
| `unsafe` 使用 | ✅ 零处（测试除外） |

---

*GeoYuan / TRIP Team · 2026-09-20*
