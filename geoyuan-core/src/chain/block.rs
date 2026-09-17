//! Block structure and validation

use serde::{Serialize, Deserialize};
use crate::chain::{BlockHeight, BlockHash, StateHash, Transaction};
use crate::identity::GyId;

/// Block header
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlockHeader {
    /// Block height
    pub height: BlockHeight,
    /// Previous block hash
    pub prev_hash: BlockHash,
    /// State root (Merkle Patricia Trie root)
    pub state_root: StateHash,
    /// Transactions root (Merkle tree root)
    pub tx_root: StateHash,
    /// Timestamp (milliseconds since Unix epoch)
    pub timestamp: u64,
    /// Producer GyID
    pub producer: GyId,
    /// VRF proof for producer selection
    pub vrf_proof: VRFProof,
    /// Block signature
    pub signature: BlockSignature,
}

/// VRF (Verifiable Random Function) proof
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VRFProof {
    /// VRF output
    pub output: [u8; 32],
    /// VRF proof
    pub proof: Vec<u8>,
}

/// Block signature
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlockSignature {
    /// Signer GyID
    pub signer: GyId,
    /// Signature bytes
    pub signature: Vec<u8>,
}

/// Complete block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Block header
    pub header: BlockHeader,
    /// Transactions
    pub transactions: Vec<Transaction>,
    /// Shard proofs (for sharded storage)
    pub shard_proofs: Vec<ShardProof>,
}

/// Shard proof for sharded storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardProof {
    /// Shard ID
    pub shard_id: u16,
    /// Merkle root of shard data
    pub shard_root: StateHash,
}

impl Block {
    /// Create new block
    pub fn new(
        height: BlockHeight,
        prev_hash: BlockHash,
        producer: GyId,
        transactions: Vec<Transaction>,
    ) -> Self {
        Self {
            header: BlockHeader {
                height,
                prev_hash,
                state_root: [0u8; 32],
                tx_root: [0u8; 32],
                timestamp: 0,
                producer: producer.clone(),
                vrf_proof: VRFProof {
                    output: [0u8; 32],
                    proof: Vec::new(),
                },
                signature: BlockSignature {
                    signer: producer.clone(),
                    signature: Vec::new(),
                },
            },
            transactions,
            shard_proofs: Vec::new(),
        }
    }
    
    /// Compute block hash
    pub fn hash(&self) -> BlockHash {
        use blake3::Hasher;
        
        let mut hasher = Hasher::new();
        hasher.update(&self.header.height.to_le_bytes());
        hasher.update(&self.header.prev_hash);
        hasher.update(&self.header.state_root);
        hasher.update(&self.header.tx_root);
        hasher.update(&self.header.timestamp.to_le_bytes());
        hasher.update(self.header.producer.as_bytes());
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(result.as_bytes());
        hash
    }
    
    /// Compute transactions root (Merkle tree root)
    pub fn compute_tx_root(&self) -> StateHash {
        if self.transactions.is_empty() {
            return [0u8; 32];
        }
        
        // Simple Merkle tree computation
        let leaves: Vec<_> = self.transactions
            .iter()
            .map(|tx| tx.hash())
            .collect();
        
        compute_merkle_root(&leaves)
    }
    
    /// Set timestamp
    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.header.timestamp = timestamp;
    }
    
    /// Set state root
    pub fn set_state_root(&mut self, root: StateHash) {
        self.header.state_root = root;
    }
    
    /// Set transaction root
    pub fn set_tx_root(&mut self, root: StateHash) {
        self.header.tx_root = root;
    }
    
    /// Sign block
    pub fn sign(&mut self, signer: GyId, signature: Vec<u8>) {
        self.header.signature = BlockSignature {
            signer,
            signature,
        };
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
    fn test_block_hash() {
        let gyid = GyId::new("GyIDtest123456789").unwrap();
        let block = Block::new(1, [0u8; 32], gyid, vec![]);
        let hash = block.hash();
        
        // Hash should be non-zero
        assert_ne!(hash, [0u8; 32]);
    }
    
    #[test]
    fn test_merkle_root() {
        let leaves = vec![
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            [4u8; 32],
        ];
        
        let root = compute_merkle_root(&leaves);
        assert_ne!(root, [0u8; 32]);
    }
}