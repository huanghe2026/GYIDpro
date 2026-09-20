//! `gyid anchor …`：`GeoTITRegistry` 的 EVM JSON-RPC 客户端（GYIP-0003 §5.3）。
//!
//! 覆盖三次写操作（`register` / `anchorEpoch` / `claimHandle`）+ 合约部署 +
//! 两类查询（epoch 根 / 交易回执）。交易构造与签名全部复用
//! `trip-core::anchor`（可单测、无 I/O），本模块只负责 JSON-RPC 往返。
//!
//! 三条硬约束（与 GYIP-0003 §3 一致）：
//! 1. **只锚存在性**：往链上写的是 DID 后缀、epoch 的 Merkle 根、handle；
//!    不写 cell、不写照片、不写任何原始位置；
//! 2. **平台代付 gas**：写操作的签名者应是 Verifier / 中继的 EVM 地址
//!    （合约里 `verifier` 门控），普通用户无需持币；
//! 3. **身份密钥 ≠ 付费密钥**：身份根是 Ed25519（`trip-core::ProtocolKey`），
//!    这里用的是独立的 secp256k1 付费密钥（`EVM_PRIVATE_KEY`）。
//!
//! 私钥来源：`--key <hex>` 或环境变量 `EVM_PRIVATE_KEY`（推荐，避免进 shell 历史）。

use clap::Subcommand;
use serde_json::{json, Value};
use std::path::PathBuf;

use trip_core::anchor::{address_hex, bytes_hex, AnchorCall, EcdsaKey, Eip1559Tx};

/// 已知网络（name, chainId, rpc, explorer）。
const NETWORKS: &[(&str, u64, &str, &str)] = &[
    (
        "Base Mainnet",
        8453,
        "https://mainnet.base.org",
        "https://basescan.org",
    ),
    (
        "Base Sepolia",
        84532,
        "https://sepolia.base.org",
        "https://sepolia.basescan.org",
    ),
];

const DEFAULT_GAS_MULTIPLIER_NUM: u64 = 12;
const DEFAULT_GAS_MULTIPLIER_DEN: u64 = 10;

