# GYIDpro — 长期记忆

## 项目定位
- 主线：**IETF TRIP 协议的首个开源实现**，协议基线 `draft-ayerbe-trip-protocol-04`。
  对外品牌仍是 **GyID / 地理元**；旧 README 里"硬件指纹算 GyID"那套已退役（`gyid-core`/`geoyuan-core` 已从 workspace 删除）。
- 核心方案文档：`docs/GYIP-0003-TRIP-GeoYuan-Dev-Plan.md`（技术落地）+ `docs/GYID2_UNIFIED_IDENTITY_PROPOSAL.md`（身份哲学：did:geoyuan + 凭证累积 + 双链锚定）。
- 路线图：W1–W12（见 GYIP-0003 §8）。**W1–W8 已全部闭合**（2026-09-25）。
  - W7 已完成：`trip-core::engine::calibration`（GeoLife PLT 解析 + 预处理 + PSD/Levy 标定 + **真实双样本 ROC/AUC** + 人群 α 边界校准）、`calibration_report`（数据集无关的 Markdown 白皮书生成器）、CLI `gyid calibrate {synth|geolife|whitepaper}`、`tools/calibration/` 数据脚本、`docs/CALIBRATION-W7-WHITEPAPER.md`、黄金测试 `tests/calibration_golden.rs` + `tests/vectors/calibration-vectors.json`。
  - W8 已完成：`trip-core::did`/`tit`/`anchor`、CLI `did`/`tit`/`anchor`、`trip-server` 链上中继 `src/chain.rs`、`contracts/GeoTITRegistry.sol`（12 测试）。
  - W9（NeuroCriticality）、W10（TRIP-Arena）、W11（diliy 整合）、W12（主网+发布）未开始。
- TIT 字段编号是 GYIP-0003 内部约定（-04 未规定），黄金向量见 `trip-core/tests/tit_golden.rs`，待 -05 重新 pin。
- DID = `did:geoyuan:z<base58btc(pubkey32)>`；DID 展示名/handle 走 `alsoKnownAs`，不再编码进身份根。

## Workspace 结构（根 `Cargo.toml` members）
| crate | 角色 |
|---|---|
| `trip-core` | 纯协议核心：确定性 CBOR、Ed25519、面包屑哈希链、Epoch/Merkle、Liveness、PoH(15字段)、DID/TIT、`engine/`(psd/levy/behavior/hamiltonian/trust/sim/calibration/calibration_report)、`anchor`(feature) |
| `trip-cli` | CLI，二进制名 **`gyid`**（命令：`init`/`collect`/`verify`/`poh-*`/`did`/`tit`/`anchor`/`calibrate`/`simulate`） |
| `trip-server` | Verifier HTTP/WS 服务（`/v1/evidence`、`/v1/verify`、`WS /v1/challenge`、`/v1/poh`、`/v1/identity/:hex`、`/v1/pohs`、`/.well-known/verifier.json`），MVP 内存存储 |
| `gyid-shared` | 三端共享业务层（identity 加密、collect_breadcrumb、Chain、liveness、poh、verifier_client） |
| `gyid-wasm` | 浏览器 Attester 桥（tsify 生成 d.ts，字节一律 hex 字符串，纯函数） |
| `gyid-web` | SolidJS 演示页（Identity/Collect/Import/Verify/Certificates/Explorer） |
| `gyid-android-rs` / `gyid-android` | UniFFI 安卓绑定雏形 |

## 协议约定（改代码必须守）
- **原始 GPS 永不出端**：端上只产生 H3 cell(res 7..=10, 默认 10) + 哈希；Verifier 只存量化数据。
- 链规则（`trip-core::ChainRules::default`，前端 `gyid-web/src/stores/chain.ts` 常量必须对齐）：
  `min_interval=900s`、`hard_min=300s`(探索会话)、`max_per_cell=10`、相邻同 cell 拒绝、index 从 0 连续、prev_hash 链、Ed25519 每条签名。
- 确定性 CBOR：任何平台同逻辑同字节；配套黄金向量 `trip-core/tests/vectors/`。
- PoH 证书 15 字段（key 0..=14），字段 12/13（RP nonce / 链头）必填。
- 经典引擎是互操作基线，AI 增强只能作独立签名的扩展证据。

## 工程规范
- `cargo clippy --workspace -- -D warnings` 必须零警告；`cargo fmt --all`；禁止 `unsafe`；错误用 `?` 不用 `unwrap()`（测试除外）。
- 提交信息 `<type>(<scope>): <message>`（feat/fix/docs/refactor/test/chore）。
- 前端：`pnpm typecheck`(tsc --noEmit) + `pnpm build` 必须通过。

## 环境备忘
- `cargo` 不在默认 PATH：`export PATH=$HOME/.cargo/bin:$PATH`（写绝对路径 `/home/huanghe/.cargo/bin/cargo`）。
- `node`/`pnpm` 在 `/home/huanghe/.nvm/versions/node/v22.23.1/bin`。
- **lean-ctx 已于 2026-09-25 完全卸载**（`cargo uninstall` + 删二进制/数据目录 + 从 `~/.codebuddy/mcp.json` 移除）。**不要再用任何 `ctx_*` 工具**，改用原生 `read_file`/`search_content`/`execute_command`。
- `playwright-cli` 需 `--browser=chromium`（默认 chrome channel 在 `/opt/google/chrome` 不存在）；其 `snapshot --filename` 相对 CWD，需写 `.playwright-cli/...` 全路径。
- git 工作区在 2026-09-25 已整理干净（5 个规范提交）；`.codebuddy/memory/*.md` 是**有意入库**的，不要 ignore 或删除。
