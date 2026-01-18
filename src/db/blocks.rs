use anyhow::Result;
use rusqlite::{params, Connection};

/// Block record for database storage
#[derive(Debug, Clone)]
pub struct BlockRecord {
    pub block_number: u64,
    pub block_hash: String,
    pub parent_hash: String,
    pub state_root: String,
    pub extrinsics_root: String,
    pub slot_number: u64,
    /// Mainchain epoch (24h on preview, 5 days on mainnet)
    pub epoch: u64,
    /// Sidechain epoch (2h on preview, 10h on mainnet) - determines committee rotation
    pub sidechain_epoch: u64,
    pub timestamp: i64,
    pub is_finalized: bool,
    pub author_key: Option<String>,
    pub extrinsics_count: u32,
}

/// Sync status record
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SyncStatusRecord {
    pub last_synced_block: u64,
    pub last_finalized_block: u64,
    pub chain_tip_block: u64,
    pub current_epoch: u64,
    pub is_syncing: bool,
    pub last_updated: i64,
}

/// Insert a block into the database
pub fn insert_block(conn: &Connection, block: &BlockRecord) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO blocks
         (block_number, block_hash, parent_hash, state_root, extrinsics_root,
          slot_number, epoch, sidechain_epoch, timestamp, is_finalized, author_key, extrinsics_count, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            block.block_number as i64,
            &block.block_hash,
            &block.parent_hash,
            &block.state_root,
            &block.extrinsics_root,
            block.slot_number as i64,
            block.epoch as i64,
            block.sidechain_epoch as i64,
            block.timestamp,
            block.is_finalized as i32,
            &block.author_key,
            block.extrinsics_count as i32,
            chrono::Utc::now().timestamp()
        ],
    )?;
    Ok(())
}

/// Get a block by number
pub fn get_block(conn: &Connection, block_number: u64) -> Result<Option<BlockRecord>> {
    let mut stmt = conn.prepare(
        "SELECT block_number, block_hash, parent_hash, state_root, extrinsics_root,
                slot_number, epoch, sidechain_epoch, timestamp, is_finalized, author_key, extrinsics_count
         FROM blocks WHERE block_number = ?1",
    )?;

    let result = stmt.query_row(params![block_number as i64], |row| {
        Ok(BlockRecord {
            block_number: row.get::<_, i64>(0)? as u64,
            block_hash: row.get(1)?,
            parent_hash: row.get(2)?,
            state_root: row.get(3)?,
            extrinsics_root: row.get(4)?,
            slot_number: row.get::<_, i64>(5)? as u64,
            epoch: row.get::<_, i64>(6)? as u64,
            sidechain_epoch: row.get::<_, i64>(7)? as u64,
            timestamp: row.get(8)?,
            is_finalized: row.get::<_, i32>(9)? != 0,
            author_key: row.get(10)?,
            extrinsics_count: row.get::<_, i32>(11)? as u32,
        })
    });

    match result {
        Ok(block) => Ok(Some(block)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Get the highest block number in the database
pub fn get_max_block_number(conn: &Connection) -> Result<Option<u64>> {
    let result: Option<i64> = conn.query_row(
        "SELECT MAX(block_number) FROM blocks",
        [],
        |row| row.get(0),
    )?;
    Ok(result.map(|n| n as u64))
}

/// Mark blocks as finalized up to a given block number
pub fn mark_finalized(conn: &Connection, up_to_block: u64) -> Result<usize> {
    let updated = conn.execute(
        "UPDATE blocks SET is_finalized = 1 WHERE block_number <= ?1 AND is_finalized = 0",
        params![up_to_block as i64],
    )?;
    Ok(updated)
}

/// Get sync status
pub fn get_sync_status(conn: &Connection) -> Result<SyncStatusRecord> {
    conn.query_row(
        "SELECT last_synced_block, last_finalized_block, chain_tip_block,
                current_epoch, is_syncing, last_updated
         FROM sync_status WHERE id = 1",
        [],
        |row| {
            Ok(SyncStatusRecord {
                last_synced_block: row.get::<_, i64>(0)? as u64,
                last_finalized_block: row.get::<_, i64>(1)? as u64,
                chain_tip_block: row.get::<_, i64>(2)? as u64,
                current_epoch: row.get::<_, i64>(3)? as u64,
                is_syncing: row.get::<_, i32>(4)? != 0,
                last_updated: row.get(5)?,
            })
        },
    )
    .map_err(Into::into)
}

/// Update sync status
pub fn update_sync_status(
    conn: &Connection,
    last_synced: u64,
    finalized: u64,
    tip: u64,
    epoch: u64,
    is_syncing: bool,
) -> Result<()> {
    conn.execute(
        "UPDATE sync_status SET
         last_synced_block = ?1,
         last_finalized_block = ?2,
         chain_tip_block = ?3,
         current_epoch = ?4,
         is_syncing = ?5,
         last_updated = ?6
         WHERE id = 1",
        params![
            last_synced as i64,
            finalized as i64,
            tip as i64,
            epoch as i64,
            is_syncing as i32,
            chrono::Utc::now().timestamp()
        ],
    )?;
    Ok(())
}

/// Count total blocks in database
pub fn count_blocks(conn: &Connection) -> Result<u64> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM blocks", [], |row| row.get(0))?;
    Ok(count as u64)
}

/// Count finalized blocks
pub fn count_finalized_blocks(conn: &Connection) -> Result<u64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM blocks WHERE is_finalized = 1",
        [],
        |row| row.get(0),
    )?;
    Ok(count as u64)
}

