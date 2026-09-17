//! Wallet module - Geoyuan minting and transfer
//!
//! Each photo with GPS generates one Geoyuan (NFT)

pub mod coin;
pub mod mint;
pub mod mint_engine;
pub mod transfer;

pub use coin::{Geoyuan, CoinStatus, MintType};
pub use mint::CoinMinter;
pub use mint_engine::{
    CheckinChainMinter, DiscoveryMinter, DualVerificationMinter, PolNodeMinter, TrackMinter,
};
pub use transfer::CoinTransfer;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Wallet containing GyID and Geoyuans
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    /// Owner GyID
    pub gy_id: String,

    /// Wallet public key (for signing transactions)
    pub public_key: String,

    /// Owned Geoyuans
    pub coins: Vec<Geoyuan>,

    /// Total coin count
    pub balance: u64,

    /// Wallet created at
    pub created_at: DateTime<Utc>,

    /// Last updated at
    pub updated_at: DateTime<Utc>,
}

impl Wallet {
    /// Create new wallet
    pub fn new(gy_id: String, public_key: String) -> Self {
        let now = Utc::now();
        Self {
            gy_id,
            public_key,
            coins: Vec::new(),
            balance: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a coin to wallet
    pub fn add_coin(&mut self, coin: Geoyuan) {
        self.coins.push(coin);
        self.balance = self.coins.len() as u64;
        self.updated_at = Utc::now();
    }

    /// Remove a coin from wallet (for transfer)
    pub fn remove_coin(&mut self, coin_id: &str) -> Option<Geoyuan> {
        if let Some(pos) = self.coins.iter().position(|c| c.id == coin_id) {
            let coin = self.coins.remove(pos);
            self.balance = self.coins.len() as u64;
            self.updated_at = Utc::now();
            Some(coin)
        } else {
            None
        }
    }

    /// Get coin by ID
    pub fn get_coin(&self, coin_id: &str) -> Option<&Geoyuan> {
        self.coins.iter().find(|c| c.id == coin_id)
    }

    /// Total value in milli-GY (sum of all coins)
    pub fn total_value_milli_gy(&self) -> u64 {
        self.coins.iter().map(|c| c.value_milli_gy).sum()
    }

    /// Total value in GY (float)
    pub fn total_value_gy(&self) -> f64 {
        self.total_value_milli_gy() as f64 / 1000.0
    }

    /// Count coins by mint type
    pub fn count_by_type(&self, mint_type: MintType) -> u64 {
        self.coins
            .iter()
            .filter(|c| c.mint_type == mint_type)
            .count() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    
    #[test]
    fn test_wallet_operations() {
        let mut wallet = Wallet::new(
            "GyID7xK9m2AbCdEfGh123456".to_string(),
            "0".repeat(64),
        );
        
        assert_eq!(wallet.balance, 0);
        
        let coin = Geoyuan::new(
            Uuid::new_v4().to_string(),
            wallet.gy_id.clone(),
            39.9042,
            116.4074,
        );
        
        wallet.add_coin(coin.clone());
        assert_eq!(wallet.balance, 1);
        
        let retrieved = wallet.get_coin(&coin.id);
        assert!(retrieved.is_some());
        
        let removed = wallet.remove_coin(&coin.id);
        assert!(removed.is_some());
        assert_eq!(wallet.balance, 0);
    }
}