#[derive(Subcommand)]
pub enum AnchorCmd {
    /// 列出已知网络与默认 RPC
    Networks,
    /// 部署 GeoTITRegistry（读取 solc 产出的 artifact JSON）
    Deploy {
        /// RPC 端点
        #[arg(long)]
        rpc: String,
        /// 部署者私钥（hex；或设 EVM_PRIVATE_KEY）
        #[arg(long)]
        key: Option<String>,
        /// artifact JSON 路径
        #[arg(long, default_value = "contracts/artifacts/GeoTITRegistry.json")]
        artifact: PathBuf,
        /// Verifier / 中继地址（缺省 = 部署者）
        #[arg(long)]
        verifier: Option<String>,
        /// 只构造并打印，不广播
        #[arg(long)]
        dry_run: bool,
    },
    /// 登记身份（register：公钥 → DID 后缀，一次写定）
    Register {
        #[arg(long)]
        rpc: String,
        /// GeoTITRegistry 合约地址
        #[arg(long)]
        registry: String,
        #[arg(long)]
        key: Option<String>,
        /// 身份 Ed25519 公钥（64 hex）
        #[arg(long)]
        pubkey: String,
        /// DID 后缀（`z…`）；缺省由公钥派生
        #[arg(long)]
        did_suffix: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// 锚定 epoch（anchorEpoch：序号须从 0 起严格连续）
    Epoch {
        #[arg(long)]
        rpc: String,
        #[arg(long)]
        registry: String,
        #[arg(long)]
        key: Option<String>,
        /// 身份 Ed25519 公钥（64 hex）
        #[arg(long)]
        pubkey: String,
        /// epoch 序号
        #[arg(long)]
        epoch: u64,
        /// Merkle 根（64 hex）
        #[arg(long)]
        merkle: String,
        /// 该 epoch 内 unique H3 cell 数
        #[arg(long)]
        unique_cells: u32,
        #[arg(long)]
        dry_run: bool,
    },
    /// 声明 handle（claimHandle：需 n≥100 且 T≥20）
    Handle {
        #[arg(long)]
        rpc: String,
        #[arg(long)]
        registry: String,
        #[arg(long)]
        key: Option<String>,
        #[arg(long)]
        pubkey: String,
        /// 展示名（geoyuan.com 昵称）
        #[arg(long)]
        name: String,
        /// 面包屑总数
        #[arg(long)]
        breadcrumbs: u64,
        /// 信任分 ×100（T=62.5 → 6250）
        #[arg(long)]
        trust_x100: u64,
        #[arg(long)]
        dry_run: bool,
    },
    /// 查链上 epoch 根（eth_call）
    EpochRoot {
        #[arg(long)]
        rpc: String,
        #[arg(long)]
        registry: String,
        #[arg(long)]
        pubkey: String,
        #[arg(long)]
        epoch: u64,
    },
    /// 查交易回执（eth_getTransactionReceipt）
    Status {
        #[arg(long)]
        rpc: String,
        /// 交易哈希（0x…）
        tx_hash: String,
    },
}

/// 入口：由 `main` 调用。
pub async fn dispatch(cmd: AnchorCmd) {
    match cmd {
        AnchorCmd::Networks => {
            println!("已知网络（GYIP-0003 §5.3）：");
            for (name, chain_id, rpc, explorer) in NETWORKS {
                println!("  {name:<14} chainId={chain_id:<6} rpc={rpc}  explorer={explorer}");
            }
            println!();
            println!("写操作由 Verifier / 中继地址签名（合约 verifier 门控，平台代付 gas）。");
            println!("DID Document 的 `#anchor` 端点用 CAIP-2 形式，例如：");
            println!("  TRIP_ANCHOR=eip155:84532:0x<GeoTITRegistry 地址>");
        }
        AnchorCmd::Deploy {
            rpc,
            key,
            artifact,
            verifier,
            dry_run,
        } => {
            cmd_deploy(
                &rpc,
                key.as_deref(),
                &artifact,
                verifier.as_deref(),
                dry_run,
            )
            .await
        }
        AnchorCmd::Register {
            rpc,
            registry,
            key,
            pubkey,
            did_suffix,
            dry_run,
        } => {
            let pubkey = parse_bytes32(&pubkey, "pubkey");
            // 缺省用 multibase(base58btc) 后缀，与 did::encode 完全一致
            let suffix = did_suffix.unwrap_or_else(|| trip_core::did::multibase(&pubkey));
            let call = AnchorCall::Register {
                pubkey,
                did_suffix: suffix,
            };
            cmd_write(&rpc, &registry, key.as_deref(), call, dry_run).await;
        }
        AnchorCmd::Epoch {
            rpc,
            registry,
            key,
            pubkey,
            epoch,
            merkle,
            unique_cells,
            dry_run,
        } => {
            let call = AnchorCall::AnchorEpoch {
                pubkey: parse_bytes32(&pubkey, "pubkey"),
                epoch_no: epoch,
                merkle_root: parse_bytes32(&merkle, "merkle"),
                unique_cells,
            };
            cmd_write(&rpc, &registry, key.as_deref(), call, dry_run).await;
        }
        AnchorCmd::Handle {
            rpc,
            registry,
            key,
            pubkey,
            name,
            breadcrumbs,
            trust_x100,
            dry_run,
        } => {
            let call = AnchorCall::ClaimHandle {
                pubkey: parse_bytes32(&pubkey, "pubkey"),
                name,
                breadcrumbs,
                trust_x100,
            };
            cmd_write(&rpc, &registry, key.as_deref(), call, dry_run).await;
        }
        AnchorCmd::EpochRoot {
            rpc,
            registry,
            pubkey,
            epoch,
        } => cmd_epoch_root(&rpc, &registry, &pubkey, epoch).await,
        AnchorCmd::Status { rpc, tx_hash } => cmd_status(&rpc, &tx_hash).await,
    }
}

/* ------------------------------------------------------------------ */
/* 写路径                                                              */
/* ------------------------------------------------------------------ */

/// 构造 → 签名 → （可选）广播一笔合约调用。
async fn cmd_write(
    rpc: &str,
    registry: &str,
    key_hex: Option<&str>,
    call: AnchorCall,
    dry_run: bool,
) {
    let registry = parse_address(registry, "registry");
    let key = load_key(key_hex);

    let data = call.calldata();
    println!("方法      = {}", call.signature());
    println!("选择器    = 0x{}", hex::encode(call.selector()));
    println!("calldata  = {} bytes", data.len());
    println!("签名者    = {}", address_hex(&key.address()));

    let client = http_client();
    let chain_id = match rpc_u64(&client, rpc, "eth_chainId").await {
        Ok(v) => v,
        Err(e) => fail(&format!("读取 chainId 失败：{e}")),
    };
    warn_unknown_chain(chain_id);

    let from = key.address();
    let tx = match build_tx(&client, rpc, chain_id, &from, Some(registry), data, false).await {
        Ok(tx) => tx,
        Err(e) => fail(&e),
    };

    let raw = match key.sign_eip1559(&tx) {
        Ok(r) => r,
        Err(e) => fail(&format!("签名失败：{e}")),
    };

    print_tx_summary(&tx, &raw);

    if dry_run {
        println!("\n--dry-run：未广播。裸交易（可直接 eth_sendRawTransaction）：");
        println!("{}", bytes_hex(&raw));
        return;
    }

    match rpc_call(
        &client,
        rpc,
        "eth_sendRawTransaction",
        json!([bytes_hex(&raw)]),
    )
    .await
    {
        Ok(hash) => {
            println!("\n✓ 已广播: {}", hash.as_str().unwrap_or("?"));
            explorer_hint(chain_id, hash.as_str().unwrap_or(""));
        }
        Err(e) => fail(&format!("广播失败：{e}")),
    }
}

/// 部署合约（读 artifact 的 `bytecode`，`to = None`）。
async fn cmd_deploy(
    rpc: &str,
    key_hex: Option<&str>,
    artifact: &PathBuf,
    verifier: Option<&str>,
    dry_run: bool,
) {
    let key = load_key(key_hex);
    let text = match std::fs::read_to_string(artifact) {
        Ok(t) => t,
        Err(e) => fail(&format!(
            "读取 artifact 失败（先跑 `cd contracts && npm run compile`）：{e}"
        )),
    };
    let art: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => fail(&format!("artifact 不是合法 JSON：{e}")),
    };
    let bytecode_hex = art
        .get("bytecode")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| fail("artifact 缺少 bytecode 字段"));
    let bytecode = match hex::decode(bytecode_hex.trim_start_matches("0x")) {
        Ok(b) => b,
        Err(e) => fail(&format!("bytecode 不是合法 hex：{e}")),
    };

    let deployer = key.address();
    // 构造函数参数：address verifier_（默认 = 部署者），ABI 编码为 32 字节字
    let verifier_addr = match verifier {
        Some(s) => parse_address(s, "verifier"),
        None => deployer,
    };
    let mut data = bytecode.clone();
    let mut arg = [0u8; 32];
    arg[12..].copy_from_slice(&verifier_addr);
    data.extend_from_slice(&arg);

    println!("bytecode   = {} bytes（含构造参数 32）", data.len());
    println!("部署者     = {}", address_hex(&deployer));
    println!("verifier   = {}", address_hex(&verifier_addr));

    let client = http_client();
    let chain_id = match rpc_u64(&client, rpc, "eth_chainId").await {
        Ok(v) => v,
        Err(e) => fail(&format!("读取 chainId 失败：{e}")),
    };
    warn_unknown_chain(chain_id);

    let tx = match build_tx(&client, rpc, chain_id, &deployer, None, data, true).await {
        Ok(tx) => tx,
        Err(e) => fail(&e),
    };
    let raw = match key.sign_eip1559(&tx) {
        Ok(r) => r,
        Err(e) => fail(&format!("签名失败：{e}")),
    };
    print_tx_summary(&tx, &raw);

    if dry_run {
        println!("\n--dry-run：未广播。");
        return;
    }

    let hash = match rpc_call(
        &client,
        rpc,
        "eth_sendRawTransaction",
        json!([bytes_hex(&raw)]),
    )
    .await
    {
        Ok(h) => h.as_str().unwrap_or_default().to_string(),
        Err(e) => fail(&format!("广播失败：{e}")),
    };
    println!("\n✓ 已广播部署交易: {hash}");
    explorer_hint(chain_id, &hash);

    // 轮询回执拿合约地址
    for _ in 0..40 {
        match rpc_call(&client, rpc, "eth_getTransactionReceipt", json!([hash])).await {
            Ok(receipt) if !receipt.is_null() => {
                let status = receipt
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                if status != "0x1" {
                    fail("部署交易回执 status != 0x1");
                }
                match receipt.get("contractAddress").and_then(|v| v.as_str()) {
                    Some(addr) => {
                        println!("✓ GeoTITRegistry = {addr}");
                        println!("  DID Document 的锚定指针请设为：");
                        println!("  TRIP_ANCHOR=eip155:{chain_id}:{addr}");
                    }
                    None => fail("回执缺少 contractAddress"),
                }
                return;
            }
            Ok(_) => tokio::time::sleep(std::time::Duration::from_millis(1500)).await,
            Err(e) => fail(&format!("查询回执失败：{e}")),
        }
    }
    eprintln!("⚠ 60s 内未拿到回执，请用 `gyid anchor status --rpc {rpc} {hash}` 复查");
}

