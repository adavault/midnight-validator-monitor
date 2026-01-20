use rusqlite::Connection;
use anyhow::Result;

/// SQL schema for MVM database
pub const SCHEMA: &str = r#"
-- Synchronized block headers
-- Note: 'epoch' is mainchain epoch (24h on preview, 5 days on mainnet)
-- 'sidechain_epoch' is sidechain epoch (2h on preview, 10h on mainnet)
CREATE TABLE IF NOT EXISTS blocks (
    block_number INTEGER PRIMARY KEY,
    block_hash TEXT NOT NULL UNIQUE,
    parent_hash TEXT NOT NULL,
    state_root TEXT NOT NULL,
    extrinsics_root TEXT NOT NULL,
    slot_number INTEGER NOT NULL,
    epoch INTEGER NOT NULL,
    sidechain_epoch INTEGER NOT NULL DEFAULT 0,
    timestamp INTEGER NOT NULL,
    is_finalized INTEGER DEFAULT 0,
    author_key TEXT,
    extrinsics_count INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_blocks_hash ON blocks(block_hash);
CREATE INDEX IF NOT EXISTS idx_blocks_slot ON blocks(slot_number);
CREATE INDEX IF NOT EXISTS idx_blocks_epoch ON blocks(epoch);
CREATE INDEX IF NOT EXISTS idx_blocks_sidechain_epoch ON blocks(sidechain_epoch);
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

-- Committee snapshots (for historical committee composition)
-- Note: 'epoch' is SIDECHAIN epoch (~2h preview, ~10h mainnet) - committees rotate each sidechain epoch
-- Prior to v1.0, this incorrectly stored mainchain epochs - existing data may need migration
CREATE TABLE IF NOT EXISTS committee_snapshots (
    epoch INTEGER NOT NULL,
    position INTEGER NOT NULL,
    aura_key TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (epoch, position)
);

CREATE INDEX IF NOT EXISTS idx_committee_epoch ON committee_snapshots(epoch);
CREATE INDEX IF NOT EXISTS idx_committee_aura_key ON committee_snapshots(aura_key);

-- Validator epoch snapshots (per-validator-per-epoch data for stake/seats analysis)
CREATE TABLE IF NOT EXISTS validator_epochs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sidechain_epoch INTEGER NOT NULL,
    sidechain_key TEXT NOT NULL,
    aura_key TEXT NOT NULL,
    committee_seats INTEGER NOT NULL DEFAULT 0,
    committee_size INTEGER NOT NULL DEFAULT 0,
    is_permissioned INTEGER NOT NULL DEFAULT 0,
    stake_lovelace INTEGER,
    captured_at INTEGER NOT NULL,
    UNIQUE(sidechain_epoch, sidechain_key)
);

CREATE INDEX IF NOT EXISTS idx_validator_epochs_epoch ON validator_epochs(sidechain_epoch);
CREATE INDEX IF NOT EXISTS idx_validator_epochs_key ON validator_epochs(sidechain_key);

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
        assert!(tables.contains(&"committee_snapshots".to_string()));
        assert!(tables.contains(&"validator_epochs".to_string()));
        assert!(tables.contains(&"sync_status".to_string()));
    }
}
