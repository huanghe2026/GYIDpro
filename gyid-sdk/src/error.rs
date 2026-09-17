//! SDK 错误类型

use thiserror::Error;

/// SDK 统一错误类型
#[derive(Debug, Error)]
pub enum SdkError {
    #[error("硬件指纹采集失败: {0}")]
    Fingerprint(String),

    #[error("地理位置获取失败: {0}")]
    Geo(String),

    #[error("头像处理失败: {0}")]
    Avatar(String),

    #[error("加密运算失败: {0}")]
    Crypto(String),

    #[error("存储操作失败: {0}")]
    Storage(String),

    #[error("设备关联失败: {0}")]
    DeviceLink(String),

    #[error("无效参数: {0}")]
    InvalidParam(String),

    #[error("权限不足: {0}")]
    Permission(String),

    #[error("网络请求失败: {0}")]
    Network(String),

    #[error("序列化失败: {0}")]
    Serialization(String),

    #[error("平台不支持: {0}")]
    PlatformNotSupported(String),
}

impl From<gyid_core::GyIdError> for SdkError {
    fn from(e: gyid_core::GyIdError) -> Self {
        match e {
            gyid_core::GyIdError::FingerprintError(s) => SdkError::Fingerprint(s),
            gyid_core::GyIdError::GeoError(s) => SdkError::Geo(s),
            gyid_core::GyIdError::AvatarError(s) => SdkError::Avatar(s),
            gyid_core::GyIdError::CryptoError(s) => SdkError::Crypto(s),
            gyid_core::GyIdError::StorageError(s) => SdkError::Storage(s),
            gyid_core::GyIdError::DeviceLinkError(s) => SdkError::DeviceLink(s),
            gyid_core::GyIdError::InvalidParam(s) => SdkError::InvalidParam(s),
            gyid_core::GyIdError::PermissionDenied(s) => SdkError::Permission(s),
        }
    }
}

impl From<serde_json::Error> for SdkError {
    fn from(e: serde_json::Error) -> Self {
        SdkError::Serialization(e.to_string())
    }
}

/// SDK 结果类型
pub type SdkResult<T> = Result<T, SdkError>;
