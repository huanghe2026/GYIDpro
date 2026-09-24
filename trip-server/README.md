# trip-server — TRIP Verifier 服务

`draft-ayerbe-trip-protocol-04` §12 Active Verification 的 Verifier 服务实现
（GYIP-0003 §5.2）。提供 HTTP/WS 接口，跑通 **RP ↔ Verifier ↔ Attester**
三方实时验证流程，直接复用 `trip-core` 的经典引擎
（PSD / Levy / Hamiltonian / Trust）并签发 PoH 证书。

## 构建与运行

```bash
cargo run --release -p trip-server
# 默认监听 127.0.0.1:8080
```

生产启动请显式提供 Verifier 长密钥（32 字节种子的 hex）：

```bash
TRIP_VERIFIER_SEED=$(head -c32 /dev/urandom | xxd -p -c64) \
TRIP_LISTEN=0.0.0.0:8080 \
cargo run --release -p trip-server
```

未设置 `TRIP_VERIFIER_SEED` 时使用固定开发种子（`[0x07;32]`）并打 WARN，
**仅限本地开发**。

## 配置（环境变量）

| 变量 | 默认值 | 说明 |
|---|---|---|
| `TRIP_VERIFIER_SEED` | （开发种子） | Verifier Ed25519 32 字节种子，hex |
| `TRIP_LISTEN` | `127.0.0.1:8080` | 监听地址 |
| `TRIP_VALIDITY_SECS` | `3600` | PoH 证书有效期 |
| `TRIP_CHALLENGE_TTL_SECS` | `60` | 活体挑战有效期 |
| `TRIP_MIN_CONFIDENCE` | `0.1` | RP 策略最小临界置信度 |
| `TRIP_MIN_TRUST` | `20.0` | RP 策略最小信任分 |
| `TRIP_CORS_ORIGINS` | `*` | 允许的 Origin，逗号分隔（`*` 仅开发环境） |
| `TRIP_PUBLIC_URL` | `http://127.0.0.1:8080` | 对外 base URL，写入 DID Document 的 `#verifier` / `#tit` 端点 |
| `TRIP_ANCHOR` | （空） | EVM 锚定指针，CAIP-2 `eip155:<chain_id>:<registry>`，写入 `#anchor`；与 `EVM_PRIVATE_KEY` 同时设置时启用链上中继 |
| `EVM_PRIVATE_KEY` | （空） | GeoTITRegistry **付费**密钥（secp256k1，hex，可带 `0x`），与 `TRIP_VERIFIER_SEED`（Ed25519 身份密钥）相互独立；其地址必须是合约的 verifier |
| `TRIP_EVM_RPC_URL` | 按 chain id 选 Base 官方 RPC | GeoTITRegistry JSON-RPC 端点（84532 → sepolia.base.org，8453 → mainnet.base.org） |
| `TRIP_EPOCH_SIZE` | `100` | 每个链上 epoch 覆盖的面包屑数；凑满即自动 `anchorEpoch` |
| `RUST_LOG` | `info,tower_http=warn` | 日志过滤 |

### 链上中继（可选，缺省关闭）

设置 `TRIP_ANCHOR` + `EVM_PRIVATE_KEY` 后，Verifier 在后台 actor 中代付 gas 写
GeoTITRegistry（任何链上失败只记 WARN，不影响验证流程）：

- **首次主动验证成功**（PoH 落库）→ 自动 `register(pubkey, multibase 后缀)`，写前查 `identityOf`，幂等；
- **证据链落库后** → 每凑满 `TRIP_EPOCH_SIZE` 条连续面包屑，自动
  `anchorEpoch(pubkey, epoch_no, merkle_root, unique_cells)`，epoch 序号按链上
  `epochCount` 严格连续推进，进程重启不重放；
- 链上**只锚存在性**（DID 后缀 / SHA-256 Merkle 根 / 唯一网格数），
  不写坐标、cell、照片；`claimHandle` 暂不自动触发（需 n≥100 且 T≥20，用
  `gyid anchor handle` 手动声明）。

本地联调示例（Base Sepolia）：

```bash
TRIP_ANCHOR=eip155:84532:0x<GeoTITRegistry 地址> \
EVM_PRIVATE_KEY=0x<verifier secp256k1 私钥> \
cargo run -p trip-server
```

## 三方验证流程

```
Attester                Verifier (trip-server)               RP
   │                            │                              │
   │  POST /v1/evidence (CBOR)  │                              │
   │ ─────────────────────────► │  验签+链规则+落库             │
   │                            │                              │
   │  WS /v1/challenge (长连接) │                              │
   │ ◄═══════════════════════► │  ready                       │
   │                            │  POST /v1/verify {nonce}     │
   │                            │ ◄──────────────────────────  │
   │  LivenessChallenge (二进制)│                              │
   │ ◄────────────────────────  │  challenge_id ─────────────► │
   │  LivenessResponse (签名)   │                              │
   │ ────────────────────────►  │  引擎评估 → 签发 PoH          │
   │  {ok:true} 文本回执         │                              │
   │                            │  POST /v1/poh {challenge_id} │
   │                            │ ◄──────────────────────────  │
   │                            │  PoH CBOR ─────────────────► │
```

PoH 证书只含统计指数（α/β/κ/Π/置信度/T/聚合计数）+ RP nonce + 链头，
**不含任何原始位置或 cell**。

## 端点

### `POST /v1/evidence`（Attester）

