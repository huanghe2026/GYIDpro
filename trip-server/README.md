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
| `RUST_LOG` | `info,tower_http=warn` | 日志过滤 |

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

# 全部质量门
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo test --workspace
```

集成测试覆盖：513 条轨迹分页上传 → 完整三方流程 → PoH 验签/验策略
（α∈[0.30,0.80]、T≥20）、未应答 202、篡改 evidence 400、超量批次 413。
