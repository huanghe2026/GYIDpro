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

## 生产部署（2026-09-25 上线）
- **唯一目标域名**：`gyid.geoyuan.com`。用户明确要求**不向 `diliy.cn` 部署任何东西**，也不要改其他站点配置。
- **服务器**：`ssh server`（`8.136.127.182`，root，`~/.ssh/id_ed25519`），Ubuntu 24.04 / glibc 2.39。
  - 本机 glibc 2.43 > 服务器 → **必须静态链接**：`RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --target x86_64-unknown-linux-gnu`
- **站点布局**（`/var/www/html/gyid.geoyuan.com/`）：Astro 站占根 + `/verify/` 是其子目录副本。
  - 一套源码出两个变体：`BASE_PATH`/`OUT_DIR` 环境变量控制（`dist` / `dist-verify`，见 `.gitignore`）。
- **trip-server**：systemd `trip-server.service`（User=www-data，`RuntimeDirectory=trip-server`），env `/etc/trip-server.env`。
  - 监听 **Unix socket** `unix:/run/trip-server/verifier.sock`，**零 TCP 端口**（`ListenTarget` 见 `trip-server/src/listen.rs`）。
  - axum 0.7 的 `axum::serve` 只吃 `TcpListener` → Unix 一路手动驱动 hyper 1.x，且**必须 `.with_upgrades()`** 否则 WS 101 失败。
- **8080 属于 AnimeGAN**（`geoyuan.com` 与 `diliy.cn` 的 `/animegan/` 都反代它）；trip-server **不要**占 8080。
- **nginx 两个必踩的坑**：
  1. `gyid.geoyuan.com.conf` 的 `location ~ /\.` 会拦掉 `/.well-known/*` → 放通必须用 `^~` 前缀（优先于正则）。
  2. `try_files $uri $uri/ =404` + `error_page 404 /目标.html`，若目标文件**不存在**会形成内部重定向环返回 **500**。
- **Astro 站点约定**：404 必须用原生 `src/pages/404.astro`（固定输出 `/404.html`）；挂到 `[...slug]` 注册表会把键 `404` 变成 **number** 字面量污染 `PageSlug` 类型，且产不出页面。