/// Get blocks in a range
pub fn get_blocks_in_range(
    conn: &Connection,
    from: u64,
    to: u64,
    limit: Option<u32>,
) -> Result<Vec<BlockRecord>> {
    let limit_clause = limit.map_or(String::new(), |l| format!(" LIMIT {}", l));
    let sql = format!(
        "SELECT block_number, block_hash, parent_hash, state_root, extrinsics_root,
                slot_number, epoch, sidechain_epoch, timestamp, is_finalized, author_key, extrinsics_count
         FROM blocks WHERE block_number >= ?1 AND block_number <= ?2
         ORDER BY block_number ASC{}",
        limit_clause
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![from as i64, to as i64], |row| {
        Ok(BlockRecord {
            block_number: row.get::<_, i64>(0)? as u64,
            block_hash: row.get(1)?,
            parent_hash: row.get(2)?,
            state_root: row.get(3)?,
            extrinsics_root: row.get(4)?,
            slot_number: row.get::<_, i64>(5)? as u64,
            epoch: row.get::<_, i64>(6)? as u64,
            sidechain_epoch: row.get::<_, i64>(7)? as u64,
            timestamp: row.get(8)?,
            is_finalized: row.get::<_, i32>(9)? != 0,
            author_key: row.get(10)?,
            extrinsics_count: row.get::<_, i32>(11)? as u32,
        })
    })?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

/// Find gaps in block sequence
pub fn find_gaps(conn: &Connection) -> Result<Vec<(u64, u64)>> {
    let mut stmt = conn.prepare(
        "SELECT b1.block_number + 1 AS gap_start,
                MIN(b2.block_number) - 1 AS gap_end
         FROM blocks b1
         LEFT JOIN blocks b2 ON b1.block_number < b2.block_number
         WHERE NOT EXISTS (
             SELECT 1 FROM blocks c WHERE c.block_number = b1.block_number + 1
         )
         AND b2.block_number IS NOT NULL
         GROUP BY b1.block_number
         HAVING gap_end >= gap_start",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)? as u64, row.get::<_, i64>(1)? as u64))
    })?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

/// Count blocks by author in a specific epoch
#[allow(dead_code)]
pub fn count_blocks_by_author_in_epoch(
    conn: &Connection,
    author_key: &str,
    epoch: u64,
) -> Result<u64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM blocks WHERE author_key = ?1 AND epoch = ?2",
        params![author_key, epoch as i64],
        |row| row.get(0),
    )?;
    Ok(count as u64)
}

/// Count blocks by author since a given timestamp (in seconds)
pub fn count_blocks_by_author_since(
    conn: &Connection,
    author_key: &str,
    since_timestamp: i64,
) -> Result<u64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM blocks WHERE author_key = ?1 AND timestamp >= ?2",
        params![author_key, since_timestamp],
        |row| row.get(0),
    )?;
    Ok(count as u64)
}

