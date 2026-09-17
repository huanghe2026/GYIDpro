//! 面包屑（Breadcrumb）：TRIP Evidence 的原子单位（draft-04 §3）。
//!
//! CBOR 字段（key 0..=8）：
//!
//! | key | 含义 |
//! |-----|------|
//! | 0 | index 序号 |
//! | 1 | 身份公钥 Ed25519（32） |
//! | 2 | Unix 秒时间戳 |
//! | 3 | H3 cell index（u64） |
//! | 4 | H3 分辨率（7..=10） |
//! | 5 | context digest（32） |
//! | 6 | 前一块哈希（32）/ 创世为 null |
//! | 7 | meta flags map |
//! | 8 | Ed25519 签名（64） |
//!
//! 签名覆盖字段 0..=7 的确定性 CBOR；块哈希 = SHA-256(字段 0..=8 的完整编码)。

use sha2::{Digest, Sha256};

use crate::cbor::{parse_all, Value, Writer};
use crate::crypto;
use crate::error::{Result, TripError};

/// 面包屑字段 7：环境/采集标志。
///
/// key 0..=4 均以 bool 写入嵌套 CBOR map。GeoYuan 在草案开放的 meta 空间内
/// 增加 `photo_present`（多模态照片面包屑扩展，见 GYIP-0003 §4.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MetaFlags {
    /// 显式“探索会话”中：允许 5..15 分钟的较短采集间隔（§4.2）。
    pub exploration: bool,
    /// context digest 中包含 Wi-Fi BSSID 分量。
    pub wifi_present: bool,
    /// context digest 中包含基站分量。
    pub cell_present: bool,
    /// context digest 中包含 IMU 分量。
    pub imu_present: bool,
    /// GeoYuan 扩展：context digest 中包含照片分量。
    pub photo_present: bool,
}

impl MetaFlags {
    /// 全 false 的标志集。
    pub fn new() -> Self {
        Self::default()
    }

    fn entries(&self) -> [(u64, bool); 5] {
        [
            (0, self.exploration),
            (1, self.wifi_present),
            (2, self.cell_present),
            (3, self.imu_present),
            (4, self.photo_present),
        ]
    }

    /// 编码为独立的确定性 CBOR 字节（嵌套进面包屑 map）。
    pub fn to_cbor(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.map(5);
        for (k, v) in self.entries() {
            w.uint(k).bool(v);
        }
        w.into_bytes()
    }

    /// 从 CBOR Value 解析；未知 key 忽略（前向兼容）。
    fn from_value(v: &Value) -> Result<Self> {
        let mut flags = MetaFlags::default();
        if let Value::Map(pairs) = v {
            for (k, val) in pairs {
                let b = val.as_bool()?;
                match k.as_uint()? {
                    0 => flags.exploration = b,
                    1 => flags.wifi_present = b,
                    2 => flags.cell_present = b,
                    3 => flags.imu_present = b,
                    4 => flags.photo_present = b,
                    _ => {}
                }
            }
        } else {
            return Err(TripError::Cbor("meta flags must be a cbor map".into()));
        }
        Ok(flags)
    }
}

/// 一条面包屑。字段顺序即 CBOR key，禁止重排（签名稳定性）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Breadcrumb {
    /// 字段 0：链内序号，从 0 起连续。
    pub index: u64,
    /// 字段 1：身份公钥。
    pub identity: [u8; 32],
    /// 字段 2：Unix 秒时间戳（单调非减）。
    pub timestamp: u64,
    /// 字段 3：H3 cell index（原始 GPS 已在端上量化）。
    pub h3_cell: u64,
    /// 字段 4：H3 分辨率，合规范围 7..=10。
    pub h3_resolution: u8,
    /// 字段 5：context digest（环境信号的 SHA-256 承诺）。
    pub context_digest: [u8; 32],
    /// 字段 6：前一块哈希；创世面包屑为 None（CBOR null）。
    pub prev_hash: Option<[u8; 32]>,
    /// 字段 7：meta 标志。
    pub meta: MetaFlags,
    /// 字段 8：对字段 0..=7 的 Ed25519 签名。
    pub signature: [u8; 64],
}