/* ------------------------------------------------------------------ */
/* 读路径                                                              */
/* ------------------------------------------------------------------ */

async fn cmd_epoch_root(rpc: &str, registry: &str, pubkey: &str, epoch: u64) {
    let registry = parse_address(registry, "registry");
    let pubkey = parse_bytes32(pubkey, "pubkey");
    let client = http_client();

    // epochRoot(bytes32,uint64) → 选择器 + 两个静态字
    let mut data = Vec::with_capacity(4 + 64);
    data.extend_from_slice(&trip_core::anchor::selector_of("epochRoot(bytes32,uint64)"));
    data.extend_from_slice(&pubkey);
    let mut epoch_word = [0u8; 32];
    epoch_word[24..].copy_from_slice(&epoch.to_be_bytes());
    data.extend_from_slice(&epoch_word);

    let result = match rpc_call(
        &client,
        rpc,
        "eth_call",
        json!([{ "to": address_hex(&registry), "data": bytes_hex(&data) }, "latest"]),
    )
    .await
    {
        Ok(v) => v,
        Err(e) => fail(&format!("eth_call 失败：{e}")),
    };

    let hex_str = result.as_str().unwrap_or_default();
    let bytes = match hex::decode(hex_str.trim_start_matches("0x")) {
        Ok(b) => b,
        Err(e) => fail(&format!("返回值不是 hex：{e}")),
    };
    if bytes.len() < 32 {
        fail(&format!("返回值长度异常：{} 字节", bytes.len()));
    }
    let root = &bytes[bytes.len() - 32..];
    let all_zero = root.iter().all(|b| *b == 0);

    println!("registry    = {}", address_hex(&registry));
    println!("pubkey      = {}", hex::encode(pubkey));
    println!("epoch       = {epoch}");
    println!(
        "epochKey    = 0x{}",
        hex::encode(AnchorCall::epoch_key(&pubkey, epoch))
    );
    if all_zero {
        println!("merkle_root = （未锚定）");
        eprintln!("\n✗ 该 epoch 尚无链上锚定记录");
    } else {
        println!("merkle_root = 0x{}", hex::encode(root));
        eprintln!("\n✓ 已锚定");
    }
}

