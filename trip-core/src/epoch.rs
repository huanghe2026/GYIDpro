//! Epoch：一批面包屑的 Merkle 检查点（draft-04 §5）。
//!
//! CBOR 字段（key 0..=8）：
//!
//! | key | 含义 |
//! |-----|------|
//! | 0 | epoch 序号 |
//! | 1 | 身份公钥（32） |
//! | 2 | 首个面包屑 index |
//! | 3 | 末个面包屑 index |
//! | 4 | 首条时间戳 |
//! | 5 | 末条时间戳 |
//! | 6 | 面包屑块哈希的 Merkle root（32） |
//! | 7 | unique H3 cell 数 |
//! | 8 | 对字段 0..=7 的 Ed25519 签名（64） |
//!
//! Merkle 树使用 SHA-256、左右规范序；奇数节点复制最后一个哈希（Bitcoin 式），
//! 该选择属于 GeoYuan 实现约定，将写入 companion 规范。

use sha2::{Digest, Sha256};
use std::collections::HashSet;

use crate::breadcrumb::{fixed32, fixed64, Breadcrumb};
use crate::cbor::{parse_all, Writer};
use crate::crypto::{self, ProtocolKey};
use crate::error::{Result, TripError};

/// 对一组块哈希计算规范序 SHA-256 Merkle root。
///
/// - 空输入返回全零（调用方 [`Epoch::seal`] 禁止空 epoch）；
/// - 单层直接返回该哈希；
/// - 每层两两 `SHA-256(left || right)`，奇数则复制末节点。
pub fn merkle_root(hashes: &[[u8; 32]]) -> [u8; 32] {
    if hashes.is_empty() {
        return [0u8; 32];
    }
    let mut level: Vec<[u8; 32]> = hashes.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        let mut i = 0;
        while i < level.len() {
            let left = &level[i];
            let right = if i + 1 < level.len() {
                &level[i + 1]
            } else {
                // 奇数：复制最后一个节点
                &level[i]
            };
            let mut hasher = Sha256::new();
            hasher.update(left);
            hasher.update(right);
            let mut out = [0u8; 32];
            out.copy_from_slice(&hasher.finalize());
            next.push(out);
            i += 2;
        }
        level = next;
    }
    level[0]
}

/// 一个已密封的 epoch 检查点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Epoch {
    /// 字段 0：epoch 序号（从 0 起）。
    pub number: u64,
    /// 字段 1：身份公钥。
    pub identity: [u8; 32],
    /// 字段 2：首个面包屑 index。
    pub first_index: u64,
    /// 字段 3：末个面包屑 index。
    pub last_index: u64,
    /// 字段 4：首条时间戳。
    pub first_timestamp: u64,
    /// 字段 5：末条时间戳。
    pub last_timestamp: u64,
    /// 字段 6：Merkle root。
    pub merkle_root: [u8; 32],
    /// 字段 7：unique H3 cell 数。
    pub unique_cells: u64,
    /// 字段 8：签名。
    pub signature: [u8; 64],
}

impl Epoch {
    /// 用一批连续面包屑密封 epoch 并用身份密钥签名。
    pub fn seal(number: u64, crumbs: &[Breadcrumb], key: &ProtocolKey) -> Result<Self> {
        if crumbs.is_empty() {
            return Err(TripError::InvalidEpoch("cannot seal an empty epoch".into()));
        }
        let identity = key.public_bytes();
        if crumbs.iter().any(|c| c.identity != identity) {
            return Err(TripError::InvalidEpoch(
                "all breadcrumbs must be signed by the sealing identity key".into(),
            ));
        }

        let first = &crumbs[0];
        let last = &crumbs[crumbs.len() - 1];

        // index 连续性与时间单调性（Merkle 输入必须是有序的链片段）
        for (i, c) in crumbs.iter().enumerate() {
            let expected = first.index + i as u64;
            if c.index != expected {
                return Err(TripError::InvalidEpoch(format!(
                    "non-contiguous breadcrumb index: expected {expected}, got {}",
                    c.index
                )));
            }
            if c.timestamp < first.timestamp || c.timestamp > last.timestamp {
                return Err(TripError::InvalidEpoch(format!(
                    "timestamp {} outside epoch bounds",
                    c.timestamp
                )));
            }
        }

        let block_hashes: Vec<[u8; 32]> = crumbs.iter().map(Breadcrumb::block_hash).collect();
        let unique_cells = crumbs
            .iter()
            .map(|c| c.h3_cell)
            .collect::<HashSet<_>>()
            .len() as u64;

        let mut epoch = Self {
            number,
            identity,
            first_index: first.index,
            last_index: last.index,
            first_timestamp: first.timestamp,
            last_timestamp: last.timestamp,
            merkle_root: merkle_root(&block_hashes),
            unique_cells,
            signature: [0u8; 64],
        };
        epoch.signature = key.sign(&epoch.signable_bytes());
        Ok(epoch)
    }

    fn write_fields(&self, w: &mut Writer) {
        w.uint(0).uint(self.number);
        w.uint(1).bstr(&self.identity);
        w.uint(2).uint(self.first_index);
        w.uint(3).uint(self.last_index);
        w.uint(4).uint(self.first_timestamp);
        w.uint(5).uint(self.last_timestamp);
        w.uint(6).bstr(&self.merkle_root);
        w.uint(7).uint(self.unique_cells);
    }

