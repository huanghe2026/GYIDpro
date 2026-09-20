# gyid — GyID 命令行工具

`draft-ayerbe-trip-protocol-04` 的命令行实现，二进制名为 `gyid`。提供两类命令：

- **用户命令**：身份创建、面包屑采集、Active Verification、PoH 查询
- **高级命令**（协议开发者）：密钥、面包屑、链、Epoch、PoH 原始签发/验签、仿真

## 构建

```bash
cargo build --release -p trip-cli
# 二进制位于 target/release/gyid
```

或直接通过 `cargo run`：

```bash
cargo run --release -p trip-cli -- <command>
```

## 用户命令

| 命令 | 说明 |
|------|------|
| `gyid init` | 生成新 GyID 身份（输出 seed + pubkey） |
| `gyid collect` | daemon 模式持续采集面包屑并上传 Verifier |
| `gyid verify` | RP 发起 Active Verification 并取回 PoH |
| `gyid poh-list` | 列出某 attester 已签发的 PoH |
| `gyid poh-show` | pretty-print PoH 证书全部字段 |

### 1. 创建身份

```bash
$ gyid init
seed    = 072fbf79eecc79216f7ae1503d686a1b214ba258291b387e1e36e44e24f7a55c
pubkey  = e79799f9ddeff4f59344ba0fc3575e031f4ea83f101aaa9982bfe1d24cc2cd2b
✓ 身份已创建，请保管好 seed
```

### 2. 持续采集面包屑

```bash
gyid collect --verifier http://localhost:8080 \
             --seed 072fbf79... \
             --interval 900
```

启动时自动从 Verifier 拉取当前链状态（index / chain_head / last_ts），按间隔
持续采集 → H3 res-10 量化 → Ed25519 签名 → 上传。Ctrl-C 停止。

> **GPS 说明**：桌面 Linux 通常无 GPS 硬件，`collect` 会提示无法定位并退出。
> 移动设备/嵌入式平台可在 `get_location()` 接入 geoclue2 / Android
> LocationManager / NMEA 串口。开发测试可用 `gyid-shared` 的
> `examples/seed_chain.rs` 生成合规链夹具。

### 3. 发起 Active Verification（RP 角色）

```bash
gyid verify --verifier http://localhost:8080 \
            --attester e79799f9... \
            # --rp-nonce 00112233...   # 不传则随机生成
```

流程：POST /v1/verify → 提示在 Attester 端签名（CLI 无法自动签名）→
轮询 POST /v1/poh（Pending 时 1s 重试）→ 取回 PoH → 本地 verify_poh
校验 Verifier 签名 + nonce 新鲜性 + 策略门槛 → 打印结果。

PoH hex 从 stdout 输出（可管道保存），人类可读信息走 stderr。

### 4. 列出 PoH

```bash
$ gyid poh-list --verifier http://localhost:8080 --attester e79799f9...
Attester: e79799f9...
已签发 PoH: 3 张
  [0] a1b2c3d4...
  [1] e5f6a7b8...
```

### 5. 查看 PoH 证书字段

```bash
gyid poh-show <poh_hex_cbor>
```

打印 identity / issued_at / alpha / beta / kappa / pi / confidence / trust /
unique_cells / breadcrumb_count / validity / nonce / chain_head / signature，
并给出 `meets_policy(0.1, 20.0)` 结论。

---

## Advanced（协议开发者命令）

| 命令 | 说明 |
|------|------|
| `gyid keygen` | 生成 Ed25519 密钥对（seed + pubkey） |
| `gyid breadcrumb sign` | 签名一条面包屑 |
| `gyid breadcrumb verify` | 验证面包屑签名 |
| `gyid chain <file>` | 验证面包屑链 |
| `gyid epoch seal` | 从面包屑封装 epoch |
| `gyid epoch verify` | 验证 epoch 签名 + Merkle 覆盖 |
| `gyid poh issue` | 签发 PoH 证书（Verifier 侧） |
| `gyid poh verify` | 验证 PoH 证书（RP 侧，带门槛） |
| `gyid simulate` | 端到端仿真（轨迹→面包屑→画像→PoH） |

高级命令示例与原 trip-cli 相同，详见 Git 历史。

## 数据格式约定

- **面包屑 / Epoch / PoH**：hex 编码的确定性 CBOR
- **链文件**：每行一条 hex CBOR 面包屑（支持空行）
- **密钥 seed**：64 hex 字符（32 字节）；**RP nonce**：32 hex 字符（16 字节）
- **stdout** 只输出机器可读数据（hex），**stderr** 输出进度/统计，便于管道拼接
- **退出码**：0 = 成功，1 = 验证失败或参数错误

## 与 gyid-shared / trip-core 的关系

用户命令的业务逻辑（身份、面包屑采集、PoH 校验、Verifier HTTP 客户端）由
`gyid-shared` 提供；CBOR 编解码、签名、哈希链、PSD/Levy/Hamiltonian/Trust
引擎由 `trip-core` 完成。`main.rs` 只做 clap 参数解析 + I/O + 调度。
