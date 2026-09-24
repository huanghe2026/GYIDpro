//! 可选的链上中继：Verifier 代付 gas，把**存在性**写入 GeoTITRegistry
//! （GYIP-0003 §5.3，对齐合约 `onlyVerifier` 门控）。
//!
//! ## 启用条件（全部满足才启用，缺任一即静默关闭并打日志）
//! - `TRIP_ANCHOR=eip155:<chain_id>:<registry>`（CAIP-2，复用配置）；
//! - `EVM_PRIVATE_KEY=<secp256k1 hex>`：**付费**密钥，与 TRIP 身份密钥
//!   （`TRIP_VERIFIER_SEED` 的 Ed25519）相互独立，其地址必须是合约 verifier；
//! - `TRIP_EVM_RPC_URL`：可选；缺省按 chain id 选 Base 官方公共 RPC
//!   （84532 → sepolia.base.org，8453 → mainnet.base.org）。
//!
//! ## 触发点
//! - 首次主动验证成功（PoH 落库）→ `register(pubkey, multibase 后缀)`；
//! - 证据链被接受后 → 每凑满 `TRIP_EPOCH_SIZE`（默认 100）条连续面包屑，
//!   锚定一个 epoch：`anchorEpoch(pubkey, n, merkle_root, unique_cells)`。
//!
//! ## 设计约束
//! - 链上**只锚存在性**：DID 后缀、Merkle 根、唯一网格计数；不写坐标/照片/cell；
//! - 中继在独立 actor 任务中执行，所有链调用失败只记录 WARN：不阻塞
//!   HTTP/WS 响应，不影响验证流程；下次事件会重试；
//! - actor 单任务串行发交易，天然避免 nonce 竞争；每笔交易后轮询回执确认；
//! - 写前先 `identityOf` 查链，register/epoch 都按链上真实状态推进，
//!   进程重启后不会重放已完成的写操作（epoch 序号链上要求严格连续）。

use std::collections::HashSet;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::sync::mpsc;
use trip_core::anchor::{address_hex, bytes_hex, selector_of, AnchorCall, EcdsaKey, Eip1559Tx};
use trip_core::did::{multibase, AnchorReference};
use trip_core::epoch::merkle_root;
use trip_core::Breadcrumb;

/// 默认 RPC（chain id → Base 官方公共端点）。
const DEFAULT_RPC: &[(&str, u64, &str)] = &[
    ("Base Sepolia", 84532, "https://sepolia.base.org"),
    ("Base Mainnet", 8453, "https://mainnet.base.org"),
];

/// gas 估算安全余量（与 trip-cli 一致：×1.2）。
const GAS_NUM: u128 = 12;
const GAS_DEN: u128 = 10;
/// 回执轮询：最多 20 次 × 1.5s = 30s（Base 出块约 2s）。
const RECEIPT_POLLS: u32 = 20;
const RECEIPT_INTERVAL_MS: u64 = 1500;

/// 发给中继 actor 的内部事件。
enum RelayMsg {
    /// 主动验证通过、PoH 已签发 → 确保身份已登记。
    IdentityVerified { pubkey: [u8; 32] },
    /// 服务端接受了一条（合并后的）完整证据链 → 补齐所有已满的 epoch。
    EvidenceAccepted {
        pubkey: [u8; 32],
        chain: Vec<Breadcrumb>,
    },
}

/// 中继句柄（廉价 clone，可放进 AppState）。未启用时 handler 不持有它。
#[derive(Clone)]
pub struct ChainRelay {
    tx: mpsc::UnboundedSender<RelayMsg>,
}

impl ChainRelay {
    /// PoH 签发后调用：登记身份（幂等，actor 会先查链）。
    pub fn identity_verified(&self, pubkey: [u8; 32]) {
        let _ = self.tx.send(RelayMsg::IdentityVerified { pubkey });
    }

    /// 证据链落库后调用：锚定所有已满的 epoch（幂等，按链上 epochCount 推进）。
    pub fn evidence_accepted(&self, pubkey: [u8; 32], chain: Vec<Breadcrumb>) {
        let _ = self.tx.send(RelayMsg::EvidenceAccepted { pubkey, chain });
    }

