//! `did:geoyuan` —— TRIP 身份层的 W3C DID 绑定（GYIP-0003 §5.4）。
//!
//! 身份根从"算出来的字符串"变成"持有的密钥"（GYID2 §3.2）：DID 直接由
//! Ed25519 公钥派生，不含时间戳/随机盐，因此**同一密钥永远是同一身份**。
//!
//! ## DID 语法
//!
//! ```text
//! did:geoyuan:z<base58btc(Ed25519 public key, 32 bytes)>
//!             │
//!             └── 'z' = multibase 前缀，表示 base58btc（与 did:key 一致）
//! ```
//!
//! ## 与 TRIP 协议面的关系
//!
//! - TIT（[`crate::tit`]）持同一把公钥，故 TIT 天然绑定本 DID；
//! - PoH 证书字段 0（identity）即本公钥；
//! - 旧 `GyID…` 字符串退化为**品牌展示名**，由 [`DidDocument::also_known_as`]
//!   的 handle 与 geoyuan.com 侧映射承担，不再编码进身份根。
//!
//! ## DID Document
//!
//! 按 W3C DID Core 生成 JSON：`verificationMethod` 的
//! `publicKeyMultibase` 与 DID 用同一 multibase 编码；`service` 按需追加
//! TIT 端点、Verifier 端点、GPv1 libp2p 端点与链上锚定指针。

use base58::{FromBase58, ToBase58};
use serde::{Deserialize, Serialize};

use crate::error::{Result, TripError};

/// DID method 名（`did:geoyuan`）。
pub const DID_METHOD: &str = "geoyuan";
/// DID 前缀。
pub const DID_PREFIX: &str = "did:geoyuan:";
/// multibase 前缀 'z' = base58btc。
pub const MULTIBASE_BASE58BTC: char = 'z';
/// W3C DID Core 上下文。
pub const CONTEXT_DID_V1: &str = "https://www.w3.org/ns/did/v1";
/// GeoYuan 扩展上下文（TIT / handle / EVM 锚定指针）。
pub const CONTEXT_GYID_V1: &str = "https://gyid.geoyuan.com/did/v1";
/// 验证方法类型。
pub const KEY_TYPE: &str = "Ed25519VerificationKey2020";
/// 主密钥片段。
pub const KEY_FRAGMENT: &str = "#key-1";

/// service 类型：TIT 分发端点。
pub const SERVICE_TIT: &str = "TrajectoryIdentityToken";
/// service 类型：TRIP Verifier 端点（Active Verification / PoH）。
pub const SERVICE_VERIFIER: &str = "TripVerifier";
/// service 类型：GPv1 libp2p 端点。
pub const SERVICE_P2P: &str = "GPv1Libp2p";
/// service 类型：EVM 链上锚定指针。
pub const SERVICE_ANCHOR: &str = "EVMAnchor";

/// 32 字节公钥 → multibase(base58btc) 字符串（含 'z' 前缀）。
pub fn multibase(public_key: &[u8; 32]) -> String {
    format!("{MULTIBASE_BASE58BTC}{}", public_key.to_base58())
}

/// 32 字节公钥 → `did:geoyuan:z…`。
pub fn encode(public_key: &[u8; 32]) -> String {
    format!("{DID_PREFIX}{}", multibase(public_key))
}

/// `did:geoyuan:z…` → 32 字节公钥。
///
/// 严格拒绝：缺少 method 前缀、缺少 multibase 前缀、base58 解码失败、
/// 解码后不是 32 字节（§5.4 身份根只接受 Ed25519 公钥）。
pub fn decode(did: &str) -> Result<[u8; 32]> {
    let trimmed = did.trim();
    let rest = trimmed
        .strip_prefix(DID_PREFIX)
        .ok_or_else(|| TripError::InvalidDid(format!("missing prefix {DID_PREFIX}")))?;
    let rest = rest.strip_prefix(MULTIBASE_BASE58BTC).ok_or_else(|| {
        TripError::InvalidDid(format!("missing multibase '{MULTIBASE_BASE58BTC}' prefix"))
    })?;
    if rest.is_empty() {
        return Err(TripError::InvalidDid("empty base58 payload".into()));
    }
    let bytes = rest
        .from_base58()
        .map_err(|e| TripError::InvalidDid(format!("base58 decode failed: {e:?}")))?;
    let len = bytes.len();
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| TripError::InvalidPublicKeyLength(len))
}