请求体为**多条面包屑完整 CBOR map 的顺序拼接**（CBOR 自分隔）。
单批上限 **300 条**（超出 `413`，请分页；服务端按 index 合并后对全链验证）。

```bash
curl -s -X POST http://127.0.0.1:8080/v1/evidence \
  -H 'content-type: application/cbor' \
  --data-binary @breadcrumbs.cbor
# {"identity":"..","stored":513,"unique_cells":498,"chain_head":".."}
```

### `POST /v1/verify`（RP）

```bash
curl -s -X POST http://127.0.0.1:8080/v1/verify \
  -H 'content-type: application/json' \
  -d '{"attester":"<hex32 公钥>","rp_nonce":"<hex16>"}'
# {"challenge_id":"..","expires_at":1700000060,"delivered":true}
```

`delivered:false` 表示 Attester 当前无 WS 连接（挑战保留至 deadline）。

### `WS /v1/challenge?attester=<hex32>`（Attester）

- 连上先收到文本 `{"type":"ready",...}`；
- 二进制下行帧 = `LivenessChallenge` CBOR（trip-core `liveness` 模块）；
- Attester 对 `challenge_id || rp_nonce || chain_head || index` 签名，
  回二进制帧 = `LivenessResponse` CBOR；
- 处理结果以文本 JSON 回执（`poh_issued` / 错误）。

### `POST /v1/poh`（RP）

```bash
curl -s -X POST http://127.0.0.1:8080/v1/poh \
  -H 'content-type: application/json' \
  -d '{"challenge_id":"<hex16>"}' \
  --output poh.cbor
```

- `200` + `application/cbor`：PoH 证书；
- `202`：Attester 尚未应答，继续轮询；
- `410`：挑战过期；`404`：未知 challenge。

RP 用 `/.well-known/verifier.json` 公布的公钥，调
`PohCertificate::verify_freshness(pubkey, nonce, now)` 与
`meets_policy(min_confidence, min_trust)` 完成校验。

### `GET /v1/did/:did`（任意）

`did:geoyuan:z<base58btc(Ed25519 公钥)>` → W3C DID Document
（`Content-Type: application/did+json`）。DID 完全由公钥派生，无需任何链上
或数据库状态即可解析。

```bash
curl -s http://127.0.0.1:8080/v1/did/did:geoyuan:zF25s3DdjXdCxYBhh2z8FBusVEMT4b9bGNFVKJi3wFoF4
```

返回 `@context` / `id` / `verificationMethod`（`publicKeyMultibase` 与 DID 同
一 multibase）/ `authentication` / `assertionMethod` / `service`
（`#verifier`、`#tit` 来自 `TRIP_PUBLIC_URL`，`#anchor` 来自 `TRIP_ANCHOR`）。
DID 语法非法 → `400`。

### `GET /v1/tit/:hex`（任意）

Verifier 依据**已上传并通过链规则校验**的面包屑链核算统计量，签发
Verifier 背书的 **TIT**（Trajectory Identity Token，GYIP-0003 §5.4）。

```bash
curl -s http://127.0.0.1:8080/v1/tit/<hex32 公钥>
# {"attester":"..","did":"did:geoyuan:z..","issuer":"verifier",
#  "epochs":5,"breadcrumbs":513,"unique_cells":498,"trust":80.3,
#  "alpha":..,"confidence":..,"handle_ok":true,
#  "tit_cbor_hex":"..","tit_base64url":"..","did_document_url":"/v1/did/.."}
```

与 PoH 的分工：PoH 绑定一次性 RP nonce、含完整统计指数；TIT 长期有效
（`TRIP_VALIDITY_SECS`）、可直接放进二维码/DID Document 用于展示与发现。
验签用 `/.well-known/verifier.json` 的同一把 Verifier 公钥：

```rust
let tit = trip_core::Tit::from_base64url(b64)?;
tit.verify(Some(&verifier_pubkey), now_unix)?;   // 签名 + 新鲜性
```

`404`：该公钥无 evidence；`400`：hex 不合法；`500`：存量链自检失败或引擎评估失败。

### `GET /.well-known/verifier.json`

```bash
curl -s http://127.0.0.1:8080/.well-known/verifier.json
```

返回 Verifier 公钥、证书有效期、策略阈值、保留策略与扩展列表。

## 存储与部署说明（MVP 边界）

- **内存存储，无持久化**：进程重启后 evidence / 挑战 / PoH 全部丢失。
  试点生产前需替换为 Postgres（evidence）+ Redis（nonce/挑战）。
- **CPU 密集计算全部走 `spawn_blocking`**；PSD/Levy 只取最近 256 条窗口，
  链验签覆盖全链。
- 硬件：开发/演示 2 vCPU / 2 GB 即可；试点（加 PG/Redis）建议 4 vCPU /
  8 GB / 100 GB SSD。经典引擎无需 GPU；GPU 是 W9 NeuroCriticality 阶段才需要。
- Attester WS 连接以 query 参数声明身份（不做连接级鉴权），由
  LivenessResponse 的 Attester 签名兜底；生产前置于 TLS 之后。

## 测试

```bash
# 三方联机验收门（真 TCP + WebSocket）
cargo test -p trip-server --test three_party_flow

# DID 解析 + TIT 签发验收门
cargo test -p trip-server --test did_tit

# 全部质量门
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo test --workspace
```

集成测试覆盖：513 条轨迹分页上传 → 完整三方流程 → PoH 验签/验策略
（α∈[0.30,0.80]、T≥20）、未应答 202、篡改 evidence 400、超量批次 413。
