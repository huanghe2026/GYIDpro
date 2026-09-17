//! Transaction types and validation

use serde::{Serialize, Deserialize};
use crate::chain::{TxHash, Address};
use crate::crypto::KeyPair;
use crate::identity::GyId;
use crate::geo::GeoLocation;

/// Transaction types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionType {
    /// GyID minting
    MintGyID,
    /// GY transfer
    Transfer,
    /// Device linking
    LinkDevice,
    /// Smart contract call
    ContractCall,
    /// NFT minting
    MintNFT,
}

impl std::fmt::Display for TransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionType::MintGyID => write!(f, "MintGyID"),
            TransactionType::Transfer => write!(f, "Transfer"),
            TransactionType::LinkDevice => write!(f, "LinkDevice"),
            TransactionType::ContractCall => write!(f, "ContractCall"),
            TransactionType::MintNFT => write!(f, "MintNFT"),
        }
    }
}

/// Transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction type
    pub tx_type: TransactionType,
    /// Nonce (transaction sequence number)
    pub nonce: u64,
    /// Sender address
    pub from: Address,
    /// Transaction payload
    pub payload: TxPayload,
    /// Gas price (always 0 in GeoYuan)
    pub gas_price: u64,
    /// Gas limit (always 0 in GeoYuan)
    pub gas_limit: u64,
    /// Transaction signature
    pub signature: TxSignature,
    /// Transaction hash (computed)
    #[serde(skip)]
    pub hash: Option<TxHash>,
}

/// Transaction payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TxPayload {
    /// GyID minting payload
    MintGyID {
        /// GyID to mint
        gy_id: GyId,
        /// Location
        location: GeoLocation,
        /// Timestamp
        timestamp: u64,
        /// Proof of location
        proof: GyIDProof,
    },
    /// Transfer payload
    Transfer {
        /// Recipient address
        to: Address,
        /// Amount (nanoGY)
        amount: u64,
        /// Memo
        memo: Option<String>,
    },
    /// Device linking payload
    LinkDevice {
        /// Master GyID
        master: GyId,
        /// Slave GyID
        slave: GyId,
        /// Link proof
        proof: LinkProof,
    },
    /// Contract call payload
    ContractCall {
        /// Contract address
        contract: Address,
        /// Method name
        method: String,
        /// Arguments
        args: Vec<u8>,
    },
    /// NFT minting payload
    MintNFT {
        /// NFT metadata
        metadata: NFTMetadata,
        /// Location
        location: GeoLocation,
    },
}

/// GyID proof for minting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GyIDProof {
    /// Photo hash
    pub photo_hash: [u8; 32],
    /// GPS coordinates
    pub gps: (f64, f64),
    /// Timestamp
    pub timestamp: u64,
    /// Hardware fingerprint
    pub hardware_fp: [u8; 32],
}

/// Device link proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkProof {
    /// Master signature
    pub master_sig: Vec<u8>,
    /// Slave signature
    pub slave_sig: Vec<u8>,
    /// Timestamp
    pub timestamp: u64,
}

/// NFT metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NFTMetadata {
    /// NFT ID
    pub id: String,
    /// Name
    pub name: String,
    /// Description
    pub description: String,
    /// Image URL
    pub image_url: String,
    /// Attributes
    pub attributes: Vec<NFTAttribute>,
}

/// NFT attribute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NFTAttribute {
    /// Trait type
    pub trait_type: String,
    /// Value
    pub value: String,
}

/// Transaction signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxSignature {
    /// Signer GyID
    pub signer: GyId,
    /// Signer public key (Ed25519)
    pub public_key: [u8; 32],
    /// Signature bytes (Ed25519, 64 bytes)
    pub signature: Vec<u8>,
}

/// Transaction receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionReceipt {
    /// Transaction hash
    pub tx_hash: TxHash,
    /// Block hash
    pub block_hash: BlockHash,
    /// Block height
    pub block_height: u64,
    /// Gas used (always 0)
    pub gas_used: u64,
    /// Status
    pub status: TxStatus,
    /// Logs
    pub logs: Vec<Log>,
}

/// Block hash type alias
pub type BlockHash = [u8; 32];

/// Transaction status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TxStatus {
    /// Success
    Success,
    /// Failed
    Failed,
    /// Reverted
    Reverted,
}

impl std::fmt::Display for TxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TxStatus::Success => write!(f, "Success"),
            TxStatus::Failed => write!(f, "Failed"),
            TxStatus::Reverted => write!(f, "Reverted"),
        }
    }
}

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    /// Address that generated the log
    pub address: Address,
    /// Topics
    pub topics: Vec<[u8; 32]>,
    /// Data
    pub data: Vec<u8>,
}

