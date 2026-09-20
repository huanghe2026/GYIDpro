//! 本地链管理：append、采集 + 自动续链、CBOR 帧流拼接、自检。
//!
//! - [`Chain`] 持有序面包屑 [`trip_core::Breadcrumb`]；
//! - [`Chain::collect_and_append`] 端上 GPS 采集一条并自动续链（index / prev_hash
//!   由链尾推出）；
//! - [`Chain::verify_self`] 用 [`trip_core::ChainRules::default`] 跑全链校验；
//! - [`Chain::to_cbor_stream`] / [`Chain::from_cbor_stream`] 适配
//!   `POST /v1/evidence` 的多帧拼接 body。

use trip_core::breadcrumb::Breadcrumb;
use trip_core::cbor::split_value;
use trip_core::chain::ChainRules;

use crate::breadcrumb::{collect_breadcrumb, ContextExtras};
use crate::identity::Identity;
use crate::{GyidError, GyidResult};

/// 本地有序面包屑集合（Attester 端）。
#[derive(Debug, Default, Clone)]
pub struct Chain {
    pub crumbs: Vec<Breadcrumb>,
}

impl Chain {
    /// 空链。
    pub fn new() -> Self {
        Self::default()
    }

    /// 从已有面包屑列表构造（不做校验；如需校验请随后调用 [`verify_self`](Self::verify_self)）。
    pub fn from_vec(crumbs: Vec<Breadcrumb>) -> Self {
        Self { crumbs }
    }

    pub fn len(&self) -> usize {
        self.crumbs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.crumbs.is_empty()
    }

    pub fn last(&self) -> Option<&Breadcrumb> {
        self.crumbs.last()
    }

    /// 链尾块哈希（PoH 绑定与续链用）。空链返回 `None`。
    pub fn block_hash_of_last(&self) -> Option<[u8; 32]> {
        self.crumbs.last().map(|c| c.block_hash())
    }

    /// 追加一条**已签名**面包屑。仅做基本结构衔接检查（index / prev_hash）；
    /// 不做完整链规则校验——调用方在合适时机调用 [`verify_self`](Self::verify_self)。
    pub fn append(&mut self, crumb: Breadcrumb) -> GyidResult<()> {
        if let Some(last) = self.crumbs.last() {
            if crumb.index != last.index + 1 {
                return Err(GyidError::BadInput(format!(
                    "non-contiguous index: expected {}, got {}",
                    last.index + 1,
                    crumb.index
                )));
            }
            if crumb.prev_hash != Some(last.block_hash()) {
                return Err(GyidError::BadInput(
                    "prev_hash does not match last block hash".into(),
                ));
            }
        } else if crumb.index != 0 || crumb.prev_hash.is_some() {
            return Err(GyidError::BadInput(
                "genesis crumb must have index=0 and prev_hash=None".into(),
            ));
        }
        self.crumbs.push(crumb);
        Ok(())
    }

    /// 端上采集一条面包屑并自动 append：`index` / `prev_hash` 由链尾推出。
    /// 返回新追加的面包屑（克隆）。
    #[allow(clippy::too_many_arguments)] // 端上采集一次需要的入参就是这样
    pub fn collect_and_append(
        &mut self,
        identity: &Identity,
        lat: f64,
        lng: f64,
        h3_resolution: u8,
        timestamp: u64,
        exploration: bool,
        extras: Option<&ContextExtras>,
    ) -> GyidResult<Breadcrumb> {
        let (index, prev_hash) = match self.crumbs.last() {
            Some(last) => (last.index + 1, Some(last.block_hash())),
            None => (0, None),
        };
        let crumb = collect_breadcrumb(
            identity,
            lat,
            lng,
            h3_resolution,
            timestamp,
            index,
            prev_hash,
            exploration,
            extras,
        )?;
        self.crumbs.push(crumb.clone());
        Ok(crumb)
    }

    /// 用 `ChainRules::default()` 跑全链自检（§4.1/§4.2/§4.3）。
    pub fn verify_self(&self) -> GyidResult<()> {
        if self.crumbs.is_empty() {
            return Err(GyidError::BadInput("empty chain".into()));
        }
        ChainRules::default()
            .verify(&self.crumbs)
            .map_err(GyidError::from)
    }

    /// 拼接所有面包屑 CBOR 帧，便于 `POST /v1/evidence` body。
    pub fn to_cbor_stream(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.crumbs.len() * 260);
        for c in &self.crumbs {
            out.extend_from_slice(&c.to_cbor());
        }
        out
    }

    /// 从 CBOR 帧流（多条面包屑顺序拼接）解析整链。
    pub fn from_cbor_stream(bytes: &[u8]) -> GyidResult<Self> {
        let mut crumbs = Vec::new();
        let mut rest = bytes;
        while !rest.is_empty() {
            let (one, tail) = split_value(rest).map_err(GyidError::from)?;
            let crumb = Breadcrumb::from_cbor(one).map_err(GyidError::from)?;
            crumbs.push(crumb);
            rest = tail;
        }
        Ok(Self { crumbs })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_id() -> Identity {
        Identity::from_seed_hex(&"07".repeat(32)).unwrap()
    }

    #[test]
    fn collect_two_breadcrumbs_passes_chain_rules() {
        let id = fake_id();
        let mut chain = Chain::new();
        chain
            .collect_and_append(&id, 39.9042, 116.4074, 10, 1_700_000_000, false, None)
            .unwrap();
        // 探索会话允许 5..15 分钟间隔
        chain
            .collect_and_append(&id, 39.9050, 116.4080, 10, 1_700_000_400, true, None)
            .unwrap();
        assert_eq!(chain.len(), 2);
        chain.verify_self().unwrap();
    }

    #[test]
    fn reject_genesis_with_prev_hash() {
        let id = fake_id();
        let bc = collect_breadcrumb(
            &id,
            39.9042,
            116.4074,
            10,
            1_700_000_000,
            0,
            Some([0u8; 32]),
            false,
            None,
        )
        .unwrap();
        let mut chain = Chain::new();
        assert!(chain.append(bc).is_err());
    }

    #[test]
    fn cbor_stream_roundtrip() {
        let id = fake_id();
        let mut chain = Chain::new();
        // 满足 ChainRules 默认 15 分钟间隔，跳过不同 cell
        chain
            .collect_and_append(&id, 39.9042, 116.4074, 10, 1_700_000_000, false, None)
            .unwrap();
        chain
            .collect_and_append(&id, 39.9150, 116.4270, 10, 1_700_000_900, false, None)
            .unwrap();

        let stream = chain.to_cbor_stream();
        let parsed = Chain::from_cbor_stream(&stream).unwrap();
        assert_eq!(parsed.len(), chain.len());
        // 解析后再跑一次全链规则
        parsed.verify_self().unwrap();
    }
}