async fn cmd_status(rpc: &str, tx_hash: &str) {
    let hash = if tx_hash.starts_with("0x") {
        tx_hash.to_string()
    } else {
        format!("0x{tx_hash}")
    };
    let client = http_client();
    let receipt = match rpc_call(&client, rpc, "eth_getTransactionReceipt", json!([hash])).await {
        Ok(v) => v,
        Err(e) => fail(&format!("查询失败：{e}")),
    };
    if receipt.is_null() {
        println!("交易 {hash} 尚未上链（pending 或不存在）");
        return;
    }
    println!("tx          = {hash}");
    for key in [
        "status",
        "blockNumber",
        "gasUsed",
        "from",
        "to",
        "contractAddress",
    ] {
        if let Some(v) = receipt.get(key) {
            if !v.is_null() {
                println!("{key:<12}= {}", v.as_str().unwrap_or(&v.to_string()));
            }
        }
    }
    if receipt.get("status").and_then(|v| v.as_str()) == Some("0x1") {
        eprintln!("\n✓ 交易成功");
    } else {
        eprintln!("\n✗ 交易失败（revert）");
    }
}

/* ------------------------------------------------------------------ */
/* JSON-RPC 与工具                                                     */
/* ------------------------------------------------------------------ */

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|e| fail(&format!("构造 HTTP 客户端失败：{e}")))
}

