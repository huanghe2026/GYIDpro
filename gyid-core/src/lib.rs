//! GyID Core Library
//!
//! 去中心化身份系统的核心库，提供多维度账号生成功能。
//!
//! # Feature 说明
//! - `native` (默认): 完整功能，包含 SQLite 存储、sysinfo 硬件指纹、reqwest 网络
//! - `wasm`: WASM 平台，跳过所有不兼容的 native 依赖
//! - `chain-signing`: 链上签名功能 (k256 + keccak)

// 纯计算模块（所有平台可用）
pub mod crypto;
pub mod identity;
pub mod device;

// 头像处理（所有平台，image crate 支持 WASM）
pub mod avatar;

// 地理位置（所有平台，但实现因平台而异）
pub mod geo;

// 硬件指纹（所有平台，但实现因平台而异）
pub mod fingerprint;

// 存储模块（仅 native 平台）
#[cfg(feature = "native")]
pub mod storage;

// native 平台才有 storage 相关导出
#[cfg(feature = "native")]
pub use storage::{GyIdAnchor, ChainAnchor};
#[cfg(feature = "native")]
pub use storage::chain::Blockchain;

// WASM 平台提供 stub GyIdAnchor（保持类型兼容）
#[cfg(not(feature = "native"))]
mod storage_stub;
#[cfg(not(feature = "native"))]
pub use storage_stub::GyIdAnchor;

pub use identity::{GyIdGenerator, GyId};
pub use identity::GyIdValidator;
pub use identity::GyIdConfig;
pub use identity::GeneratorConfig;
pub use crypto::{GyIdHasher as Hasher, Base58Encoder as Encoder};
pub use geo::{GpsLocator, GpsAvailability, GeoPrecisionLevel, GeoLocation};
pub use geo::h3::{H3Grid, H3Cell};
pub use geo::nominatim::{NominatimGeocoder, ReverseGeoResult};

/// GyID 错误类型
pub type Result<T> = std::result::Result<T, GyIdError>;

/// GyID 核心错误
#[derive(Debug, thiserror::Error)]
pub enum GyIdError {
    #[error("硬件指纹采集失败: {0}")]
    FingerprintError(String),

    #[error("地理位置获取失败: {0}")]
    GeoError(String),

    #[error("头像处理失败: {0}")]
    AvatarError(String),

    #[error("加密运算失败: {0}")]
    CryptoError(String),

    #[error("存储操作失败: {0}")]
    StorageError(String),

    #[error("设备关联失败: {0}")]
    DeviceLinkError(String),

    #[error("无效的参数: {0}")]
    InvalidParam(String),

    #[error("权限不足: {0}")]
    PermissionDenied(String),
}
