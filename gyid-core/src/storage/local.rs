//! 本地存储模块

use crate::{GyIdError, Result, identity::GyId, device::DeviceLink};
use crate::storage::GyIdAnchor;
use rusqlite::{Connection, params};
use std::path::PathBuf;

/// 本地存储
pub struct LocalStorage {
    conn: Connection,
}

impl LocalStorage {
    /// 创建新的本地存储
    pub fn new(path: Option<PathBuf>) -> Result<Self> {
        let db_path = path.unwrap_or_else(|| {
            let mut path = dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."));
            path.push("gyid");
            path.push("gyid.db");
            path
        });
        
        // 确保目录存在
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| GyIdError::StorageError(format!("创建目录失败: {}", e)))?;
        }
        
        let conn = Connection::open(&db_path)
            .map_err(|e| GyIdError::StorageError(format!("数据库打开失败: {}", e)))?;
        
        let storage = Self { conn };
        storage.init_tables()?;
        
        Ok(storage)
    }
    
    /// 初始化数据库表
    fn init_tables(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS gyid (
                id TEXT PRIMARY KEY,
                hash TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                linked_devices INTEGER DEFAULT 0
            )",
            [],
        ).map_err(|e| GyIdError::StorageError(format!("表创建失败: {}", e)))?;
        
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS device_links (
                link_id TEXT PRIMARY KEY,
                master_id TEXT NOT NULL,
                linked_id TEXT NOT NULL,
                auth_code TEXT NOT NULL,
                linked_at INTEGER NOT NULL,
                active INTEGER DEFAULT 1,
                FOREIGN KEY (master_id) REFERENCES gyid(id)
            )",
            [],
        ).map_err(|e| GyIdError::StorageError(format!("表创建失败: {}", e)))?;
        
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        ).map_err(|e| GyIdError::StorageError(format!("表创建失败: {}", e)))?;
        
        Ok(())
    }
    
    /// 保存 GyID
    pub fn save_gyid(&self, gyid: &GyId) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO gyid (id, hash, created_at, linked_devices) VALUES (?1, ?2, ?3, ?4)",
            params![gyid.id, gyid.hash, gyid.created_at as i64, gyid.linked_devices as i32],
        ).map_err(|e| GyIdError::StorageError(format!("保存 GyID 失败: {}", e)))?;
        
        Ok(())
    }
    
    /// 获取 GyID
    pub fn get_gyid(&self, id: &str) -> Result<Option<GyId>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, hash, created_at, linked_devices FROM gyid WHERE id = ?1"
        ).map_err(|e| GyIdError::StorageError(format!("查询失败: {}", e)))?;

        let result = stmt.query_row(params![id], |row| {
            Ok(GyId {
                id: row.get(0)?,
                hash: row.get(1)?,
                created_at: row.get::<_, i64>(2)? as u64,
                linked_devices: row.get::<_, i32>(3)? as u32,
            })
        });

        match result {
            Ok(gyid) => Ok(Some(gyid)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(GyIdError::StorageError(format!("查询失败: {}", e))),
        }
    }

    /// 获取所有 GyID
    pub fn list_all_gyids(&self) -> Result<Vec<GyId>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, hash, created_at, linked_devices FROM gyid ORDER BY created_at DESC"
        ).map_err(|e| GyIdError::StorageError(format!("查询失败: {}", e)))?;

        let gyids: Vec<GyId> = stmt.query_map([], |row| {
            Ok(GyId {
                id: row.get(0)?,
                hash: row.get(1)?,
                created_at: row.get::<_, i64>(2)? as u64,
                linked_devices: row.get::<_, i32>(3)? as u32,
            })
        }).map_err(|e| GyIdError::StorageError(format!("查询失败: {}", e)))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| GyIdError::StorageError(format!("数据映射失败: {}", e)))?;

        Ok(gyids)
    }

    /// 删除 GyID
    pub fn delete_gyid(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM gyid WHERE id = ?1",
            params![id],
        ).map_err(|e| GyIdError::StorageError(format!("删除 GyID 失败: {}", e)))?;
        Ok(())
    }

    /// 删除设备关联
    pub fn delete_link(&self, link_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM device_links WHERE link_id = ?1",
            params![link_id],
        ).map_err(|e| GyIdError::StorageError(format!("删除关联失败: {}", e)))?;
        Ok(())
    }
    
    /// 保存设备关联
    pub fn save_link(&self, link: &DeviceLink) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO device_links (link_id, master_id, linked_id, auth_code, linked_at, active) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![link.link_id, link.master_id, link.linked_id, link.auth_code, link.linked_at as i64, link.active as i32],
        ).map_err(|e| GyIdError::StorageError(format!("保存关联失败: {}", e)))?;
        
        Ok(())
    }
    
    /// 获取主设备的所有关联
    pub fn get_links(&self, master_id: &str) -> Result<Vec<DeviceLink>> {
        let mut stmt = self.conn.prepare(
            "SELECT link_id, master_id, linked_id, auth_code, linked_at, active FROM device_links WHERE master_id = ?1"
        ).map_err(|e| GyIdError::StorageError(format!("查询失败: {}", e)))?;
        
        let links: Vec<DeviceLink> = stmt.query_map(params![master_id], |row| {
            Ok(DeviceLink {
                link_id: row.get(0)?,
                master_id: row.get(1)?,
                linked_id: row.get(2)?,
                auth_code: row.get(3)?,
                linked_at: row.get::<_, i64>(4)? as u64,
                active: row.get::<_, i32>(5)? != 0,
            })
        }).map_err(|e| GyIdError::StorageError(format!("查询失败: {}", e)))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| GyIdError::StorageError(format!("数据映射失败: {}", e)))?;
        
        Ok(links)
    }
    
    /// 保存配置
    pub fn save_config(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
            params![key, value],
        ).map_err(|e| GyIdError::StorageError(format!("保存配置失败: {}", e)))?;
        Ok(())
    }
    
    /// 获取配置
    pub fn get_config(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT value FROM config WHERE key = ?1"
        ).map_err(|e| GyIdError::StorageError(format!("查询失败: {}", e)))?;
        
        let result = stmt.query_row(params![key], |row| row.get(0));
        
        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(GyIdError::StorageError(format!("查询失败: {}", e))),
        }
    }

    /// 获取所有设备关联
    pub fn get_all_links(&self) -> Result<Vec<DeviceLink>> {
        let mut stmt = self.conn.prepare(
            "SELECT link_id, master_id, linked_id, auth_code, linked_at, active FROM device_links ORDER BY linked_at DESC"
        ).map_err(|e| GyIdError::StorageError(format!("查询失败: {}", e)))?;

        let links: Vec<DeviceLink> = stmt.query_map([], |row| {
            Ok(DeviceLink {
                link_id: row.get(0)?,
                master_id: row.get(1)?,
                linked_id: row.get(2)?,
                auth_code: row.get(3)?,
                linked_at: row.get::<_, i64>(4)? as u64,
                active: row.get::<_, i32>(5)? != 0,
            })
        }).map_err(|e| GyIdError::StorageError(format!("查询失败: {}", e)))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| GyIdError::StorageError(format!("数据映射失败: {}", e)))?;

        Ok(links)
    }

    /// 更新设备关联状态
    pub fn update_link_status(&self, link_id: &str, active: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE device_links SET active = ?1 WHERE link_id = ?2",
            params![active as i32, link_id],
        ).map_err(|e| GyIdError::StorageError(format!("更新关联状态失败: {}", e)))?;
        Ok(())
    }

    /// 获取 GyID 总数
    pub fn count_gyids(&self) -> Result<u32> {
        let count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM gyid",
            [],
            |row| row.get(0),
        ).map_err(|e| GyIdError::StorageError(format!("统计失败: {}", e)))?;
        Ok(count as u32)
    }

    /// 获取设备关联总数
    pub fn count_links(&self) -> Result<u32> {
        let count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM device_links WHERE active = 1",
            [],
            |row| row.get(0),
        ).map_err(|e| GyIdError::StorageError(format!("统计失败: {}", e)))?;
        Ok(count as u32)
    }

    /// 获取数据库文件路径
    pub fn get_db_path(&self) -> std::path::PathBuf {
        self.conn.path()
            .map(std::path::PathBuf::from)
            .unwrap_or_default()
    }

    /// 执行带事务的操作
    pub fn with_transaction<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(&Connection) -> Result<()>,
    {
        let tx = self.conn.unchecked_transaction()
            .map_err(|e| GyIdError::StorageError(format!("开启事务失败: {}", e)))?;
        
        let result = f(&tx);
        
        match result {
            Ok(_) => {
                tx.commit()
                    .map_err(|e| GyIdError::StorageError(format!("提交事务失败: {}", e)))?;
                Ok(())
            }
            Err(e) => {
                let _ = tx.rollback();
                Err(e)
            }
        }
    }

    /// 备份数据库到指定路径
    pub fn backup(&self, dest_path: &std::path::Path) -> Result<()> {
        let db_path = self.get_db_path();
        if db_path.as_os_str().is_empty() {
            return Err(GyIdError::StorageError("无法获取数据库路径".to_string()));
        }

        // 确保目标目录存在
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| GyIdError::StorageError(format!("创建备份目录失败: {}", e)))?;
        }

        // 复制数据库文件
        std::fs::copy(&db_path, dest_path)
            .map_err(|e| GyIdError::StorageError(format!("备份失败: {}", e)))?;

        Ok(())
    }

    /// 从备份恢复数据库
    pub fn restore(&self, backup_path: &std::path::Path) -> Result<()> {
        let db_path = self.get_db_path();
        if db_path.as_os_str().is_empty() {
            return Err(GyIdError::StorageError("无法获取数据库路径".to_string()));
        }

        if !backup_path.exists() {
            return Err(GyIdError::StorageError("备份文件不存在".to_string()));
        }

        // 关闭当前连接后复制备份文件
        let _ = self;

        std::fs::copy(backup_path, &db_path)
            .map_err(|e| GyIdError::StorageError(format!("恢复失败: {}", e)))?;

        Ok(())
    }
}