impl Transaction {
    /// Create new transaction
    pub fn new(tx_type: TransactionType, from: Address, payload: TxPayload, nonce: u64) -> Self {
        Self {
            tx_type,
            nonce,
            from,
            payload,
            gas_price: 0,
            gas_limit: 0,
            signature: TxSignature {
                signer: GyId::default(),
                public_key: [0u8; 32],
                signature: Vec::new(),
            },
            hash: None,
        }
    }
    
    /// Compute transaction hash
    pub fn hash(&self) -> TxHash {
        use blake3::Hasher;
        
        let mut hasher = Hasher::new();
        hasher.update(&[self.tx_type.clone() as u8]);
        hasher.update(&self.nonce.to_le_bytes());
        hasher.update(&self.from);
        
        // Hash payload
        let payload_bytes = bincode::serialize(&self.payload).unwrap_or_default();
        hasher.update(&payload_bytes);
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(result.as_bytes());
        hash
    }
    
    /// Get cached hash or compute
    pub fn get_hash(&mut self) -> TxHash {
        if let Some(hash) = self.hash {
            hash
        } else {
            let hash = self.hash();
            self.hash = Some(hash);
            hash
        }
    }
    
    /// Sign transaction with Ed25519
    pub fn sign(&mut self, signer: GyId, public_key: [u8; 32], signature: Vec<u8>) {
        self.signature = TxSignature {
            signer,
            public_key,
            signature,
        };
    }
    
    /// Verify transaction signature using Ed25519
    pub fn verify_signature(&self) -> bool {
        if self.signature.signature.len() != 64 {
            return false;
        }
        let sig: [u8; 64] = match self.signature.signature.as_slice().try_into() {
            Ok(s) => s,
            Err(_) => return false,
        };
        let msg = self.hash();
        KeyPair::verify_signature(&msg, &sig, &self.signature.public_key)
    }
    
    /// Get transfer amount (if transfer transaction)
    pub fn transfer_amount(&self) -> Option<u64> {
        match &self.payload {
            TxPayload::Transfer { amount, .. } => Some(*amount),
            _ => None,
        }
    }
    
    /// Get recipient (if transfer transaction)
    pub fn recipient(&self) -> Option<&Address> {
        match &self.payload {
            TxPayload::Transfer { to, .. } => Some(to),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;
    
    #[test]
    fn test_transaction_hash() {
        let from = [1u8; 20];
        let payload = TxPayload::Transfer {
            to: [2u8; 20],
            amount: 1000,
            memo: None,
        };
        
        let tx = Transaction::new(TransactionType::Transfer, from, payload, 0);
        let hash = tx.hash();
        
        assert_ne!(hash, [0u8; 32]);
    }
    
    #[test]
    fn test_transfer_amount() {
        let from = [1u8; 20];
        let payload = TxPayload::Transfer {
            to: [2u8; 20],
            amount: 5000,
            memo: Some("Test".to_string()),
        };
        
        let tx = Transaction::new(TransactionType::Transfer, from, payload, 0);
        
        assert_eq!(tx.transfer_amount(), Some(5000));
        assert_eq!(tx.recipient(), Some(&[2u8; 20]));
    }
    
    #[test]
    fn test_verify_signature_valid() {
        let kp = KeyPair::generate();
        let from = [1u8; 20];
        let to = [2u8; 20];
        let payload = TxPayload::Transfer {
            to,
            amount: 1000,
            memo: None,
        };
        
        let mut tx = Transaction::new(TransactionType::Transfer, from, payload, 0);
        let msg = tx.hash();
        let sig = kp.sign(&msg);
        tx.sign(
            GyId::default(),
            kp.public_bytes(),
            sig.to_bytes().to_vec(),
        );
        
        assert!(tx.verify_signature());
    }
    
    #[test]
    fn test_verify_signature_invalid() {
        let kp = KeyPair::generate();
        let wrong_kp = KeyPair::generate();
        let from = [1u8; 20];
        let to = [2u8; 20];
        let payload = TxPayload::Transfer {
            to,
            amount: 1000,
            memo: None,
        };
        
        let mut tx = Transaction::new(TransactionType::Transfer, from, payload, 0);
        let msg = tx.hash();
        let sig = kp.sign(&msg);
        // Use a different public key — verification should fail
        tx.sign(
            GyId::default(),
            wrong_kp.public_bytes(),
            sig.to_bytes().to_vec(),
        );
        
        assert!(!tx.verify_signature());
    }
    
    #[test]
    fn test_verify_signature_empty_fails() {
        let from = [1u8; 20];
        let payload = TxPayload::Transfer {
            to: [2u8; 20],
            amount: 1000,
            memo: None,
        };
        
        let tx = Transaction::new(TransactionType::Transfer, from, payload, 0);
        
        assert!(!tx.verify_signature());
    }
}