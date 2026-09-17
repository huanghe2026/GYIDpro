//! chain 命令 - 链上锚定管理
//!
//! 将 GyID 锚定到 Polygon 区块链，实现去中心化永久存储

use clap::{Parser, Subcommand};
use gyid_core::storage::{ChainAnchor, Blockchain, GyIdAnchor, LocalStorage};

/// 链上锚定命令
#[derive(Parser, Debug)]
#[command(name = "chain")]
pub struct ChainArgs {
    /// 子命令
    #[command(subcommand)]
    pub action: ChainAction,
}

/// 链上操作类型
#[derive(Subcommand, Debug, Clone)]
pub enum ChainAction {
    /// 将 GyID 锚定到链上
    Anchor {
        /// GyID 字符串
        gyid: String,
        
        /// 目标链 (默认 Polygon Amoy 测试网)
        #[arg(long, default_value = "amoy")]
        chain: String,
    },
    
    /// 验证链上锚定
    Verify {
        /// GyID 哈希
        gyid_hash: String,
        
        /// 交易哈希
        tx_hash: String,
        
        /// 目标链
        #[arg(long, default_value = "amoy")]
        chain: String,
    },
    
    /// 查询交易状态
    Status {
        /// 交易哈希
        tx_hash: String,
        
        /// 目标链
        #[arg(long, default_value = "polygon")]
        chain: String,
    },
    
    /// 显示支持的链网络
    Networks,
}

/// 运行链上命令
pub async fn run(action: &ChainAction) -> anyhow::Result<()> {
    match action {
        ChainAction::Anchor { gyid, chain } => {
            let chain = parse_chain(chain)?;
            anchor_gyid(gyid, chain).await?;
        }
        ChainAction::Verify { gyid_hash, tx_hash, chain } => {
            let chain = parse_chain(chain)?;
            verify_anchor(gyid_hash, tx_hash, chain).await?;
        }
        ChainAction::Status { tx_hash, chain } => {
            let chain = parse_chain(chain)?;
            query_status(tx_hash, chain).await?;
        }
        ChainAction::Networks => {
            show_networks()?;
        }
    }
    Ok(())
}

/// 解析链名称
fn parse_chain(name: &str) -> anyhow::Result<Blockchain> {
    match name.to_lowercase().as_str() {
        "polygon" | "mainnet" => Ok(Blockchain::Polygon),
        "amoy" | "testnet" | "mumbai" => Ok(Blockchain::PolygonAmoy),
        "aptos" => Ok(Blockchain::Aptos),
        "aptos-testnet" | "aptos-test" => Ok(Blockchain::AptosTestnet),
        _ => Err(anyhow::anyhow!(
            "未知的链: {} (支持: polygon, amoy, aptos, aptos-testnet)", name
        )),
    }
}

/// 链名称显示
fn chain_name(chain: Blockchain) -> &'static str {
    chain.name()
}

/// 锚定 GyID 到链上
async fn anchor_gyid(gyid_str: &str, chain: Blockchain) -> anyhow::Result<()> {
    println!("🔗 GyID 链上锚定");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  网络: {}", chain_name(chain));
    println!();
    
    // 检查环境变量（Aptos 有不同的变量名）
    if chain.is_aptos() {
        check_aptos_env_vars()?;
    } else {
        check_env_vars()?;
    }
    
    // 从 GyID 提取哈希
    let gyid_hash = extract_gyid_hash(gyid_str)?;
    println!("  GyID: {}", gyid_str);
    println!("  哈希: {}...", &gyid_hash[..16.min(gyid_hash.len())]);
    println!();
    
    // 创建锚定数据
    let anchor = GyIdAnchor::new(
        gyid_hash.clone(),
        format!("gyid_metadata_{}", chrono::Utc::now().timestamp_millis()),
        0, // linked_count
    );
    
    println!("  📡 正在发送交易...");
    println!();
    
    // 执行锚定
    match ChainAnchor::anchor_to(&anchor, chain).await {
        Ok(tx_hash) => {
            // 保存锚定记录到本地数据库
            if let Ok(storage) = LocalStorage::new(None) {
                let anchor_with_tx = anchor
                    .with_tx_hash(tx_hash.clone())
                    .with_chain(chain.name());
                let _ = storage.save_anchor(&anchor_with_tx);
            }

            println!("✅ 锚定成功！");
            println!();
            println!("📋 交易信息:");
            println!("  交易哈希: {}", tx_hash);
            println!("  区块: 待确认...");
            println!();
            let chain_arg = match chain {
                Blockchain::PolygonAmoy => "amoy",
                Blockchain::Aptos => "aptos",
                Blockchain::AptosTestnet => "aptos-testnet",
                _ => "polygon",
            };
            println!("💡 提示:");
            println!("  - 使用 'gyid chain status {} --chain {}' 查询状态", tx_hash, chain_arg);
            println!("  - 等待约 1-2 分钟确认后链上可查");
            println!("  - 建议保存交易哈希作为凭证");
        }
        Err(e) => {
            println!("❌ 锚定失败: {}", e);
            println!();
            if chain.is_aptos() {
                println!("💡 Aptos 常见问题:");
                println!("  - 确保 GYID_APTOS_KEY 包含有效的 ED25519 私钥 (64位 hex)");
                println!("  - 确保 GYID_APTOS_ADDR 为正确的 Aptos 账户地址");
                println!("  - 确保账户有足够的测试 APT (使用水龙头领取)");
                println!("  - 检查私钥格式：64 位 hex 字符，不带 0x 前缀");
            } else {
                println!("💡 Polygon 常见问题:");
                println!("  - 确保 GYID_CHAIN_KEY 环境变量包含有效的私钥");
                println!("  - 确保钱包有足够的 MATIC 支付 Gas");
                println!("  - 检查网络连接");
            }
        }
    }
    
    Ok(())
}

