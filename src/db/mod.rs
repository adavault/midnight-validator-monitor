mod blocks;
mod schema;
mod validators;

pub use blocks::{BlockRecord, SyncStatusRecord, ValidatorEpochRecord, ValidatorEpochHistoryRecord};
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
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open database at {}", path.display()))?;

        // Initialize schema
        init_schema(&conn)?;

        // Enable WAL mode for better performance and foreign key enforcement
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;")?;

        Ok(Self { conn })
    }

    /// Open an in-memory database (for testing)
    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        init_schema(&conn)?;
        Ok(Self { conn })
    }

    /// Get a reference to the underlying connection
    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn count_blocks_by_author_in_epoch(&self, author_key: &str, epoch: u64) -> Result<u64> {
        blocks::count_blocks_by_author_in_epoch(&self.conn, author_key, epoch)
    }

    pub fn count_blocks_by_author_since(&self, author_key: &str, since_timestamp: i64) -> Result<u64> {
        blocks::count_blocks_by_author_since(&self.conn, author_key, since_timestamp)
    }

    pub fn get_block_counts_bucketed(
        &self,
        author_keys: &[String],
        bucket_duration_secs: i64,
        num_buckets: usize,
    ) -> Result<Vec<u64>> {
        blocks::get_block_counts_bucketed(&self.conn, author_keys, bucket_duration_secs, num_buckets)
    }

    // Committee snapshot operations
    pub fn store_committee_snapshot(&self, epoch: u64, committee: &[String]) -> Result<()> {
        blocks::store_committee_snapshot(&self.conn, epoch, committee)
    }

    #[allow(dead_code)]
    pub fn get_committee_snapshot(&self, epoch: u64) -> Result<Option<Vec<String>>> {
        blocks::get_committee_snapshot(&self.conn, epoch)
    }

    #[allow(dead_code)]
    pub fn get_committee_size(&self, epoch: u64) -> Result<Option<usize>> {
        blocks::get_committee_size(&self.conn, epoch)
    }

    #[allow(dead_code)]
    pub fn list_committee_epochs(&self) -> Result<Vec<u64>> {
        blocks::list_committee_epochs(&self.conn)
    }

    // Validator epoch snapshot operations
    pub fn store_validator_epoch(&self, record: &ValidatorEpochRecord) -> Result<()> {
        blocks::store_validator_epoch(&self.conn, record)
    }

    #[allow(dead_code)]
    pub fn get_validator_epoch(
        &self,
        sidechain_epoch: u64,
        sidechain_key: &str,
    ) -> Result<Option<ValidatorEpochRecord>> {
        blocks::get_validator_epoch(&self.conn, sidechain_epoch, sidechain_key)
    }

    pub fn get_validators_for_epoch(&self, sidechain_epoch: u64) -> Result<Vec<ValidatorEpochRecord>> {
        blocks::get_validators_for_epoch(&self.conn, sidechain_epoch)
    }

    pub fn get_latest_validator_epochs(&self) -> Result<Vec<ValidatorEpochRecord>> {
        blocks::get_latest_validator_epochs(&self.conn)
    }

    pub fn has_validator_epoch_snapshot(&self, sidechain_epoch: u64) -> Result<bool> {
        blocks::has_validator_epoch_snapshot(&self.conn, sidechain_epoch)
    }

    pub fn get_total_seats_for_epochs(
        &self,
        sidechain_keys: &[String],
        current_epoch: u64,
        num_epochs: usize,
    ) -> Result<u64> {
        blocks::get_total_seats_for_epochs(&self.conn, sidechain_keys, current_epoch, num_epochs)
    }

    pub fn get_validator_epoch_history(
        &self,
        sidechain_key: &str,
        limit: usize,
    ) -> Result<Vec<blocks::ValidatorEpochHistoryRecord>> {
        blocks::get_validator_epoch_history(&self.conn, sidechain_key, limit)
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
            sidechain_epoch: 120,
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
