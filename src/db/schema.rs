use rusqlite::Connection;
use anyhow::{Result, bail};
use tracing::info;

/// Current schema version - increment when making schema changes
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// SQL schema for MVM database
pub const SCHEMA: &str = r#"
-- Schema metadata for version tracking and migrations
CREATE TABLE IF NOT EXISTS schema_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

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

/// Check if schema_meta table exists (for detecting pre-v1.0 databases)
fn has_schema_meta(conn: &Connection) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_meta'",
        [],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Get a metadata value from schema_meta
pub fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>> {
    let result = conn.query_row(
        "SELECT value FROM schema_meta WHERE key = ?1",
        [key],
        |row| row.get(0),
    );
    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Set a metadata value in schema_meta
pub fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO schema_meta (key, value) VALUES (?1, ?2)",
        [key, value],
    )?;
    Ok(())
}

/// Get the current schema version from the database (0 if not set)
pub fn get_schema_version(conn: &Connection) -> Result<u32> {
    match get_meta(conn, "schema_version")? {
        Some(v) => Ok(v.parse().unwrap_or(0)),
        None => Ok(0),
    }
}

/// Initialize schema metadata for a new or migrated database
fn init_schema_meta(conn: &Connection, version: u32, app_version: &str) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    set_meta(conn, "schema_version", &version.to_string())?;
    set_meta(conn, "created_at", &now)?;
    set_meta(conn, "created_by", app_version)?;
    set_meta(conn, "last_migration", &now)?;
    set_meta(conn, "last_migrated_by", app_version)?;
    Ok(())
}

/// Update migration metadata after a successful migration
fn update_migration_meta(conn: &Connection, version: u32, app_version: &str) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    set_meta(conn, "schema_version", &version.to_string())?;
    set_meta(conn, "last_migration", &now)?;
    set_meta(conn, "last_migrated_by", app_version)?;
    Ok(())
}

/// Run database migrations to bring schema up to current version
///
/// This function:
/// 1. Creates schema_meta table if it doesn't exist (pre-v1.0 databases)
/// 2. Checks current version against CURRENT_SCHEMA_VERSION
/// 3. Runs any needed migrations sequentially
/// 4. Refuses to open if database version is newer than app version
pub fn run_migrations(conn: &Connection) -> Result<()> {
    let app_version = env!("CARGO_PKG_VERSION");

    // Check if this is a pre-v1.0 database (no schema_meta table)
    if !has_schema_meta(conn)? {
        // Create the schema_meta table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
            [],
        )?;

        // Initialize as version 1 (the current schema without migrations)
        init_schema_meta(conn, 1, app_version)?;
        info!("Initialized schema metadata for existing database (version 1)");
        return Ok(());
    }

    // Get current database version
    let db_version = get_schema_version(conn)?;

    // Check if database is from the future (newer than this app)
    if db_version > CURRENT_SCHEMA_VERSION {
        bail!(
            "Database schema version ({}) is newer than this application supports ({}). \
             Please upgrade mvm to a newer version.",
            db_version,
            CURRENT_SCHEMA_VERSION
        );
    }

    // Check if database needs migration
    if db_version == 0 {
        // New database - initialize metadata
        init_schema_meta(conn, CURRENT_SCHEMA_VERSION, app_version)?;
        info!("Initialized new database with schema version {}", CURRENT_SCHEMA_VERSION);
        return Ok(());
    }

    if db_version == CURRENT_SCHEMA_VERSION {
        // Already at current version - nothing to do
        return Ok(());
    }

    // Run migrations from db_version to CURRENT_SCHEMA_VERSION
    info!(
        "Migrating database from version {} to {}",
        db_version, CURRENT_SCHEMA_VERSION
    );

    for version in (db_version + 1)..=CURRENT_SCHEMA_VERSION {
        run_migration(conn, version)?;
        update_migration_meta(conn, version, app_version)?;
        info!("Completed migration to version {}", version);
    }

    Ok(())
}

/// Run a specific migration
/// Add new migrations here as match arms when schema changes
fn run_migration(_conn: &Connection, to_version: u32) -> Result<()> {
    match to_version {
        // Version 1 is the base schema - no migration needed
        1 => Ok(()),

        // Future migrations go here:
        // 2 => {
        //     _conn.execute("ALTER TABLE blocks ADD COLUMN new_field TEXT", [])?;
        //     Ok(())
        // }

        _ => bail!("Unknown migration version: {}", to_version),
    }
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
        assert!(tables.contains(&"schema_meta".to_string()));
    }

    #[test]
    fn test_schema_versioning() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        run_migrations(&conn).unwrap();

        // Check version is set correctly
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);

        // Check metadata was set
        let created_by = get_meta(&conn, "created_by").unwrap();
        assert!(created_by.is_some());
        assert!(created_by.unwrap().contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn test_migration_from_pre_v1() {
        let conn = Connection::open_in_memory().unwrap();

        // Simulate a pre-v1.0 database by creating tables without schema_meta
        conn.execute_batch(r#"
            CREATE TABLE blocks (block_number INTEGER PRIMARY KEY);
            CREATE TABLE validators (id INTEGER PRIMARY KEY);
            CREATE TABLE sync_status (id INTEGER PRIMARY KEY);
        "#).unwrap();

        // Run migrations - should detect missing schema_meta and initialize it
        run_migrations(&conn).unwrap();

        // Verify schema_meta was created and version set to 1
        assert!(has_schema_meta(&conn).unwrap());
        assert_eq!(get_schema_version(&conn).unwrap(), 1);
    }
}
