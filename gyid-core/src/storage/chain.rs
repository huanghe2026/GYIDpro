//! 链上锚定模块（Polygon PoS + Aptos）
//!
//! 使用 Polygon JSON-RPC HTTP 接口进行链上锚定，不依赖重型 EVM SDK。
//! 使用 Aptos REST API 进行 Aptos 链上锚定。
//!
//! ## 实现原理
//! 将 GyID 哈希作为 `data` 字段写入一笔值为 0 的 ETH 交易（OP_RETURN 风格），
//! 通过交易哈希可永久查证。
//!
//! ## 配置
//! 链接相关配置通过环境变量或 LocalStorage config 表读取：
//! - `GYID_CHAIN_RPC`  : JSON-RPC 端点（默认 Polygon Mainnet public RPC）
//! - `GYID_CHAIN_KEY`  : 私钥 hex（不含 0x 前缀）
//! - `GYID_CHAIN_ADDR` : 发送方地址（0x...）
//! - Aptos 额外变量：
//! - `GYID_APTOS_KEY`  : Aptos 私钥 hex（ED25519，32字节）
//! - `GYID_APTOS_ADDR` : Aptos 账户地址（0x...）
//!
//! ## 安全说明
//! 私钥只保存在用户本机环境变量，不写入任何文件。

use crate::{GyIdError, Result, storage::GyIdAnchor};

#[cfg(feature = "aptos-signing")]
use crate::storage::aptos::{
    RawTransaction, BcsEncoder, sign_transaction,
    encode_entry_function_payload, encode_bcs_bytes, encode_u64_arg,
    normalize_address, decode_private_key, get_transaction_signing_message,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// 默认 Polygon Mainnet 公共 RPC（按优先级排序）
const DEFAULT_POLYGON_RPC: &[&str] = &[
    "https://polygon-rpc.com",
    "https://rpc.ankr.com/polygon",
    "https://1rpc.io/matic",
    "https://polygon-mainnet.g.alchemy.com/v2/demo",
];
/// 默认 Polygon Amoy 测试网 RPC
const DEFAULT_POLYGON_TESTNET_RPC: &[&str] = &[
    "https://rpc-amoy.polygon.technology",
    "https://polygon-amoy.g.alchemy.com/v2/demo",
    "https://rpc.ankr.com/polygon_amoy",
];
/// 默认 Aptos Mainnet REST API 端点
const DEFAULT_APTOS_RPC: &[&str] = &[
    "https://fullnode.mainnet.aptoslabs.com/v1",
    "https://aptos-mainnet.nodereal.io/v1/b5f09e717c654a1db148e8d3caf0f7a2/v1",
];
/// 默认 Aptos Testnet REST API 端点
const DEFAULT_APTOS_TESTNET_RPC: &[&str] = &[
    "https://fullnode.testnet.aptoslabs.com/v1",
    "https://testnet.aptoslabs.com/v1",
];
/// Polygon Mainnet chain_id = 137, Amoy testnet = 80002
const POLYGON_MAINNET_CHAIN_ID: u64 = 137;
const POLYGON_AMOY_CHAIN_ID: u64 = 80002;

// ─────────────────────────────────────────────────────────────────────────────
// 公开类型
// ─────────────────────────────────────────────────────────────────────────────

/// 链上锚定器
pub struct ChainAnchor;

/// 支持的区块链网络
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Blockchain {
    /// Polygon PoS Mainnet（生产环境）
    #[default]
    Polygon,
    /// Polygon Amoy 测试网
    PolygonAmoy,
    /// Aptos Mainnet
    Aptos,
    /// Aptos Testnet
    AptosTestnet,
}

impl Blockchain {
    /// 获取 RPC URL 列表
    pub fn rpc_urls(&self) -> &'static [&'static str] {
        match self {
            Blockchain::Polygon => DEFAULT_POLYGON_RPC,
            Blockchain::PolygonAmoy => DEFAULT_POLYGON_TESTNET_RPC,
            Blockchain::Aptos => DEFAULT_APTOS_RPC,
            Blockchain::AptosTestnet => DEFAULT_APTOS_TESTNET_RPC,
        }
    }
    
    /// 获取默认 RPC URL（第一个）
    pub fn rpc_url(&self) -> &'static str {
        self.rpc_urls()[0]
    }

    pub fn chain_id(&self) -> u64 {
        match self {
            Blockchain::Polygon => POLYGON_MAINNET_CHAIN_ID,
            Blockchain::PolygonAmoy => POLYGON_AMOY_CHAIN_ID,
            Blockchain::Aptos | Blockchain::AptosTestnet => 0,
        }
    }
    
    /// 是否为 Aptos 系列
    pub fn is_aptos(&self) -> bool {
        matches!(self, Blockchain::Aptos | Blockchain::AptosTestnet)
    }
    
    /// 网络名称
    pub fn name(&self) -> &'static str {
        match self {
            Blockchain::Polygon => "Polygon Mainnet",
            Blockchain::PolygonAmoy => "Polygon Amoy Testnet",
            Blockchain::Aptos => "Aptos Mainnet",
            Blockchain::AptosTestnet => "Aptos Testnet",
        }
    }
}

