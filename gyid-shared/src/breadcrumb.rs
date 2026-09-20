//! 面包屑采集：从 (lat, lng) 经 H3 量化并签名一条面包屑。
//!
//! - [`collect_breadcrumb`] 端上采集 + 签名的封装：
//!   1. 校验 lat/lng 范围与 H3 分辨率 7..=10；
//!   2. `h3o::LatLng::new(lat, lng).to_cell(res)` 量化；
//!   3. 计算 §2.2 context digest（h3_cell + 5 分钟时间桶 + 可选环境分量）；
//!   4. 构造 [`trip_core::Breadcrumb`] 并 Ed25519 签名。
//!
//! 浏览器/移动端没有 GPS 时不要调用本函数；端上应缓存最近一次 GPS 锁定
//! 结果并以最短 5 分钟间隔触发（draft-04 §4.2 硬下限）。

use sha2::{Digest, Sha256};
use trip_core::breadcrumb::{Breadcrumb, MetaFlags};
use trip_core::ProtocolKey;

use crate::identity::Identity;
use crate::{GyidError, GyidResult};

/// context digest 域分离前缀（防跨域哈希碰撞）。
const CONTEXT_DOMAIN: &[u8] = b"gyid:context:v1";

/// context digest 默认时间桶大小（秒）——draft-01 §2.2 SHOULD ≥ 300s。
const CONTEXT_BUCKET_SECS: u64 = 300;

/// 上下文附加材料（可选；端上没有就 `None`）。
///
/// 各字段都进 SHA-256；空字段不写入。是否携带被镜像到
/// [`Breadcrumb::meta`] 的对应 `*_present` 标志。
#[derive(Debug, Default, Clone)]
pub struct ContextExtras {
    /// Wi-Fi BSSID（6 字节 MAC）。
    pub wifi_bssid: Option<[u8; 6]>,
    /// 基站 Cell-ID 字节序列（PLMN+CID 已由端上打包）。
    pub cell_id: Option<Vec<u8>>,
    /// IMU 摘要（端上已 HMAC 简化的 32 字节）。
    pub imu_digest: Option<[u8; 32]>,
}

impl ContextExtras {
    fn has_any(&self) -> bool {
        self.wifi_bssid.is_some() || self.cell_id.is_some() || self.imu_digest.is_some()
    }

    fn write_into(&self, hasher: &mut Sha256) {
        if let Some(b) = &self.wifi_bssid {
            hasher.update(b"wifi:");
            hasher.update(b);
        }
        if let Some(c) = &self.cell_id {
            hasher.update(b"cell:");
            hasher.update(c);
        }
        if let Some(i) = &self.imu_digest {
            hasher.update(b"imu:");
            hasher.update(i);
        }
    }
}

/// 计算 §2.2 context digest：`SHA-256(domain || h3_cell || ts_bucket_5min || extras)`。
///
/// 仅含 h3 + 5min 时间桶时为最小可用形式（草案允许的最弱强度）。
pub fn compute_context_digest(
    h3_cell: u64,
    timestamp: u64,
    extras: Option<&ContextExtras>,
) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(CONTEXT_DOMAIN);
    h.update(h3_cell.to_be_bytes());
    let bucket = timestamp / CONTEXT_BUCKET_SECS;
    h.update(bucket.to_be_bytes());
    if let Some(e) = extras {
        e.write_into(&mut h);
    }
    let out = h.finalize();
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&out);
    digest
}

