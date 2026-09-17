//! TRIP 协议错误类型。

use thiserror::Error;

/// trip-core 所有操作的统一错误。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TripError {
    /// Ed25519 签名验证失败（密钥、消息或签名不匹配）。
    #[error("invalid ed25519 signature")]
    InvalidSignature,

    /// CBOR 编码/解码错误。
    #[error("cbor error: {0}")]
    Cbor(String),

    /// 面包屑哈希链不合法（index / 时间戳 / prevHash / 去重 / 间隔）。
    #[error("invalid breadcrumb chain: {0}")]
    InvalidChain(String),

    /// Epoch 密封输入不合法。
    #[error("invalid epoch: {0}")]
    InvalidEpoch(String),

    /// 公钥字节长度错误（期望 32）。
    #[error("invalid public key length: expected 32, got {0}")]
    InvalidPublicKeyLength(usize),

    /// 签名字节长度错误（期望 64）。
    #[error("invalid signature length: expected 64, got {0}")]
    InvalidSignatureLength(usize),

    /// PSD 样本数不足（DFT 至少需要 64 个位移样本）。
    #[error("insufficient PSD samples: need at least {0}, got {1}")]
    InsufficientPsdSamples(usize, usize),

    /// 位移信号退化（含非正数/非有限值，或频谱出现零点，无法做对数拟合）。
    #[error("degenerate displacement signal")]
    DegenerateSignal,

    /// 无效的 H3 cell index。
    #[error("invalid h3 cell index: 0x{0:016x}")]
    InvalidH3Cell(u64),

    /// Levy 参数估计错误。
    #[error("levy estimation error: {0}")]
    LevyFit(String),
}

/// trip-core 标准 Result。
pub type Result<T> = core::result::Result<T, TripError>;
