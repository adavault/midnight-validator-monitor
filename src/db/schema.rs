use rusqlite::Connection;
use anyhow::Result;

/// SQL schema for MVM database
pub const SCHEMA: &str = r#"
-- Synchronized block headers
CREATE TABLE IF NOT EXISTS blocks (
    block_number INTEGER PRIMARY KEY,
    block_hash TEXT NOT NULL UNIQUE,
    parent_hash TEXT NOT NULL,
    state_root TEXT NOT NULL,
    extrinsics_root TEXT NOT NULL,
    slot_number INTEGER NOT NULL,
    epoch INTEGER NOT NULL,
    timestamp INTEGER NOT NULL,
    is_finalized INTEGER DEFAULT 0,
    author_key TEXT,
    extrinsics_count INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_blocks_hash ON blocks(block_hash);
CREATE INDEX IF NOT EXISTS idx_blocks_slot ON blocks(slot_number);
CREATE INDEX IF NOT EXISTS idx_blocks_epoch ON blocks(epoch);
CREATE INDEX IF NOT EXISTS idx_blocks_author ON blocks(author_key);
CREATE INDEX IF NOT EXISTS idx_blocks_timestamp ON blocks(timestamp);

-- Tracked validators
CREATE TABLE IF NOT EXISTS validators (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sidechain_key TEXT UNIQUE NOT NULL,
    aura_key TEXT,
    grandpa_key TEXT,
    label TEXT,
    is_ours INTEGER DEFAULT 0,
    registration_status TEXT,
    first_seen_epoch INTEGER,
    total_blocks INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_validators_sidechain ON validators(sidechain_key);
CREATE INDEX IF NOT EXISTS idx_validators_ours ON validators(is_ours);

-- Sync progress (singleton row)
CREATE TABLE IF NOT EXISTS sync_status (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    last_synced_block INTEGER NOT NULL DEFAULT 0,
    last_finalized_block INTEGER NOT NULL DEFAULT 0,
    chain_tip_block INTEGER NOT NULL DEFAULT 0,
    current_epoch INTEGER NOT NULL DEFAULT 0,
    is_syncing INTEGER DEFAULT 1,
    last_updated INTEGER NOT NULL
);

-- Initialize singleton
INSERT OR IGNORE INTO sync_status (id, last_synced_block, last_finalized_block, chain_tip_block, current_epoch, last_updated)
VALUES (1, 0, 0, 0, 0, 0);
"#;

/// Initialize database schema
pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creation() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"blocks".to_string()));
        assert!(tables.contains(&"validators".to_string()));
        assert!(tables.contains(&"sync_status".to_string()));
    }
}
