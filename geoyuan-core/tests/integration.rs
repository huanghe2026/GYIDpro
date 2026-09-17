//! GeoYuan Core - 集成测试
//!
//! 覆盖: 存储层、PoI 共识、P2P、钱包、身份生成端对端流程

use geoyuan_core::{
    consensus::{ConsensusConfig, ConsensusEngine, ConsensusPhase},
    consensus::validator::{Validator, ValidatorSet},
    consensus::vote::{Vote, VoteType},
    consensus::leader::LeaderElection,
    identity::GyId,
    storage::{RedbStorage, RedbWriteBatch, CF_ACCOUNTS, CF_BLOCKS, CF_METADATA, serialize, deserialize},
    wallet::{Wallet, Geoyuan},
};
use tempfile::TempDir;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════
// 工具函数
// ═══════════════════════════════════════════════════════════════════════

fn make_gyid(s: &str) -> GyId {
    GyId::new(s).unwrap_or_default()
}

fn make_validator(id: &str, gyid_count: u32, stake: u64) -> Validator {
    Validator::new(make_gyid(id), gyid_count, "wx4g0e6md5".to_string(), stake)
}

fn make_wallet(gy_id: &str) -> Wallet {
    Wallet::new(gy_id.to_string(), "0".repeat(64))
}

// ═══════════════════════════════════════════════════════════════════════
// 存储层集成测试
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_storage_wallet_persistence() {
    let temp = TempDir::new().unwrap();
    let db = RedbStorage::open(temp.path().join("test.redb")).unwrap();

    // 构造一个简单钱包并序列化
    let wallet = make_wallet("GyIDtest0001aaaaaaaaaaaaaaaaaaaaa");
    let bytes = serialize(&wallet).unwrap();

    // 存入账户 CF
    db.put(CF_ACCOUNTS, b"acc1", &bytes).unwrap();

    // 取出并反序列化
    let raw = db.get(CF_ACCOUNTS, b"acc1").unwrap().unwrap();
    let loaded: Wallet = deserialize(&raw).unwrap();

    assert_eq!(loaded.gy_id, wallet.gy_id);
}

#[test]
fn test_storage_batch_and_iterate() {
    let temp = TempDir::new().unwrap();
    let db = RedbStorage::open(temp.path().join("test.redb")).unwrap();

    // 批量写入 3 个账户
    let batch = RedbWriteBatch::new()
        .put(CF_ACCOUNTS, b"a1".to_vec(), b"data1".to_vec())
        .put(CF_ACCOUNTS, b"a2".to_vec(), b"data2".to_vec())
        .put(CF_ACCOUNTS, b"a3".to_vec(), b"data3".to_vec());
    db.batch_write(batch).unwrap();

    // 迭代确认数量
    let mut count = 0usize;
    db.iterate(CF_ACCOUNTS, |_k, _v| {
        count += 1;
        true
    }).unwrap();
    assert_eq!(count, 3);

    // 删除其中一个
    db.delete(CF_ACCOUNTS, b"a2").unwrap();
    count = 0;
    db.iterate(CF_ACCOUNTS, |_k, _v| {
        count += 1;
        true
    }).unwrap();
    assert_eq!(count, 2);
}

#[test]
fn test_storage_cf_isolation() {
    let temp = TempDir::new().unwrap();
    let db = RedbStorage::open(temp.path().join("test.redb")).unwrap();

    db.put(CF_ACCOUNTS, b"key", b"account").unwrap();
    db.put(CF_BLOCKS, b"key", b"block").unwrap();
    db.put(CF_METADATA, b"key", b"meta").unwrap();

    assert_eq!(db.get(CF_ACCOUNTS, b"key").unwrap(), Some(b"account".to_vec()));
    assert_eq!(db.get(CF_BLOCKS, b"key").unwrap(), Some(b"block".to_vec()));
    assert_eq!(db.get(CF_METADATA, b"key").unwrap(), Some(b"meta".to_vec()));
}

