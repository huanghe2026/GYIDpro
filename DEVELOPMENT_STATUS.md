# GeoYuan / TRIP 开发状态总览

> **最后更新**: 2026-09-25
> **基于**: `docs/GYIP-0003-TRIP-GeoYuan-Dev-Plan.md` + 实际代码盘点
>
> **当前阶段**: **W1–W8 已全部闭合**，W9–W12 待开始

---

## ✅ 已完成（W1–W8）

| 模块 | 内容 | 测试 |
|------|------|------|
| **trip-core** | 纯协议核心：确定性 CBOR、Ed25519、面包屑哈希链、Epoch/Merkle、Liveness、PoH(15字段)、DID/DID Document、TIT、`engine/`(psd/levy/behavior/hamiltonian/trust/sim/**calibration**/**calibration_report**)、**anchor**(ABI/EIP-1559/选择器) | 见下 |
| **trip-cli** | CLI（二进制名 `gyid`）：`did`/`tit`/`anchor`/`calibrate` 命令 | 编译通过 |
| **trip-server** | Verifier HTTP/WS 服务（`/v1/evidence`、`/v1/verify`、`WS /v1/challenge`、`/v1/poh`、`/v1/identity/:hex`、`/v1/pohs`、`/v1/explorer`、`/.well-known/verifier.json`） | 编译通过 |
| **contracts/GeoTITRegistry** | 链上存在性登记簿：register/anchorEpoch/claimHandle，verifier 门控，零外部依赖测试 | **12 用例全过**（内存 EVM） |
| **gyid-shared** | 三端共享业务层（identity 加密、collect_breadcrumb、Chain、liveness、poh、verifier_client） | — |
| **gyid-wasm** | 浏览器 Attester 桥（tsify 生成 d.ts） | — |
| **gyid-web** | SolidJS SPA（Identity/Collect/Import/Verify/Certificates/Explorer + 官网页） | typecheck + build 通过 |
| **gyid-android-rs / gyid-android** | UniFFI 绑定 + Compose 采集/身份/验证三屏 | — |

---

## 🆕 W7：GeoLife 人群标定与判别能力（本次闭合）

> 目标：用真实人群轨迹校准经典临界性引擎的判定边界，并产出可对外提交的判别能力报告
> （对应草案 §7.1 NOTE「α 边界需按人群校准」与 §7.4「单轨迹集成统计量的 ROC 未经验证」）。

### 1. 修正既有缺陷（结论可信的前提）

| 缺陷 | 原状 | 现状态 |
|---|---|---|
| `utc_now_iso()` | 用 `1970 + secs/31536000%100` 当时/月/日，**产出错误日期**并写入报告元数据 | 新增 Howard Hinnant `civil_from_days`（与 `days_from_civil` 互逆）+ `iso8601_from_unix`，附往返与已知时间戳测试 |
| NaN panic | α/β 排序与 Youden 取极值处共 3 处 `partial_cmp(..).unwrap()`，遇脏数据**直接 panic** 中断整批标定 | 统一 `sorted_finite()`（先剔除非有限值再比较） |
| 伪 ROC | 只用单个人类群体，把「低于阈值且高于 0.10 的比例」当 FPR，`auc` 硬编码 `None` | 改为**真实双样本 ROC**（见下），无对照组时 `roc = None` 而**不伪造** |

### 2. 真实 ROC / AUC（核心科学产出）

- **评分函数**：`score(α) = −|α − 0.55|`（越接近粉噪中心越像人；白噪/棕噪两侧同时降分）。
- **三族合成攻击对照**（负类）：

| 族 | 生成器 | 期望 α |
|---|---|---|
| `iid_levy` | 草案 §7.3.3 字面生成器（i.i.d. 截断 Levy 步进） | ≈ 0（谱平坦） |
| `replay_drift` | 重放同一录制序列 + 线性漂移 | ≳ 1.2（趋势谱） |
| `correlated_gaussian` | AR(1) 速度积分 | ≳ 1.0（强低频） |