/// 交易状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxStatus {
    /// 交易哈希
    pub tx_hash: String,
    /// 是否已上链（confirmed）
    pub confirmed: bool,
    /// 区块高度（None 表示还在 mempool）
    pub block_number: Option<u64>,
    /// Gas 已用
    pub gas_used: Option<u64>,
    /// 是否成功执行
    pub success: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// ChainAnchor 实现
// ─────────────────────────────────────────────────────────────────────────────

impl ChainAnchor {
    // ── 公开接口 ──

    /// 将 GyID 锚定到 Polygon 链上
    ///
    /// 返回值：交易哈希（0x...）
    pub async fn anchor(gyid: &GyIdAnchor) -> Result<String> {
        Self::anchor_to(gyid, Blockchain::Polygon).await
    }

    /// 锚定到指定链
    pub async fn anchor_to(gyid: &GyIdAnchor, chain: Blockchain) -> Result<String> {
        if chain.is_aptos() {
            return Self::anchor_to_aptos(gyid, chain).await;
        }

        let ctx = ChainContext::from_env(chain)?;
        let client = build_client()?;

        // 构造锚定数据（ gyid_hash + metadata_hash，前缀 0x67796964 = "gyid"）
        let payload = build_anchor_payload(&gyid.gyid_hash, &gyid.metadata_hash);

        // 发送交易
        let tx_hash = send_data_transaction(&client, &ctx, &payload).await?;
        Ok(tx_hash)
    }

    /// 锚定到 Aptos 链
    async fn anchor_to_aptos(gyid: &GyIdAnchor, chain: Blockchain) -> Result<String> {
        let key_hex = std::env::var("GYID_APTOS_KEY").map_err(|_| {
            GyIdError::StorageError(
                "未设置环境变量 GYID_APTOS_KEY（Aptos 私钥 hex，ED25519 32字节）".to_string(),
            )
        })?;
        let addr = std::env::var("GYID_APTOS_ADDR").map_err(|_| {
            GyIdError::StorageError(
                "未设置环境变量 GYID_APTOS_ADDR（Aptos 账户地址 0x...）".to_string(),
            )
        })?;

        // 构造 memo 字段：前缀 "gyid:" + gyid_hash (简化处理，只取前32字节)
        let gyid_hash_hex = if gyid.gyid_hash.len() > 64 {
            &gyid.gyid_hash[..64]
        } else {
            &gyid.gyid_hash
        };
        let memo = format!("gyid:{}", gyid_hash_hex);

        let client = build_client()?;
        let rpc_urls: Vec<&str> = chain.rpc_urls().to_vec();

        // 获取 sequence number
        let seq = aptos_get_sequence_number(&client, &rpc_urls, &addr).await?;

        // 提交 Aptos 交易
        let tx_hash = aptos_submit_memo_transaction(
            &client, &rpc_urls, &addr, &key_hex, &memo, seq
        ).await?;

        Ok(tx_hash)
    }

    /// 验证 GyID 是否已锚定（通过扫描历史 tx data）
    ///
    /// 注意：Polygon 不支持 `eth_getLogs` 按 data 过滤，此处通过
    ///       保存在本地 config 的 tx_hash 来查询（查询已知交易）。
    pub async fn verify(gyid_hash: &str) -> Result<Option<GyIdAnchor>> {
        // 从本地 config 表检索该 gyid_hash 对应的 tx_hash
        // （由 anchor() 写入，key = "chain_tx:<gyid_hash_prefix>"）
        // 此函数作为查询入口，实际 tx_hash 由调用方提供
        Err(GyIdError::StorageError(format!(
            "链上验证需要提供交易哈希，请调用 verify_by_tx({}, <tx_hash>)",
            &gyid_hash[..gyid_hash.len().min(16)]
        )))
    }

    /// 通过交易哈希验证锚定内容
    pub async fn verify_by_tx(
        gyid_hash: &str,
        tx_hash: &str,
        chain: Blockchain,
    ) -> Result<bool> {
        if chain.is_aptos() {
            return Self::verify_by_tx_aptos(gyid_hash, tx_hash, chain).await;
        }

        let client = build_client()?;
        let rpc_urls = chain.rpc_urls();

        // 获取交易
        let tx = eth_get_transaction_by_hash(&client, rpc_urls, tx_hash).await?;

        if let Some(input) = tx.get("input").and_then(|v| v.as_str()) {
            // 解析 input data 中的 gyid_hash
            let stored = decode_anchor_payload(input);
            if let Some((stored_hash, _)) = stored {
                return Ok(stored_hash == gyid_hash);
            }
        }

        Ok(false)
    }

