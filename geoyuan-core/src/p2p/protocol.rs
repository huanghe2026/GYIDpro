//! P2P protocol definitions
//! 
//! Defines the communication protocol between peers

use serde::{Deserialize, Serialize};
use std::io;

use crate::crypto::KeyPair;

// ─────────────────────────────────────────────────────────────────────────────
// PoL — Proof of Location 位置证明
// ─────────────────────────────────────────────────────────────────────────────

/// PoL 位置证明：每个 P2P 消息可附加物理位置签名
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationProof {
    /// GPS 纬度
    pub latitude: f64,
    /// GPS 经度
    pub longitude: f64,
    /// UTC 时间戳（毫秒）
    pub timestamp_millis: u64,
    /// H3 cell (Res 12) 索引
    pub h3_cell: u64,
    /// Ed25519 签名: sign(lat || lon || timestamp || h3_cell || payload_hash)
    pub signature: Vec<u8>,
    /// 签名者公钥
    pub public_key: [u8; 32],
}

impl LocationProof {
    /// 构造签名消息: lat(8) || lon(8) || timestamp(8) || h3_cell(8) || payload_hash(32)
    fn message_to_sign(lat: f64, lon: f64, timestamp: u64, h3_cell: u64, payload_hash: &[u8; 32]) -> Vec<u8> {
        let mut msg = Vec::with_capacity(64);
        msg.extend_from_slice(&lat.to_le_bytes());
        msg.extend_from_slice(&lon.to_le_bytes());
        msg.extend_from_slice(&timestamp.to_le_bytes());
        msg.extend_from_slice(&h3_cell.to_le_bytes());
        msg.extend_from_slice(payload_hash);
        msg
    }

    /// 创建 PoL 证明
    pub fn new(
        lat: f64,
        lon: f64,
        h3_cell: u64,
        payload_hash: &[u8; 32],
        keypair: &KeyPair,
    ) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let msg = Self::message_to_sign(lat, lon, timestamp, h3_cell, payload_hash);
        let sig = keypair.sign(&msg);
        Self {
            latitude: lat,
            longitude: lon,
            timestamp_millis: timestamp,
            h3_cell,
            signature: sig.to_bytes().to_vec(),
            public_key: keypair.public_bytes(),
        }
    }

    /// 验证 PoL 证明
    pub fn verify(&self, payload_hash: &[u8; 32]) -> bool {
        if self.signature.len() != 64 {
            return false;
        }
        let msg = Self::message_to_sign(
            self.latitude,
            self.longitude,
            self.timestamp_millis,
            self.h3_cell,
            payload_hash,
        );
        let sig: [u8; 64] = match self.signature.as_slice().try_into() {
            Ok(s) => s,
            Err(_) => return false,
        };
        KeyPair::verify_signature(&msg, &sig, &self.public_key)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Envelope 消息信封
// ─────────────────────────────────────────────────────────────────────────────

/// GeoYuan P2P protocol identifier
pub const GEOYUAN_PROTOCOL: &str = "/geoyuan/1.0";

/// Message envelope for P2P communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    /// Message type
    pub msg_type: MessageType,
    
    /// Sender peer ID
    pub sender: String,
    
    /// Message payload (JSON serialized)
    pub payload: Vec<u8>,
    
    /// Timestamp
    pub timestamp: i64,
    
    /// Signature (Ed25519 hex, optional)
    pub signature: Option<String>,

    /// PoL 位置证明（可选，GeoCast 消息必要）
    #[serde(default)]
    pub location_proof: Option<LocationProof>,
}

impl Envelope {
    /// Create new envelope (no signature, no PoL)
    pub fn new(msg_type: MessageType, sender: &str, payload: Vec<u8>) -> Self {
        Self {
            msg_type,
            sender: sender.to_string(),
            payload,
            timestamp: chrono::Utc::now().timestamp(),
            signature: None,
            location_proof: None,
        }
    }

    /// Create new envelope with PoL proof
    pub fn new_with_pol(
        msg_type: MessageType,
        sender: &str,
        payload: Vec<u8>,
        location_proof: Option<LocationProof>,
    ) -> Self {
        Self {
            msg_type,
            sender: sender.to_string(),
            payload,
            timestamp: chrono::Utc::now().timestamp(),
            signature: None,
            location_proof,
        }
    }

