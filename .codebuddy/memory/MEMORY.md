# GYIDpro — 长期记忆

## 项目定位
- 主线：**IETF TRIP 协议的首个开源实现**，协议基线 `draft-ayerbe-trip-protocol-04`。
  对外品牌仍是 **GyID / 地理元**；旧 README 里"硬件指纹算 GyID"那套已退役（`gyid-core`/`geoyuan-core` 已从 workspace 删除）。
- 核心方案文档：`docs/GYIP-0003-TRIP-GeoYuan-Dev-Plan.md`（技术落地）+ `docs/GYID2_UNIFIED_IDENTITY_PROPOSAL.md`（身份哲学：did:geoyuan + 凭证累积 + 双链锚定）。
- 路线图：W1–W12（见 GYIP-0003 §8）。**W1–W6 已完成**；**W8 已完全闭合**（`trip-core::did` DID/DID Document、`trip-core::tit` TIT、CLI `did`/`tit`/`anchor` 命令、`trip-server` `/v1/did/:did` 与 `/v1/tit/:hex`、**GeoTITRegistry.sol 合约 + 12 个测试 + trip-core::anchor ABI/EIP-1559 签名**）；W7（GeoLife/MDC 标定）、W9（NeuroCriticality）、W10（TRIP-Arena）、W11（diliy 整合）、W12（主网+发布）未开始。
- TIT 字段编号是 GYIP-0003 内部约定（-04 未规定），黄金向量见 `trip-core/tests/tit_golden.rs`，待 -05 重新 pin。
- DID = `did:geoyuan:z<base58btc(pubkey32)>`；DID 展示名/handle 走 `alsoKnownAs`，不再编码进身份根。

## Workspace 结构（根 `Cargo.toml` members）
| crate | 角色 |
|---|---|
| `trip-core` | 纯协议核心：确定性 CBOR、Ed25519、面包屑哈希链、Epoch/Merkle、Liveness、PoH(15字段)、`engine/`(psd/levy/behavior/hamiltonian/trust/sim) |
| `trip-cli` | CLI，二进制名 **`gyid`** |
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
- lean-ctx 会把 `CARGO_TARGET_DIR` 重定向到 `/home/huanghe/.local/share/lean-ctx/build-cache/cargo-target`。
- 仓库当前有大量未提交变更（老 `gyid-*`/`geoyuan-*` 删除 + 新 `trip-*`/`gyid-shared|wasm|web|android-rs`），尚未做 W1–W6 的整理提交。