/// 验证链上锚定
async fn verify_anchor(gyid_hash: &str, tx_hash: &str, chain: Blockchain) -> anyhow::Result<()> {
    // 验证 tx_hash 格式
    validate_tx_hash(tx_hash)?;
    
    println!("🔍 验证链上锚定");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  网络: {}", chain_name(chain));
    println!("  GyID 哈希: {}...", &gyid_hash[..gyid_hash.len().min(16)]);
    println!("  交易哈希: {}", tx_hash);
    println!();
    
    println!("  📡 正在查询链上数据...");
    
    match ChainAnchor::verify_by_tx(gyid_hash, tx_hash, chain).await {
        Ok(valid) => {
            if valid {
                println!();
                println!("✅ 验证通过！");
                println!();
                println!("📋 验证详情:");
                println!("  - 链上存储的哈希与本地哈希匹配");
                println!("  - 锚定时间: 链上可查");
                println!("  - 永久性: 已确认");
            } else {
                println!();
                println!("❌ 验证失败！");
                println!("  链上存储的哈希与本地哈希不匹配");
            }
        }
        Err(e) => {
            println!();
            println!("❌ 验证出错: {}", e);
        }
    }
    
    Ok(())
}

/// 查询交易状态
async fn query_status(tx_hash: &str, chain: Blockchain) -> anyhow::Result<()> {
    // 验证 tx_hash 格式
    validate_tx_hash(tx_hash)?;
    
    println!("📊 查询交易状态");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  网络: {}", chain_name(chain));
    println!("  交易哈希: {}", tx_hash);
    println!();
    
    println!("  📡 正在查询...");
    
    match ChainAnchor::get_tx_status_on(tx_hash, chain).await {
        Ok(status) => {
            println!();
            println!("📋 交易状态:");
            
            if status.confirmed {
                println!("  ✅ 已确认");
                println!("  区块高度: {}", status.block_number.unwrap_or(0));
            } else {
                println!("  ⏳ 待确认 (可能在 mempool 中)");
            }
            
            println!("  执行结果: {}", if status.success { "成功 ✅" } else { "失败 ❌" });
            
            if let Some(gas) = status.gas_used {
                println!("  Gas 消耗: {}", gas);
            }
            
            println!();
            println!("🔗 区块浏览器:");
            let explorer = match chain {
                Blockchain::Polygon => "https://polygonscan.com/tx/",
                Blockchain::PolygonAmoy => "https://www.oklink.com/amoy/tx/",
                _ => "https://polygonscan.com/tx/",
            };
            println!("  {}{}", explorer, tx_hash);
        }
        Err(e) => {
            println!();
            println!("❌ 查询失败: {}", e);
        }
    }
    
    Ok(())
}

