# GYIP-0001: GeoYuan Protocol Specification

**Status**: Draft  
**Version**: 0.1.0  
**Date**: 2026-04-19  
**Author**: SAM DATAMAN  

---

## 1. 概述

GeoYuan 是一个基于地理位置身份的联邦式区块链网络。每个运行 `geoyuan-gui` 的 Windows 设备即为一个轻量级网络节点，通过 P2P 协议参与区块生产和状态同步。

### 1.1 核心特性

- **Proof of Identity (PoI)**: 以 GyID 身份作为权益证明，非代币质押
- **联邦网络**: 节点可选择加入/退出特定联邦，联邦间通过桥接通信
- **分片存储**: 每个节点仅存储部分数据，通过 DHT 路由查询
- **零 Gas**: 用户无需支付代币即可铸造 GyID 和转账 GY
- **轻量级**: 8GB RAM 普通电脑可流畅运行

---

## 2. 网络架构

### 2.1 联邦模型 (Federated Network)

```
┌─────────────────────────────────────────────────────────────────┐
│                     GeoYuan Global Network                       │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │  Federation  │  │  Federation  │  │  Federation  │          │
│  │   "Asia"     │  │   "Europe"   │  │  "Americas"  │          │
│  │  (Zone: 1)   │  │  (Zone: 2)   │  │  (Zone: 3)   │          │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘          │
│         │                 │                 │                  │
│         └─────────────────┼─────────────────┘                  │
│                           │                                    │
│                    ┌──────▼──────┐                            │
│                    │  Bridge     │                            │
│                    │  Nodes      │                            │
│                    └─────────────┘                            │
└─────────────────────────────────────────────────────────────────┘
```

**联邦规则**:
- 每个联邦有独立的区块生产节奏
- 联邦内节点共享完整的联邦状态
- 跨联邦交易通过桥接节点中继
- 用户可选择加入多个联邦

### 2.2 节点类型

| 类型 | 描述 | 资源需求 |
|------|------|----------|
| **Full Node** | 存储完整联邦状态，参与共识 | 4GB+ RAM, 10GB+ 存储 |
| **Light Node** | 仅存储区块头和本地相关状态 | 512MB+ RAM, 1GB+ 存储 |
| **Bridge Node** | 连接多个联邦，跨链路由 | 8GB+ RAM, 50GB+ 存储 |
| **Bootstrap Node** | 帮助新节点发现网络 | 低，仅提供地址服务 |

**默认**: `geoyuan-gui` 以 Light Node 模式运行

---

## 3. 数据模型

### 3.1 区块结构

```rust
pub struct Block {
    /// 区块头
    pub header: BlockHeader,
    /// 交易列表
    pub transactions: Vec<Transaction>,
    /// 分片证明 (仅分片节点需要)
    pub shard_proofs: Vec<ShardProof>,
}

pub struct BlockHeader {
    /// 区块高度
    pub height: u64,
    /// 前一区块哈希
    pub prev_hash: Hash,
    /// 状态根 (Merkle Patricia Trie Root)
    pub state_root: Hash,
    /// 交易根
    pub tx_root: Hash,
    /// 时间戳 (毫秒)
    pub timestamp: u64,
    /// 生产者 GyID
    pub producer: GyID,
    /// VRF 证明 (随机选择证明)
    pub vrf_proof: VRFProof,
    /// 签名
    pub signature: Signature,
}
```

### 3.2 交易类型

```rust
pub enum Transaction {
    /// GyID 铸造
    MintGyID {
        gy_id: GyID,
        location: GeoLocation,
        timestamp: u64,
        proof: GyIDProof,
    },
    /// GY 转账
    Transfer {
        from: Address,
        to: Address,
        amount: u64,  // 最小单位: nanoGY (1 GY = 10^9 nanoGY)
        memo: Option<String>,
    },
    /// 设备关联
    LinkDevice {
        master: GyID,
        slave: GyID,
        proof: LinkProof,
    },
    /// 智能合约调用 (未来扩展)
    ContractCall {
        contract: Address,
        method: String,
        args: Vec<u8>,
    },
    /// NFT 铸造 (未来扩展)
    MintNFT {
        metadata: NFTMetadata,
        location: GeoLocation,
    },
}
```

### 3.3 账户状态

```rust
pub struct AccountState {
    /// 账户地址
    pub address: Address,
    /// GY 余额 (nanoGY)
    pub balance: u64,
    /// 关联的 GyID 列表
    pub gy_ids: Vec<GyID>,
    /// 交易计数 (nonce)
    pub nonce: u64,
    /// 存储的根哈希 (用于合约)
    pub storage_root: Option<Hash>,
}
```