    /// 按配置构造并启动中继；任何前置条件不满足都返回错误说明（调用方降级为关闭）。
    pub fn spawn(
        anchor: &AnchorReference,
        rpc_override: Option<&str>,
        epoch_size: usize,
    ) -> Result<ChainRelay, String> {
        let rpc = match rpc_override {
            Some(url) => url.to_string(),
            None => DEFAULT_RPC
                .iter()
                .find(|(_, id, _)| *id == anchor.chain_id)
                .map(|(_, _, url)| (*url).to_string())
                .ok_or_else(|| {
                    format!(
                        "chain id {} has no default RPC; set TRIP_EVM_RPC_URL explicitly",
                        anchor.chain_id
                    )
                })?,
        };
        let registry = parse_address(&anchor.registry)?;
        let key_hex = std::env::var("EVM_PRIVATE_KEY").map_err(|_| {
            "EVM_PRIVATE_KEY not set (secp256k1 payer key, distinct from TRIP_VERIFIER_SEED)"
        })?;
        let key =
            EcdsaKey::from_hex(&key_hex).map_err(|e| format!("invalid EVM_PRIVATE_KEY: {e}"))?;
        if epoch_size == 0 {
            return Err("epoch size must be positive".into());
        }

        let (tx, rx) = mpsc::unbounded_channel();
        let actor = RelayActor {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| format!("build HTTP client: {e}"))?,
            rpc,
            chain_id: anchor.chain_id,
            registry,
            key,
            epoch_size,
            registered: HashSet::new(),
        };
        let payer = address_hex(&actor.key.address());
        tokio::spawn(actor.run(rx));
        tracing::info!(
            chain_id = anchor.chain_id,
            registry = %address_hex(&registry),
            payer = %payer,
            epoch_size,
            "GeoTITRegistry on-chain relay ENABLED (verifier pays gas)"
        );
        Ok(ChainRelay { tx })
    }
}

/// actor 内部状态（单任务独占，无需锁）。
struct RelayActor {
    client: reqwest::Client,
    rpc: String,
    chain_id: u64,
    registry: [u8; 20],
    key: EcdsaKey,
    epoch_size: usize,
    /// 本进程已确认登记的公钥，省一次 eth_call；链上状态是最终事实源。
    registered: HashSet<[u8; 32]>,
}

impl RelayActor {
    async fn run(mut self, mut rx: mpsc::UnboundedReceiver<RelayMsg>) {
        while let Some(msg) = rx.recv().await {
            match msg {
                RelayMsg::IdentityVerified { pubkey } => {
                    if let Err(e) = self.ensure_registered(pubkey).await {
                        tracing::warn!(%e, pubkey = %hex::encode(pubkey), "register relay failed; will retry on next verification");
                    }
                }
                RelayMsg::EvidenceAccepted { pubkey, chain } => {
                    if let Err(e) = self.anchor_ready_epochs(pubkey, &chain).await {
                        tracing::warn!(%e, pubkey = %hex::encode(pubkey), "epoch anchor relay failed; will retry on next evidence upload");
                    }
                }
            }
        }
    }

    /// 查链确认登记状态；未登记则发 register。
    async fn ensure_registered(&mut self, pubkey: [u8; 32]) -> Result<(), String> {
        if self.registered.contains(&pubkey) {
            return Ok(());
        }
        let (is_registered, _) = self.identity_of(&pubkey).await?;
        if is_registered {
            self.registered.insert(pubkey);
            return Ok(());
        }
        let call = AnchorCall::Register {
            pubkey,
            did_suffix: multibase(&pubkey),
        };
        self.submit(call).await?;
        self.registered.insert(pubkey);
        tracing::info!(pubkey = %hex::encode(pubkey), "identity registered on GeoTITRegistry");
        Ok(())
    }

    /// 按链上 epochCount 依次锚定所有已凑满面包屑的 epoch；中途失败即停（下个事件重试）。
    async fn anchor_ready_epochs(
        &mut self,
        pubkey: [u8; 32],
        chain: &[Breadcrumb],
    ) -> Result<(), String> {
        // register 不前置：合约 anchorEpoch 也要求身份存在。证据先于验证到达时，
        // 未登记是正常状态（register 由首次验证触发），直接跳过。
        let (is_registered, onchain_count) = self.identity_of(&pubkey).await?;
        if !is_registered {
            return Ok(());
        }
        self.registered.insert(pubkey);

        for (epoch_no, start, end) in
            ready_epoch_ranges(onchain_count, self.epoch_size, chain.len())
        {
            let slice = &chain[start..end];
            // 链规则已保证 index 连续；这里再校验切片边界，防止非 0 起始链误用。
            let first_idx = slice.first().map(|c| c.index).unwrap_or(0);
            let last_idx = slice.last().map(|c| c.index).unwrap_or(0);
            if first_idx != start as u64 || last_idx != end as u64 - 1 {
                return Err(format!(
                    "epoch {epoch_no} slice index boundary mismatch: chain starts at {first_idx}, need index {start}..={}",
                    end - 1
                ));
            }
            let block_hashes: Vec<[u8; 32]> = slice.iter().map(Breadcrumb::block_hash).collect();
            let root = merkle_root(&block_hashes);
            let unique_cells = slice
                .iter()
                .map(|c| c.h3_cell)
                .collect::<HashSet<_>>()
                .len();
            let unique_cells = u32::try_from(unique_cells).unwrap_or(u32::MAX);

            let call = AnchorCall::AnchorEpoch {
                pubkey,
                epoch_no,
                merkle_root: root,
                unique_cells,
            };
            self.submit(call).await?;
            tracing::info!(
                pubkey = %hex::encode(pubkey),
                epoch = epoch_no,
                breadcrumbs = slice.len(),
                unique_cells,
                "epoch anchored on GeoTITRegistry"
            );
        }
        Ok(())
    }