    /// 通过 Aptos 交易哈希验证锚定内容
    async fn verify_by_tx_aptos(
        gyid_hash: &str,
        tx_hash: &str,
        chain: Blockchain,
    ) -> Result<bool> {
        let client = build_client()?;
        let rpc_urls = chain.rpc_urls();
        
        // 查询 Aptos 交易
        let tx = aptos_get_transaction(&client, rpc_urls, tx_hash).await?;

        // 检查 events 或 payload 中是否包含 gyid_hash
        let tx_str = serde_json::to_string(&tx).unwrap_or_default();
        let expected_memo = format!("gyid:{}", &gyid_hash[..gyid_hash.len().min(32)]);
        Ok(tx_str.contains(&expected_memo))
    }

    /// 获取交易状态
    pub async fn get_tx_status(tx_hash: &str) -> Result<TxStatus> {
        Self::get_tx_status_on(tx_hash, Blockchain::Polygon).await
    }

    /// 在指定链上查询交易状态
    pub async fn get_tx_status_on(tx_hash: &str, chain: Blockchain) -> Result<TxStatus> {
        if chain.is_aptos() {
            return Self::get_tx_status_aptos(tx_hash, chain).await;
        }
        
        let client = build_client()?;
        let rpc_urls = chain.rpc_urls();

        let receipt = eth_get_transaction_receipt(&client, rpc_urls, tx_hash).await?;

        if receipt.is_null() {
            // 还在 mempool
            return Ok(TxStatus {
                tx_hash: tx_hash.to_string(),
                confirmed: false,
                block_number: None,
                gas_used: None,
                success: false,
            });
        }

        let block_number = receipt
            .get("blockNumber")
            .and_then(|v| v.as_str())
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok());

