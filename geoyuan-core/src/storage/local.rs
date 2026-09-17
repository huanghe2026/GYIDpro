//! Local SQLite storage
//! 
//! Stores wallet data, GeoIDs, and GeoCoins locally

use super::{StoredGeoId, StoredCoin, StoredTransfer};
use crate::{Result, GeoYuanError};
use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::Mutex;

/// Local storage using SQLite
pub struct LocalStorage {
    conn: Mutex<Connection>,
}

impl LocalStorage {
    /// Open or create database at path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)
            .map_err(|e| GeoYuanError::Storage(e.to_string()))?;
        
        let storage = Self {
            conn: Mutex::new(conn),
        };
        
        storage.init_tables()?;
        Ok(storage)
    }
    
    /// Initialize database tables
    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute_batch(r#"
            -- GeoID records
            CREATE TABLE IF NOT EXISTS geo_ids (
                id TEXT PRIMARY KEY,
                hash TEXT NOT NULL,
                photo_hash TEXT NOT NULL,
                latitude REAL NOT NULL,
                longitude REAL NOT NULL,
                geohash TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            
            -- GeoCoin records
            CREATE TABLE IF NOT EXISTS coins (
                id TEXT PRIMARY KEY,
                owner TEXT NOT NULL,
                latitude REAL NOT NULL,
                longitude REAL NOT NULL,
                geohash TEXT NOT NULL,
                photo_hash TEXT NOT NULL,
                minted_at TEXT NOT NULL,
                status TEXT NOT NULL,
                chain_tx TEXT
            );
            
            -- Transfer records
            CREATE TABLE IF NOT EXISTS transfers (
                tx_id TEXT PRIMARY KEY,
                coin_id TEXT NOT NULL,
                from_geo_id TEXT NOT NULL,
                to_geo_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                signature TEXT NOT NULL,
                status TEXT NOT NULL
            );
            
            -- Wallet data
            CREATE TABLE IF NOT EXISTS wallets (
                geo_id TEXT PRIMARY KEY,
                public_key TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            
            -- Indexes
            CREATE INDEX IF NOT EXISTS idx_coins_owner ON coins(owner);
            CREATE INDEX IF NOT EXISTS idx_coins_geohash ON coins(geohash);
            CREATE INDEX IF NOT EXISTS idx_transfers_from ON transfers(from_geo_id);
            CREATE INDEX IF NOT EXISTS idx_transfers_to ON transfers(to_geo_id);
        "#).map_err(|e| GeoYuanError::Storage(e.to_string()))?;
        
        Ok(())
    }
    
    // ==================== GeoID Operations ====================
    
    /// Save GeoID
    pub fn save_geo_id(&self, geo_id: &StoredGeoId) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute(
            r#"INSERT OR REPLACE INTO geo_ids 
               (id, hash, photo_hash, latitude, longitude, geohash, created_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            params![
                geo_id.id,
                geo_id.hash,
                geo_id.photo_hash,
                geo_id.latitude,
                geo_id.longitude,
                geo_id.geohash,
                geo_id.created_at.to_rfc3339(),
            ],
        ).map_err(|e| GeoYuanError::Storage(e.to_string()))?;
        
        Ok(())
    }
    
    /// Get GeoID by ID
    pub fn get_geo_id(&self, id: &str) -> Result<Option<StoredGeoId>> {
        let conn = self.conn.lock().unwrap();
        
        let mut stmt = conn.prepare(
            "SELECT id, hash, photo_hash, latitude, longitude, geohash, created_at FROM geo_ids WHERE id = ?1"
        ).map_err(|e| GeoYuanError::Storage(e.to_string()))?;
        
        let result = stmt.query_row(params![id], |row| {
            let created_at_str: String = row.get(6)?;
            Ok(StoredGeoId {
                id: row.get(0)?,
                hash: row.get(1)?,
                photo_hash: row.get(2)?,
                latitude: row.get(3)?,
                longitude: row.get(4)?,
                geohash: row.get(5)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
            })
        });
        
        match result {
            Ok(geo_id) => Ok(Some(geo_id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(GeoYuanError::Storage(e.to_string())),
        }
    }
    
    /// Get all GeoIDs
    pub fn get_all_geo_ids(&self) -> Result<Vec<StoredGeoId>> {
        let conn = self.conn.lock().unwrap();
        
        let mut stmt = conn.prepare(
            "SELECT id, hash, photo_hash, latitude, longitude, geohash, created_at FROM geo_ids ORDER BY created_at DESC"
        ).map_err(|e| GeoYuanError::Storage(e.to_string()))?;
        
        let rows = stmt.query_map([], |row| {
            let created_at_str: String = row.get(6)?;
            Ok(StoredGeoId {
                id: row.get(0)?,
                hash: row.get(1)?,
                photo_hash: row.get(2)?,
                latitude: row.get(3)?,
                longitude: row.get(4)?,
                geohash: row.get(5)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
            })
        }).map_err(|e| GeoYuanError::Storage(e.to_string()))?;
        
        let mut geo_ids = Vec::new();
        for row in rows {
            geo_ids.push(row.map_err(|e| GeoYuanError::Storage(e.to_string()))?);
        }
        
        Ok(geo_ids)
    }
    
    // ==================== Coin Operations ====================
    
    /// Save coin
    pub fn save_coin(&self, coin: &StoredCoin) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute(
            r#"INSERT OR REPLACE INTO coins 
               (id, owner, latitude, longitude, geohash, photo_hash, minted_at, status, chain_tx)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            params![
                coin.id,
                coin.owner,
                coin.latitude,
                coin.longitude,
                coin.geohash,
                coin.photo_hash,
                coin.minted_at.to_rfc3339(),
                coin.status,
                coin.chain_tx,
            ],
        ).map_err(|e| GeoYuanError::Storage(e.to_string()))?;
        
        Ok(())
    }
    
    /// Get coins by owner
    pub fn get_coins_by_owner(&self, owner: &str) -> Result<Vec<StoredCoin>> {
        let conn = self.conn.lock().unwrap();
        
        let mut stmt = conn.prepare(
            r#"SELECT id, owner, latitude, longitude, geohash, photo_hash, minted_at, status, chain_tx
               FROM coins WHERE owner = ?1 ORDER BY minted_at DESC"#
        ).map_err(|e| GeoYuanError::Storage(e.to_string()))?;
        
        let rows = stmt.query_map(params![owner], Self::map_coin_row)
            .map_err(|e| GeoYuanError::Storage(e.to_string()))?;
        
        let mut coins = Vec::new();
        for row in rows {
            coins.push(row.map_err(|e| GeoYuanError::Storage(e.to_string()))?);
        }
        
        Ok(coins)
    }
    
    /// Get coin by ID
    pub fn get_coin(&self, id: &str) -> Result<Option<StoredCoin>> {
        let conn = self.conn.lock().unwrap();
        
        let mut stmt = conn.prepare(
            r#"SELECT id, owner, latitude, longitude, geohash, photo_hash, minted_at, status, chain_tx
               FROM coins WHERE id = ?1"#
        ).map_err(|e| GeoYuanError::Storage(e.to_string()))?;
        
        let result = stmt.query_row(params![id], Self::map_coin_row);
        
        match result {
            Ok(coin) => Ok(Some(coin)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(GeoYuanError::Storage(e.to_string())),
        }
    }
    
    fn map_coin_row(row: &rusqlite::Row) -> rusqlite::Result<StoredCoin> {
        let minted_at_str: String = row.get(6)?;
        Ok(StoredCoin {
            id: row.get(0)?,
            owner: row.get(1)?,
            latitude: row.get(2)?,
            longitude: row.get(3)?,
            geohash: row.get(4)?,
            photo_hash: row.get(5)?,
            minted_at: chrono::DateTime::parse_from_rfc3339(&minted_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            status: row.get(7)?,
            chain_tx: row.get(8)?,
        })
    }
    
    // ==================== Transfer Operations ====================
    
    /// Save transfer
    pub fn save_transfer(&self, transfer: &StoredTransfer) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute(
            r#"INSERT OR REPLACE INTO transfers 
               (tx_id, coin_id, from_geo_id, to_geo_id, timestamp, signature, status)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            params![
                transfer.tx_id,
                transfer.coin_id,
                transfer.from,
                transfer.to,
                transfer.timestamp.to_rfc3339(),
                transfer.signature,
                transfer.status,
            ],
        ).map_err(|e| GeoYuanError::Storage(e.to_string()))?;
        
        Ok(())
    }
    
    /// Get transfers by coin
    pub fn get_transfers_by_coin(&self, coin_id: &str) -> Result<Vec<StoredTransfer>> {
        let conn = self.conn.lock().unwrap();
        
        let mut stmt = conn.prepare(
            r#"SELECT tx_id, coin_id, from_geo_id, to_geo_id, timestamp, signature, status
               FROM transfers WHERE coin_id = ?1 ORDER BY timestamp DESC"#
        ).map_err(|e| GeoYuanError::Storage(e.to_string()))?;
        
        let rows = stmt.query_map(params![coin_id], |row| {
            let timestamp_str: String = row.get(4)?;
            Ok(StoredTransfer {
                tx_id: row.get(0)?,
                coin_id: row.get(1)?,
                from: row.get(2)?,
                to: row.get(3)?,
                timestamp: chrono::DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                signature: row.get(5)?,
                status: row.get(6)?,
            })
        }).map_err(|e| GeoYuanError::Storage(e.to_string()))?;
        
        let mut transfers = Vec::new();
        for row in rows {
            transfers.push(row.map_err(|e| GeoYuanError::Storage(e.to_string()))?);
        }
        
        Ok(transfers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_storage() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        
        let storage = LocalStorage::open(&db_path).unwrap();
        
        // Save GeoID
        let geo_id = StoredGeoId {
            id: "GeoID7xK9m2AbCdEfGh123456".to_string(),
            hash: "0".repeat(64),
            photo_hash: "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9".to_string(),
            latitude: 39.9042,
            longitude: 116.4074,
            geohash: "wx4g0e6md5".to_string(),
            created_at: chrono::Utc::now(),
        };
        
        storage.save_geo_id(&geo_id).unwrap();
        
        // Retrieve GeoID
        let retrieved = storage.get_geo_id(&geo_id.id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, geo_id.id);
    }
}