async fn rpc_call(
    client: &reqwest::Client,
    url: &str,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
    let resp = client
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("请求 {method} 失败（{url}）：{e}"))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("读取 {method} 响应失败（HTTP {status}）：{e}"))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|e| format!("{method} 响应不是 JSON（HTTP {status}）：{e}"))?;
    if let Some(err) = value.get("error") {
        return Err(format!("{method} RPC 错误：{err}"));
    }
    value
        .get("result")
        .cloned()
        .ok_or_else(|| format!("{method} 响应缺少 result"))
}

async fn rpc_u64(client: &reqwest::Client, url: &str, method: &str) -> Result<u64, String> {
    let v = rpc_call(client, url, method, json!([])).await?;
    parse_quantity_u64(v.as_str().unwrap_or_default())
}

/// 组一笔 EIP-1559 交易：查 nonce / 估 gas / 查 gasPrice。
async fn build_tx(
    client: &reqwest::Client,
    url: &str,
    chain_id: u64,
    from: &[u8; 20],
    to: Option<[u8; 20]>,
    data: Vec<u8>,
    is_deploy: bool,
) -> Result<Eip1559Tx, String> {
    let from_hex = address_hex(from);

    let nonce_v = rpc_call(
        client,
        url,
        "eth_getTransactionCount",
        json!([from_hex, "pending"]),
    )
    .await?;
    let nonce = parse_quantity_u64(nonce_v.as_str().unwrap_or_default())?;

    let mut tx_obj = json!({ "from": from_hex, "data": bytes_hex(&data), "value": "0x0" });
    if let Some(addr) = to {
        tx_obj["to"] = json!(address_hex(&addr));
    }
    let gas_v = rpc_call(client, url, "eth_estimateGas", json!([tx_obj])).await?;
    let estimated = parse_quantity_u64(gas_v.as_str().unwrap_or_default())?;
    let gas_limit = estimated * DEFAULT_GAS_MULTIPLIER_NUM / DEFAULT_GAS_MULTIPLIER_DEN;

    let gas_price_v = rpc_call(client, url, "eth_gasPrice", json!([])).await?;
    let gas_price = parse_quantity_u128(gas_price_v.as_str().unwrap_or_default())?;

    let hint = if is_deploy {
        "（部署）"
    } else {
        "（合约调用）"
    };
    println!("nonce     = {nonce}");
    println!("gasLimit  = {gas_limit}（估算 {estimated} ×1.2）{hint}");
    println!("gasPrice  = {gas_price} wei");

    Ok(Eip1559Tx {
        chain_id,
        nonce,
        max_priority_fee_per_gas: gas_price,
        max_fee_per_gas: gas_price.saturating_mul(2),
        gas_limit,
        to,
        value: 0,
        data,
    })
}

