# GeoTITRegistry — GyID / TRIP 链上锚定合约

`GeoTITRegistry.sol` 是 GYIP-0003 §5.3 的落地：把 **TRIP 身份的存在性**写到 EVM 链上，
供任何人独立查证。它 **不发币、不锚轨迹、不存任何位置数据**。

## 设计原则（对应 GYIP-0003 §3）

| # | 原则 | 在合约里的体现 |
|---|------|----------------|
| 1 | 链上只锚**存在性** | 只登记 `didSuffix`、epoch 的 `merkleRoot`、handle 绑定；没有 cell、没有照片、没有坐标 |
| 2 | 原始位置永不出端 | 链上数据全是聚合值（Merkle 根 + 计数） |
| 3 | **平台代付 gas** | 三个写函数全部由 `verifier` 地址门控（Verifier / 中继），普通用户不持币；`owner` 可轮换该地址 |
| 4 | 可升级性最小化 | 无代理、无自毁，只有 `owner` / `verifier` 两个可轮换地址 |

语义要点：

- `register(pubkey, didSuffix)`：把「Ed25519 公钥 → DID 后缀」写成**一次写定、不可更改**的绑定
  （重复登记直接 revert），防止身份根被置换；
- `anchorEpoch(pubkey, epochNo, merkleRoot, uniqueCells)`：epoch 序号必须**从 0 起严格连续**，
  重复提交即 revert（对应 GYIP-0003「同 epoch 重复提交即告警」）；
- `claimHandle(pubkey, name, breadcrumbs, trustX100)`：要求 `n ≥ 100` 且 `T ≥ 20`，且 handle
  **只能声明一次**（改名需要新的身份根），避免抢注与反复改名。

> 链上记录只是**索引与存在性证明**，不是真相来源。DID↔公钥映射、证书有效性一律由端上/RP
> 依据 `trip-core` 的确定性编码自行验证。

## 目录

```
contracts/
├── src/GeoTITRegistry.sol        # 合约本体
├── test/GeoTITRegistry.t.sol     # 行为测试（零外部依赖，不用 forge-std）
├── tools/
│   ├── compile.mjs               # solc-js 编译 → artifacts/*.json + 打印选择器
│   ├── test-inproc.mjs           # 在内存 EVM（@ethereumjs/vm）里跑上面的测试
│   └── deploy.mjs                # 部署到 Base Sepolia / 任意 EVM
├── artifacts/GeoTITRegistry.json # ABI + bytecode（入库，供 Rust CLI 直接用）
└── foundry.toml                  # forge 配置（solc 0.8.37 / evm paris / optimizer 200）
```

## 编译

两条等价路径（`solc 0.8.37` / `evm_version = paris` / `optimizer 200`）：

```bash
# A. 无 forge：solc-js
cd contracts && npm install && npm run compile
#   ✓ 编译通过  GeoTITRegistry runtime = 4076 bytes
#   选择器（keccak256 前 4 字节）:
#     register(bytes32,string)                    0xcf2d31fb
#     anchorEpoch(bytes32,uint64,bytes32,uint32)  0x4f65a1cc
#     claimHandle(bytes32,string,uint64,uint64)   0xa5188f93
#     epochKey(bytes32,uint64)                    0x06175e38
#     epochRoot(bytes32,uint64)                   0x16f6d768

# B. 有 forge
cd contracts && forge build
```

`evm_version = paris` 是刻意的保守选择：合约不使用 PUSH0 / TSTORE / MCOPY，
换来对老节点与内存 EVM 的最大兼容性，代价只是少量 gas。

## 测试

测试合约 `test/GeoTITRegistry.t.sol` **不依赖 forge-std**（用 `assert` +
一个 `Caller` 中间合约替代 `vm.prank`），因此同一份测试有两种跑法：

```bash
# A. 有 forge
cd contracts && forge test

# B. 无 forge：内存 EVM（@ethereumjs/vm，纯 JS，无需原生模块）
cd contracts && npm run test
```

覆盖：登记与重复登记、零公钥/空后缀拒绝、epoch 顺序与重复锚定、未登记锚定、
零 Merkle 根、handle 阈值（n=99 / T=19.99 / 恰好达标）、handle 只能声明一次与抢注、
非 verifier 写权限、owner 轮换 verifier、构造函数零地址。共 **12 个用例**。

## 部署到 Base Sepolia

```bash
cd contracts
PRIVATE_KEY=0x<部署者私钥> VERIFIER=0x<Verifier/中继地址> npm run deploy
# 或先干跑（不广播，只打印参数与 ABI 函数表）
PRIVATE_KEY=0x... node tools/deploy.mjs --dry-run
```

