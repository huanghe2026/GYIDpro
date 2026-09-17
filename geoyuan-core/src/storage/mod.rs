//! Storage module
//! 
//! Local SQLite storage for wallet and identity data
//! redb (pure-Rust) storage for blockchain data

pub mod local;
pub mod redb_store;

pub use local::LocalStorage;
pub use redb_store::{
    RedbStorage, RedbWriteBatch, serialize, deserialize,
    CF_BLOCKS, CF_HEADERS, CF_HEIGHTS, CF_ACCOUNTS,
    CF_GYID_INDEX, CF_GYID_METADATA, CF_TRANSACTIONS,
    CF_TX_RECEIPTS, CF_PENDING_TXS, CF_SHARD_DATA,
    CF_SHARD_ASSIGNMENTS, CF_PEERS, CF_METADATA, CF_SYNC_STATE,
};

// Backward-compatibility aliases (in case other modules use "RocksStorage")
pub type RocksStorage = RedbStorage;
pub type RocksWriteBatch = RedbWriteBatch;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Stored GeoID record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredGeoId {
    pub id: String,
    pub hash: String,
    pub photo_hash: String,
    pub latitude: f64,
    pub longitude: f64,
    pub geohash: String,
    pub created_at: DateTime<Utc>,
}

/// Stored GeoCoin record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredCoin {
    pub id: String,
    pub owner: String,
    pub latitude: f64,
    pub longitude: f64,
    pub geohash: String,
    pub photo_hash: String,
    pub minted_at: DateTime<Utc>,
    pub status: String,
    pub chain_tx: Option<String>,
}

/// Stored transfer record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTransfer {
    pub tx_id: String,
    pub coin_id: String,
    pub from: String,
    pub to: String,
    pub timestamp: DateTime<Utc>,
    pub signature: String,
    pub status: String,
}
