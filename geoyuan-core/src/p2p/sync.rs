//! Data synchronization module
//! 
//! Syncs wallet data between peers

use crate::wallet::Wallet;
use crate::identity::GyId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sync message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMessage {
    /// Request wallet data
    RequestWallet { geo_id: String },
    /// Response with wallet data
    ResponseWallet { wallet: Wallet },
    /// Request GyID data
    RequestGeoId { gy_id: String },
    /// Response with GyID
    ResponseGeoId { gy_id: GyId },
    /// New coin minted (broadcast)
    CoinMinted { coin_id: String, owner: String, location: (f64, f64) },
    /// Coin transferred (broadcast)
    CoinTransferred { coin_id: String, from: String, to: String },
}

/// Sync state
#[derive(Debug, Clone, Default)]
pub struct SyncState {
    /// Known wallets by GyID
    wallets: HashMap<String, Wallet>,
    
    /// Known GyIDs
    geo_ids: HashMap<String, GyId>,
    
    /// Pending requests
    pending_requests: HashMap<String, std::time::Instant>,
}

impl SyncState {
    /// Create new sync state
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Store wallet
    pub fn store_wallet(&mut self, wallet: Wallet) {
        self.wallets.insert(wallet.gy_id.clone(), wallet);
    }
    
    /// Get wallet by GeoID
    pub fn get_wallet(&self, geo_id: &str) -> Option<&Wallet> {
        self.wallets.get(geo_id)
    }
    
    /// Store GyID
    pub fn store_gy_id(&mut self, gy_id: GyId) {
        self.geo_ids.insert(gy_id.id.clone(), gy_id);
    }
    
    /// Get GyID
    pub fn get_gy_id(&self, id: &str) -> Option<&GyId> {
        self.geo_ids.get(id)
    }
    
    /// Add pending request
    pub fn add_pending(&mut self, request_id: String) {
        self.pending_requests.insert(request_id, std::time::Instant::now());
    }
    
    /// Check if request is pending
    pub fn is_pending(&self, request_id: &str) -> bool {
        if let Some(instant) = self.pending_requests.get(request_id) {
            // Timeout after 30 seconds
            instant.elapsed().as_secs() < 30
        } else {
            false
        }
    }
    
    /// Cleanup old pending requests
    pub fn cleanup_pending(&mut self) {
        self.pending_requests.retain(|_, instant| {
            instant.elapsed().as_secs() < 30
        });
    }
}

/// Sync handler
pub struct SyncHandler {
    state: SyncState,
}

impl SyncHandler {
    /// Create new sync handler
    pub fn new() -> Self {
        Self {
            state: SyncState::new(),
        }
    }
    
    /// Process incoming sync message
    pub fn process_message(&mut self, message: SyncMessage) -> Option<SyncMessage> {
        match message {
            SyncMessage::RequestWallet { geo_id } => {
                // Return wallet if we have it
                self.state.get_wallet(&geo_id).map(|wallet| {
                    SyncMessage::ResponseWallet {
                        wallet: wallet.clone(),
                    }
                })
            }
            SyncMessage::ResponseWallet { wallet } => {
                // Store wallet
                self.state.store_wallet(wallet);
                None
            }
            SyncMessage::RequestGeoId { gy_id } => {
                self.state.get_gy_id(&gy_id).map(|gy_id| {
                    SyncMessage::ResponseGeoId {
                        gy_id: gy_id.clone(),
                    }
                })
            }
            SyncMessage::ResponseGeoId { gy_id } => {
                self.state.store_gy_id(gy_id);
                None
            }
            // Broadcast messages don't need response
            SyncMessage::CoinMinted { .. } => None,
            SyncMessage::CoinTransferred { .. } => None,
        }
    }
    
    /// Get sync state
    pub fn state(&self) -> &SyncState {
        &self.state
    }
    
    /// Mutable state access
    pub fn state_mut(&mut self) -> &mut SyncState {
        &mut self.state
    }
}

impl Default for SyncHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sync_handler_wallet() {
        let mut handler = SyncHandler::new();
        
        let wallet = Wallet::new(
            "GeoID7xK9m2AbCdEfGh123456".to_string(),
            "0".repeat(64),
        );
        
        // Store wallet
        handler.state_mut().store_wallet(wallet.clone());
        
        // Request wallet
        let response = handler.process_message(
            SyncMessage::RequestWallet { geo_id: wallet.gy_id.clone() }
        );
        
        assert!(response.is_some());
        if let SyncMessage::ResponseWallet { wallet: w } = response.unwrap() {
            assert_eq!(w.gy_id, wallet.gy_id);
        }
    }
}