---

## 4. 共识机制: Proof of Identity (PoI)

### 4.1 核心思想

以 **GyID 身份**作为权益证明，每个有效 GyID 代表一票。

**优势**:
- 无需购买代币即可参与共识
- 身份即权益，与 GeoYuan 定位一致
- 女巫攻击成本高（需要真实地理位置）

### 4.2 区块生产者选择

```rust
/// 使用 VRF (可验证随机函数) 选择生产者
pub fn select_producer(
    epoch: u64,
    valid_gyids: Vec<GyID>,
    seed: Hash,
) -> Option<GyID> {
    // 1. 计算每个 GyID 的 VRF 输出
    let mut candidates: Vec<(GyID, VRFOutput)> = valid_gyids
        .iter()
        .map(|gyid| {
            let output = vrf_evaluate(gyid.private_key, seed);
            (gyid.clone(), output)
        })
        .collect();
    
    // 2. 按 VRF 输出排序，取最小值
    candidates.sort_by(|a, b| a.1.cmp(&b.1));
    
    // 3. 返回获胜者
    candidates.first().map(|(gyid, _)| gyid.clone())
}
```

### 4.3 区块验证流程

1. **签名验证**: 验证区块签名是否来自声明的生产者
2. **VRF 验证**: 验证 VRF 证明有效且生产者确实被选中
3. **交易验证**: 验证所有交易的签名和状态转换
4. **状态更新**: 应用交易，计算新的状态根
5. **接受区块**: 如果状态根匹配，接受区块

### 4.4 惩罚机制

**作恶行为**:
- 双签 (在同一高度签署两个不同区块)
- 无效区块 (包含无效交易或错误状态转换)

**惩罚**:
- 首次: 警告，暂停 1 个 epoch 的出块资格
- 重复: 吊销相关 GyID 的共识参与权

---

## 5. 分片存储 (Sharded Storage)

### 5.1 分片策略

基于 **地理位置** 和 **账户地址** 双维度分片。

```rust
/// 计算数据所属分片
pub fn compute_shard(
    address: &Address,
    location: Option<&GeoLocation>,
    total_shards: u16,
) -> ShardId {
    // 1. 如果有地理位置，优先按地理分区
    if let Some(loc) = location {
        let zone = geo_to_zone(loc);  // Asia=1, Europe=2, Americas=3
        return ShardId(zone % total_shards);
    }
    
    // 2. 否则按地址哈希
    let hash = blake3_hash(address.as_bytes());
    ShardId(u16::from_le_bytes([hash[0], hash[1]]) % total_shards)
}
```

### 5.2 节点分片责任

```rust
/// 节点存储的分片集合
pub struct ShardResponsibility {
    /// 本节点存储的分片 ID 列表
    pub local_shards: Vec<ShardId>,
    /// 邻居节点 (用于路由查询)
    pub neighbors: Vec<PeerId>,
    /// DHT 路由表
    pub routing_table: KademliaDHT,
}

impl ShardResponsibility {
    /// 查询数据
    pub async fn query(&self, key: &Key) -> Result<Value, Error> {
        let shard = compute_shard_from_key(key);
        
        if self.local_shards.contains(&shard) {
            // 本地查询
            self.local_query(key).await
        } else {
            // 路由到负责节点
            self.route_query(shard, key).await
        }
    }
}
```

### 5.3 数据可用性保证

- **冗余**: 每个分片数据在 3 个不同节点冗余存储
- **纠删码**: 使用 Reed-Solomon 编码，允许部分节点离线
- **随机抽样**: 轻节点随机抽样验证数据可用性

---

## 6. P2P 网络协议

### 6.1 协议栈

```
┌─────────────────────────────────────┐
│         Application Layer           │
│    (Block Sync / Tx Gossip / DHT)   │
├─────────────────────────────────────┤
│         libp2p-gossipsub            │
│    (交易和区块传播)                  │
├─────────────────────────────────────┤
│         libp2p-kad                  │
│    (节点发现和 DHT 路由)             │
├─────────────────────────────────────┤
│         libp2p-noise                │
│    (加密传输)                        │
├─────────────────────────────────────┤
│         TCP / QUIC                  │
│    (传输层)                          │
└─────────────────────────────────────┘
```

### 6.2 GossipSub 配置

