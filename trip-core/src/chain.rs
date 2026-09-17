//! 面包屑链管理（draft-04 §4）：去重、采集间隔、链验证。

use std::collections::HashMap;

use crate::breadcrumb::Breadcrumb;
use crate::error::{Result, TripError};

/// 链规则参数（草案默认值；部署方可调，但不得突破硬下限）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainRules {
    /// 默认最小采集间隔（秒），草案 SHOULD ≥ 900（15 分钟）。
    pub min_interval_secs: u64,
    /// 绝对硬下限（秒），即使探索会话也 MUST NOT 短于 300（5 分钟）。
    pub hard_min_interval_secs: u64,
    /// 单个 H3 cell 累计面包屑上限（防静止耕作，默认 10）。
    pub max_per_cell: u64,
}

impl Default for ChainRules {
    fn default() -> Self {
        Self {
            min_interval_secs: 15 * 60,
            hard_min_interval_secs: 5 * 60,
            max_per_cell: 10,
        }
    }
}

impl ChainRules {
    /// 验证完整面包屑链（按时间顺序排列）。
    ///
    /// 逐项执行草案 §4.3 的四条强制检查 + §4.1/§4.2 的链规则：
    ///
    /// 1. index 从 0 起连续；
    /// 2. 时间戳单调非减，且间隔满足采集规则；
    /// 3. prevHash 与前一块哈希一致，创世为 null；
    /// 4. 每条 Ed25519 签名可由字段 1 公钥验证；
    /// 5. 相邻面包屑 H3 cell 不得相同；
    /// 6. 每个 cell 累计数量不超上限。
    pub fn verify(&self, crumbs: &[Breadcrumb]) -> Result<()> {
        if crumbs.is_empty() {
            return Err(TripError::InvalidChain("empty breadcrumb chain".into()));
        }

        let identity = crumbs[0].identity;
        let mut prev_hash: Option<[u8; 32]> = None;
        let mut cell_counts: HashMap<u64, u64> = HashMap::new();

        for (i, crumb) in crumbs.iter().enumerate() {
            let expected_index = i as u64;

            // (1) index 连续
            if crumb.index != expected_index {
                return Err(TripError::InvalidChain(format!(
                    "non-contiguous index at position {i}: expected {expected_index}, got {}",
                    crumb.index
                )));
            }

            // 同一身份：字段 1 公钥必须恒定
            if crumb.identity != identity {
                return Err(TripError::InvalidChain(format!(
                    "identity public key changed at index {}",
                    crumb.index
                )));
            }

            // (4) 签名
            crumb.verify_signature()?;

            // (3) 哈希链
            if crumb.prev_hash != prev_hash {
                return Err(TripError::InvalidChain(format!(
                    "prev_hash mismatch at index {}: expected {:?}, got {:?}",
                    crumb.index, prev_hash, crumb.prev_hash
                )));
            }

            // (6) 单 cell 计数（在“与前一条不同”检查后累计）
            let count = cell_counts.entry(crumb.h3_cell).or_insert(0);
            *count += 1;
            if *count > self.max_per_cell {
                return Err(TripError::InvalidChain(format!(
                    "h3 cell {} recorded {} times, exceeds cap {}",
                    crumb.h3_cell, *count, self.max_per_cell
                )));
            }

            // (2)/(5)：相对前一条的时间与去重
            if i > 0 {
                let prev = &crumbs[i - 1];

                // (5) 相邻同 cell 必须拒绝（§4.1）
                if crumb.h3_cell == prev.h3_cell {
                    return Err(TripError::InvalidChain(format!(
                        "duplicate h3 cell {} at consecutive index {}",
                        crumb.h3_cell, crumb.index
                    )));
                }

                // (2) 单调非减
                if crumb.timestamp < prev.timestamp {
                    return Err(TripError::InvalidChain(format!(
                        "timestamp went backwards at index {}: {} < {}",
                        crumb.index, crumb.timestamp, prev.timestamp
                    )));
                }

                let delta = crumb.timestamp - prev.timestamp;
                if delta < self.hard_min_interval_secs {
                    return Err(TripError::InvalidChain(format!(
                        "interval {}s at index {} below hard minimum {}s",
                        delta, crumb.index, self.hard_min_interval_secs
                    )));
                }
                if delta < self.min_interval_secs && !crumb.meta.exploration {
                    return Err(TripError::InvalidChain(format!(
                        "interval {}s at index {} below default {}s without exploration flag",
                        delta, crumb.index, self.min_interval_secs
                    )));
                }
            }

            prev_hash = Some(crumb.block_hash());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::breadcrumb::{Breadcrumb, MetaFlags};
    use crate::crypto::ProtocolKey;

    fn make_chain(n: usize, step: u64, cells: &[u64]) -> Vec<Breadcrumb> {
        let key = ProtocolKey::from_seed(&[7u8; 32]);
        let pk = key.public_bytes();
        let mut crumbs = Vec::new();
        let mut prev: Option<[u8; 32]> = None;
        for i in 0..n {
            let mut c = Breadcrumb::new_unsigned(
                i as u64,
                pk,
                1_700_000_000 + i as u64 * step,
                cells[i % cells.len()],
                10,
                [0u8; 32],
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
    fn valid_chain_passes() {
        let chain = make_chain(4, 900, &[100, 101, 102, 103]);
        ChainRules::default().verify(&chain).unwrap();
    }

    #[test]
    fn exploration_allows_short_interval() {
        let key = ProtocolKey::from_seed(&[7u8; 32]);
        let pk = key.public_bytes();
        let mut c0 = Breadcrumb::new_unsigned(0, pk, 1000, 1, 10, [0; 32], None, MetaFlags::new());
        c0.sign(&key).unwrap();
        let h0 = c0.block_hash();
        let mut flags = MetaFlags::new();
        flags.exploration = true;
        let mut c1 = Breadcrumb::new_unsigned(1, pk, 1000 + 600, 2, 10, [0; 32], Some(h0), flags);
        c1.sign(&key).unwrap();

        ChainRules::default().verify(&[c0, c1]).unwrap();
    }

    #[test]
    fn duplicate_adjacent_cell_rejected() {
        let chain = make_chain(2, 900, &[100, 100]);
        assert!(ChainRules::default().verify(&chain).is_err());
    }

    #[test]
    fn too_short_interval_rejected() {
        // 200s < 300s 硬下限，即便探索标志也不允许（探索标志未设也同样拒绝）。
        let chain = make_chain(2, 200, &[100, 101]);
        assert!(ChainRules::default().verify(&chain).is_err());
    }

    #[test]
    fn tampered_signature_rejected() {
        let mut chain = make_chain(2, 900, &[100, 101]);
        chain[1].signature[0] ^= 0x01;
        assert!(ChainRules::default().verify(&chain).is_err());
    }
}