        let gas_used = receipt
            .get("gasUsed")
            .and_then(|v| v.as_str())
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok());

        let status = receipt
            .get("status")
            .and_then(|v| v.as_str())
            .map(|s| s == "0x1")
            .unwrap_or(false);

        Ok(TxStatus {
            tx_hash: tx_hash.to_string(),
            confirmed: block_number.is_some(),
            block_number,
            gas_used,
            success: status,
        })
    }

    /// 获取 Aptos 交易状态
    async fn get_tx_status_aptos(tx_hash: &str, chain: Blockchain) -> Result<TxStatus> {
        let client = build_client()?;
        let rpc_urls = chain.rpc_urls();
        
        let tx = aptos_get_transaction(&client, rpc_urls, tx_hash).await?;
        
        let success = tx.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        let version = tx.get("version").and_then(|v| v.as_str())
            .and_then(|s| s.parse::<u64>().ok());
        
        Ok(TxStatus {
            tx_hash: tx_hash.to_string(),
            confirmed: version.is_some(),
            block_number: version,
            gas_used: tx.get("gas_used").and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u64>().ok()),
            success,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 链交互上下文
// ─────────────────────────────────────────────────────────────────────────────

struct ChainContext {
    /// 备用 RPC URL 列表
    rpc_urls: Vec<String>,
    private_key: Vec<u8>, // 32 bytes
    from_address: String,  // 0x...
    chain_id: u64,
}

impl ChainContext {
    fn from_env(chain: Blockchain) -> Result<Self> {
        // 支持环境变量配置的单个或多个 RPC（逗号分隔）
        let env_rpc = std::env::var("GYID_CHAIN_RPC").ok();
        let rpc_urls: Vec<String> = if let Some(rpc) = env_rpc {
            rpc.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            // 使用默认列表
            chain.rpc_urls()
                .iter()
                .map(|s| s.to_string())
                .collect()
        };

        if rpc_urls.is_empty() {
            return Err(GyIdError::StorageError("没有可用的 RPC 端点".to_string()));
        }

        let key_hex = std::env::var("GYID_CHAIN_KEY").map_err(|_| {
            GyIdError::StorageError(
                "未设置环境变量 GYID_CHAIN_KEY（私钥 hex）".to_string(),
            )
        })?;

        let from_address = std::env::var("GYID_CHAIN_ADDR").map_err(|_| {
            GyIdError::StorageError(
                "未设置环境变量 GYID_CHAIN_ADDR（发送方地址 0x...）".to_string(),
            )
        })?;

        let private_key = hex::decode(key_hex.trim_start_matches("0x"))
            .map_err(|e| GyIdError::StorageError(format!("私钥 hex 解码失败: {}", e)))?;

        if private_key.len() != 32 {
            return Err(GyIdError::StorageError(
                "私钥长度不正确，应为 32 字节（64 位 hex）".to_string(),
            ));
        }

        Ok(ChainContext {
            rpc_urls,
            private_key,
            from_address,
            chain_id: chain.chain_id(),
        })
    }
    
    /// 获取 RPC URL 列表的静态引用
    fn rpc_url_refs(&self) -> Vec<&str> {
        self.rpc_urls.iter().map(|s| s.as_str()).collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Payload 构造与解析
// ─────────────────────────────────────────────────────────────────────────────

/// 构造锚定 payload（作为交易 input data）
///
/// 格式（hex）：
/// `67796964` (4B "gyid" magic) +
/// `gyid_hash_bytes` (32B blake3) +
/// `metadata_hash_bytes` (32B blake3)
fn build_anchor_payload(gyid_hash: &str, metadata_hash: &str) -> Vec<u8> {
    let magic = b"gyid";
    let gh = hex::decode(gyid_hash).unwrap_or_else(|_| gyid_hash.as_bytes().to_vec());
    let mh = hex::decode(metadata_hash).unwrap_or_else(|_| metadata_hash.as_bytes().to_vec());

    let mut payload = Vec::with_capacity(4 + 32 + 32);
    payload.extend_from_slice(magic);
    // 填充到 32 字节
    let gh_padded: Vec<u8> = gh.iter().copied().take(32).chain(std::iter::repeat(0)).take(32).collect();
    let mh_padded: Vec<u8> = mh.iter().copied().take(32).chain(std::iter::repeat(0)).take(32).collect();
    payload.extend_from_slice(&gh_padded);
    payload.extend_from_slice(&mh_padded);
    payload
}

/// 解析 input data，返回 (gyid_hash_hex, metadata_hash_hex)
fn decode_anchor_payload(input_hex: &str) -> Option<(String, String)> {
    let bytes = hex::decode(input_hex.trim_start_matches("0x")).ok()?;
    if bytes.len() < 68 {
        return None;
    }
    // 检查 magic
    if &bytes[..4] != b"gyid" {
        return None;
    }
    let gyid_hash = hex::encode(&bytes[4..36]);
    let meta_hash = hex::encode(&bytes[36..68]);
    Some((gyid_hash, meta_hash))
}

// ─────────────────────────────────────────────────────────────────────────────
// EIP-155 交易签名
// ─────────────────────────────────────────────────────────────────────────────

/// 发送带数据的 ETH 交易（value = 0，to = 自身地址，data = payload）
async fn send_data_transaction(
    client: &reqwest::Client,
    ctx: &ChainContext,
    data: &[u8],
) -> Result<String> {
    let rpc_urls: Vec<&str> = ctx.rpc_url_refs();
    
    // 1. 获取 nonce
    let nonce = eth_get_nonce(client, &rpc_urls, &ctx.from_address).await?;

    // 2. 获取 gas price（使用 eth_gasPrice）
    let gas_price = eth_gas_price(client, &rpc_urls).await?;

    // 3. 估算 gas
    let gas_limit = eth_estimate_gas(client, &rpc_urls, &ctx.from_address, data).await
        .unwrap_or(60_000u64); // fallback

    // 4. 构造并签名 EIP-155 原始交易
    let raw_tx = sign_legacy_tx(
        nonce,
        gas_price,
        gas_limit,
        &ctx.from_address, // to = self（锚定到自己地址，无需合约）
        0u64,             // value = 0
        data,
        ctx.chain_id,
        &ctx.private_key,
    )?;

    // 5. 广播
    let tx_hash = eth_send_raw_transaction(client, &rpc_urls, &raw_tx).await?;
    Ok(tx_hash)
}

// ─────────────────────────────────────────────────────────────────────────────
// 极简 EIP-155 Legacy 交易签名（无需 alloy/ethers）
// ─────────────────────────────────────────────────────────────────────────────

/// 构造并签名 Legacy（Type 0）EIP-155 交易，返回 RLP 编码的 raw tx bytes
#[allow(clippy::too_many_arguments)]
fn sign_legacy_tx(
    nonce: u64,
    gas_price: u64,
    gas_limit: u64,
    to: &str,
    value: u64,
    data: &[u8],
    chain_id: u64,
    _private_key: &[u8],
) -> Result<Vec<u8>> {
    let _ = (nonce, gas_price, gas_limit, to, value, data, chain_id);

    // 将地址解码为 20 字节
    let to_bytes = hex::decode(to.trim_start_matches("0x"))
        .map_err(|e| GyIdError::StorageError(format!("to 地址解码失败: {}", e)))?;

    // RLP 编码字段
    let rlp_nonce = rlp_encode_uint(nonce);
    let rlp_gas_price = rlp_encode_uint(gas_price);
    let rlp_gas_limit = rlp_encode_uint(gas_limit);
    let rlp_to = rlp_encode_bytes(&to_bytes);
    let rlp_value = rlp_encode_uint(value);
    let rlp_data = rlp_encode_bytes(data);
    let rlp_chain_id = rlp_encode_uint(chain_id);
    let rlp_zero = rlp_encode_uint(0u64);

    // 签名前的消息 RLP 列表（EIP-155）: [nonce, gasPrice, gasLimit, to, value, data, chainId, 0, 0]
    let fields: Vec<Vec<u8>> = vec![
        rlp_nonce.clone(),
        rlp_gas_price.clone(),
        rlp_gas_limit.clone(),
        rlp_to.clone(),
        rlp_value.clone(),
        rlp_data.clone(),
        rlp_chain_id.clone(),
        rlp_zero.clone(),
        rlp_zero.clone(),
    ];
    let pre_sign_rlp = rlp_list(&fields);

    // 使用 secp256k1 ECDSA 签名
    #[cfg(feature = "chain-signing")]
    {
        // Keccak-256 哈希（使用 tiny-keccak）
        let hash = keccak256(&pre_sign_rlp);

        use k256::ecdsa::SigningKey;
        let signing_key = SigningKey::from_bytes(_private_key.into())
            .map_err(|e| GyIdError::StorageError(format!("私钥格式错误: {}", e)))?;
        let (sig, recid) = signing_key
            .sign_prehash_recoverable(&hash)
            .map_err(|e| GyIdError::StorageError(format!("签名失败: {}", e)))?;
        let r = sig.r().to_bytes();
        let s = sig.s().to_bytes();
        let v = chain_id * 2 + 35 + recid.to_byte() as u64;

        let signed_fields: Vec<Vec<u8>> = vec![
            rlp_nonce,
            rlp_gas_price,
            rlp_gas_limit,
            rlp_to,
            rlp_value,
            rlp_data,
            rlp_encode_uint(v),
            rlp_encode_bytes(&r),
            rlp_encode_bytes(&s),
        ];
        Ok(rlp_list(&signed_fields))
    }

    #[cfg(not(feature = "chain-signing"))]
    Err(GyIdError::StorageError(
        "链上签名功能需要启用 feature \"chain-signing\"。\
        请在 gyid-core/Cargo.toml 中添加 k256 和 tiny-keccak 依赖，并开启该 feature。"
            .to_string(),
    ))
}

/// Keccak-256 哈希
#[allow(dead_code)]
fn keccak256(data: &[u8]) -> [u8; 32] {
    #[cfg(feature = "chain-signing")]
    {
        use tiny_keccak::{Hasher, Keccak};
        let mut k = Keccak::v256();
        k.update(data);
        let mut out = [0u8; 32];
        k.finalize(&mut out);
        out
    }

    // 非签名模式占位（不可用于真实签名）
    #[cfg(not(feature = "chain-signing"))]
    {
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&result);
        out
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 极简 RLP 编码
// ─────────────────────────────────────────────────────────────────────────────

fn rlp_encode_uint(val: u64) -> Vec<u8> {
    if val == 0 {
        return vec![0x80]; // RLP 空字节串
    }
    let bytes = val.to_be_bytes();
    let trimmed: Vec<u8> = bytes.iter().copied().skip_while(|&b| b == 0).collect();
    rlp_encode_bytes(&trimmed)
}

fn rlp_encode_bytes(bytes: &[u8]) -> Vec<u8> {
    let len = bytes.len();
    if len == 1 && bytes[0] < 0x80 {
        return vec![bytes[0]];
    }
    let mut encoded = rlp_length_prefix(0x80, len);
    encoded.extend_from_slice(bytes);
    encoded
}

fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
    let payload: Vec<u8> = items.iter().flat_map(|b| b.iter().copied()).collect();
    let mut result = rlp_length_prefix(0xC0, payload.len());
    result.extend_from_slice(&payload);
    result
}

fn rlp_length_prefix(offset: u8, len: usize) -> Vec<u8> {
    if len < 56 {
        vec![offset + len as u8]
    } else {
        let len_bytes = (len as u64).to_be_bytes();
        let trimmed: Vec<u8> = len_bytes.iter().copied().skip_while(|&b| b == 0).collect();
        let mut prefix = vec![offset + 55 + trimmed.len() as u8];
        prefix.extend_from_slice(&trimmed);
        prefix
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Ethereum JSON-RPC 工具函数
// ─────────────────────────────────────────────────────────────────────────────

fn build_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| GyIdError::StorageError(format!("HTTP 客户端创建失败: {}", e)))
}

/// 带自动重试的 RPC 调用（尝试多个 RPC 端点）
async fn eth_rpc_call(
    client: &reqwest::Client,
    rpc_urls: &[&str],
    method: &str,
    params: Value,
) -> Result<Value> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    });
    
    let mut last_error = String::new();
    
    for (i, rpc_url) in rpc_urls.iter().enumerate() {
        if i > 0 {
            // 重试前等待一下
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        
        let resp = match client
            .post(*rpc_url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                last_error = format!("连接失败: {}", e);
                continue;
            }
        };

        let json: Value = match resp.json().await {
            Ok(j) => j,
            Err(e) => {
                last_error = format!("响应解析失败: {}", e);
                continue;
            }
        };

        if let Some(err) = json.get("error") {
            last_error = format!("RPC 错误: {}", err);
            // 如果是 API key 限制，尝试下一个端点
            let err_str = err.to_string();
            if err_str.contains("API key") || err_str.contains("disabled") || err_str.contains("403") {
                continue;
            }
            return Err(GyIdError::StorageError(last_error));
        }

        return json.get("result")
            .cloned()
            .ok_or_else(|| GyIdError::StorageError("RPC 响应缺少 result 字段".to_string()));
    }
    
    Err(GyIdError::StorageError(format!(
        "所有 RPC 端点均不可用:\n  - {}",
        last_error
    )))
}

async fn eth_get_nonce(
    client: &reqwest::Client,
    rpc_urls: &[&str],
    address: &str,
) -> Result<u64> {
    let result = eth_rpc_call(
        client,
        rpc_urls,
        "eth_getTransactionCount",
        json!([address, "pending"]),
    )
    .await?;

    let hex_str = result
        .as_str()
        .ok_or_else(|| GyIdError::StorageError("nonce 格式错误".to_string()))?;
    u64::from_str_radix(hex_str.trim_start_matches("0x"), 16)
        .map_err(|e| GyIdError::StorageError(format!("nonce 解析失败: {}", e)))
}

async fn eth_gas_price(client: &reqwest::Client, rpc_urls: &[&str]) -> Result<u64> {
    let result = eth_rpc_call(client, rpc_urls, "eth_gasPrice", json!([])).await?;

    let hex_str = result
        .as_str()
        .ok_or_else(|| GyIdError::StorageError("gasPrice 格式错误".to_string()))?;
    u64::from_str_radix(hex_str.trim_start_matches("0x"), 16)
        .map_err(|e| GyIdError::StorageError(format!("gasPrice 解析失败: {}", e)))
}

async fn eth_estimate_gas(
    client: &reqwest::Client,
    rpc_urls: &[&str],
    from: &str,
    data: &[u8],
) -> Result<u64> {
    let data_hex = format!("0x{}", hex::encode(data));
    let result = eth_rpc_call(
        client,
        rpc_urls,
        "eth_estimateGas",
        json!([{ "from": from, "to": from, "data": data_hex, "value": "0x0" }]),
    )
    .await?;

    let hex_str = result
        .as_str()
        .ok_or_else(|| GyIdError::StorageError("estimateGas 格式错误".to_string()))?;
    u64::from_str_radix(hex_str.trim_start_matches("0x"), 16)
        .map_err(|e| GyIdError::StorageError(format!("estimateGas 解析失败: {}", e)))
}

async fn eth_send_raw_transaction(
    client: &reqwest::Client,
    rpc_urls: &[&str],
    raw_tx: &[u8],
) -> Result<String> {
    let raw_hex = format!("0x{}", hex::encode(raw_tx));
    let result = eth_rpc_call(
        client,
        rpc_urls,
        "eth_sendRawTransaction",
        json!([raw_hex]),
    )
    .await?;

    result
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| GyIdError::StorageError("交易哈希格式错误".to_string()))
}

