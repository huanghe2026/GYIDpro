//! # trip-core
//!
//! IETF TRIP（Trajectory-based Recognition of Identity Proof，
//! `draft-ayerbe-trip-protocol-04`，2026-05-08）的纯协议核心库。
//!
//! 本 crate 只包含与传输、存储、区块链无关的协议数据结构与算法：
//!
//! - 确定性 CBOR 编解码（RFC 8949 §4.2）：[`cbor`]
//! - Ed25519 身份/Verifier 密钥：[`crypto`]
//! - 面包屑与哈希链：[`breadcrumb`]、[`chain`]
//! - Epoch 与 Merkle 检查点：[`epoch`]
//! - Active Verification 实时挑战/响应（§12）：[`liveness`]
//! - Proof-of-Humanity 证书：[`poh`]
//! - DID 绑定（`did:geoyuan` + W3C DID Document）：[`did`]
//! - 轨迹身份令牌（TIT，CBOR/Base64url）：[`tit`]
//!
//! 空间量化（GPS→H3）、context digest 采集、网络传输与链上锚定属于
//! trip-attester / trip-server，不在本 crate。
//!
//! ## 协议基线
//!
//! 所有字段编号、签名覆盖范围、哈希算法均逐字对齐 draft-04，配套
//! 黄金测试向量（`tests/vectors/golden-vectors.json`）保证跨实现互操作。

#![forbid(unsafe_code)]

#[cfg(feature = "anchor")]
pub mod anchor;
pub mod breadcrumb;
pub mod cbor;
pub mod chain;
pub mod crypto;
pub mod did;
pub mod engine;
pub mod epoch;
pub mod error;
pub mod liveness;
pub mod poh;
pub mod tit;

#[cfg(feature = "anchor")]
pub use anchor::{AnchorCall, EcdsaKey, Eip1559Tx};
pub use breadcrumb::{Breadcrumb, MetaFlags};
pub use chain::ChainRules;
pub use crypto::ProtocolKey;
pub use did::{AnchorReference, DidDocument, DidDocumentConfig};
pub use epoch::Epoch;
pub use error::{Result, TripError};
pub use liveness::{LivenessChallenge, LivenessResponse};
pub use poh::PohCertificate;
pub use tit::{Tit, TitClaims, TitIssuer};
