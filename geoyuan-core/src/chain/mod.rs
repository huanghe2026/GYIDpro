//! Blockchain core module
//!
//! Block, transaction, and state management

pub mod block;
pub mod transaction;
pub mod state;

pub use block::{Block, BlockHeader};
pub use transaction::{Transaction, TransactionType, TransactionReceipt};
pub use state::{AccountState, StateRoot};

use serde::{Serialize, Deserialize};
use crate::identity::GyId;

/// Chain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Chain ID
    pub chain_id: u64,
    /// Chain name
    pub name: String,
    /// Block time target (milliseconds)
    pub block_time_target: u64,
    /// Block gas limit (not used in GeoYuan, for compatibility)
    pub block_gas_limit: u64,
    /// Genesis timestamp
    pub genesis_timestamp: u64,
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            chain_id: 8848, // GeoYuan mainnet
            name: "GeoYuan".to_string(),
            block_time_target: 6000, // 6 seconds
            block_gas_limit: 0, // No gas limit
            genesis_timestamp: 1776000000000, // 2026-04-19
        }
    }
}

/// Block height
pub type BlockHeight = u64;

/// Block hash
pub type BlockHash = [u8; 32];

/// Transaction hash
pub type TxHash = [u8; 32];

/// State root hash
pub type StateHash = [u8; 32];

/// Address (20 bytes)
pub type Address = [u8; 20];

/// Generate address from GyID
pub fn address_from_gyid(gyid: &GyId) -> Address {
    use blake3::Hasher;
    
    let mut hasher = Hasher::new();
    hasher.update(gyid.as_bytes());
    let hash = hasher.finalize();
    
    let mut address = [0u8; 20];
    address.copy_from_slice(&hash.as_bytes()[..20]);
    address
}

/// Convert address to hex string
pub fn address_to_hex(addr: &Address) -> String {
    format!("0x{}", hex::encode(addr))
}

/// Parse address from hex string
pub fn address_from_hex(s: &str) -> Option<Address> {
    let s = s.strip_prefix("0x")?;
    let bytes = hex::decode(s).ok()?;
    if bytes.len() != 20 {
        return None;
    }
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&bytes);
    Some(addr)
}