    /// Create signed envelope
    pub fn new_signed(
        msg_type: MessageType,
        sender: &str,
        payload: Vec<u8>,
        sign_fn: impl Fn(&[u8]) -> String,
    ) -> Self {
        let timestamp = chrono::Utc::now().timestamp();
        // 签名内容 = msg_type 作为 u8 + payload + timestamp
        let mut data = vec![msg_type as u8];
        data.extend_from_slice(&payload);
        data.extend_from_slice(&timestamp.to_le_bytes());
        let signature = sign_fn(&data);
        Self {
            msg_type,
            sender: sender.to_string(),
            payload,
            timestamp,
            signature: Some(signature),
            location_proof: None,
        }
    }
    
    /// Serialize envelope (JSON)
    pub fn serialize(&self) -> io::Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
    
    /// Deserialize envelope (JSON)
    pub fn deserialize(data: &[u8]) -> io::Result<Self> {
        serde_json::from_slice(data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Serialize envelope as CBOR (与 Swarm RequestResponse 协议匹配)
    pub fn serialize_cbor(&self) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        ciborium::ser::into_writer(self, &mut buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(buf)
    }

    /// Deserialize envelope from CBOR
    pub fn deserialize_cbor(data: &[u8]) -> io::Result<Self> {
        ciborium::de::from_reader(data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 业务 Payload 结构体
// ─────────────────────────────────────────────────────────────────────────────

/// 转账消息 payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferPayload {
    /// 交易哈希
    pub tx_id: String,
    /// 发送方 GyID
    pub from_gyid: String,
    /// 接收方 GyID 或地址
    pub to_gyid: String,
    /// 转账金额 (GY)
    pub amount: f64,
    /// 发送方公钥 hex
    pub sender_pubkey: String,
}

/// 铸造消息 payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintPayload {
    /// GyID
    pub gy_id: String,
    /// GPS 纬度
    pub latitude: f64,
    /// GPS 经度
    pub longitude: f64,
}

/// 钱包同步请求 payload
///
/// 请求某个 GyID 对应节点的钱包信息（余额 + 最近交易摘要）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSyncPayload {
    /// 请求方 GyID
    pub requester_gyid: String,
    /// 请求方公钥 hex（用于验证身份）
    pub requester_pubkey: String,
    /// 请求时间戳
    pub timestamp: i64,
    /// 随机 nonce（防止重放攻击）
    pub nonce: u64,
}

/// 钱包同步响应 payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSyncResponse {
    /// 响应方 GyID
    pub responder_gyid: String,
    /// 当前余额
    pub balance: f64,
    /// 最近交易数量
    pub tx_count: usize,
    /// 最近 N 条交易的简短摘要（tx_hash + type + amount + timestamp）
    pub recent_txs: Vec<TxSummary>,
    /// 响应时间戳
    pub timestamp: i64,
}

/// 交易摘要（用于钱包同步响应，不包含完整交易数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxSummary {
    /// 交易哈希（截断前 20 字符）
    pub tx_hash_short: String,
    /// 交易类型: "mint" / "transfer" / "link" / "reward"
    pub tx_type: String,
    /// 金额
    pub amount: f64,
    /// 时间戳
    pub timestamp: u64,
}

/// 设备关联请求 payload
///
/// 从设备发起，请求与主设备建立关联
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLinkRequest {
    /// 从设备 GyID
    pub linked_gyid: String,
    /// 从设备公钥 hex
    pub linked_pubkey: String,
    /// 授权码（由主设备生成）
    pub auth_code: String,
    /// 请求时间戳
    pub timestamp: i64,
    /// 随机 nonce
    pub nonce: u64,
}

/// 设备关联响应 payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLinkResponse {
    /// 主设备 GyID
    pub master_gyid: String,
    /// 关联结果
    pub accepted: bool,
    /// 关联 ID（accepted=true 时有效）
    pub link_id: Option<String>,
    /// 拒绝原因（accepted=false 时有效）
    pub reason: Option<String>,
    /// 响应时间戳
    pub timestamp: i64,
}

/// 设备列表查询请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceListRequest {
    /// 请求方 GyID
    pub requester_gyid: String,
    /// 目标 GyID（查询谁的设备列表）
    pub target_gyid: String,
    /// 请求时间戳
    pub timestamp: i64,
}

/// 设备列表查询响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceListResponse {
    /// 目标 GyID
    pub target_gyid: String,
    /// 关联设备数量
    pub device_count: usize,
    /// 设备信息列表（脱敏：只返回 GyID 前 8 位 + 关联时间）
    pub devices: Vec<DeviceInfoEntry>,
    /// 响应时间戳
    pub timestamp: i64,
}

/// 设备信息条目（脱敏展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfoEntry {
    /// 从设备 GyID（截断前 12 位）
    pub gyid_short: String,
    /// 关联时间
    pub linked_at: i64,
    /// 是否活跃
    pub active: bool,
}