- **算法**：阈值按分数降序扫描；同分样本**合并为一组**（消除并列顺序依赖）；
  AUC 用 FPR 轴梯形积分；最优点取 Youden's J（并列时保留更高阈值）。
- 额外给出 **`optimal_alpha_band`**：把分数空间阈值翻译成可直接落地的 α 判决区间
  `[0.55 − |t|, 0.55 + |t|]`。

**离线合成基线实测**（`gyid calibrate synth --humans 150 --control 150 --window 256`）：

| 指标 | 值 |
|---|---|
| AUC | **0.9761** |
| 最优点 | TPR 0.9533 / FPR 0.1244 |
| α 判决区间 | `[0.0119, 1.0881]` |
| 人类组 α（均值 / 中位） | 0.5001 / 0.4936 |
| 生物区间覆盖率 | 55.3% |
| `iid_levy` / `replay_drift` / `correlated_gaussian` 均值 α | −0.0141 / 1.6370 / 2.0184 |

> 该基线是**生成器对生成器**的可复现标尺，**不是**真实世界性能；接真实数据需跑
> `gyid calibrate geolife`（见 `tools/calibration/README.md`）。

### 3. 命令行入口（`gyid calibrate`）

| 子命令 | 用途 |
|---|---|
| `gyid calibrate synth` | **全离线**跑通（合成人类 + 三族攻击），适合 CI 与快速自检 |
| `gyid calibrate geolife` | 接真实 GeoLife 数据目录；`--control N` 叠加对照组求 AUC |
| `gyid calibrate whitepaper` | 标定 JSON → Markdown 白皮书草稿 |

### 4. 数据获取与文档

- `tools/calibration/download_geolife.sh`：下载 + `unzip -t` 完整性测试 + 解压 + 定位根目录
  （支持 `GEOLIFE_URL` 镜像与 `GEOLIFE_SHA256` 校验；原始数据 ~1.9 GB **不入库**）
- `tools/calibration/prepare_dataset.sh`：校验 `<user>/Trajectory/*.plt` 结构并输出摘要
- `tools/calibration/README.md`：数据来源与许可、用法、合规与隐私说明
- `docs/CALIBRATION-W7-WHITEPAPER.md`：自动生成的数据白皮书草稿（含 Scope & Provenance 声明）

### 5. 回归基线

- `trip-core/tests/calibration_golden.rs`：端到端黄金测试（合成人类 + 三族对照 → AUC / 覆盖率 /
  α 分位 / JSON 往返 / 运行间确定性）
- `trip-core/tests/vectors/calibration-vectors.json`：固化聚合量，供回归比对
  （重生成：`UPDATE_CALIBRATION_VECTORS=1 cargo test -p trip-core --test calibration_golden`）

---

## 🆕 W6/W8 补充：多前端与链上锚定

### W6 三件套（已完成）

- **trip-server**：CORS（`TRIP_CORS_ORIGINS`）、`GET /v1/identity/:hex`、`GET /v1/pohs?attester=`、
  `GET /v1/explorer`（全网身份聚合）
- **gyid-wasm + gyid-web**：SolidJS SPA（Identity 多账户加密 / Collect 实时采集 /
  Verify 六步状态机 / Import GPX 与照片 / Certificates / Explorer）
- **gyid-android**：Compose 三屏（前景采集服务、断点续传、WS→签名→PoH 徽章）

### W8 链上锚定（已完成）

#### `trip-core::anchor`（Rust 侧，`anchor` feature）
- ABI 编码 `encode_register` / `encode_anchor_epoch` / `encode_claim_handle`
- EIP-1559 签名 `sign_eip1559`（k256 secp256k1，low-S、yParity 规范化）
- 选择器与 `epochKey` 与 solc 输出交叉验证；黄金向量固化在 `src/anchor.rs` 测试中

#### `gyid anchor` 子命令
`networks` / `deploy` / `register` / `epoch` / `handle` / `status` / `epoch-root`