/// 是否是合法且可解析的 `did:geoyuan`。
pub fn is_valid(did: &str) -> bool {
    decode(did).is_ok()
}

/// 主验证方法 id：`{did}#key-1`。
pub fn key_id(did: &str) -> String {
    format!("{did}{KEY_FRAGMENT}")
}

/// EVM 链上锚定指针（CAIP-2 `eip155:<chain_id>:<registry>`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorReference {
    /// EIP-155 chain id（Base 主网 = 8453）。
    pub chain_id: u64,
    /// `GeoTITRegistry` 合约地址（`0x…`）。
    pub registry: String,
}

impl AnchorReference {
    /// 新建。
    pub fn new(chain_id: u64, registry: impl Into<String>) -> Self {
        Self {
            chain_id,
            registry: registry.into(),
        }
    }

    /// CAIP-2 形式的 serviceEndpoint。
    pub fn to_service_endpoint(&self) -> String {
        format!("eip155:{}:{}", self.chain_id, self.registry)
    }

    /// 解析 CAIP-2 形式；非法返回 `None`。
    pub fn parse(s: &str) -> Option<Self> {
        let rest = s.strip_prefix("eip155:")?;
        let (chain, registry) = rest.split_once(':')?;
        let chain_id = chain.parse().ok()?;
        if registry.is_empty() {
            return None;
        }
        Some(Self::new(chain_id, registry))
    }
}

/// DID Document 构建参数（全部可选，缺省即不生成对应 service）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DidDocumentConfig {
    /// `#tit` 端点（TIT 分发/查询）。
    pub tit_endpoint: Option<String>,
    /// `#verifier` 端点（TRIP Verifier base URL）。
    pub verifier_endpoint: Option<String>,
    /// `#p2p` 端点（libp2p multiaddr 串）。
    pub p2p_endpoint: Option<String>,
    /// `#anchor` 链上锚定指针。
    pub anchor: Option<AnchorReference>,
    /// geoyuan.com 展示名（`@nickname`）；生成 `alsoKnownAs`。
    pub handle: Option<String>,
    /// 文档更新时间（Unix 秒）。
    pub updated: Option<u64>,
}

/// W3C DID Core 验证方法条目。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationMethod {
    /// 绝对 id（`{did}#key-1`）。
    pub id: String,
    /// 密钥类型。
    #[serde(rename = "type")]
    pub key_type: String,
    /// 控制者 DID。
    pub controller: String,
    /// multibase 公钥。
    #[serde(rename = "publicKeyMultibase")]
    pub public_key_multibase: String,
}

/// W3C DID Core service 条目。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DidService {
    /// 绝对 id（`{did}#tit` 等）。
    pub id: String,
    /// service 类型。
    #[serde(rename = "type")]
    pub service_type: String,
    /// 端点（URL / multiaddr / CAIP-2）。
    #[serde(rename = "serviceEndpoint")]
    pub service_endpoint: String,
}

/// `did:geoyuan` 的 DID Document（W3C DID Core）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DidDocument {
    /// JSON-LD 上下文，`@context` 必须在首位。
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    /// DID 本身。
    pub id: String,
    /// 验证方法列表（首条为主密钥）。
    #[serde(rename = "verificationMethod")]
    pub verification_method: Vec<VerificationMethod>,
    /// 认证用途引用。
    pub authentication: Vec<String>,
    /// 断言用途引用（TIT / PoH 声明）。
    #[serde(
        rename = "assertionMethod",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub assertion_method: Vec<String>,
    /// service 端点。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub service: Vec<DidService>,
    /// 其他标识（geoyuan.com handle 等）。
    #[serde(rename = "alsoKnownAs", default, skip_serializing_if = "Vec::is_empty")]
    pub also_known_as: Vec<String>,
    /// 更新时间（Unix 秒）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<u64>,
}