/// 显示支持的链网络
fn show_networks() -> anyhow::Result<()> {
    println!("🌐 GyID 支持的区块链网络");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("  📦 Polygon PoS");
    println!("     主网: Polygon Mainnet (chainId: 137)  --chain polygon");
    println!("     测试网: Polygon Amoy (chainId: 80002) --chain amoy");
    println!("     RPC: https://polygon-rpc.com");
    println!();
    println!("  📦 Aptos (完整签名支持)");
    println!("     主网: Aptos Mainnet                   --chain aptos");
    println!("     测试网: Aptos Testnet                 --chain aptos-testnet");
    println!("     API: https://fullnode.mainnet.aptoslabs.com/v1");
    println!("     签名: ED25519 + BCS 编码");
    println!();
    println!("💡 Polygon 快速开始:");
    println!("  $env:GYID_CHAIN_KEY = '<你的 EVM 私钥 hex>'");
    println!("  $env:GYID_CHAIN_ADDR = '<你的 0x... 地址>'");
    println!("  gyid chain anchor <GYID> --chain amoy");
    println!();
    println!("💡 Aptos 快速开始:");
    println!("  $env:GYID_APTOS_KEY = '<ED25519 私钥 hex (64字符)>'");
    println!("  $env:GYID_APTOS_ADDR = '<Aptos 地址 0x...>'");
    println!("  gyid chain anchor <GYID> --chain aptos-testnet");
    println!();
    println!("💡 获取 Aptos 测试网水龙头:");
    println!("  https://aptoslabs.com/testnet-faucet");
    println!("  需要先领取测试 APT 才能发起交易");
    
    Ok(())
}

/// 检查 Polygon 必需的环境变量
fn check_env_vars() -> anyhow::Result<()> {
    let key = std::env::var("GYID_CHAIN_KEY")
        .map(|_| "[已设置]".to_string())
        .unwrap_or_else(|_| "[未设置] ⚠️".to_string());
    
    let addr = std::env::var("GYID_CHAIN_ADDR")
        .map(|_| "[已设置]".to_string())
        .unwrap_or_else(|_| "[未设置] ⚠️".to_string());
    
    println!("  私钥状态 (GYID_CHAIN_KEY): {}", key);
    println!("  地址状态 (GYID_CHAIN_ADDR): {}", addr);
    println!();
    
    if key.contains("未设置") || addr.contains("未设置") {
        return Err(anyhow::anyhow!(
            "请先设置环境变量:\n  $env:GYID_CHAIN_KEY='<你的私钥>'\n  $env:GYID_CHAIN_ADDR='<你的地址>'"
        ));
    }
    
    Ok(())
}

/// 检查 Aptos 必需的环境变量
fn check_aptos_env_vars() -> anyhow::Result<()> {
    let key = std::env::var("GYID_APTOS_KEY")
        .map(|_| "[已设置]".to_string())
        .unwrap_or_else(|_| "[未设置] ⚠️".to_string());
    
    let addr = std::env::var("GYID_APTOS_ADDR")
        .map(|_| "[已设置]".to_string())
        .unwrap_or_else(|_| "[未设置] ⚠️".to_string());
    
    println!("  私钥状态 (GYID_APTOS_KEY): {}", key);
    println!("  地址状态 (GYID_APTOS_ADDR): {}", addr);
    println!();
    
    if key.contains("未设置") || addr.contains("未设置") {
        return Err(anyhow::anyhow!(
            "请先设置 Aptos 环境变量:\n  $env:GYID_APTOS_KEY='<ED25519私钥 hex>'\n  $env:GYID_APTOS_ADDR='<账户地址>'"
        ));
    }
    
    Ok(())
}

/// 验证交易哈希格式
fn validate_tx_hash(tx_hash: &str) -> anyhow::Result<()> {
    // 检查是否以 0x 开头
    if !tx_hash.starts_with("0x") && !tx_hash.starts_with("0X") {
        return Err(anyhow::anyhow!(
            "交易哈希格式错误：应使用 0x 前缀\n  例如: 0x1234abcd... (66 字符)"
        ));
    }
    
    // 检查长度 (0x + 64 字符 = 66 字符)
    let hex_part = &tx_hash[2..];
    if hex_part.len() != 64 {
        return Err(anyhow::anyhow!(
            "交易哈希长度错误：期望 66 字符 (0x + 64 hex)，实际 {} 字符\n  输入: {}",
            tx_hash.len(),
            tx_hash
        ));
    }
    
    // 检查是否为有效的 hex 字符
    if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(anyhow::anyhow!(
            "交易哈希包含无效字符：只允许 0-9, a-f, A-F\n  输入: {}",
            tx_hash
        ));
    }
    
    Ok(())
}

/// 从 GyID 字符串提取哈希
fn extract_gyid_hash(gyid_str: &str) -> anyhow::Result<String> {
    // GyID 格式: GyID<base58字符串>
    if !gyid_str.starts_with("GyID") {
        return Err(anyhow::anyhow!("无效的 GyID 格式，应以 'GyID' 开头"));
    }
    
    // 使用 GyID 字符串的哈希作为示例
    use gyid_core::crypto::GyIdHasher;
    let hash = GyIdHasher::hash_strings(&[gyid_str]);
    Ok(hash)
}