/// Message types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    /// Handshake message
    Handshake,
    /// Wallet sync request
    WalletRequest,
    /// Wallet sync response
    WalletResponse,
    /// GeoID sync request
    GeoIdRequest,
    /// GeoID sync response
    GeoIdResponse,
    /// Device link request (slave → master)
    DeviceLinkRequest,
    /// Device link response (master → slave)
    DeviceLinkResponse,
    /// Device list query request
    DeviceListRequest,
    /// Device list query response
    DeviceListResponse,
    /// Coin minted broadcast
    CoinMinted,
    /// Coin transferred broadcast
    CoinTransferred,
    /// Ping message
    Ping,
    /// Pong message
    Pong,
    /// GeoCast 地理广播（携带 H3 cell，接收端校验邻近性）
    GeoCast,
    /// GeoCast 响应
    GeoCastResponse,
}

impl MessageType {
    /// Get protocol name for this message type
    pub fn protocol_name(&self) -> &'static str {
        match self {
            MessageType::Handshake => "/geoyuan/handshake",
            MessageType::WalletRequest => "/geoyuan/wallet/request",
            MessageType::WalletResponse => "/geoyuan/wallet/response",
            MessageType::GeoIdRequest => "/geoyuan/geoid/request",
            MessageType::GeoIdResponse => "/geoyuan/geoid/response",
            MessageType::DeviceLinkRequest => "/geoyuan/device/link/request",
            MessageType::DeviceLinkResponse => "/geoyuan/device/link/response",
            MessageType::DeviceListRequest => "/geoyuan/device/list/request",
            MessageType::DeviceListResponse => "/geoyuan/device/list/response",
            MessageType::CoinMinted => "/geoyuan/coin/minted",
            MessageType::CoinTransferred => "/geoyuan/coin/transferred",
            MessageType::Ping => "/geoyuan/ping",
            MessageType::Pong => "/geoyuan/pong",
            MessageType::GeoCast => "/geoyuan/geocast",
            MessageType::GeoCastResponse => "/geoyuan/geocast/response",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;

    #[test]
    fn test_location_proof_create_and_verify() {
        let kp = KeyPair::generate();
        let payload_hash = [0u8; 32]; // 示例 payload hash
        let proof = LocationProof::new(39.9042, 116.4074, 0x8a1fb46622dffff, &payload_hash, &kp);
        assert!(proof.verify(&payload_hash));
    }

    #[test]
    fn test_location_proof_wrong_hash_fails() {
        let kp = KeyPair::generate();
        let payload_hash = [0u8; 32];
        let proof = LocationProof::new(39.9042, 116.4074, 0x8a1fb46622dffff, &payload_hash, &kp);
        let wrong_hash = [1u8; 32];
        assert!(!proof.verify(&wrong_hash));
    }

    #[test]
    fn test_location_proof_empty_signature_fails() {
        let proof = LocationProof {
            latitude: 39.9042,
            longitude: 116.4074,
            timestamp_millis: 0,
            h3_cell: 0,
            signature: vec![],
            public_key: [0u8; 32],
        };
        assert!(!proof.verify(&[0u8; 32]));
    }

    #[test]
    fn test_envelope_new_with_pol() {
        let kp = KeyPair::generate();
        let payload_hash = blake3::hash(b"test").into();
        let proof = LocationProof::new(39.9042, 116.4074, 0x8a1fb46622dffff, &payload_hash, &kp);
        let envelope = Envelope::new_with_pol(
            MessageType::GeoCast,
            "test_peer",
            b"hello".to_vec(),
            Some(proof),
        );
        assert!(envelope.location_proof.is_some());
        assert_eq!(envelope.msg_type, MessageType::GeoCast);
    }

    #[test]
    fn test_pol_serialization_roundtrip() {
        let kp = KeyPair::generate();
        let payload_hash = blake3::hash(b"test").into();
        let proof = LocationProof::new(39.9042, 116.4074, 0x8a1fb46622dffff, &payload_hash, &kp);
        let envelope = Envelope::new_with_pol(
            MessageType::GeoCast,
            "test_peer",
            b"hello".to_vec(),
            Some(proof),
        );
        // CBOR roundtrip
        let bytes = envelope.serialize_cbor().unwrap();
        let deserialized = Envelope::deserialize_cbor(&bytes).unwrap();
        assert!(deserialized.location_proof.is_some());
        assert!(deserialized.location_proof.unwrap().verify(&payload_hash));
    }
}