    /// 字段 0..=7 的确定性 CBOR（签名输入）。
    pub fn signable_bytes(&self) -> Vec<u8> {
        let mut w = Writer::with_capacity(200);
        w.map(8);
        self.write_fields(&mut w);
        w.into_bytes()
    }

    /// 字段 0..=8 的完整 CBOR。
    pub fn to_cbor(&self) -> Vec<u8> {
        let mut w = Writer::with_capacity(270);
        w.map(9);
        self.write_fields(&mut w);
        w.uint(8).bstr(&self.signature);
        w.into_bytes()
    }

    /// 验证 epoch 自身签名。
    pub fn verify_signature(&self) -> Result<()> {
        crypto::verify(&self.identity, &self.signable_bytes(), &self.signature)
    }

    /// 校验给定面包屑片段与本 epoch 的 Merkle root 及边界一致。
    ///
    /// 调用方仍需先用 [`crate::chain::ChainRules`] 验证面包屑链本身。
    pub fn verify_coverage(&self, crumbs: &[Breadcrumb]) -> Result<()> {
        if crumbs.is_empty() {
            return Err(TripError::InvalidEpoch("no breadcrumbs to cover".into()));
        }
        let first = &crumbs[0];
        let last = &crumbs[crumbs.len() - 1];
        if first.index != self.first_index || last.index != self.last_index {
            return Err(TripError::InvalidEpoch(
                "breadcrumb index range does not match epoch bounds".into(),
            ));
        }
        if first.timestamp != self.first_timestamp || last.timestamp != self.last_timestamp {
            return Err(TripError::InvalidEpoch(
                "timestamp range does not match epoch bounds".into(),
            ));
        }
        if crumbs.iter().any(|c| c.identity != self.identity) {
            return Err(TripError::InvalidEpoch(
                "breadcrumb identity differs from epoch identity".into(),
            ));
        }
        let unique = crumbs
            .iter()
            .map(|c| c.h3_cell)
            .collect::<HashSet<_>>()
            .len() as u64;
        if unique != self.unique_cells {
            return Err(TripError::InvalidEpoch(format!(
                "unique cell mismatch: epoch claims {}, slice has {unique}",
                self.unique_cells
            )));
        }
        let hashes: Vec<[u8; 32]> = crumbs.iter().map(Breadcrumb::block_hash).collect();
        if merkle_root(&hashes) != self.merkle_root {
            return Err(TripError::InvalidEpoch(
                "merkle root mismatch: breadcrumb slice not covered by this epoch".into(),
            ));
        }
        Ok(())
    }

    /// 从完整 CBOR 解析（带规范化自检）。
    pub fn from_cbor(bytes: &[u8]) -> Result<Self> {
        let root = parse_all(bytes)?;
        let epoch = Self {
            number: root.field(0)?.as_uint()?,
            identity: fixed32(root.field(1)?.as_bstr()?)?,
            first_index: root.field(2)?.as_uint()?,
            last_index: root.field(3)?.as_uint()?,
            first_timestamp: root.field(4)?.as_uint()?,
            last_timestamp: root.field(5)?.as_uint()?,
            merkle_root: fixed32(root.field(6)?.as_bstr()?)?,
            unique_cells: root.field(7)?.as_uint()?,
            signature: fixed64(root.field(8)?.as_bstr()?)?,
        };
        if epoch.to_cbor() != bytes {
            return Err(TripError::Cbor(
                "epoch cbor is not in canonical deterministic form".into(),
            ));
        }
        Ok(epoch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::breadcrumb::{Breadcrumb, MetaFlags};

    fn chain_of(n: usize) -> Vec<Breadcrumb> {
        let key = ProtocolKey::from_seed(&[9u8; 32]);
        let pk = key.public_bytes();
        let mut crumbs = Vec::new();
        let mut prev = None;
        for i in 0..n {
            let mut c = Breadcrumb::new_unsigned(
                i as u64,
                pk,
                2_000_000_000 + i as u64 * 900,
                500 + i as u64,
                10,
                [0xabu8; 32],
                prev,
                MetaFlags::new(),
            );
            c.sign(&key).unwrap();
            prev = Some(c.block_hash());
            crumbs.push(c);
        }
        crumbs
    }

    #[test]
    fn seal_and_verify_coverage() {
        let crumbs = chain_of(100);
        let key = ProtocolKey::from_seed(&[9u8; 32]);
        let epoch = Epoch::seal(0, &crumbs, &key).unwrap();
        assert_eq!(epoch.unique_cells, 100);
        epoch.verify_signature().unwrap();
        epoch.verify_coverage(&crumbs).unwrap();
    }

    #[test]
    fn merkle_duplicate_last_hash_for_odd_nodes() {
        let a = [1u8; 32];
        // 三个节点：root = sha256(sha256(a,b) || sha256(c,c))
        let r3 = merkle_root(&[a, [2; 32], [3; 32]]);
        let mut hab = Sha256::new();
        hab.update([a, [2; 32]].concat());
        let hab = hab.finalize();
        let mut hcc = Sha256::new();
        hcc.update([[3u8; 32], [3u8; 32]].concat());
        let hcc = hcc.finalize();
        let mut top = Sha256::new();
        top.update(hab);
        top.update(hcc);
        let mut expected = [0u8; 32];
        expected.copy_from_slice(&top.finalize());
        assert_eq!(r3, expected);
    }
}