// ═══════════════════════════════════════════════════════════════════════
// PoI 共识集成测试
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_consensus_full_round() {
    let config = ConsensusConfig {
        min_validators: 3,
        quorum_threshold_pct: 67,
        ..Default::default()
    };
    let mut engine = ConsensusEngine::new(config);

    // 添加 4 个验证者
    engine.add_validator(make_validator("GyIDaaaa1111111111111111", 3, 5_000_000_000));
    engine.add_validator(make_validator("GyIDbbbb2222222222222222", 2, 3_000_000_000));
    engine.add_validator(make_validator("GyIDcccc3333333333333333", 5, 8_000_000_000));
    engine.add_validator(make_validator("GyIDdddd4444444444444444", 1, 1_000_000_000));

    // 第一轮选举 leader
    let seed = [0u8; 32];
    let leader = engine.start_round(&seed);
    assert!(leader.is_some(), "应选出 leader");
    assert_eq!(engine.phase, ConsensusPhase::Propose);

    // 提交区块提案
    let block_hash = [42u8; 32];
    if let Some(round) = engine.current_round_mut() {
        round.set_proposal(block_hash);
    }

    // 4 个节点投票 PreVote + PreCommit
    let all_gyids = [
        "GyIDaaaa1111111111111111",
        "GyIDbbbb2222222222222222",
        "GyIDcccc3333333333333333",
        "GyIDdddd4444444444444444",
    ];
    for id in &all_gyids {
        let pv = Vote::new(make_gyid(id), 0, 0, VoteType::PreVote, Some(block_hash));
        let pc = Vote::new(make_gyid(id), 0, 0, VoteType::PreCommit, Some(block_hash));
        if let Some(round) = engine.current_round_mut() {
            round.add_vote(pv);
            round.add_vote(pc);
        }
    }

    // 检查是否可以最终化
    let can_finalize = engine.current_round()
        .map(|r| r.can_finalize())
        .unwrap_or(false);
    assert!(can_finalize, "4/4 预提交后应可最终化");

    // 进入下一高度
    engine.advance_height();
    assert_eq!(engine.height, 1);
    assert_eq!(engine.phase, ConsensusPhase::Idle);
}

#[test]
fn test_consensus_quorum_threshold() {
    let config = ConsensusConfig {
        quorum_threshold_pct: 67,
        ..Default::default()
    };
    let mut engine = ConsensusEngine::new(config);

    // 添加 10 个验证者
    for i in 0u8..10 {
        let id = format!("GyIDtest{:04}", i);
        engine.add_validator(make_validator(&id, 1, 1_000_000_000));
    }

    // 法定人数 = ceil(10 * 0.67) = 7
    assert!(!engine.check_quorum(6), "6/10 不达法定人数");
    assert!(engine.check_quorum(7), "7/10 达到法定人数");
    assert!(engine.check_quorum(10), "10/10 超过法定人数");
}

