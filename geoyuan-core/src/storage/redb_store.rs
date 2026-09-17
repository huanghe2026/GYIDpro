//! redb Storage Layer
//!
//! Pure-Rust embedded key-value storage for blockchain data.
//! Replaces RocksDB to avoid LLVM/bindgen build dependencies.
//!
//! Column families are emulated via separate redb tables.

use redb::{Database, TableDefinition};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use crate::{Result, GeoYuanError};

// ─────────────────────────────────────────────────────────────────────────────
// Table definitions (equivalent to RocksDB column families)
// ─────────────────────────────────────────────────────────────────────────────

/// A generic table that maps bytes → bytes.
/// We use a single table `CF_NAME -> key -> value` pattern by encoding
/// the table name into the key prefix: `"<cf>/<key>"`.
/// For simplicity we keep one flat table and use prefixed keys.
const MAIN_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("main");

/// Column family constants (used as key prefixes)
pub const CF_BLOCKS: &str = "blocks";
pub const CF_HEADERS: &str = "headers";
pub const CF_HEIGHTS: &str = "heights";
pub const CF_ACCOUNTS: &str = "accounts";
pub const CF_GYID_INDEX: &str = "gyid_index";
pub const CF_GYID_METADATA: &str = "gyid_metadata";
pub const CF_TRANSACTIONS: &str = "transactions";
pub const CF_TX_RECEIPTS: &str = "tx_receipts";
pub const CF_PENDING_TXS: &str = "pending_txs";
pub const CF_SHARD_DATA: &str = "shard_data";
pub const CF_SHARD_ASSIGNMENTS: &str = "shard_assignments";
pub const CF_PEERS: &str = "peers";
pub const CF_METADATA: &str = "metadata";
pub const CF_SYNC_STATE: &str = "sync_state";

// ─────────────────────────────────────────────────────────────────────────────
// Storage engine
// ─────────────────────────────────────────────────────────────────────────────

/// redb-based storage engine (drop-in replacement for RocksStorage)
pub struct RedbStorage {
    db: Arc<Database>,
}

impl RedbStorage {
    /// Open or create database at path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = Database::create(path)
            .map_err(|e| GeoYuanError::Storage(format!("Failed to open redb: {}", e)))?;

        // Ensure the main table exists
        {
            let write_txn = db.begin_write()
                .map_err(|e| GeoYuanError::Storage(format!("Begin write failed: {}", e)))?;
            {
                write_txn.open_table(MAIN_TABLE)
                    .map_err(|e| GeoYuanError::Storage(format!("Open table failed: {}", e)))?;
            }
            write_txn.commit()
                .map_err(|e| GeoYuanError::Storage(format!("Commit failed: {}", e)))?;
        }

