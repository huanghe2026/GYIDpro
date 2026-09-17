//! Account state and state root management

use serde::{Serialize, Deserialize};
use crate::chain::{StateHash, Address};
use crate::identity::GyId;
use std::collections::HashMap;

/// Account state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccountState {
    /// Account address
    pub address: Address,
    /// GY balance (nanoGY)
    pub balance: u64,
    /// Associated GyID list
    pub gy_ids: Vec<GyId>,
    /// Transaction nonce
    pub nonce: u64,
    /// Storage root (for contracts)
    pub storage_root: Option<StateHash>,
    /// Code hash (for contracts)
    pub code_hash: Option<StateHash>,
}

impl AccountState {
    /// Create new account
    pub fn new(address: Address) -> Self {
        Self {
            address,
            balance: 0,
            gy_ids: Vec::new(),
            nonce: 0,
            storage_root: None,
            code_hash: None,
        }
    }
    
    /// Create account with initial balance
    pub fn with_balance(address: Address, balance: u64) -> Self {
        Self {
            address,
            balance,
            gy_ids: Vec::new(),
            nonce: 0,
            storage_root: None,
            code_hash: None,
        }
    }
    
    /// Add GyID to account
    pub fn add_gyid(&mut self, gyid: GyId) {
        if !self.gy_ids.contains(&gyid) {
            self.gy_ids.push(gyid);
        }
    }
    
    /// Increment nonce
    pub fn increment_nonce(&mut self) {
        self.nonce += 1;
    }
    
    /// Add balance
    pub fn add_balance(&mut self, amount: u64) {
        self.balance += amount;
    }
    
    /// Subtract balance
    pub fn sub_balance(&mut self, amount: u64) -> bool {
        if self.balance >= amount {
            self.balance -= amount;
            true
        } else {
            false
        }
    }
    
    /// Check if account is contract
    pub fn is_contract(&self) -> bool {
        self.code_hash.is_some()
    }
}

/// State root (Merkle Patricia Trie wrapper)
pub struct StateRoot {
    /// Root hash
    pub root: StateHash,
    /// State cache
    cache: HashMap<Address, AccountState>,
}

impl StateRoot {
    /// Create empty state root
    pub fn empty() -> Self {
        Self {
            root: [0u8; 32],
            cache: HashMap::new(),
        }
    }
    
    /// Get account state
    pub fn get_account(&self, address: &Address) -> Option<&AccountState> {
        self.cache.get(address)
    }
    
    /// Insert or update account
    pub fn insert_account(&mut self, state: AccountState) {
        self.cache.insert(state.address, state);
        self.recompute_root();
    }
    
    /// Remove account
    pub fn remove_account(&mut self, address: &Address) {
        self.cache.remove(address);
        self.recompute_root();
    }
    
    /// Recompute state root
    fn recompute_root(&mut self) {
        // Sort addresses for deterministic hashing
        let mut addresses: Vec<_> = self.cache.keys().collect();
        addresses.sort();
        
        // Compute simple Merkle tree root
        let leaves: Vec<_> = addresses
            .iter()
            .map(|addr| {
                let state = self.cache.get(*addr).unwrap();
                let bytes = bincode::serialize(state).unwrap_or_default();
                
                use blake3::Hasher;
                let mut hasher = Hasher::new();
                hasher.update(&bytes);
                let result = hasher.finalize();
                
                let mut hash = [0u8; 32];
                hash.copy_from_slice(result.as_bytes());
                hash
            })
            .collect();
        
        self.root = compute_merkle_root(&leaves);
    }
    
    /// Get root hash
    pub fn root(&self) -> StateHash {
        self.root
    }
    
    /// Apply state changes
    pub fn apply_changes(&mut self, changes: StateChanges) {
        for (address, change) in changes.changes {
            match change {
                StateChange::Update(state) => {
                    self.cache.insert(address, state);
                }
                StateChange::Delete => {
                    self.cache.remove(&address);
                }
            }
        }
        self.recompute_root();
    }
}

/// State change type
pub enum StateChange {
    /// Update account state
    Update(AccountState),
    /// Delete account
    Delete,
}

/// State changes batch
pub struct StateChanges {
    changes: HashMap<Address, StateChange>,
}

impl StateChanges {
    /// Create new state changes
    pub fn new() -> Self {
        Self {
            changes: HashMap::new(),
        }
    }
    
    /// Add update
    pub fn update(&mut self, state: AccountState) {
        self.changes.insert(state.address, StateChange::Update(state));
    }
    
    /// Add delete
    pub fn delete(&mut self, address: Address) {
        self.changes.insert(address, StateChange::Delete);
    }
}

impl Default for StateChanges {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute Merkle tree root
fn compute_merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }
    
    if leaves.len() == 1 {
        return leaves[0];
    }
    
    let mut current_level: Vec<[u8; 32]> = leaves.to_vec();
    
    while current_level.len() > 1 {
        let mut next_level = Vec::new();
        
        for chunk in current_level.chunks(2) {
            let left = chunk[0];
            let right = if chunk.len() > 1 { chunk[1] } else { chunk[0] };
            
            use blake3::Hasher;
            let mut hasher = Hasher::new();
            hasher.update(&left);
            hasher.update(&right);
            
            let result = hasher.finalize();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(result.as_bytes());
            next_level.push(hash);
        }
        
        current_level = next_level;
    }
    
    current_level[0]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_account_state() {
        let addr = [1u8; 20];
        let mut state = AccountState::with_balance(addr, 1000);
        
        assert_eq!(state.balance, 1000);
        
        state.add_balance(500);
        assert_eq!(state.balance, 1500);
        
        assert!(state.sub_balance(1000));
        assert_eq!(state.balance, 500);
        
        assert!(!state.sub_balance(1000)); // Insufficient balance
    }
    
    #[test]
    fn test_state_root() {
        let mut root = StateRoot::empty();
        
        let addr1 = [1u8; 20];
        let state1 = AccountState::with_balance(addr1, 1000);
        root.insert_account(state1);
        
        let addr2 = [2u8; 20];
        let state2 = AccountState::with_balance(addr2, 2000);
        root.insert_account(state2);
        
        assert_ne!(root.root(), [0u8; 32]);
        
        let retrieved = root.get_account(&addr1);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().balance, 1000);
    }
}