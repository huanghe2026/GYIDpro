# trip-cli — TRIP 协议命令行工具

`draft-ayerbe-trip-protocol-04` 的命令行实现。封装 `trip-core` 的全部能力：
密钥生成、面包屑签名/验签、链验证、Epoch 封装、PoH 证书签发/验签、端到端仿真。

## 构建

```bash
cargo build --release -p trip-cli
# 二进制位于 target/release/trip-cli
```

或直接通过 `cargo run`：

```bash
cargo run --release -p trip-cli -- <command>
```

## 命令一览

| 命令 | 说明 |
|------|------|
| `trip keygen` | 生成 Ed25519 密钥对（seed + pubkey） |
| `trip breadcrumb sign` | 签名一条面包屑 |
| `trip breadcrumb verify` | 验证面包屑签名 |
| `trip chain <file>` | 验证面包屑链 |
| `trip epoch seal` | 从面包屑封装 epoch |
| `trip epoch verify` | 验证 epoch 签名 + Merkle 覆盖 |
| `trip poh issue` | 签发 PoH 证书（Verifier 侧） |
| `trip poh verify` | 验证 PoH 证书（RP 侧） |
| `trip simulate` | 端到端仿真（轨迹→面包屑→画像→PoH） |

## 数据格式

- **面包屑**：hex 编码的确定性 CBOR，stdout 输出
- **链文件**：每行一条 hex CBOR 面包屑（支持空行）
- **Epoch/PoH**：hex 编码 CBOR
- **密钥 seed**：64 个 hex 字符（32 字节）
- **block_hash/pubkey**：输出到 stderr，不干扰 stdout 管道

## 使用示例

### 1. 生成密钥

```bash
$ trip keygen
seed     = 072fbf79eecc79216f7ae1503d686a1b214ba258291b387e1e36e44e24f7a55c
pubkey   = e79799f9ddeff4f59344ba0fc3575e031f4ea83f101aaa9982bfe1d24cc2cd2b
# 保管 seed，它是你的身份私钥。pubkey 可公开。
```

### 2. 签名面包屑并构建链

```bash
SEED="2a2a2a...2a"  # 64 hex chars

# 创世面包屑（无 prev_hash）
BC0=$(trip breadcrumb sign --seed $SEED --index 0 --timestamp 1700000000 --cell 1000 --resolution 10 2>/dev/null)
# stderr: block_hash = <hash0>

# 第二条（prev = hash0）
BC1=$(trip breadcrumb sign --seed $SEED --index 1 --timestamp 1700000900 --cell 1001 --prev <hash0> 2>/dev/null)

# 写入链文件
echo -e "$BC0\n$BC1" > chain.hex
```

### 3. 验证链

```bash
$ trip chain chain.hex
验证 2 条面包屑…… OK
index 范围: 0..=1
身份公钥: 197f6b23e16c8532c6abc838facd5ea789be0c76b2920334029bfa8b3d368d61
链头哈希: 2c2a30f125bdc42b5c6f64005df5ea7923929f646eb69c2fbdffabee63f8ae82
```

### 4. 封装 Epoch

```bash
$ cat chain.hex | trip epoch seal --seed $SEED --number 0
a90000015820197f6b23e...
epoch 0  index 0..=1  unique_cells=2  merkle=a8aa7a190624fb28...
```

### 5. 验证 Epoch

```bash
$ trip epoch verify <epoch_hex> chain.hex
验证 epoch 0 签名…… OK
验证 Merkle 覆盖…… OK
```

### 6. 签发 PoH 证书

```bash
$ trip poh issue \
    --verifier-seed <verifier_seed> \
    --identity <pubkey_hex> \
    --alpha 0.59 --beta 1.72 --kappa 5.0 \
    --confidence 0.15 --trust 80.0 \
    --unique-cells 3 --breadcrumb-count 3 \
    --chain-head <chain_head_hex>
af005820197f6b23e...
PoH 证书已签发（236 字节 CBOR）
```

### 7. 验证 PoH 证书

```bash
$ trip poh verify <poh_hex> \
    --verifier-pubkey <verifier_pubkey_hex> \
    --nonce 000102030405060708090a0b0c0d0e0f \
    --now 1700256001
验签 + 新鲜性…… OK
策略检查…… PASS
α=0.5900  confidence=0.1500  trust=80.00  unique=3  crumbs=3
```

### 8. 端到端仿真

```bash
$ trip simulate --seed 2024 --count 512
=== TRIP 仿真（seed=2024, crumbs=512）===
面包屑链:  513 条,  unique cells: 407
PSD:       α = 0.6147  (R² = 0.1780, class = pink)
Levy MLE:  β = 3.0000  κ = 75.5888
Hamiltonian: H = 3.1765  (alert = suspicious, baseline = -2.5499)
  spatial=1.582  temporal=6.217  kinetic=6.908  flock=0.373  contextual=0.000  structure=1.000
Trust:     T = 80.05  (α∈bio = true, handle_eligible = true)
PoH:       236 字节 CBOR
RP 验签:   signature OK, confidence = 0.1319, policy = PASS

=== PASS ===
```

## 设计约定

- **stdout 只输出机器可读数据**（hex CBOR），便于管道拼接
- **stderr 输出人类可读信息**（block_hash、pubkey、进度、统计）
- **退出码**：0 = 成功，1 = 验证失败或参数错误
- **确定性**：`simulate` 的 seed 固定时输出完全可复现

## 与 trip-core 的关系

`trip-cli` 是 `trip-core` 协议库的薄封装，不引入额外的协议逻辑。所有 CBOR 编解码、签名、哈希、PSD/Levy/Hamiltonian/Trust 计算均由 `trip-core` 完成，CLI 仅负责参数解析与 I/O。