impl DidDocument {
    /// 生成最小 DID Document（仅主密钥 + authentication/assertionMethod）。
    pub fn new(public_key: &[u8; 32]) -> Self {
        Self::build(public_key, &DidDocumentConfig::default())
    }

    /// 按配置生成 DID Document。
    pub fn build(public_key: &[u8; 32], cfg: &DidDocumentConfig) -> Self {
        let did = encode(public_key);
        let key_id = key_id(&did);
        let mb = multibase(public_key);

        let mut service = Vec::new();
        if let Some(ep) = &cfg.tit_endpoint {
            service.push(DidService {
                id: format!("{did}#tit"),
                service_type: SERVICE_TIT.into(),
                service_endpoint: ep.clone(),
            });
        }
        if let Some(ep) = &cfg.verifier_endpoint {
            service.push(DidService {
                id: format!("{did}#verifier"),
                service_type: SERVICE_VERIFIER.into(),
                service_endpoint: ep.clone(),
            });
        }
        if let Some(ep) = &cfg.p2p_endpoint {
            service.push(DidService {
                id: format!("{did}#p2p"),
                service_type: SERVICE_P2P.into(),
                service_endpoint: ep.clone(),
            });
        }
        if let Some(anchor) = &cfg.anchor {
            service.push(DidService {
                id: format!("{did}#anchor"),
                service_type: SERVICE_ANCHOR.into(),
                service_endpoint: anchor.to_service_endpoint(),
            });
        }

        let also_known_as = cfg
            .handle
            .as_deref()
            .map(|h| {
                vec![format!(
                    "https://geoyuan.com/@{}",
                    h.trim_start_matches('@')
                )]
            })
            .unwrap_or_default();

        Self {
            context: vec![CONTEXT_DID_V1.into(), CONTEXT_GYID_V1.into()],
            id: did,
            verification_method: vec![VerificationMethod {
                id: key_id.clone(),
                key_type: KEY_TYPE.into(),
                controller: encode(public_key),
                public_key_multibase: mb,
            }],
            authentication: vec![key_id.clone()],
            assertion_method: vec![key_id],
            service,
            also_known_as,
            updated: cfg.updated,
        }
    }

