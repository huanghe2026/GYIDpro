//! GeoYuan Core Library
//! 
//! Photo-based GeoID generation and GeoCoin wallet system
//! Blockchain-based distributed identity network
//! 
//! # Core Modules
//! 
//! - [`photo`] - EXIF parsing and photo hashing
//! - [`geo`] - Geographic location (AMap SDK integration)
//! - [`crypto`] - Cryptographic operations (BLAKE3, SHA256, Ed25519)
//! - [`identity`] - GeoID generation and validation
//! - [`wallet`] - GeoCoin minting and transfer
//! - [`consensus`] - PoI (Proof of Identity) consensus
//! - [`p2p`] - Distributed P2P networking
//! - [`storage`] - Local SQLite and redb storage
//! - [`chain`] - Blockchain core (blocks, transactions, state)

pub mod photo;
pub mod geo;
pub mod crypto;
pub mod identity;
pub mod wallet;
pub mod consensus;
pub mod p2p;
pub mod storage;
pub mod chain;
pub mod device;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GeoYuanError {
    #[error("Photo error: {0}")]
    Photo(String),
    
    #[error("Geolocation error: {0}")]
    Geo(String),
    
    #[error("Crypto error: {0}")]
    Crypto(String),
    
    #[error("Identity error: {0}")]
    Identity(String),
    
    #[error("Wallet error: {0}")]
    Wallet(String),
    
    #[error("P2P error: {0}")]
    P2P(String),
    
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Chain error: {0}")]
    Chain(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, GeoYuanError>;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