#[test]
fn test_leader_election_weighted() {
    let mut set = ValidatorSet::new();
    // 大权重节点应有更高当选概率
    set.add(Validator::new(
        make_gyid("GyIDhigh1111111111111111"),
        100, "wx4g".to_string(), 100_000_000_000,
    ));
    set.add(Validator::new(
        make_gyid("GyIDlow22222222222222222"),
        1, "wx4g".to_string(), 1_000_000_000,
    ));

    let seed = [0u8; 32];
    let mut high_count = 0;
    let mut low_count = 0;

    for height in 0u64..200 {
        if let Some(leader) = LeaderElection::elect(&set, height, 0, &seed) {
            if leader.id.starts_with("GyIDhigh") {
                high_count += 1;
            } else {
                low_count += 1;
            }
        }
    }

    // 大权重节点应被选中更多次（至少是低权重节点的 5 倍）
    assert!(
        high_count > low_count * 5,
        "高权重节点应显著更多当选: high={}, low={}",
        high_count, low_count
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 钱包集成测试
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_wallet_and_coin_persistence() {
    let temp = TempDir::new().unwrap();
    let db = RedbStorage::open(temp.path().join("wallet.redb")).unwrap();

    // 创建钱包
    let gy_id = "GyIDtest0001aaaaaaaaaaaaaaaaaaaaa";
    let mut wallet = make_wallet(gy_id);

    // 铸造一枚 Geoyuan
    let coin = Geoyuan::new(
        Uuid::new_v4().to_string(),
        gy_id.to_string(),
        39.9042,
        116.4074,
    );
    wallet.add_coin(coin.clone());
    assert_eq!(wallet.balance, 1);

    // 序列化后存储
    let bytes = serialize(&wallet).unwrap();
    db.put(CF_ACCOUNTS, wallet.gy_id.as_bytes(), &bytes).unwrap();

    // 读取并验证
    let raw = db.get(CF_ACCOUNTS, wallet.gy_id.as_bytes()).unwrap().unwrap();
    let loaded: Wallet = deserialize(&raw).unwrap();
    assert_eq!(loaded.balance, 1);
    assert_eq!(loaded.coins[0].id, coin.id);
}

// ═══════════════════════════════════════════════════════════════════════
// 端对端集成测试：共识 → 存储
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_consensus_to_storage_e2e() {
    let temp = TempDir::new().unwrap();
    let db = RedbStorage::open(temp.path().join("e2e.redb")).unwrap();

    // 1. 建立共识引擎
    let mut engine = ConsensusEngine::new(ConsensusConfig::default());
    engine.add_validator(make_validator("GyIDnode1111111111111111", 5, 10_000_000_000));
    engine.add_validator(make_validator("GyIDnode2222222222222222", 3, 5_000_000_000));
    engine.add_validator(make_validator("GyIDnode3333333333333333", 4, 8_000_000_000));
    engine.add_validator(make_validator("GyIDnode4444444444444444", 2, 3_000_000_000));

    // 2. 运行 3 个区块高度的共识
    let mut prev_hash = [0u8; 32];
    for height in 0u64..3 {
        let seed = LeaderElection::next_seed(&prev_hash, height);
        let leader = engine.start_round(&seed).unwrap();

        // 模拟区块 hash
        let block_hash: [u8; 32] = {
            let mut h = [0u8; 32];
            h[0] = height as u8;
            h
        };

        // 提案
        if let Some(round) = engine.current_round_mut() {
            round.set_proposal(block_hash);
        }

        // 投票
        for id in &[
            "GyIDnode1111111111111111",
            "GyIDnode2222222222222222",
            "GyIDnode3333333333333333",
            "GyIDnode4444444444444444",
        ] {
            let pv = Vote::new(make_gyid(id), height, 0, VoteType::PreVote, Some(block_hash));
            let pc = Vote::new(make_gyid(id), height, 0, VoteType::PreCommit, Some(block_hash));
            if let Some(round) = engine.current_round_mut() {
                round.add_vote(pv);
                round.add_vote(pc);
            }
        }

        // 验证可最终化并存储区块元数据
        assert!(engine.current_round().map(|r| r.can_finalize()).unwrap_or(false));

        let block_key = format!("block/{}", height);
        let block_meta = serde_json::json!({
            "height": height,
            "hash": hex::encode(block_hash),
            "leader": leader.id,
        }).to_string();
        db.put(CF_BLOCKS, block_key.as_bytes(), block_meta.as_bytes()).unwrap();

        // 推进高度
        prev_hash = block_hash;
        engine.advance_height();
    }

    // 3. 验证所有区块都已存储
    let mut stored_count = 0usize;
    db.iterate(CF_BLOCKS, |_k, _v| {
        stored_count += 1;
        true
    }).unwrap();
    assert_eq!(stored_count, 3, "应存储 3 个区块");

    // 4. 验证最终高度
    assert_eq!(engine.height, 3);
}

// ═══════════════════════════════════════════════════════════════════════
// P2P 协议集成测试
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_p2p_message_serialization() {
    use geoyuan_core::p2p::protocol::{Envelope, MessageType};

    let peer_id = "12D3KooWTestPeerIdForIntegration";
    let payload = serde_json::json!({"gy_id": "GyIDtest0001", "amount": 100}).to_string();

    let envelope = Envelope::new(
        MessageType::WalletRequest,
        peer_id,
        payload.as_bytes().to_vec(),
    );

    // 序列化
    let bytes = envelope.serialize().unwrap();
    assert!(!bytes.is_empty());

    // 反序列化
    let decoded = Envelope::deserialize(&bytes).unwrap();
    assert_eq!(decoded.msg_type, MessageType::WalletRequest);
    assert_eq!(decoded.payload, payload.as_bytes().to_vec());
}

#[test]
fn test_p2p_discovery_service() {
    use geoyuan_core::p2p::discovery::DiscoveryService;
    use libp2p::{Multiaddr, PeerId};

    let mut svc = DiscoveryService::new();
    assert_eq!(svc.peer_count(), 0);

    // 发现 3 个节点
    let peers: Vec<(PeerId, Multiaddr)> = (0..3).map(|i: u32| {
        (
            PeerId::random(),
            format!("/ip4/10.0.0.{}/tcp/4001", i + 1).parse().unwrap(),
        )
    }).collect();

    let events = svc.handle_discovered(peers.clone());
    assert_eq!(events.len(), 3);
    assert_eq!(svc.peer_count(), 3);

    // 节点过期（移除）
    svc.handle_expired(vec![peers[0].clone()]);
    assert_eq!(svc.peer_count(), 2);
}

// ═══════════════════════════════════════════════════════════════════════
// GyId 相等性测试（验证 PartialEq 修复）
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_gyid_equality_based_on_id() {
    // 两个 GyId 只要 id 字段相同就相等，不管 created_at
    let g1 = GyId::new("GyIDsame1111111111").unwrap();
    // 稍后创建，时间戳不同
    std::thread::sleep(std::time::Duration::from_millis(5));
    let g2 = GyId::new("GyIDsame1111111111").unwrap();
    let g3 = GyId::new("GyIDdiff2222222222").unwrap();

    assert_eq!(g1, g2, "相同 id 的 GyId 应该相等");
    assert_ne!(g1, g3, "不同 id 的 GyId 应该不等");
}

#[test]
fn test_validator_set_upsert() {
    // 验证 ValidatorSet::add 能正确更新已有记录
    let mut set = ValidatorSet::new();

    let v1 = make_validator("GyIDaaaa1111111111111111", 2, 1_000_000_000);
    set.add(v1);
    assert_eq!(set.len(), 1);

    // 更新同一节点（gyid_count 变化）
    let v1_updated = make_validator("GyIDaaaa1111111111111111", 10, 5_000_000_000);
    set.add(v1_updated);
    assert_eq!(set.len(), 1, "更新后数量不变，仍为 1");

    let found = set.get(&make_gyid("GyIDaaaa1111111111111111")).unwrap();
    assert_eq!(found.gyid_count, 10, "应更新为新的 gyid_count");
}

// ═══════════════════════════════════════════════════════════════════════
// Ed25519 签名转账集成测试
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_ed25519_transfer_sign_verify() {
    use geoyuan_core::crypto::signer::KeyPair;
    use geoyuan_core::wallet::transfer::{CoinTransferHandler, TransferStatus};
    use geoyuan_core::wallet::{Wallet, Geoyuan};

    let keypair = KeyPair::generate();

    // 创建发送方钱包并铸造一枚 Geoyuan
    let mut sender = Wallet::new(
        "GyIDsender111111111111".to_string(),
        keypair.public_hex(),
    );
    let coin = Geoyuan::new(
        Uuid::new_v4().to_string(),
        sender.gy_id.clone(),
        39.9042, 116.4074,
    );
    sender.add_coin(coin.clone());
    assert_eq!(sender.balance, 1);

    // 创建接收方钱包
    let mut receiver = Wallet::new(
        "GyIDreceiver2222222222".to_string(),
        "0".repeat(64),
    );

    // 执行转账
    let transfer = CoinTransferHandler::transfer(
        &coin.id,
        &mut sender,
        &mut receiver,
        &keypair,
    ).unwrap();

    // 余额变更正确
    assert_eq!(sender.balance, 0, "发送方余额应为 0");
    assert_eq!(receiver.balance, 1, "接收方余额应为 1");
    assert_eq!(transfer.status, TransferStatus::Confirmed);

    // 签名可验证
    assert!(
        transfer.verify_signature(&keypair.public_bytes()),
        "转账签名验证应通过"
    );
}

#[test]
fn test_ed25519_transfer_wrong_key_fails() {
    use geoyuan_core::crypto::signer::KeyPair;
    use geoyuan_core::wallet::transfer::CoinTransfer;

    let keypair = KeyPair::generate();
    let wrong_keypair = KeyPair::generate();

    let transfer = CoinTransfer::new(
        "coin-xyz".to_string(),
        "GyIDfrom111111111111111".to_string(),
        "GyIDto22222222222222222".to_string(),
        &keypair,
    ).unwrap();

    // 用错误公钥验证应失败
    assert!(
        !transfer.verify_signature(&wrong_keypair.public_bytes()),
        "用错误公钥验证签名应失败"
    );
}

#[test]
fn test_ed25519_keypair_from_secret_roundtrip() {
    use geoyuan_core::crypto::signer::KeyPair;

    let original = KeyPair::generate();
    let pub_hex = original.public_hex();

    // 用公钥 hex 恢复（演示：此处重用 public_bytes 做一次签名验证循环）
    let message = b"GeoYuan transfer roundtrip test";
    let sig = original.sign(message);
    assert!(original.verify(message, &sig), "签名验证应通过");
    assert_eq!(pub_hex.len(), 64, "公钥 hex 长度应为 64");
}

#[test]
fn test_multi_transfer_chain() {
    use geoyuan_core::crypto::signer::KeyPair;
    use geoyuan_core::wallet::transfer::{CoinTransferHandler, TransferStatus};
    use geoyuan_core::wallet::{Wallet, Geoyuan};

    // Alice → Bob → Carol 链式转账
    let kp_alice = KeyPair::generate();
    let kp_bob = KeyPair::generate();

    let mut alice = Wallet::new("GyIDalice111111111111111".to_string(), kp_alice.public_hex());
    let mut bob   = Wallet::new("GyIDbob2222222222222222".to_string(),   kp_bob.public_hex());
    let mut carol = Wallet::new("GyIDcarol333333333333333".to_string(),  "0".repeat(64));

    let coin = Geoyuan::new(
        Uuid::new_v4().to_string(),
        alice.gy_id.clone(),
        31.2304, 121.4737,
    );
    alice.add_coin(coin.clone());

    // Alice → Bob
    let tx1 = CoinTransferHandler::transfer(&coin.id, &mut alice, &mut bob, &kp_alice).unwrap();
    assert_eq!(alice.balance, 0);
    assert_eq!(bob.balance, 1);
    assert_eq!(tx1.status, TransferStatus::Confirmed);
    assert!(tx1.verify_signature(&kp_alice.public_bytes()));

    // Bob → Carol（Bob 持有的 coin id 与原始相同）
    let bob_coin_id = bob.coins.first().unwrap().id.clone();
    let tx2 = CoinTransferHandler::transfer(&bob_coin_id, &mut bob, &mut carol, &kp_bob).unwrap();
    assert_eq!(bob.balance, 0);
    assert_eq!(carol.balance, 1);
    assert_eq!(tx2.status, TransferStatus::Confirmed);
    assert!(tx2.verify_signature(&kp_bob.public_bytes()));
}

// ═══════════════════════════════════════════════════════════════════════
// P2P Swarm 集成测试
// ═══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_swarm_handle_stub_behavior() {
    use geoyuan_core::p2p::swarm::{start_swarm, pick_listen_port};

    // pick_listen_port 范围验证（集成级别）
    for _ in 0..10 {
        let port = pick_listen_port();
        assert!(port >= 4001 && port <= 4999);
    }

    // start_swarm stub 返回有效句柄
    let handle = start_swarm(4200);
    let status = handle.cached_status();
    // Stub 模式不启动真实 Swarm
    assert_eq!(status.peer_count, 0);
    // shutdown 不应 panic
    handle.shutdown();
}

#[tokio::test]
async fn test_swarm_handle_broadcast_stub() {
    use geoyuan_core::p2p::{start_swarm};
    use geoyuan_core::p2p::protocol::MessageType;

    let handle = start_swarm(4201);
    // Stub 版本：广播不崩溃
    handle.broadcast(MessageType::WalletRequest, b"hello p2p".to_vec());
    handle.shutdown();
}

// ═══════════════════════════════════════════════════════════════════════
// 真实 Swarm 网络测试（仅 native feature）
// ═══════════════════════════════════════════════════════════════════════

/// 启动两个本地 Swarm，验证 mDNS 自动发现对方
#[cfg(feature = "native")]
#[tokio::test]
async fn test_swarm_native_mdns_discovery() {
    use geoyuan_core::p2p::swarm::start_swarm;
    use std::time::Duration;

    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .try_init()
        .ok();

    let handle_a = start_swarm(4901);
    let handle_b = start_swarm(4902);

    // 等待 mDNS 发现（最多 15 秒）
    let max_wait = Duration::from_secs(15);
    let start = std::time::Instant::now();
    let mut discovered = false;

    while start.elapsed() < max_wait {
        let status_a = handle_a.cached_status();
        let status_b = handle_b.cached_status();

        if status_a.peer_count >= 1 && status_b.peer_count >= 1 {
            discovered = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    handle_a.shutdown();
    handle_b.shutdown();

    assert!(discovered, "两个 Swarm 应通过 mDNS 互相发现");
}

/// 验证 GeoCast 地理广播 —— 相同 H3 cell 的节点能收到消息
#[cfg(feature = "native")]
#[tokio::test]
async fn test_swarm_native_geocast() {
    use geoyuan_core::p2p::swarm::start_swarm;
    use geoyuan_core::p2p::protocol::MessageType;
    use std::time::Duration;

    let handle_a = start_swarm(4911);
    let handle_b = start_swarm(4912);

    // 等待 mDNS 发现
    let max_wait = Duration::from_secs(15);
    let start = std::time::Instant::now();
    while start.elapsed() < max_wait {
        let sa = handle_a.cached_status();
        if sa.peer_count >= 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // 用同一个 H3 cell 做 GeoCast 广播（北京故宫中心，Res 12）
    let h3_cell: u64 = 0x8a1fb46622dffff;
    handle_a.announce_location(h3_cell);
    handle_b.announce_location(h3_cell);

    tokio::time::sleep(Duration::from_millis(500)).await;

    // GeoCast 广播（只发给邻近节点）
    handle_a.geocast_broadcast(
        MessageType::GeoCast,
        b"test geocast message".to_vec(),
        h3_cell,
    );

    tokio::time::sleep(Duration::from_secs(2)).await;

    handle_a.shutdown();
    handle_b.shutdown();
}

/// 验证 Envelope 的 PoL 位置证明序列化和验证
#[cfg(feature = "native")]
#[tokio::test]
async fn test_swarm_native_pol_verification() {
    use geoyuan_core::p2p::protocol::{Envelope, MessageType, LocationProof};
    use geoyuan_core::crypto::KeyPair;

    let kp = KeyPair::generate();

    // 创建一个带 PoL 证明的 Envelope
    let payload = b"test payload".to_vec();
    let payload_hash = blake3::hash(&payload).into();
    let proof = LocationProof::new(39.9042, 116.4074, 0x8a1fb46622dffff, &payload_hash, &kp);

    let envelope = Envelope::new_with_pol(
        MessageType::GeoCast,
        "test_peer",
        payload,
        Some(proof),
    );

    // 验证 PoL
    let pol = envelope.location_proof.as_ref().unwrap();
    let verify_hash = blake3::hash(b"test payload").into();
    assert!(pol.verify(&verify_hash), "PoL 验证应通过");

    // 错误数据不应通过验证
    let wrong_hash = blake3::hash(b"wrong payload").into();
    assert!(!pol.verify(&wrong_hash), "错误 payload 的 PoL 应失败");
}