        Ok(Self {
            db: Arc::new(db),
        })
    }

    /// Build prefixed key: `<cf>/<raw_key>`
    fn prefixed(cf: &str, key: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(cf.len() + 1 + key.len());
        result.extend_from_slice(cf.as_bytes());
        result.push(b'/');
        result.extend_from_slice(key);
        result
    }

    /// Put value to column family
    pub fn put(&self, cf: &str, key: &[u8], value: &[u8]) -> Result<()> {
        let pk = Self::prefixed(cf, key);
        let write_txn = self.db.begin_write()
            .map_err(|e| GeoYuanError::Storage(format!("Begin write: {}", e)))?;
        {
            let mut table = write_txn.open_table(MAIN_TABLE)
                .map_err(|e| GeoYuanError::Storage(format!("Open table: {}", e)))?;
            table.insert(pk.as_slice(), value)
                .map_err(|e| GeoYuanError::Storage(format!("Insert: {}", e)))?;
        }
        write_txn.commit()
            .map_err(|e| GeoYuanError::Storage(format!("Commit: {}", e)))?;
        Ok(())
    }

    /// Get value from column family
    pub fn get(&self, cf: &str, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let pk = Self::prefixed(cf, key);
        let read_txn = self.db.begin_read()
            .map_err(|e| GeoYuanError::Storage(format!("Begin read: {}", e)))?;
        let table = read_txn.open_table(MAIN_TABLE)
            .map_err(|e| GeoYuanError::Storage(format!("Open table: {}", e)))?;
        let value = table.get(pk.as_slice())
            .map_err(|e| GeoYuanError::Storage(format!("Get: {}", e)))?
            .map(|v| v.value().to_vec());
        Ok(value)
    }

    /// Delete key from column family
    pub fn delete(&self, cf: &str, key: &[u8]) -> Result<()> {
        let pk = Self::prefixed(cf, key);
        let write_txn = self.db.begin_write()
            .map_err(|e| GeoYuanError::Storage(format!("Begin write: {}", e)))?;
        {
            let mut table = write_txn.open_table(MAIN_TABLE)
                .map_err(|e| GeoYuanError::Storage(format!("Open table: {}", e)))?;
            table.remove(pk.as_slice())
                .map_err(|e| GeoYuanError::Storage(format!("Remove: {}", e)))?;
        }
        write_txn.commit()
            .map_err(|e| GeoYuanError::Storage(format!("Commit: {}", e)))?;
        Ok(())
    }

    /// Batch write operations
    pub fn batch_write(&self, batch: RedbWriteBatch) -> Result<()> {
        let write_txn = self.db.begin_write()
            .map_err(|e| GeoYuanError::Storage(format!("Begin write: {}", e)))?;
        {
            let mut table = write_txn.open_table(MAIN_TABLE)
                .map_err(|e| GeoYuanError::Storage(format!("Open table: {}", e)))?;

            for (cf, key, value) in &batch.ops {
                let pk = Self::prefixed(cf, key);
                table.insert(pk.as_slice(), value.as_slice())
                    .map_err(|e| GeoYuanError::Storage(format!("Batch insert: {}", e)))?;
            }

            for (cf, key) in &batch.deletes {
                let pk = Self::prefixed(cf, key);
                table.remove(pk.as_slice())
                    .map_err(|e| GeoYuanError::Storage(format!("Batch remove: {}", e)))?;
            }
        }
        write_txn.commit()
            .map_err(|e| GeoYuanError::Storage(format!("Commit: {}", e)))?;
        Ok(())
    }

    /// Iterate over all keys in a column family
    pub fn iterate<F>(&self, cf: &str, mut f: F) -> Result<()>
    where
        F: FnMut(&[u8], &[u8]) -> bool,
    {
        let prefix = {
            let mut p = cf.as_bytes().to_vec();
            p.push(b'/');
            p
        };

        let read_txn = self.db.begin_read()
            .map_err(|e| GeoYuanError::Storage(format!("Begin read: {}", e)))?;
        let table = read_txn.open_table(MAIN_TABLE)
            .map_err(|e| GeoYuanError::Storage(format!("Open table: {}", e)))?;

        // Range: from `cf/` to `cf0` (next byte after '/')
        let mut end_prefix = cf.as_bytes().to_vec();
        end_prefix.push(b'/' + 1); // exclusive upper bound

        let range = table
            .range(prefix.as_slice()..end_prefix.as_slice())
            .map_err(|e| GeoYuanError::Storage(format!("Range: {}", e)))?;

        let prefix_len = cf.len() + 1; // "cf/"

        for item in range {
            let (k, v) = item
                .map_err(|e| GeoYuanError::Storage(format!("Iteration: {}", e)))?;
            let raw_key = &k.value()[prefix_len..];
            if !f(raw_key, v.value()) {
                break;
            }
        }

        Ok(())
    }

    /// Flush (no-op for redb — writes are durable after commit)
    pub fn flush(&self) -> Result<()> {
        Ok(())
    }

    /// Get database stats as a string
    pub fn property(&self, name: &str) -> Option<String> {
        match name {
            "stats" => Some(format!("redb storage ({})", name)),
            _ => None,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Write batch
// ─────────────────────────────────────────────────────────────────────────────

/// Write batch builder
pub struct RedbWriteBatch {
    ops: Vec<(String, Vec<u8>, Vec<u8>)>,
    deletes: Vec<(String, Vec<u8>)>,
}

impl RedbWriteBatch {
    /// Create new batch
    pub fn new() -> Self {
        Self {
            ops: Vec::new(),
            deletes: Vec::new(),
        }
    }

    /// Add put operation
    pub fn put(mut self, cf: &str, key: Vec<u8>, value: Vec<u8>) -> Self {
        self.ops.push((cf.to_string(), key, value));
        self
    }

    /// Add delete operation
    pub fn delete(mut self, cf: &str, key: Vec<u8>) -> Self {
        self.deletes.push((cf.to_string(), key));
        self
    }
}

impl Default for RedbWriteBatch {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Serialization helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Serialize value to bytes using bincode
pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    bincode::serialize(value)
        .map_err(|e| GeoYuanError::Serialization(e.to_string()))
}

/// Deserialize bytes to value using bincode
pub fn deserialize<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T> {
    bincode::deserialize(bytes)
        .map_err(|e| GeoYuanError::Serialization(e.to_string()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_basic_operations() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let storage = RedbStorage::open(&db_path).unwrap();

        // put and get
        storage.put(CF_METADATA, b"key1", b"value1").unwrap();
        let value = storage.get(CF_METADATA, b"key1").unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));

        // non-existent key
        let value = storage.get(CF_METADATA, b"nonexistent").unwrap();
        assert_eq!(value, None);

        // delete
        storage.delete(CF_METADATA, b"key1").unwrap();
        let value = storage.get(CF_METADATA, b"key1").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_batch_write() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let storage = RedbStorage::open(&db_path).unwrap();

        let batch = RedbWriteBatch::new()
            .put(CF_METADATA, b"key1".to_vec(), b"value1".to_vec())
            .put(CF_METADATA, b"key2".to_vec(), b"value2".to_vec());

        storage.batch_write(batch).unwrap();

        assert_eq!(storage.get(CF_METADATA, b"key1").unwrap(), Some(b"value1".to_vec()));
        assert_eq!(storage.get(CF_METADATA, b"key2").unwrap(), Some(b"value2".to_vec()));
    }

    #[test]
    fn test_iterate() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let storage = RedbStorage::open(&db_path).unwrap();

        // Write to different CFs
        storage.put(CF_ACCOUNTS, b"acc1", b"data1").unwrap();
        storage.put(CF_ACCOUNTS, b"acc2", b"data2").unwrap();
        storage.put(CF_METADATA, b"meta1", b"mdata1").unwrap();

        // Iterate only CF_ACCOUNTS
        let mut keys = Vec::new();
        storage.iterate(CF_ACCOUNTS, |k, _v| {
            keys.push(k.to_vec());
            true
        }).unwrap();

        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&b"acc1".to_vec()));
        assert!(keys.contains(&b"acc2".to_vec()));
    }

    #[test]
    fn test_cross_cf_isolation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let storage = RedbStorage::open(&db_path).unwrap();

        storage.put(CF_ACCOUNTS, b"same_key", b"accounts_value").unwrap();
        storage.put(CF_METADATA, b"same_key", b"metadata_value").unwrap();

        let v1 = storage.get(CF_ACCOUNTS, b"same_key").unwrap();
        let v2 = storage.get(CF_METADATA, b"same_key").unwrap();

        assert_eq!(v1, Some(b"accounts_value".to_vec()));
        assert_eq!(v2, Some(b"metadata_value".to_vec()));
    }
}