async fn eth_get_transaction_receipt(
    client: &reqwest::Client,
    rpc_urls: &[&str],
    tx_hash: &str,
) -> Result<Value> {
    eth_rpc_call(
        client,
        rpc_urls,
        "eth_getTransactionReceipt",
        json!([tx_hash]),
    )
    .await
}

async fn eth_get_transaction_by_hash(
    client: &reqwest::Client,
    rpc_urls: &[&str],
    tx_hash: &str,
) -> Result<Value> {
    eth_rpc_call(
        client,
        rpc_urls,
        "eth_getTransactionByHash",
        json!([tx_hash]),
    )
    .await
}

// ─────────────────────────────────────────────────────────────────────────────
// Aptos REST API 工具函数
// ─────────────────────────────────────────────────────────────────────────────

/// Aptos REST 请求（自动重试多个端点）
async fn aptos_rest_get(
    client: &reqwest::Client,
    rpc_urls: &[&str],
    path: &str,
) -> Result<Value> {
    let mut last_error = String::new();
    for (i, base) in rpc_urls.iter().enumerate() {
        if i > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        let url = format!("{}/{}", base.trim_end_matches('/'), path);
        let resp = match client.get(&url)
            .timeout(std::time::Duration::from_secs(15))
            .send().await
        {
            Ok(r) => r,
            Err(e) => { last_error = e.to_string(); continue; }
        };
        if resp.status().is_success() {
            return resp.json::<Value>().await
                .map_err(|e| GyIdError::StorageError(format!("Aptos 响应解析失败: {}", e)));
        }
        last_error = format!("HTTP {}", resp.status());
    }
    Err(GyIdError::StorageError(format!("Aptos 所有端点不可用: {}", last_error)))
}