    /// 序列化为紧凑 JSON。
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|e| TripError::InvalidDid(format!("to_json: {e}")))
    }

    /// 序列化为可读 JSON。
    pub fn to_json_pretty(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| TripError::InvalidDid(format!("to_json_pretty: {e}")))
    }

    /// 从 JSON 解析，并校验 `id` 与主验证方法自洽。
    pub fn from_json(json: &str) -> Result<Self> {
        let doc: Self = serde_json::from_str(json)
            .map_err(|e| TripError::InvalidDid(format!("from_json: {e}")))?;
        doc.public_key()?;
        Ok(doc)
    }

    /// 由 `id` 反解公钥，并校验 `verificationMethod` 首条与之一致。
    pub fn public_key(&self) -> Result<[u8; 32]> {
        let key = decode(&self.id)?;
        let vm = self
            .verification_method
            .first()
            .ok_or_else(|| TripError::InvalidDid("verificationMethod is empty".into()))?;
        if vm.controller != self.id {
            return Err(TripError::InvalidDid(format!(
                "verificationMethod controller {} does not match id {}",
                vm.controller, self.id
            )));
        }
        let mb = vm
            .public_key_multibase
            .strip_prefix(MULTIBASE_BASE58BTC)
            .ok_or_else(|| {
                TripError::InvalidDid(format!(
                    "publicKeyMultibase missing multibase '{MULTIBASE_BASE58BTC}' prefix"
                ))
            })?;
        let bytes = mb
            .from_base58()
            .map_err(|e| TripError::InvalidDid(format!("publicKeyMultibase base58: {e:?}")))?;
        let len = bytes.len();
        let vm_key: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| TripError::InvalidPublicKeyLength(len))?;
        if vm_key != key {
            return Err(TripError::InvalidDid(
                "publicKeyMultibase does not match did id".into(),
            ));
        }
        Ok(key)
    }

    /// 按 service 类型取端点。
    pub fn service_endpoint(&self, service_type: &str) -> Option<&str> {
        self.service
            .iter()
            .find(|s| s.service_type == service_type)
            .map(|s| s.service_endpoint.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_key_encodes_to_32_ones() {
        // base58btc 把 32 个前导零字节编成 32 个 '1'
        let did = encode(&[0u8; 32]);
        assert_eq!(did, format!("did:geoyuan:z{}", "1".repeat(32)));
        assert_eq!(decode(&did).unwrap(), [0u8; 32]);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let key = [0x9au8; 32];
        let did = encode(&key);
        assert!(did.starts_with("did:geoyuan:z"));
        assert_eq!(decode(&did).unwrap(), key);
        assert!(is_valid(&did));
    }

    #[test]
    fn decode_rejects_malformed() {
        assert!(decode("").is_err());
        assert!(decode("did:geoyuan:").is_err());
        assert!(decode("did:geoyuan:1").is_err()); // 缺 multibase
        assert!(decode("did:key:z6Mk").is_err()); // 错 method
        assert!(decode("did:geoyuan:z").is_err()); // 空 payload
        assert!(decode(&encode(&[0u8; 32]).replace("geoyuan", "geoyuan2")).is_err());
        // 合法 base58 但长度不足 32 字节
        assert!(decode("did:geoyuan:z2").is_err());
    }

    #[test]
    fn document_build_and_json_roundtrip() {
        let key = [7u8; 32];
        let cfg = DidDocumentConfig {
            tit_endpoint: Some("https://verify.geoyuan.com/v1/tit".into()),
            verifier_endpoint: Some("https://verify.geoyuan.com".into()),
            p2p_endpoint: Some("/ip4/1.2.3.4/tcp/4001/p2p/12D3KooW".into()),
            anchor: Some(AnchorReference::new(8453, "0xGeoTITRegistry")),
            handle: Some("@huanghe".into()),
            updated: Some(1_700_000_000),
        };
        let doc = DidDocument::build(&key, &cfg);

        assert_eq!(doc.id, encode(&key));
        assert_eq!(doc.public_key().unwrap(), key);
        assert_eq!(doc.context[0], CONTEXT_DID_V1);
        assert_eq!(doc.authentication[0], key_id(&doc.id));
        assert_eq!(doc.assertion_method[0], key_id(&doc.id));
        assert_eq!(
            doc.service_endpoint(SERVICE_TIT),
            Some("https://verify.geoyuan.com/v1/tit")
        );
        assert_eq!(
            doc.service_endpoint(SERVICE_ANCHOR),
            Some("eip155:8453:0xGeoTITRegistry")
        );
        assert_eq!(doc.also_known_as, vec!["https://geoyuan.com/@huanghe"]);

        // @context 必须序列化在首位（JSON-LD 要求）
        let json = doc.to_json().unwrap();
        assert!(json.starts_with(r#"{"@context":["https://www.w3.org/ns/did/v1""#));

        let parsed = DidDocument::from_json(&json).unwrap();
        assert_eq!(parsed, doc);
        assert_eq!(
            parsed,
            DidDocument::from_json(&doc.to_json_pretty().unwrap()).unwrap()
        );
    }

    #[test]
    fn document_rejects_inconsistent_verification_method() {
        let key = [7u8; 32];
        let mut doc = DidDocument::new(&key);
        doc.verification_method[0].public_key_multibase = multibase(&[8u8; 32]);
        assert!(doc.public_key().is_err());
    }

    #[test]
    fn anchor_reference_roundtrip() {
        let a = AnchorReference::new(8453, "0xAbC");
        assert_eq!(a.to_service_endpoint(), "eip155:8453:0xAbC");
        assert_eq!(AnchorReference::parse("eip155:8453:0xAbC"), Some(a));
        assert_eq!(AnchorReference::parse("eip155:8453:"), None);
        assert_eq!(AnchorReference::parse("solana:1:x"), None);
    }
}