    /// `identityOf(pubkey)` → (registered, epochCount)。
    /// RPC 对无合约地址可能返回空 `0x`，按 (false, 0) 处理（与网页端约定一致）。
    async fn identity_of(&self, pubkey: &[u8; 32]) -> Result<(bool, u32), String> {
        let mut data = selector_of("identityOf(bytes32)").to_vec();
        data.extend_from_slice(pubkey);
        let result = self
            .rpc_call(
                "eth_call",
                json!([
                    { "to": address_hex(&self.registry), "data": bytes_hex(&data) },
                    "latest"
                ]),
            )
            .await?;
        Ok(decode_identity(result.as_str().unwrap_or("0x")))
    }

    /// 构造 → 签名 → 广播 → 等回执（status 必须 0x1）。
    async fn submit(&self, call: AnchorCall) -> Result<(), String> {
        let from = self.key.address();
        let data = call.calldata();

        let nonce = self
            .rpc_call(
                "eth_getTransactionCount",
                json!([address_hex(&from), "pending"]),
            )
            .await
            .and_then(|v| parse_u64(v.as_str().unwrap_or_default()))?;

        let mut tx_obj = json!({
            "from": address_hex(&from),
            "to": address_hex(&self.registry),
            "data": bytes_hex(&data),
            "value": "0x0",
        });
        let gas_estimate = match self
            .rpc_call("eth_estimateGas", json!([tx_obj.clone()]))
            .await
            .and_then(|v| parse_u128(v.as_str().unwrap_or_default()))
        {
            Ok(g) => g,
            // 公共 RPC 偶发不支持 estimateGas：回退保守上限（register/epoch 均 <10 万）。
            Err(e) => {
                tracing::warn!(%e, "eth_estimateGas failed; falling back to 200_000 gas");
                tx_obj["gas"] = json!("0x30d40");
                200_000u128
            }
        };
        let gas_limit = gas_estimate.saturating_mul(GAS_NUM) / GAS_DEN;
        let gas_price = self
            .rpc_call("eth_gasPrice", json!([]))
            .await
            .and_then(|v| parse_u128(v.as_str().unwrap_or_default()))?;

        let tx = Eip1559Tx {
            chain_id: self.chain_id,
            nonce,
            max_priority_fee_per_gas: gas_price,
            max_fee_per_gas: gas_price.saturating_mul(2),
            gas_limit: u64::try_from(gas_limit).map_err(|_| "gas limit overflow")?,
            to: Some(self.registry),
            value: 0,
            data,
        };
        let raw = self.key.sign_eip1559(&tx).map_err(|e| e.to_string())?;
        let hash = self
            .rpc_call("eth_sendRawTransaction", json!([bytes_hex(&raw)]))
            .await
            .and_then(|v| {
                v.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| "sendRawTransaction returned non-string".to_string())
            })?;

        self.wait_mined(&hash, call.signature()).await
    }

    /// 轮询回执；status != 0x1 视为失败（触发上层重试）。
    async fn wait_mined(&self, hash: &str, method: &str) -> Result<(), String> {
        for _ in 0..RECEIPT_POLLS {
            tokio::time::sleep(Duration::from_millis(RECEIPT_INTERVAL_MS)).await;
            let receipt = self
                .rpc_call("eth_getTransactionReceipt", json!([hash]))
                .await?;
            if receipt.is_null() {
                continue;
            }
            return match receipt.get("status").and_then(|v| v.as_str()) {
                Some("0x1") => Ok(()),
                other => Err(format!("{method} tx {hash} reverted (status={other:?})")),
            };
        }
        Err(format!(
            "{method} tx {hash} not mined within {RECEIPT_POLLS} polls"
        ))
    }

    async fn rpc_call(&self, method: &str, params: Value) -> Result<Value, String> {
        let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
        let resp = self
            .client
            .post(&self.rpc)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("{method} request failed ({})：{e}", self.rpc))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("{method} read body failed (HTTP {status}): {e}"))?;
        let value: Value = serde_json::from_str(&text)
            .map_err(|e| format!("{method} non-JSON response (HTTP {status}): {e}"))?;
        if let Some(err) = value.get("error") {
            return Err(format!("{method} RPC error: {err}"));
        }
        value
            .get("result")
            .cloned()
            .ok_or_else(|| format!("{method} response missing result"))
    }
}