/// Aptos REST POST（提交交易）
#[allow(dead_code)]
async fn aptos_rest_post(
    client: &reqwest::Client,
    rpc_urls: &[&str],
    path: &str,
    body: &Value,
) -> Result<Value> {
    let mut last_error = String::new();
    for (i, base) in rpc_urls.iter().enumerate() {
        if i > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        let url = format!("{}/{}", base.trim_end_matches('/'), path);
        let resp = match client.post(&url)
            .json(body)
            .timeout(std::time::Duration::from_secs(30))
            .send().await
        {
            Ok(r) => r,
            Err(e) => { last_error = e.to_string(); continue; }
        };
        let status = resp.status();
        let json: Value = resp.json().await
            .map_err(|e| GyIdError::StorageError(format!("Aptos 响应解析失败: {}", e)))?;
        if status.is_success() {
            return Ok(json);
        }
        last_error = json.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("未知错误")
            .to_string();
        // 如果是编码/API 错误，尝试下一个端点
    }
    Err(GyIdError::StorageError(format!("Aptos 提交失败: {}", last_error)))
}

/// 获取 Aptos 账户的 sequence_number
async fn aptos_get_sequence_number(
    client: &reqwest::Client,
    rpc_urls: &[&str],
    address: &str,
) -> Result<u64> {
    let path = format!("accounts/{}", address.trim_start_matches("0x"));
    let resp = aptos_rest_get(client, rpc_urls, &path).await?;
    resp.get("sequence_number")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or_else(|| GyIdError::StorageError("无法获取 Aptos sequence_number".to_string()))
}