/// 链上锚定存储扩展
impl LocalStorage {
    /// 保存链上锚定记录
    pub fn save_anchor(&self, anchor: &GyIdAnchor) -> Result<()> {
        // 创建表（如果不存在）
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS chain_anchors (
                gyid_hash TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL,
                metadata_hash TEXT NOT NULL,
                linked_count INTEGER DEFAULT 0,
                tx_hash TEXT,
                block_height INTEGER,
                chain TEXT
            )",
            [],
        ).map_err(|e| GyIdError::StorageError(format!("创建表失败: {}", e)))?;

        // 保存锚定记录
        self.conn.execute(
            "INSERT OR REPLACE INTO chain_anchors 
             (gyid_hash, created_at, metadata_hash, linked_count, tx_hash, block_height, chain) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                anchor.gyid_hash,
                anchor.created_at as i64,
                anchor.metadata_hash,
                anchor.linked_count as i32,
                anchor.tx_hash,
                anchor.block_height.map(|h| h as i64),
                anchor.chain,
            ],
        ).map_err(|e| GyIdError::StorageError(format!("保存锚定失败: {}", e)))?;

        Ok(())
    }

    /// 获取链上锚定记录
    pub fn get_anchor(&self, gyid_hash: &str) -> Result<Option<GyIdAnchor>> {
        let result = self.conn.query_row(
            "SELECT gyid_hash, created_at, metadata_hash, linked_count, tx_hash, block_height, chain 
             FROM chain_anchors WHERE gyid_hash = ?1",
            params![gyid_hash],
            |row| {
                Ok(GyIdAnchor {
                    gyid_hash: row.get(0)?,
                    created_at: row.get::<_, i64>(1)? as u64,
                    metadata_hash: row.get(2)?,
                    linked_count: row.get::<_, i32>(3)? as u32,
                    tx_hash: row.get(4)?,
                    block_height: row.get::<_, Option<i64>>(5)?.map(|h| h as u64),
                    chain: row.get(6)?,
                })
            },
        );

        match result {
            Ok(anchor) => Ok(Some(anchor)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(GyIdError::StorageError(format!("查询失败: {}", e))),
        }
    }

    /// 获取所有链上锚定记录
    pub fn list_anchors(&self) -> Result<Vec<GyIdAnchor>> {
        let mut stmt = self.conn.prepare(
            "SELECT gyid_hash, created_at, metadata_hash, linked_count, tx_hash, block_height, chain 
             FROM chain_anchors ORDER BY created_at DESC"
        ).map_err(|e| GyIdError::StorageError(format!("查询失败: {}", e)))?;

        let anchors: Vec<GyIdAnchor> = stmt.query_map([], |row| {
            Ok(GyIdAnchor {
                gyid_hash: row.get(0)?,
                created_at: row.get::<_, i64>(1)? as u64,
                metadata_hash: row.get(2)?,
                linked_count: row.get::<_, i32>(3)? as u32,
                tx_hash: row.get(4)?,
                block_height: row.get::<_, Option<i64>>(5)?.map(|h| h as u64),
                chain: row.get(6)?,
            })
        }).map_err(|e| GyIdError::StorageError(format!("查询失败: {}", e)))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| GyIdError::StorageError(format!("数据映射失败: {}", e)))?;

        Ok(anchors)
    }

    /// 获取链上锚定总数
    pub fn count_anchors(&self) -> Result<u32> {
        // 先检查表是否存在
        let table_exists: std::result::Result<i32, _> = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='chain_anchors'",
            [],
            |row| row.get(0),
        );

        if table_exists.unwrap_or(0) == 0 {
            return Ok(0);
        }

        let count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM chain_anchors",
            [],
            |row| row.get(0),
        ).map_err(|e| GyIdError::StorageError(format!("统计失败: {}", e)))?;
        Ok(count as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage() {
        let storage = LocalStorage::new(Some(PathBuf::from(":memory:"))).unwrap();
        
        let gyid = GyId::new(
            "GyID_test1234567890abcdefghijklmnop".to_string(),
            "abc123".to_string(),
            1711814400000,
        );
        
        storage.save_gyid(&gyid).unwrap();
        
        let loaded = storage.get_gyid(&gyid.id).unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().id, gyid.id);
    }
}