#### `contracts/GeoTITRegistry.sol`
- 设计原则：不发币、不锚轨迹、平台代付 gas（verifier 门控）、可升级性最小化
- 写函数：`register`（一次写定）、`anchorEpoch`（序号从 0 严格连续）、`claimHandle`（n≥100 且 T≥20）
- 治理：`owner` 可轮换 `verifier`；无代理、无自毁
- 工具链：`compile.mjs`（solc-js）/ `test-inproc.mjs`（内存 EVM）/ `deploy.mjs` / `decode-calldata.mjs`

#### `trip-server` 链上中继
- `src/chain.rs`：GeoTITRegistry JSON-RPC 中继（`TRIP_ANCHOR` + `EVM_PRIVATE_KEY` 齐备才启用，否则静默关闭）

---

## 🔍 端到端浏览器验证（W7 附带产出）

用 `playwright-cli`（chromium）对 `gyid-web` + `trip-server` 实测主流程：

| 主流程 | 结果 |
|---|---|
| 身份创建 / 加锁 / 解锁（刷新后自动上锁） | ✅ |
| GPS 采集 + 签名（注入 geolocation） | ✅ |
| GPX 导入 + 链规则预演（6→5 接受 / 1 同 cell 去重） | ✅ |
| 上传到 Verifier（`breadcrumb_count` 1 → 6 → **116**，链头一致） | ✅ |
| 主动验证（WS 挑战 → CBOR 签名应答 → PoH 签发） | ✅ |
| PoH 本地验签 + RP 策略门（`fresh=true`, `policy_pass` 按 `min_confidence` 判定） | ✅ |

协议规则在两处同时生效：前端「全链自检」与服务端 `POST /v1/evidence` 均拒绝
「相邻同 cell」（§4.1）；< 64 位移样本时 Verifier 正确拒发 PoH（§7.4.1 Bootstrap）。

> 验证记录与截图保存在 `.playwright-cli/e2e-w7/`（**不入库**）。
> 已记录的 UX 改进项：Collect 页同 cell 采集未做前置拦截，会阻塞其后所有面包屑的上传。

---

## 📋 待开始（W9–W12）

| 阶段 | 内容 | 状态 |
|------|------|------|
| **W9** | NeuroCriticality v1 训练（FM + 分类头） | ❌ 未开始 |
| **W10** | TRIP-Arena v1（6 类攻击器 + 扩展证书容器） | ❌ 未开始 |
| **W11** | diliy 内测（铸卡即面包屑 + GyID 成长页） | ❌ 未开始 |
| **W12** | 主网锚定 + PoH-VC + SIWG demo + 开源发布 | ❌ 未开始 |

---

## 工程质量

| 检查项 | 状态 |
|--------|------|
| `cargo test --workspace` | ✅ **166 测试全绿** |
| `cargo clippy --workspace --all-targets -D warnings` | ✅ 零警告 |
| `cargo fmt --all --check` | ✅ 通过 |
| 合约内存 EVM 测试（`npm test` in `contracts/`） | ✅ 12/12 通过 |
| 前端 `pnpm typecheck` + `pnpm build` | ✅ 通过 |
| `unsafe` 使用 | ✅ 零处（`#![forbid(unsafe_code)]`） |

---

## 已知技术债

1. **Collect 页同 cell 预检缺失**：允许在同 cell 连续采集，违规条目会阻塞后续上传（见端到端验证）。
2. **地图瓦片依赖外网**：离线环境下底图空白且无降级提示。
3. **`favicon.ico` 缺失**；解锁密码框未包 `<form>`（可访问性提示）。
4. **`Cargo.lock` 未入库**（含二进制目标，建议入库）。
5. **仓库根 `tests/*.rs`** 仍 `use gyid_core::…`（该 crate 已移除），属死代码，建议清理。
6. **历史文档新旧混杂**：`ARCHITECTURE.md` / `CONTRIBUTING.md` / `DEVELOPMENT_PLAN.md` / `RTK.md`
   仍描述旧 `gyid-core`/`gyid-sdk`/`gyid-cli` 与旧定位方案。

---

*GeoYuan / TRIP Team · 2026-09-25*