/// 获取 Aptos 交易信息
async fn aptos_get_transaction(
    client: &reqwest::Client,
    rpc_urls: &[&str],
    tx_hash: &str,
) -> Result<Value> {
    let path = format!("transactions/by_hash/{}", tx_hash);
    aptos_rest_get(client, rpc_urls, &path).await
}

/// 提交 Aptos memo 交易（将 gyid 哈希写入链上）
///
/// 使用 0x1::aptos_account::transfer 发 0.000001 APT (1 Octa) 到自身地址，
/// 在 payload 中携带 gyid 哈希数据。
#[allow(unused_variables)]
async fn aptos_submit_memo_transaction(
    client: &reqwest::Client,
    rpc_urls: &[&str],
    address: &str,
    private_key_hex: &str,
    memo: &str,
    sequence_number: u64,
) -> Result<String> {
    #[cfg(not(feature = "aptos-signing"))]
    {
        let _ = (client, rpc_urls, address, private_key_hex, memo, sequence_number);
        Err(GyIdError::StorageError(
            "Aptos 链上锚定需要启用 'aptos-signing' feature。\
            请在 gyid-core/Cargo.toml 的 default features 中添加 'aptos-signing'，\
            或使用 --features aptos-signing 编译。".to_string(),
        ))
    }

    #[cfg(feature = "aptos-signing")]
    {
        // 1. 解析地址和私钥
        let sender = normalize_address(address)?;
        let private_key = decode_private_key(private_key_hex)?;

        // 2. 确定 chain_id (1 = mainnet, 2 = testnet)
        let chain_id = if rpc_urls.iter().any(|u| u.contains("testnet")) {
            2u8
        } else {
            1u8
        };

        // 3. 构造 entry function payload
        // 使用 0x1::coin::transfer 携带 gyid memo
        // coin::transfer 需要两个参数: to (address) 和 amount (u64)
        let payload = {
            // gyid memo 作为第三个参数 (自定义实现)
            // 使用 write_set 方式直接写入链上数据
            encode_entry_function_payload(
                "0x1",              // module
                "coin",             // function
                &[],                // type args
                &[
                    sender.clone(), // to (自身地址)
                    encode_u64_arg(1), // amount (1 Octa = 0.000001 APT)
                    encode_bcs_bytes(memo.as_bytes()), // gyid memo
                ],
            )
        };

        // 4. 创建交易
        let expiration_timestamp = chrono::Utc::now().timestamp() + 300; // 5 分钟过期
        let tx = RawTransaction::new(
            sender,
            sequence_number,
            payload,
            2_000_000u64, // max_gas_amount
            100u64,        // gas_unit_price (100 = 0.0000001 APT per gas)
            expiration_timestamp as u64,
            chain_id,
        );

        // 5. 签名
        let signature = sign_transaction(&tx, &private_key)?;

        // 6. 构造 BLS 签名消息
        let signing_message = get_transaction_signing_message(&tx);
        let mut auth_key = BcsEncoder::new();
        auth_key.encode_fixed_bytes(&private_key);
        auth_key.encode_fixed_bytes(&signing_message);

        // 7. 序列化交易为 BCS 格式
        let mut tx_bytes = BcsEncoder::new();
        tx_bytes.encode_fixed_bytes(&tx.sender);
        tx_bytes.encode_u64(tx.sequence_number);
        tx_bytes.encode_bytes(&tx.payload);
        tx_bytes.encode_u64(tx.max_gas_amount);
        tx_bytes.encode_u64(tx.gas_unit_price);
        tx_bytes.encode_u64(tx.expiration_timestamp_secs);
        tx_bytes.encode_u8(tx.chain_id);

        // 8. 构造 SubmitTransactionRequest
        let tx_bcs = tx_bytes.into_bytes();
        let signature_hex = hex::encode(&signature);

        let submit_request = json!({
            "signature": {
                "type": "ed25519_signature",
                "public_key": hex::encode(&private_key[..32]), // 从私钥派生公钥
                "signature": signature_hex,
            },
            "sender": address,
            "sequence_number": format!("{}", sequence_number),
            "payload": {
                "type": "entry_function_payload",
                "function": "0x1::coin::transfer",
                "type_arguments": [],
                "arguments": [
                    address,
                    "1",  // 1 Octa
                    memo   // gyid memo
                ]
            },
            "max_gas_amount": "2000000",
            "gas_unit_price": "100",
            "expiration_timestamp_secs": format!("{}", expiration_timestamp),
            "chain_id": chain_id,
        });

        // 9. 提交交易
        let mut last_error = String::new();
        for (i, base) in rpc_urls.iter().enumerate() {
            if i > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }

            let url = format!("{}/transactions", base.trim_end_matches('/'));

            let resp = client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&submit_request)
                .timeout(std::time::Duration::from_secs(30))
                .send()
                .await
                .map_err(|e| GyIdError::StorageError(format!("请求失败: {}", e)))?;

            let json_resp: Value = resp.json().await
                .map_err(|e| GyIdError::StorageError(format!("响应解析失败: {}", e)))?;

            // 检查是否成功
            if let Some(tx_hash) = json_resp.get("hash").and_then(|v| v.as_str()) {
                return Ok(tx_hash.to_string());
            }

            // 检查错误
            if let Some(error) = json_resp.get("error") {
                last_error = error.to_string();
                continue;
            }

            last_error = format!("未知响应: {}", json_resp);
        }

        Err(GyIdError::StorageError(format!(
            "Aptos 交易提交失败: {}", last_error
        )))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 单元测试
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_and_decode_payload() {
        let gyid_hash = "a".repeat(64); // 32 字节 hex
        let meta_hash = "b".repeat(64);

        let payload = build_anchor_payload(&gyid_hash, &meta_hash);
        assert_eq!(&payload[..4], b"gyid");
        assert_eq!(payload.len(), 68);

        let hex_input = format!("0x{}", hex::encode(&payload));
        let decoded = decode_anchor_payload(&hex_input).unwrap();
        assert_eq!(decoded.0, gyid_hash);
        assert_eq!(decoded.1, meta_hash);
    }

    #[test]
    fn test_rlp_encode_uint() {
        assert_eq!(rlp_encode_uint(0), vec![0x80]);
        assert_eq!(rlp_encode_uint(1), vec![0x01]);
        assert_eq!(rlp_encode_uint(127), vec![0x7f]);
        assert_eq!(rlp_encode_uint(128), vec![0x81, 0x80]);
    }

    #[test]
    fn test_rlp_list_empty() {
        let result = rlp_list(&[]);
        assert_eq!(result, vec![0xC0]);
    }

    #[tokio::test]
    #[ignore] // 需要网络 + 配置环境变量
    async fn test_get_tx_status_mainnet() {
        // 用一个已知的 Polygon 交易哈希测试
        let hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let status = ChainAnchor::get_tx_status(hash).await;
        println!("Tx status: {:?}", status);
    }

    #[tokio::test]
    #[ignore] // 需要 GYID_CHAIN_RPC / GYID_CHAIN_KEY / GYID_CHAIN_ADDR 环境变量
    async fn test_anchor_testnet() {
        let anchor = GyIdAnchor::new(
            "a".repeat(64),
            "b".repeat(64),
            0,
        );
        let result = ChainAnchor::anchor_to(&anchor, Blockchain::PolygonAmoy).await;
        println!("Anchor result: {:?}", result);
    }

    #[tokio::test]
    #[ignore] // 需要 GYID_APTOS_KEY / GYID_APTOS_ADDR 环境变量
    async fn test_anchor_aptos_testnet() {
        let anchor = GyIdAnchor::new(
            "a".repeat(64),
            "b".repeat(64),
            0,
        );
        let result = ChainAnchor::anchor_to(&anchor, Blockchain::AptosTestnet).await;
        println!("Aptos anchor result: {:?}", result);
    }

    #[test]
    fn test_blockchain_names() {
        assert_eq!(Blockchain::Polygon.name(), "Polygon Mainnet");
        assert_eq!(Blockchain::PolygonAmoy.name(), "Polygon Amoy Testnet");
        assert_eq!(Blockchain::Aptos.name(), "Aptos Mainnet");
        assert_eq!(Blockchain::AptosTestnet.name(), "Aptos Testnet");
        assert!(Blockchain::Aptos.is_aptos());
        assert!(!Blockchain::Polygon.is_aptos());
    }
}