impl Breadcrumb {
    /// 构造尚未签名的面包屑（字段 8 先置零）。
    #[allow(clippy::too_many_arguments)] // 参数对应草案 §3 Table 2 的固定字段
    pub fn new_unsigned(
        index: u64,
        identity: [u8; 32],
        timestamp: u64,
        h3_cell: u64,
        h3_resolution: u8,
        context_digest: [u8; 32],
        prev_hash: Option<[u8; 32]>,
        meta: MetaFlags,
    ) -> Self {
        Self {
            index,
            identity,
            timestamp,
            h3_cell,
            h3_resolution,
            context_digest,
            prev_hash,
            meta,
            signature: [0u8; 64],
        }
    }

    /// 用身份密钥签名字段 0..=7，并校验公钥与字段 1 一致。
    pub fn sign(&mut self, key: &crypto::ProtocolKey) -> Result<()> {
        if key.public_bytes() != self.identity {
            return Err(TripError::InvalidSignature);
        }
        self.signature = key.sign(&self.signable_bytes());
        Ok(())
    }

    /// 向写入器写入字段 0..=7 的内容（不含 map 头）。
    fn write_fields(&self, w: &mut Writer) {
        w.uint(0).uint(self.index);
        w.uint(1).bstr(&self.identity);
        w.uint(2).uint(self.timestamp);
        w.uint(3).uint(self.h3_cell);
        w.uint(4).uint(u64::from(self.h3_resolution));
        w.uint(5).bstr(&self.context_digest);
        w.uint(6);
        match self.prev_hash {
            Some(h) => {
                w.bstr(&h);
            }
            None => {
                w.null();
            }
        }
        w.uint(7).raw(&self.meta.to_cbor());
    }

    /// 字段 0..=7 的确定性 CBOR（签名输入）。
    pub fn signable_bytes(&self) -> Vec<u8> {
        let mut w = Writer::with_capacity(192);
        w.map(8);
        self.write_fields(&mut w);
        w.into_bytes()
    }

    /// 字段 0..=8 的完整确定性 CBOR（哈希输入 / 传输格式）。
    pub fn to_cbor(&self) -> Vec<u8> {
        let mut w = Writer::with_capacity(260);
        w.map(9);
        self.write_fields(&mut w);
        w.uint(8).bstr(&self.signature);
        w.into_bytes()
    }

    /// 块哈希 = SHA-256(完整 CBOR)（draft-04 §3.4）。
    pub fn block_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.to_cbor());
        let out = hasher.finalize();
        let mut h = [0u8; 32];
        h.copy_from_slice(&out);
        h
    }

    /// 用字段 1 公钥验证字段 8 签名。
    pub fn verify_signature(&self) -> Result<()> {
        crypto::verify(&self.identity, &self.signable_bytes(), &self.signature)
    }

    /// 从完整 CBOR 解析一条面包屑，逐字段做类型/长度校验。
    pub fn from_cbor(bytes: &[u8]) -> Result<Self> {
        let root = parse_all(bytes)?;

        let identity = fixed32(root.field(1)?.as_bstr()?)?;
        let context_digest = fixed32(root.field(5)?.as_bstr()?)?;
        let prev_hash = match root.field(6)? {
            Value::Null => None,
            Value::BStr(b) => Some(fixed32(b)?),
            other => {
                return Err(TripError::Cbor(format!(
                    "field 6 must be bstr(32) or null, got {other:?}"
                )))
            }
        };
        let signature = fixed64(root.field(8)?.as_bstr()?)?;
        let resolution = root.field(4)?.as_uint()? as u8;

        let crumb = Self {
            index: root.field(0)?.as_uint()?,
            identity,
            timestamp: root.field(2)?.as_uint()?,
            h3_cell: root.field(3)?.as_uint()?,
            h3_resolution: resolution,
            context_digest,
            prev_hash,
            meta: MetaFlags::from_value(root.field(7)?)?,
            signature,
        };
        // 重新编码必须与输入逐字节一致（规范化自检）。
        if crumb.to_cbor() != bytes {
            return Err(TripError::Cbor(
                "breadcrumb cbor is not in canonical deterministic form".into(),
            ));
        }
        Ok(crumb)
    }
}

/// 取定长 32 字节。
pub(crate) fn fixed32(b: &[u8]) -> Result<[u8; 32]> {
    b.try_into()
        .map_err(|_| TripError::Cbor(format!("expected 32-byte bstr, got {} bytes", b.len())))
}

/// 取定长 64 字节。
pub(crate) fn fixed64(b: &[u8]) -> Result<[u8; 64]> {
    b.try_into()
        .map_err(|_| TripError::Cbor(format!("expected 64-byte bstr, got {} bytes", b.len())))
}
