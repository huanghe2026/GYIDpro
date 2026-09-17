//! Coin transfer module
//! 
//! Handles Geoyuan transfers between wallets

use super::{CoinStatus, Wallet};
use crate::{Result, GeoYuanError};
use crate::crypto::signer::{KeyPair, SignatureBytes};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Transfer transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinTransfer {
    /// Transaction ID
    pub tx_id: String,
    
    /// Coin ID being transferred
    pub coin_id: String,
    
    /// Sender GyID
    pub from: String,
    
    /// Recipient GyID
    pub to: String,
    
    /// Transaction timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Sender signature
    pub signature: String,
    
    /// Status
    pub status: TransferStatus,
}

impl CoinTransfer {
    /// Create new transfer
    pub fn new(
        coin_id: String,
        from: String,
        to: String,
        keypair: &KeyPair,
    ) -> Result<Self> {
        if from == to {
            return Err(GeoYuanError::Wallet("Cannot transfer to self".to_string()));
        }
        
        let tx_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now();
        
        // Create message to sign
        let message = format!("{}:{}:{}:{}", tx_id, coin_id, from, to);
        let signature = keypair.sign(message.as_bytes());
        
        Ok(Self {
            tx_id,
            coin_id,
            from,
            to,
            timestamp,
            signature: SignatureBytes::from_bytes(signature.to_bytes()).to_hex(),
            status: TransferStatus::Pending,
        })
    }
    
    /// Verify transfer signature
    pub fn verify_signature(&self, public_key_bytes: &[u8; 32]) -> bool {
        let message = format!("{}:{}:{}:{}", self.tx_id, self.coin_id, self.from, self.to);
        let sig_bytes: [u8; 64] = hex::decode(&self.signature)
            .ok()
            .and_then(|v| v.try_into().ok())
            .unwrap_or([0u8; 64]);
        
        KeyPair::verify_signature(
            message.as_bytes(),
            &sig_bytes,
            public_key_bytes,
        )
    }
}

/// Transfer status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TransferStatus {
    /// Transfer initiated, pending confirmation
    #[default]
    Pending,
    
    /// Transfer confirmed on chain
    Confirmed,
    
    /// Transfer failed
    Failed,
}

/// Coin transfer handler
pub struct CoinTransferHandler;

impl CoinTransferHandler {
    /// Execute transfer between wallets
    pub fn transfer(
        coin_id: &str,
        from_wallet: &mut Wallet,
        to_wallet: &mut Wallet,
        keypair: &KeyPair,
    ) -> Result<CoinTransfer> {
        // Verify ownership
        let _coin = from_wallet.get_coin(coin_id)
            .ok_or_else(|| GeoYuanError::Wallet("Coin not found in wallet".to_string()))?;
        
        // Create transfer
        let mut transfer = CoinTransfer::new(
            coin_id.to_string(),
            from_wallet.gy_id.clone(),
            to_wallet.gy_id.clone(),
            keypair,
        )?;
        
        // Remove coin from sender
        let removed_coin = from_wallet.remove_coin(coin_id)
            .ok_or_else(|| GeoYuanError::Wallet("Failed to remove coin".to_string()))?;
        
        // Update coin ownership
        let mut new_coin = removed_coin;
        new_coin.owner = to_wallet.gy_id.clone();
        new_coin.previous_tx = Some(transfer.tx_id.clone());
        
        // Add to recipient
        to_wallet.add_coin(new_coin);
        
        // Mark transfer as confirmed (for local wallet)
        transfer.status = TransferStatus::Confirmed;
        
        Ok(transfer)
    }
    
    /// Verify transfer can be executed
    pub fn can_transfer(wallet: &Wallet, coin_id: &str) -> Result<()> {
        // Check coin exists
        let _coin = wallet.get_coin(coin_id)
            .ok_or_else(|| GeoYuanError::Wallet("Coin not found".to_string()))?;
        
        // Check coin is not being transferred
        if _coin.status == CoinStatus::Transferring {
            return Err(GeoYuanError::Wallet("Coin is being transferred".to_string()));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::Geoyuan;
    
    #[test]
    fn test_create_transfer() {
        let keypair = KeyPair::generate();
        
        let transfer = CoinTransfer::new(
            "coin123".to_string(),
            "GyID7xK9m2AbCdEfGh123456".to_string(),
            "GyID8yL0n3BcDeFgHi789012".to_string(),
            &keypair,
        ).unwrap();
        
        assert!(transfer.tx_id.len() > 0);
        assert_eq!(transfer.status, TransferStatus::Pending);
        
        // Verify signature
        assert!(transfer.verify_signature(&keypair.public_bytes()));
    }
    
    #[test]
    fn test_transfer_coins() {
        let keypair = KeyPair::generate();
        
        // Sender wallet
        let mut sender = Wallet::new(
            "GyID7xK9m2AbCdEfGh123456".to_string(),
            keypair.public_hex(),
        );
        
        // Add coin to sender
        let coin = Geoyuan::new(
            Uuid::new_v4().to_string(),
            sender.gy_id.clone(),
            39.9042,
            116.4074,
        );
        sender.add_coin(coin.clone());
        
        // Recipient wallet
        let recipient = Wallet::new(
            "GyID8yL0n3BcDeFgHi789012".to_string(),
            "1".repeat(64),
        );
        let mut recipient = recipient;
        
        let transfer = CoinTransferHandler::transfer(
            &coin.id,
            &mut sender,
            &mut recipient,
            &keypair,
        ).unwrap();
        
        assert_eq!(sender.balance, 0);
        assert_eq!(recipient.balance, 1);
        assert_eq!(transfer.status, TransferStatus::Confirmed);
    }
    
    #[test]
    fn test_transfer_to_self_fails() {
        let keypair = KeyPair::generate();
        
        let transfer = CoinTransfer::new(
            "coin123".to_string(),
            "GyID7xK9m2AbCdEfGh123456".to_string(),
            "GyID7xK9m2AbCdEfGh123456".to_string(),
            &keypair,
        );
        
        assert!(transfer.is_err());
    }
}