`VERIFIER` 缺省用部署者地址，之后可由 owner 用 `setVerifier` 轮换。脚本会：
校验 chainId（Base Sepolia = 84532）、检查余额、部署、轮询回执拿合约地址，
并打印 basescan 链接与后续锚定命令。

用 Foundry 的等价命令：

```bash
forge create src/GeoTITRegistry.sol:GeoTITRegistry \
  --rpc-url https://sepolia.base.org --private-key $PRIVATE_KEY \
  --constructor-args 0x<Verifier地址> --broadcast
```

## 锚定 epoch（Rust CLI）

`trip-core::anchor` 负责 ABI 编码与 EIP-1559 签名（纯计算、可单测），
`trip-cli` 的 `gyid anchor` 负责 JSON-RPC。付费密钥与身份密钥**相互独立**：
身份根是 Ed25519（`trip-core::ProtocolKey`），付费密钥是 secp256k1（`EVM_PRIVATE_KEY`）。

```bash
# 0. 查看已知网络
gyid anchor networks

# 1. 部署（读 contracts/artifacts/GeoTITRegistry.json）
gyid anchor deploy --rpc https://sepolia.base.org --verifier 0x<Verifier地址>
#    私钥来源：--key <hex> 或环境变量 EVM_PRIVATE_KEY（推荐）

# 2. 登记身份（首次绑 DID 后缀，缺省由公钥派生）
gyid anchor register --rpc https://sepolia.base.org --registry 0x<Registry> \
  --pubkey <64 hex Ed25519 公钥>

# 3. 锚定一个 epoch 的 Merkle 根（序号从 0 起连续）
gyid anchor epoch --rpc https://sepolia.base.org --registry 0x<Registry> \
  --pubkey <64 hex> --epoch 0 --merkle <64 hex Merkle root> --unique-cells 37

# 4. 声明 handle（n≥100 且 T≥20；trust-x100 = T×100）
gyid anchor handle --rpc https://sepolia.base.org --registry 0x<Registry> \
  --pubkey <64 hex> --name huanghe --breadcrumbs 300 --trust-x100 6250

# 加 --dry-run 只构造并打印裸交易，不广播
```

## 查证

```bash
# 链上读 epoch 根（eth_call，无需私钥）
gyid anchor epoch-root --rpc https://sepolia.base.org --registry 0x<Registry> \
  --pubkey <64 hex> --epoch 0

# 交易回执
gyid anchor status --rpc https://sepolia.base.org 0x<tx hash>

# 浏览器
#   https://sepolia.basescan.org/address/0x<Registry>
```

`epochRoot(pubkey, epochNo)` 的存储键与合约 `epochKey` 完全一致：
`keccak256(abi.encode(bytes32 pubkey, uint256 epochNo))`（用 `abi.encode` 而非
`encodePacked`，两侧都是规整的 32 字节字，避免打包编码陷阱）。

## 与 `trip-core::anchor` 的一致性

选择器、`epochKey`、地址推导、EIP-1559 签名都做过**三个独立实现**的交叉验证：

| 项 | 三方来源 | 结果 |
|----|---------|------|
| keccak256 | Rust `sha3` / pycryptodomex / ethers | 一致（`keccak("")`、`keccak("abc")` 向量） |
| 选择器 | Rust `selector_of` / pycryptodomex / solc+ethers | 一致（见上表） |
| `epochKey` | 合约 `keccak256(abi.encode(...))` / Rust / pycryptodomex | 一致（黄金向量） |
| 地址推导 | Rust `k256` / 纯 Python secp256k1 | 一致（含 Hardhat #0 权威向量） |
| EIP-1559 签名 | Rust `k256` + 手写 RLP / 纯 Python RFC6979 + 手写 RLP | 逐字节一致（含 low-S、yParity） |

Rust 侧黄金向量固化在 `trip-core/src/anchor.rs` 的测试里。

## 边界与未做

- **未上主网、未审计**：当前仅面向 Base Sepolia 验证；主网部署前需审计与多签 owner；
- **owner 中心化**：`owner` 可轮换 `verifier`，这是刻意的运维简化（无代理、无治理代币），
  代价是 owner 私钥的保管责任；
- **handle 不可改**：声明一次后不可改名（保持「不可撤销」语义），改名需新身份根；
- **不含吊销**：证书吊销注册表属于远期项（GYIP-0003 §5.3「远期」）；
- 合约不校验 DID 后缀与公钥是否匹配（链上无法廉价计算 base58），该一致性由
  端上/RP 依据 `trip-core::did` 验证。