fn print_tx_summary(tx: &Eip1559Tx, raw: &[u8]) {
    println!("chainId   = {}", tx.chain_id);
    println!(
        "to        = {}",
        tx.to
            .map(|a| address_hex(&a))
            .unwrap_or_else(|| "（部署）".into())
    );
    println!("数字签名字节 = {} bytes", raw.len());
}

fn warn_unknown_chain(chain_id: u64) {
    if !NETWORKS.iter().any(|(_, id, _, _)| *id == chain_id) {
        eprintln!(
            "⚠ chainId {chain_id} 不在已知网络列表（Base 8453 / Base Sepolia 84532），请确认"
        );
    }
}

fn explorer_hint(chain_id: u64, tx_hash: &str) {
    if tx_hash.is_empty() {
        return;
    }
    let base = match chain_id {
        8453 => "https://basescan.org",
        84532 => "https://sepolia.basescan.org",
        _ => return,
    };
    println!("浏览器    : {base}/tx/{tx_hash}");
}

fn load_key(key_hex: Option<&str>) -> EcdsaKey {
    let hex_str = key_hex
        .map(str::to_string)
        .or_else(|| std::env::var("EVM_PRIVATE_KEY").ok())
        .unwrap_or_else(|| {
            fail("缺少付费私钥：传 --key <hex> 或设 EVM_PRIVATE_KEY（与身份 Ed25519 密钥不同）")
        });
    EcdsaKey::from_hex(&hex_str).unwrap_or_else(|e| fail(&format!("付费私钥无效：{e}")))
}

fn parse_address(s: &str, what: &str) -> [u8; 20] {
    let t = s.trim().trim_start_matches("0x");
    let bytes = hex::decode(t).unwrap_or_else(|e| fail(&format!("{what} 不是合法 hex：{e}")));
    bytes.as_slice().try_into().unwrap_or_else(|_| {
        fail(&format!(
            "{what} 必须是 20 字节地址（40 hex），收到 {} 字节",
            bytes.len()
        ))
    })
}

fn parse_bytes32(s: &str, what: &str) -> [u8; 32] {
    let t = s.trim().trim_start_matches("0x");
    let bytes = hex::decode(t).unwrap_or_else(|e| fail(&format!("{what} 不是合法 hex：{e}")));
    bytes.as_slice().try_into().unwrap_or_else(|_| {
        fail(&format!(
            "{what} 必须是 32 字节（64 hex），收到 {} 字节",
            bytes.len()
        ))
    })
}

fn parse_quantity_u64(hex_str: &str) -> Result<u64, String> {
    let t = hex_str.trim();
    if t.is_empty() {
        return Ok(0);
    }
    u64::from_str_radix(t.trim_start_matches("0x"), 16)
        .map_err(|e| format!("数量解析失败（{t}）：{e}"))
}

fn parse_quantity_u128(hex_str: &str) -> Result<u128, String> {
    let t = hex_str.trim();
    if t.is_empty() {
        return Ok(0);
    }
    u128::from_str_radix(t.trim_start_matches("0x"), 16)
        .map_err(|e| format!("数量解析失败（{t}）：{e}"))
}

fn fail(msg: &str) -> ! {
    eprintln!("✗ {msg}");
    std::process::exit(1);
}