```rust
pub fn create_gossipsub_config() -> Config {
    Config::default()
        // 心跳间隔
        .heartbeat_interval(Duration::from_secs(1))
        // 消息验证模式: 严格
        .validation_mode(ValidationMode::Strict)
        // 消息ID函数: 基于内容寻址
        .message_id_fn(|msg| {
            MessageId::from(blake3_hash(&msg.data).as_bytes())
        })
        // 发布消息时自动转发给 mesh 节点
        .flood_publish(true)
        // mesh 网络参数
        .mesh_n(6)        // 目标 mesh 大小
        .mesh_n_low(4)    // 最小 mesh 大小
        .mesh_n_high(12)  // 最大 mesh 大小
        .gossip_lazy(6)   // 懒惰 gossip 数量
}
```

### 6.3 消息类型

```rust
pub enum NetworkMessage {
    /// 新交易
    NewTransaction(Transaction),
    /// 新区块
    NewBlock(Block),
    /// 区块请求
    BlockRequest { height: u64 },
    /// 区块响应
    BlockResponse(Block),
    /// 状态查询
    StateQuery { key: Key },
    /// 状态响应
    StateResponse { key: Key, value: Option<Value> },
    /// 分片数据请求
    ShardRequest { shard: ShardId, keys: Vec<Key> },
    /// 分片数据响应
    ShardResponse { shard: ShardId, data: Vec<(Key, Value)> },
}
```

---

## 7. RocksDB 存储设计

### 7.1 Column Families

```rust
pub const COLUMN_FAMILIES: &[&str] = &[
    // 区块数据
    "blocks",           // hash -> Block
    "headers",          // hash -> BlockHeader
    "heights",          // height -> hash
    
    // 状态数据
    "accounts",         // address -> AccountState
    "gyid_index",       // gyid -> address
    "gyid_metadata",    // gyid -> GyIDMetadata
    
    // 交易数据
    "transactions",     // tx_hash -> Transaction
    "tx_receipts",      // tx_hash -> TransactionReceipt
    "pending_txs",      // 待处理交易队列
    
    // 分片数据
    "shard_data",       // (shard, key) -> value
    "shard_assignments", // shard -> Vec<PeerId>
    
    // P2P 网络
    "peers",            // peer_id -> PeerInfo
    "peer_scores",      // peer_id -> ReputationScore
    
    // 元数据
    "metadata",         // 链配置、版本等
    "sync_state",       // 同步状态
];
```

### 7.2 存储接口

```rust
#[async_trait]
pub trait Storage: Send + Sync {
    /// 区块操作
    async fn put_block(&self, block: &Block) -> Result<(), Error>;
    async fn get_block(&self, hash: &Hash) -> Result<Option<Block>, Error>;
    async fn get_block_by_height(&self, height: u64) -> Result<Option<Block>, Error>;
    
    /// 状态操作
    async fn get_account(&self, address: &Address) -> Result<Option<AccountState>, Error>;
    async fn put_account(&self, address: &Address, state: &AccountState) -> Result<(), Error>;
    
    /// 交易操作
    async fn put_transaction(&self, tx: &Transaction) -> Result<(), Error>;
    async fn get_transaction(&self, hash: &Hash) -> Result<Option<Transaction>, Error>;
    
    /// 分片操作
    async fn put_shard_data(&self, shard: ShardId, key: &Key, value: &Value) -> Result<(), Error>;
    async fn get_shard_data(&self, shard: ShardId, key: &Key) -> Result<Option<Value>, Error>;
    
    /// 批量写入
    async fn batch_write(&self, batch: WriteBatch) -> Result<(), Error>;
}
```

---

## 8. 经济模型

### 8.1 GY 代币发行

**初始发行**:
- 每个新铸造的 GyID 获得 **1.0 GY** 奖励
- 无预挖，无团队分配

**通胀机制**:
- 每年通胀率: **5%**
- 通胀分配:
  - 50% 给区块生产者
  - 30% 给数据可用性提供者
  - 20% 给联邦治理基金

### 8.2 无 Gas 设计

**问题**: 如何防止垃圾交易？

**解决方案**:
1. **PoW 轻量证明**: 每笔交易需要计算简单的哈希难题 (难度可调)
2. **身份信用**: 新 GyID 有交易频率限制，老身份逐步放宽
3. **联邦配额**: 每个联邦有每日交易配额，防止单点拥堵

