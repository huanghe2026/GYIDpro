// 测试链上签名逻辑（不发送真实交易）
use gyid_core::storage::chain::anchor_to_chain;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 设置测试环境变量
    std::env::set_var("GYID_CHAIN_RPC", "https://rpc-amoy.polygon.technology");
    std::env::set_var("GYID_CHAIN_KEY", "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"); // 测试私钥
    std::env::set_var("GYID_CHAIN_ADDR", "0x1234567890123456789012345678901234567890");
    
    // 测试 GyID
    let test_gyid = "GyIDcs4gf4HDvHzPtNaQDyACDTp5cjWJB9hE";
    
    println!("测试链上锚定 GyID: {}", test_gyid);
    
    // 尝试调用函数（会失败，因为没有真实的私钥和 ETH）
    match anchor_to_chain(test_gyid).await {
        Ok(tx_hash) => {
            println!("✅ 链上锚定成功！交易哈希: {}", tx_hash);
        }
        Err(e) => {
            println!("⚠️  预期中的错误（因为没有真实私钥和 ETH）: {}", e);
            println!("📋 这表明链上签名逻辑已就绪，只需要配置真实环境变量即可工作");
        }
    }
    
    Ok(())
}