/* ------------------------------------------------------------------ */
/* 纯函数（单测覆盖）                                                    */
/* ------------------------------------------------------------------ */

/// 计算在长度 `chain_len` 的连续证据链上，链上已有 `onchain_count` 个 epoch 时，
/// 本次可以新锚定哪些 epoch：返回 (epoch_no, start_index, end_index)。
fn ready_epoch_ranges(
    onchain_count: u32,
    size: usize,
    chain_len: usize,
) -> Vec<(u64, usize, usize)> {
    let mut out = Vec::new();
    let mut n: u64 = onchain_count as u64;
    while let Some(start) = (n as usize).checked_mul(size) {
        let end = match start.checked_add(size) {
            Some(v) => v,
            None => break,
        };
        if chain_len < end {
            break;
        }
        out.push((n, start, end));
        n += 1;
    }
    out
}

/// 解 identityOf 六字元组：(registered, epochCount)。空/短返回按未登记处理。
fn decode_identity(raw_hex: &str) -> (bool, u32) {
    let Ok(bytes) = hex::decode(raw_hex.trim().trim_start_matches("0x")) else {
        return (false, 0);
    };
    // (bool, uint64, uint32, uint64, uint32, uint64) = 6 × 32 字节。
    if bytes.len() < 6 * 32 {
        return (false, 0);
    }
    let registered = bytes[31] != 0;
    let mut word = [0u8; 4];
    word.copy_from_slice(&bytes[2 * 32 + 28..2 * 32 + 32]);
    (registered, u32::from_be_bytes(word))
}

fn parse_address(s: &str) -> Result<[u8; 20], String> {
    let bytes = hex::decode(s.trim().trim_start_matches("0x"))
        .map_err(|e| format!("registry address not hex: {e}"))?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| format!("registry must be 20 bytes, got {}", bytes.len()))
}

fn parse_u64(hex_str: &str) -> Result<u64, String> {
    let t = hex_str.trim().trim_start_matches("0x");
    if t.is_empty() {
        return Ok(0);
    }
    u64::from_str_radix(t, 16).map_err(|e| format!("quantity parse: {e}"))
}

fn parse_u128(hex_str: &str) -> Result<u128, String> {
    let t = hex_str.trim().trim_start_matches("0x");
    if t.is_empty() {
        return Ok(0);
    }
    u128::from_str_radix(t, 16).map_err(|e| format!("quantity parse: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_ranges_basic() {
        // 250 条、链上 0 个、每片 100 → epoch 0、1
        let r = ready_epoch_ranges(0, 100, 250);
        assert_eq!(r, vec![(0, 0, 100), (1, 100, 200)]);

        // 链上已有 1 个 → 只补 epoch 1
        let r = ready_epoch_ranges(1, 100, 250);
        assert_eq!(r, vec![(1, 100, 200)]);

        // 不足一片
        assert!(ready_epoch_ranges(0, 100, 99).is_empty());
        // 恰好一片
        assert_eq!(ready_epoch_ranges(0, 100, 100), vec![(0, 0, 100)]);
        // 链上追平
        assert!(ready_epoch_ranges(2, 100, 250).is_empty());
        // 两片完整
        assert_eq!(
            ready_epoch_ranges(0, 100, 200),
            vec![(0, 0, 100), (1, 100, 200)]
        );
    }

    #[test]
    fn decode_identity_tuple() {
        // 空返回（Base 公共 RPC 对无合约地址的行为）→ 未登记
        assert_eq!(decode_identity("0x"), (false, 0));
        assert_eq!(decode_identity(""), (false, 0));

        // 构造 6 字元组：registered=true, registeredAt=0, epochCount=2,
        // lastEpoch=1, lastUniqueCells=7, handleClaimedAt=0
        let mut data = vec![0u8; 6 * 32];
        data[31] = 1;
        data[2 * 32 + 31] = 2;
        data[3 * 32 + 31] = 1;
        data[4 * 32 + 31] = 7;
        assert_eq!(
            decode_identity(&format!("0x{}", hex::encode(&data))),
            (true, 2)
        );

        // registered=false、epochCount=0
        let data2 = vec![0u8; 6 * 32];
        assert_eq!(
            decode_identity(&format!("0x{}", hex::encode(&data2))),
            (false, 0)
        );
    }

    #[test]
    fn address_parsing() {
        assert!(parse_address("0x000000000000000000000000000000000000dEaD").is_ok());
        assert!(parse_address("0x1234").is_err());
        assert!(parse_address("not-hex").is_err());
    }
}
