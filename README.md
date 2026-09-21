# GyID — 基于 TRIP 协议的轨迹身份系统

> GeoYuan ID：把真实移动轨迹变成可验证的身份与信任，基于 IETF 风格的 TRIP 协议（draft-04）

## 项目简介

**GyID** 实现了 TRIP（Trajectory-based Reputation & Identity Protocol）draft-04 协议栈：用户（Attester）持续采集 GPS 轨迹并签名为面包屑哈希链，Verifier（trip-server）验证链的有效性后签发 PoH（Proof of History）证书，RP（依赖方）凭 PoH 确认"这是一个持续在真实物理世界移动的人"。

### 核心特点

- 🔗 **面包屑哈希链**: 每条轨迹点签名成 CBOR 面包屑，prev_hash 环环相扣，不可篡改
- 🗺 **H3 空间量化**: (lat, lng) → H3 分辨率 7..=10 六边形 cell，协议层强制
- ⏱ **时间规则**: 相邻面包屑间隔 ≥300s 硬下限，默认 ≥900s（探索模式放宽）
- 🧬 **PoH 证书**: Verifier 签名的 15 字段证书，含 PSD α / Lévy β / 信任分 / 置信度
- 🪪 **DID + TIT**: `did:geoyuan` W3C DID Document + Verifier 背书的轨迹身份令牌
- ⛓ **链上锚定**: GeoTITRegistry 合约（EVM / Base Sepolia）登记公钥与 epoch Merkle 根
- 🌐 **多前端**: Web (WASM + SolidJS)、CLI、Android (UniFFI + Compose) 三端共享同一业务核心

## 技术栈

- **协议层**: Rust（`trip-core`，确定性 CBOR + Ed25519 + h3o + PSD/Lévy 引擎）
- **Verifier**: Rust（`trip-server`，axum HTTP/WS，in-memory MVP）
- **Web**: `gyid-wasm`（wasm-bindgen + tsify）+ `gyid-web`（SolidJS + Vite + Tailwind）
- **CLI**: `trip-cli`（二进制名 `gyid`）
- **Android**: `gyid-android-rs`（UniFFI）+ `gyid-android`（Kotlin + Jetpack Compose）
- **合约**: Solidity（`contracts/GeoTITRegistry`，零依赖内存 EVM 测试）

## 快速开始

### 编译与测试

```bash
# 全量构建 + 测试（137 个测试）
cargo build --workspace
cargo test --workspace

# 静态检查
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

### 运行 Verifier

```bash
cargo run -p trip-server
# 默认监听 0.0.0.0:8080，日志会打印 verifier 公钥
# CORS 白名单: TRIP_CORS_ORIGINS="http://localhost:5173,..."（默认 *）
```

### 运行 CLI

```bash
cargo build -p trip-cli
./target/debug/gyid --help

# 生成身份（Ed25519 seed + pubkey）
gyid init

# 采集面包屑并上传 Verifier（需 GPS；无 GPS 快速失败）
gyid collect --verifier http://127.0.0.1:8080

# RP 发起 Active Verification 并取回 PoH
gyid verify --verifier http://127.0.0.1:8080 --attester <64-hex>

# 查询某身份已签发的 PoH
gyid poh-list --verifier http://127.0.0.1:8080 --attester <64-hex>
```

### 运行 Web 前端

```bash
cd gyid-wasm && wasm-pack build --target web --release
# 产物 gyid-wasm/pkg/ 复制到 gyid-web/wasm/

cd ../gyid-web
pnpm install
pnpm dev        # http://localhost:5173
pnpm build      # 产物 dist/
pnpm typecheck
```

### 构建 Android

```bash
# Rust 侧：生成 Kotlin 绑定 + 双 ABI .so
cd gyid-android-rs && uniffi-bindgen generate --library ... # 详见 crate 文档
ANDROID_NDK_HOME=$HOME/Android/Sdk/ndk/27.2.12479018 \
  cargo ndk -t arm64-v8a -t x86_64 -o ../gyid-android-rs/jniLibs build --release -p gyid-android-rs

# Android 侧（sourceSet 直接引用 gyid-android-rs 的 kotlin/ 与 jniLibs/）
cd ../gyid-android && gradle :app:assembleDebug
```

## Verifier API（trip-server）

| 方法/协议 | 路径 | 角色 | 说明 |
|---|---|---|---|
| POST | `/v1/evidence` | Attester | 批量上传面包屑（CBOR 帧流，单批 ≤300，支持断点续传） |
| POST | `/v1/verify` | RP | 发起 Active Verification，返回 challenge_id |
| WS | `/v1/challenge?attester=<hex32>` | Attester | 接 LivenessChallenge，回 LivenessResponse |
| POST | `/v1/poh` | RP | 凭 challenge_id 取 PoH 证书（CBOR） |
| GET | `/v1/identity/:hex` | 任意 | 某 attester 链统计（count / unique_cells / chain_head） |
| GET | `/v1/pohs?attester=<hex32>` | 任意 | 列出某 attester 已签发的 PoH |
| GET | `/v1/explorer` | 任意 | 全网身份聚合浏览（Explorer 页数据源） |
| GET | `/v1/did/:did` | 任意 | `did:geoyuan` 解析 → W3C DID Document |
| GET | `/v1/tit/:hex` | 任意 | Verifier 背书签发 TIT |
| GET | `/.well-known/verifier.json` | 任意 | Verifier 公钥与策略 |

> 注意：MVP 为 in-memory 存储，进程重启数据清空。

## 项目结构

```
GYIDpro/
├── trip-core/           # 纯协议核心：CBOR、Ed25519、哈希链、Epoch、Liveness、PoH、DID、TIT、engine/
├── trip-cli/            # CLI（二进制名 gyid）：init/collect/verify/poh-* + 高级命令
├── trip-server/         # Verifier HTTP/WS 服务
├── gyid-shared/         # 三端共享业务层（identity 加密、采集、链、验证、PoH）
├── gyid-wasm/           # 浏览器 WASM 桥（tsify 自动生成 .d.ts）
├── gyid-web/            # SolidJS SPA：Identity / Collect / Verify / Certificates / Explorer
├── gyid-android-rs/     # UniFFI 安卓绑定（.udl → Kotlin）
├── gyid-android/        # Android app（Kotlin + Compose：采集前台服务 / 验证状态机）
├── contracts/           # GeoTITRegistry.sol + 编译/测试/部署工具
├── tools/               # 合约工具链（compile/test-inproc/deploy.mjs）
├── docs/                # 协议与开发文档
└── .trae/documents/     # 各阶段实施计划（W5/W6 等）
```

## 版本

当前版本: **v0.1.0**（W6 多前端已闭合，W7 标定进行中）

- ✅ trip-core 协议栈（draft-04，137 测试全绿）
- ✅ trip-server Verifier（HTTP/WS + CORS + Explorer 聚合端点）
- ✅ gyid CLI（`gyid` 二进制）
- ✅ gyid-web 演示 SPA（wasm 桥 + SolidJS）
- ✅ gyid-android（UniFFI 绑定 + Compose 三屏 + 双 ABI）
- ✅ GeoTITRegistry 合约（12 用例内存 EVM 测试）

## 文档

- [ARCHITECTURE.md](ARCHITECTURE.md) - 架构设计
- [DEVELOPMENT_STATUS.md](DEVELOPMENT_STATUS.md) - 开发状态总览
- [DEVELOPMENT_PLAN.md](DEVELOPMENT_PLAN.md) - 开发计划
- [docs/](docs/) - 协议文档（GYIP-0003 / TRIP draft-04）

## 许可证

MIT OR Apache-2.0
