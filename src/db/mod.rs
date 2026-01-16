mod blocks;
mod schema;
mod validators;

pub use blocks::*;
pub use schema::init_schema;
pub use validators::*;

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;

/// Database wrapper for MVM
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create database at the specified path
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open database at {}", path.display()))?;

        // Initialize schema
        init_schema(&conn)?;

        // Enable WAL mode for better performance
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        Ok(Self { conn })
    }

    /// Open an in-memory database (for testing)
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        init_schema(&conn)?;
        Ok(Self { conn })
    }

    /// Get a reference to the underlying connection
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    // Block operations
    pub fn insert_block(&self, block: &BlockRecord) -> Result<()> {
        blocks::insert_block(&self.conn, block)
    }

    pub fn get_block(&self, block_number: u64) -> Result<Option<BlockRecord>> {
        blocks::get_block(&self.conn, block_number)
    }

    pub fn get_max_block_number(&self) -> Result<Option<u64>> {
        blocks::get_max_block_number(&self.conn)
    }

    pub fn mark_finalized(&self, up_to_block: u64) -> Result<usize> {
        blocks::mark_finalized(&self.conn, up_to_block)
    }

    pub fn count_blocks(&self) -> Result<u64> {
        blocks::count_blocks(&self.conn)
    }

    pub fn count_finalized_blocks(&self) -> Result<u64> {
        blocks::count_finalized_blocks(&self.conn)
    }

    pub fn get_blocks_in_range(
        &self,
        from: u64,
        to: u64,
        limit: Option<u32>,
    ) -> Result<Vec<BlockRecord>> {
        blocks::get_blocks_in_range(&self.conn, from, to, limit)
    }

    pub fn find_gaps(&self) -> Result<Vec<(u64, u64)>> {
        blocks::find_gaps(&self.conn)
    }

    // Sync status operations
    pub fn get_sync_status(&self) -> Result<SyncStatusRecord> {
        blocks::get_sync_status(&self.conn)
    }

    pub fn update_sync_status(
        &self,
        last_synced: u64,
        finalized: u64,
        tip: u64,
        epoch: u64,
        is_syncing: bool,
    ) -> Result<()> {
        blocks::update_sync_status(&self.conn, last_synced, finalized, tip, epoch, is_syncing)
    }

    // Validator operations
    pub fn upsert_validator(&self, validator: &ValidatorRecord) -> Result<()> {
        validators::upsert_validator(&self.conn, validator)
    }

    pub fn get_validator(&self, sidechain_key: &str) -> Result<Option<ValidatorRecord>> {
        validators::get_validator(&self.conn, sidechain_key)
    }

    pub fn get_all_validators(&self) -> Result<Vec<ValidatorRecord>> {
        validators::get_all_validators(&self.conn)
    }

    pub fn get_our_validators(&self) -> Result<Vec<ValidatorRecord>> {
        validators::get_our_validators(&self.conn)
    }

    pub fn increment_block_count(&self, sidechain_key: &str) -> Result<()> {
        validators::increment_block_count(&self.conn, sidechain_key)
    }

    pub fn count_validators(&self) -> Result<u64> {
        validators::count_validators(&self.conn)
    }

    pub fn count_our_validators(&self) -> Result<u64> {
        validators::count_our_validators(&self.conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_operations() {
        let db = Database::open_in_memory().unwrap();

        // Test block insert and retrieve
        let block = BlockRecord {
            block_number: 100,
            block_hash: "0xhash".to_string(),
            parent_hash: "0xparent".to_string(),
            state_root: "0xstate".to_string(),
            extrinsics_root: "0xext".to_string(),
            slot_number: 10000,
            epoch: 10,
            timestamp: 1234567890,
            is_finalized: false,
            author_key: None,
            extrinsics_count: 2,
        };

        db.insert_block(&block).unwrap();
        let retrieved = db.get_block(100).unwrap().unwrap();
        assert_eq!(retrieved.block_hash, "0xhash");

        // Test sync status
        db.update_sync_status(100, 95, 105, 10, true).unwrap();
        let status = db.get_sync_status().unwrap();
        assert_eq!(status.last_synced_block, 100);
    }
}