/// 从 (lat, lng, ts) 经 H3 量化并签名一条面包屑。
///
/// - `h3_resolution` 协议要求 7..=10；
/// - `index` 是链内序号，由调用方（或 [`crate::chain::Chain::collect_and_append`]）维护；
/// - `prev_hash` 取链尾 [`crate::chain::Chain::block_hash_of_last`]；创世传 `None`；
/// - `exploration = true` 打开 §4.2 探索会话窗口（允许 5..15 分钟采集间隔）；
/// - `extras` 携带可选环境分量（Wi-Fi/Cell/IMU），没有就 `None`。
#[allow(clippy::too_many_arguments)] // 字段对齐 trip_core::Breadcrumb::new_unsigned
pub fn collect_breadcrumb(
    identity: &Identity,
    lat: f64,
    lng: f64,
    h3_resolution: u8,
    timestamp: u64,
    index: u64,
    prev_hash: Option<[u8; 32]>,
    exploration: bool,
    extras: Option<&ContextExtras>,
) -> GyidResult<Breadcrumb> {
    if !(-90.0..=90.0).contains(&lat) {
        return Err(GyidError::BadInput(format!("latitude out of range: {lat}")));
    }
    if !(-180.0..=180.0).contains(&lng) {
        return Err(GyidError::BadInput(format!("longitude out of range: {lng}")));
    }
    let res = h3o::Resolution::try_from(h3_resolution)
        .map_err(|e| GyidError::H3(format!("resolution: {e}")))?;
    if res < h3o::Resolution::Seven || res > h3o::Resolution::Ten {
        return Err(GyidError::BadInput(format!(
            "H3 resolution must be 7..=10, got {h3_resolution}"
        )));
    }
    let cell = h3o::LatLng::new(lat, lng)
        .map_err(|e| GyidError::H3(format!("lat/lng: {e}")))?
        .to_cell(res);
    let h3_cell = u64::from(cell);

    let context_digest = compute_context_digest(h3_cell, timestamp, extras);

    let mut meta = MetaFlags::new();
    meta.exploration = exploration;
    if let Some(e) = extras {
        if e.has_any() {
            meta.wifi_present = e.wifi_bssid.is_some();
            meta.cell_present = e.cell_id.is_some();
            meta.imu_present = e.imu_digest.is_some();
        }
    }

    let pubkey = identity.protocol_key().public_bytes();
    let mut bc = Breadcrumb::new_unsigned(
        index,
        pubkey,
        timestamp,
        h3_cell,
        h3_resolution,
        context_digest,
        prev_hash,
        meta,
    );
    let key: ProtocolKey = identity.protocol_key();
    bc.sign(&key)?;
    Ok(bc)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_id() -> Identity {
        Identity::from_seed_hex(&"07".repeat(32)).unwrap()
    }

    #[test]
    fn collect_basic_no_extras() {
        let id = fake_id();
        let bc = collect_breadcrumb(
            &id,
            39.9042,
            116.4074,
            10,
            1_700_000_000,
            0,
            None,
            false,
            None,
        )
        .unwrap();
        assert_eq!(bc.index, 0);
        assert_eq!(bc.h3_resolution, 10);
        assert!(bc.verify_signature().is_ok());
        // context digest 非零
        assert_ne!(bc.context_digest, [0u8; 32]);
        // meta 全 false（除 exploration=false）
        assert!(!bc.meta.exploration);
        assert!(!bc.meta.wifi_present);
    }

    #[test]
    fn collect_with_extras_sets_meta_flags() {
        let id = fake_id();
        let extras = ContextExtras {
            wifi_bssid: Some([0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01]),
            cell_id: Some(vec![0xAB, 0xCD]),
            imu_digest: Some([0x11; 32]),
        };
        let bc = collect_breadcrumb(
            &id,
            39.9042,
            116.4074,
            10,
            1_700_000_000,
            0,
            None,
            true,
            Some(&extras),
        )
        .unwrap();
        assert!(bc.meta.exploration);
        assert!(bc.meta.wifi_present);
        assert!(bc.meta.cell_present);
        assert!(bc.meta.imu_present);
    }

    #[test]
    fn reject_bad_lat_lng() {
        let id = fake_id();
        assert!(collect_breadcrumb(&id, 91.0, 0.0, 10, 0, 0, None, false, None).is_err());
        assert!(collect_breadcrumb(&id, 0.0, -181.0, 10, 0, 0, None, false, None).is_err());
    }

    #[test]
    fn reject_bad_resolution() {
        let id = fake_id();
        // res 6 / 11 都超协议范围
        assert!(collect_breadcrumb(&id, 39.9, 116.4, 6, 0, 0, None, false, None).is_err());
        assert!(collect_breadcrumb(&id, 39.9, 116.4, 11, 0, 0, None, false, None).is_err());
    }

    #[test]
    fn context_digest_is_deterministic_for_same_bucket() {
        let id = fake_id();
        // 1_700_000_000 落在 bucket 5_666_666（[1_699_999_800, 1_700_000_100)）。
        // 50 秒后仍在同桶；h3 cell 也相同 → context digest 相同。
        let bc1 = collect_breadcrumb(&id, 39.9042, 116.4074, 10, 1_700_000_000, 0, None, false, None)
            .unwrap();
        let bc2 = collect_breadcrumb(&id, 39.9042, 116.4074, 10, 1_700_000_050, 0, None, false, None)
            .unwrap();
        assert_eq!(bc1.context_digest, bc2.context_digest);
        // 跨桶 → 不同
        let bc3 = collect_breadcrumb(&id, 39.9042, 116.4074, 10, 1_700_000_300, 0, None, false, None)
            .unwrap();
        assert_ne!(bc1.context_digest, bc3.context_digest);
    }
}