/// Store a committee snapshot for an epoch
///
/// This stores the complete committee (all AURA keys in order) for a specific epoch.
/// The committee is used for correct block author attribution.
pub fn store_committee_snapshot(
    conn: &Connection,
    epoch: u64,
    committee: &[String],
) -> Result<()> {
    // Delete existing snapshot for this epoch (if any)
    conn.execute(
        "DELETE FROM committee_snapshots WHERE epoch = ?1",
        params![epoch as i64],
    )?;

    // Insert all committee members with their positions
    let timestamp = chrono::Utc::now().timestamp();
    for (position, aura_key) in committee.iter().enumerate() {
        conn.execute(
            "INSERT INTO committee_snapshots (epoch, position, aura_key, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![epoch as i64, position as i64, aura_key, timestamp],
        )?;
    }

    tracing::debug!(
        "Stored committee snapshot for epoch {} ({} members)",
        epoch,
        committee.len()
    );

    Ok(())
}

/// Retrieve a committee snapshot for an epoch
///
/// Returns the committee in order (sorted by position).
/// Returns None if no committee snapshot exists for this epoch.
#[allow(dead_code)]
pub fn get_committee_snapshot(conn: &Connection, epoch: u64) -> Result<Option<Vec<String>>> {
    let mut stmt = conn.prepare(
        "SELECT aura_key FROM committee_snapshots
         WHERE epoch = ?1
         ORDER BY position ASC",
    )?;

    let rows = stmt.query_map(params![epoch as i64], |row| row.get::<_, String>(0))?;

    let committee: Vec<String> = rows.collect::<std::result::Result<Vec<_>, _>>()?;

    if committee.is_empty() {
        Ok(None)
    } else {
        Ok(Some(committee))
    }
}

/// Get committee size for an epoch
#[allow(dead_code)]
pub fn get_committee_size(conn: &Connection, epoch: u64) -> Result<Option<usize>> {
    let count: Option<i64> = conn.query_row(
        "SELECT COUNT(*) FROM committee_snapshots WHERE epoch = ?1",
        params![epoch as i64],
        |row| row.get(0),
    )?;

    Ok(count.and_then(|c| if c > 0 { Some(c as usize) } else { None }))
}

/// List all epochs with stored committee snapshots
#[allow(dead_code)]
pub fn list_committee_epochs(conn: &Connection) -> Result<Vec<u64>> {
    let mut stmt = conn.prepare("SELECT DISTINCT epoch FROM committee_snapshots ORDER BY epoch DESC")?;

    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;

    rows.map(|r| r.map(|n| n as u64).map_err(Into::into))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::init_schema;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn test_insert_and_get_block() {
        let conn = setup_db();

        let block = BlockRecord {
            block_number: 1000,
            block_hash: "0x123".to_string(),
            parent_hash: "0x122".to_string(),
            state_root: "0xabc".to_string(),
            extrinsics_root: "0xdef".to_string(),
            slot_number: 100000,
            epoch: 100,
            sidechain_epoch: 1200,
            timestamp: 1234567890,
            is_finalized: false,
            author_key: Some("0xvalidator".to_string()),
            extrinsics_count: 5,
        };

        insert_block(&conn, &block).unwrap();

        let retrieved = get_block(&conn, 1000).unwrap().unwrap();
        assert_eq!(retrieved.block_hash, "0x123");
        assert_eq!(retrieved.slot_number, 100000);
        assert_eq!(retrieved.epoch, 100);
        assert_eq!(retrieved.sidechain_epoch, 1200);
    }

    #[test]
    fn test_sync_status() {
        let conn = setup_db();

        let status = get_sync_status(&conn).unwrap();
        assert_eq!(status.last_synced_block, 0);

        update_sync_status(&conn, 1000, 990, 1005, 100, true).unwrap();

        let status = get_sync_status(&conn).unwrap();
        assert_eq!(status.last_synced_block, 1000);
        assert_eq!(status.last_finalized_block, 990);
        assert_eq!(status.chain_tip_block, 1005);
        assert_eq!(status.current_epoch, 100);
        assert!(status.is_syncing);
    }

    #[test]
    fn test_mark_finalized() {
        let conn = setup_db();

        for i in 1..=10 {
            let block = BlockRecord {
                block_number: i,
                block_hash: format!("0x{}", i),
                parent_hash: format!("0x{}", i - 1),
                state_root: "0x".to_string(),
                extrinsics_root: "0x".to_string(),
                slot_number: i * 100,
                epoch: 1,
                sidechain_epoch: 12,
                timestamp: 1234567890 + i as i64,
                is_finalized: false,
                author_key: None,
                extrinsics_count: 0,
            };
            insert_block(&conn, &block).unwrap();
        }

        let updated = mark_finalized(&conn, 5).unwrap();
        assert_eq!(updated, 5);

        assert_eq!(count_finalized_blocks(&conn).unwrap(), 5);
    }
}