```rust
/// 交易 PoW 证明
pub struct TxPoW {
    pub nonce: u64,
    pub difficulty: u8,
    pub proof: Hash,
}

impl TxPoW {
    /// 验证 PoW
    pub fn verify(&self, tx_hash: &Hash) -> bool {
        let mut data = tx_hash.to_vec();
        data.extend_from_slice(&self.nonce.to_le_bytes());
        let hash = blake3_hash(&data);
        
        // 检查前 difficulty 位是否为 0
        let required_zeros = self.difficulty as usize;
        hash.as_bytes()[0..required_zeros/8].iter().all(|&b| b == 0)
    }
}
```

---

## 9. 安全考虑

### 9.1 女巫攻击防护

**攻击**: 攻击者创建大量虚假 GyID 控制网络

**防护**:
1. **硬件指纹**: 每个 GyID 绑定设备硬件特征
2. **地理位置**: 需要真实 GPS 坐标，同一地点大量注册触发风控
3. **时间成本**: GyID 铸造需要等待期，无法瞬间批量创建
4. **社交图谱**: 新 GyID 需要现有用户邀请才能参与共识

### 9.2 长程攻击防护

**攻击**: 攻击者从创世区块开始分叉，积累更多 GyID

**防护**:
1. **检查点**: 联邦定期发布不可篡改的检查点
2. **弱主观性**: 新节点需要获取可信的最新检查点
3. **罚没延迟**: 撤销共识权有延迟期，防止快速切换身份

---

## 10. 实现路线图

### Phase 1: 核心存储 (2 周)
- [ ] RocksDB 存储层实现
- [ ] 分片存储逻辑
- [ ] 数据序列化 (SCALE/Protobuf)

### Phase 2: P2P 网络 (3 周)
- [ ] libp2p 集成
- [ ] GossipSub 配置
- [ ] 节点发现 (mDNS + Bootstrap)
- [ ] 联邦加入/退出机制

### Phase 3: 共识引擎 (3 周)
- [ ] PoI 共识实现
- [ ] VRF 随机选择
- [ ] 区块生产和验证
- [ ] 惩罚机制

### Phase 4: 交易处理 (2 周)
- [ ] 交易池 (Mempool)
- [ ] 状态转换逻辑
- [ ] GyID 铸造上链
- [ ] GY 转账

### Phase 5: GUI 集成 (2 周)
- [ ] 钱包余额显示
- [ ] 转账界面
- [ ] 交易历史
- [ ] 节点状态监控

---

## 11. 参考实现

### 11.1 代码结构

```
geoyuan-core/
├── src/
│   ├── lib.rs
│   ├── storage/          # RocksDB 存储层
│   │   ├── mod.rs
│   │   ├── rocksdb.rs
│   │   └── shard.rs
│   ├── network/          # P2P 网络
│   │   ├── mod.rs
│   │   ├── p2p.rs
│   │   ├── gossip.rs
│   │   └── discovery.rs
│   ├── consensus/        # PoI 共识
│   │   ├── mod.rs
│   │   ├── poi.rs
│   │   ├── vrf.rs
│   │   └── validator.rs
│   ├── chain/            # 区块链核心
│   │   ├── mod.rs
│   │   ├── block.rs
│   │   ├── transaction.rs
│   │   └── state.rs
│   ├── vm/               # 虚拟机 (未来)
│   │   └── mod.rs
│   └── types/            # 共享类型
│       ├── mod.rs
│       ├── address.rs
│       ├── gyid.rs
│       └── hash.rs
└── Cargo.toml
```

---

## 附录 A: 术语表

| 术语 | 解释 |
|------|------|
| **GyID** | GeoYuan Identity，基于地理位置的数字身份 |
| **GY** | GeoYuan 代币，最小单位 nanoGY |
| **PoI** | Proof of Identity，身份权益证明 |
| **VRF** | Verifiable Random Function，可验证随机函数 |
| **DHT** | Distributed Hash Table，分布式哈希表 |
| **联邦** | Federation，自治的节点集合，有自己的区块生产 |
| **分片** | Shard，数据的逻辑分区 |

---

## 附录 B: 相关文档

- [GYIP-0002: GyID 铸造规范](./GYIP-0002-GyID-Minting.md) (待编写)
- [GYIP-0003: 联邦治理机制](./GYIP-0003-Federation-Governance.md) (待编写)
- [GYIP-0004: 跨联邦通信协议](./GYIP-0004-Cross-Federation.md) (待编写)

---

**License**: MIT  
**讨论区**: GitHub Issues
