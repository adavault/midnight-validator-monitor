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

/// Validator epoch snapshot record
/// Captures validator state (seats, registration status) for each sidechain epoch
#[derive(Debug, Clone)]
pub struct ValidatorEpochRecord {
    pub sidechain_epoch: u64,
    pub sidechain_key: String,
    pub aura_key: String,
    pub committee_seats: u32,
    pub committee_size: u32,
    pub is_permissioned: bool,
    pub stake_lovelace: Option<u64>,
    pub captured_at: i64,
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

/// Get block counts for an author bucketed by time intervals
///
/// Returns a vector of block counts, one per bucket, from oldest to newest.
/// Useful for sparkline visualization of block production over time.
///
/// # Arguments
/// * `author_keys` - List of author keys to count (combined for "our" validators)
/// * `bucket_duration_secs` - Duration of each bucket in seconds
/// * `num_buckets` - Number of buckets to return
///
/// # Returns
/// Vector of counts from oldest bucket to newest (left to right for sparkline)
pub fn get_block_counts_bucketed(
    conn: &Connection,
    author_keys: &[String],
    bucket_duration_secs: i64,
    num_buckets: usize,
) -> Result<Vec<u64>> {
    if author_keys.is_empty() || num_buckets == 0 {
        return Ok(vec![0; num_buckets]);
    }

    let now = chrono::Utc::now().timestamp();
    let total_duration = bucket_duration_secs * num_buckets as i64;
    let start_time = now - total_duration;

    // Build IN clause for multiple author keys
    let placeholders: Vec<String> = (0..author_keys.len())
        .map(|i| format!("?{}", i + 3))
        .collect();
    let in_clause = placeholders.join(", ");

    // Query blocks with timestamp bucketing
    let sql = format!(
        "SELECT ((timestamp - ?1) / ?2) as bucket, COUNT(*) as count
         FROM blocks
         WHERE author_key IN ({})
           AND timestamp >= ?1
           AND timestamp < ?1 + (?2 * {})
         GROUP BY bucket
         ORDER BY bucket",
        in_clause, num_buckets
    );

    let mut stmt = conn.prepare(&sql)?;

    // Build params: start_time, bucket_duration, then all author keys
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    params.push(Box::new(start_time));
    params.push(Box::new(bucket_duration_secs));
    for key in author_keys {
        params.push(Box::new(key.clone()));
    }

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })?;

    // Initialize all buckets to 0
    let mut buckets = vec![0u64; num_buckets];

    // Fill in counts from query results
    for row in rows {
        let (bucket_idx, count) = row?;
        if bucket_idx >= 0 && (bucket_idx as usize) < num_buckets {
            buckets[bucket_idx as usize] = count as u64;
        }
    }

    Ok(buckets)
}

/// Get block counts for validators bucketed by sidechain epoch
///
/// Returns a vector of block counts, one per epoch, from oldest to newest.
/// This aligns with how seats are counted (by epoch) for accurate comparison.
///
/// # Arguments
/// * `author_keys` - List of author keys (sidechain keys) to count
/// * `current_epoch` - Current sidechain epoch
/// * `num_epochs` - Number of epochs to return (going backwards from current)
///
/// # Returns
/// Vector of counts from oldest epoch to newest (left to right for sparkline)
pub fn get_block_counts_by_epoch(
    conn: &Connection,
    author_keys: &[String],
    current_epoch: u64,
    num_epochs: usize,
) -> Result<Vec<u64>> {
    if author_keys.is_empty() || num_epochs == 0 {
        return Ok(vec![0; num_epochs]);
    }

    // Exclude current epoch (incomplete) - show num_epochs of *completed* epochs
    let end_epoch = current_epoch.saturating_sub(1);
    let start_epoch = end_epoch.saturating_sub(num_epochs as u64 - 1);

    // Build IN clause for multiple author keys
    let placeholders: Vec<String> = (0..author_keys.len())
        .map(|i| format!("?{}", i + 3))
        .collect();
    let in_clause = placeholders.join(", ");

    // Query blocks grouped by sidechain_epoch
    let sql = format!(
        "SELECT sidechain_epoch, COUNT(*) as count
         FROM blocks
         WHERE author_key IN ({})
           AND sidechain_epoch >= ?1
           AND sidechain_epoch <= ?2
         GROUP BY sidechain_epoch
         ORDER BY sidechain_epoch",
        in_clause
    );

    let mut stmt = conn.prepare(&sql)?;

    // Build params: start_epoch, end_epoch, then all author keys
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    params.push(Box::new(start_epoch as i64));
    params.push(Box::new(end_epoch as i64));
    for key in author_keys {
        params.push(Box::new(key.clone()));
    }

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })?;

    // Initialize all epochs to 0
    let mut buckets = vec![0u64; num_epochs];

    // Fill in counts from query results
    for row in rows {
        let (epoch, count) = row?;
        // Map epoch number to bucket index (0 = oldest = start_epoch)
        let idx = (epoch as u64).saturating_sub(start_epoch) as usize;
        if idx < num_epochs {
            buckets[idx] = count as u64;
        }
    }

    Ok(buckets)
}

/// Get seat counts per epoch for specified validators (for sparkline)
/// Returns a vector of seat counts, one per epoch, oldest first
pub fn get_seats_by_epoch(
    conn: &Connection,
    sidechain_keys: &[String],
    current_epoch: u64,
    num_epochs: usize,
) -> Result<Vec<u64>> {
    if sidechain_keys.is_empty() || num_epochs == 0 {
        return Ok(vec![0; num_epochs]);
    }

    // Exclude current epoch (incomplete) - show num_epochs of *completed* epochs
    let end_epoch = current_epoch.saturating_sub(1);
    let start_epoch = end_epoch.saturating_sub(num_epochs as u64 - 1);

    // Build IN clause for multiple sidechain keys
    let placeholders: Vec<String> = (0..sidechain_keys.len())
        .map(|i| format!("?{}", i + 3))
        .collect();
    let in_clause = placeholders.join(", ");

    // Query seats grouped by sidechain_epoch from validator_epochs table
    let sql = format!(
        "SELECT sidechain_epoch, COALESCE(SUM(committee_seats), 0) as seats
         FROM validator_epochs
         WHERE sidechain_key IN ({})
           AND sidechain_epoch >= ?1
           AND sidechain_epoch <= ?2
         GROUP BY sidechain_epoch
         ORDER BY sidechain_epoch",
        in_clause
    );

    let mut stmt = conn.prepare(&sql)?;

    // Build params: start_epoch, end_epoch, then all sidechain keys
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    params.push(Box::new(start_epoch as i64));
    params.push(Box::new(end_epoch as i64));
    for key in sidechain_keys {
        params.push(Box::new(key.clone()));
    }

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })?;

    // Initialize all epochs to 0
    let mut buckets = vec![0u64; num_epochs];

    // Fill in counts from query results
    for row in rows {
        let (epoch, seats) = row?;
        // Map epoch number to bucket index (0 = oldest = start_epoch)
        let idx = (epoch as u64).saturating_sub(start_epoch) as usize;
        if idx < num_epochs {
            buckets[idx] = seats as u64;
        }
    }

    Ok(buckets)
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

/// Store a validator epoch snapshot
///
/// Captures validator state for a specific sidechain epoch including:
/// - Committee seats (how many times they appear in committee)
/// - Committee size (total committee size)
/// - Whether they're a permissioned validator
/// - Stake amount (if available)
pub fn store_validator_epoch(conn: &Connection, record: &ValidatorEpochRecord) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO validator_epochs
         (sidechain_epoch, sidechain_key, aura_key, committee_seats, committee_size,
          is_permissioned, stake_lovelace, captured_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            record.sidechain_epoch as i64,
            &record.sidechain_key,
            &record.aura_key,
            record.committee_seats as i64,
            record.committee_size as i64,
            record.is_permissioned as i32,
            record.stake_lovelace.map(|s| s as i64),
            record.captured_at
        ],
    )?;
    Ok(())
}

/// Get a validator's epoch snapshot
#[allow(dead_code)]
pub fn get_validator_epoch(
    conn: &Connection,
    sidechain_epoch: u64,
    sidechain_key: &str,
) -> Result<Option<ValidatorEpochRecord>> {
    let mut stmt = conn.prepare(
        "SELECT sidechain_epoch, sidechain_key, aura_key, committee_seats, committee_size,
                is_permissioned, stake_lovelace, captured_at
         FROM validator_epochs
         WHERE sidechain_epoch = ?1 AND sidechain_key = ?2",
    )?;

    let result = stmt.query_row(params![sidechain_epoch as i64, sidechain_key], |row| {
        Ok(ValidatorEpochRecord {
            sidechain_epoch: row.get::<_, i64>(0)? as u64,
            sidechain_key: row.get(1)?,
            aura_key: row.get(2)?,
            committee_seats: row.get::<_, i64>(3)? as u32,
            committee_size: row.get::<_, i64>(4)? as u32,
            is_permissioned: row.get::<_, i32>(5)? != 0,
            stake_lovelace: row.get::<_, Option<i64>>(6)?.map(|s| s as u64),
            captured_at: row.get(7)?,
        })
    });

    match result {
        Ok(record) => Ok(Some(record)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Get all validator epoch snapshots for a specific epoch
pub fn get_validators_for_epoch(
    conn: &Connection,
    sidechain_epoch: u64,
) -> Result<Vec<ValidatorEpochRecord>> {
    let mut stmt = conn.prepare(
        "SELECT sidechain_epoch, sidechain_key, aura_key, committee_seats, committee_size,
                is_permissioned, stake_lovelace, captured_at
         FROM validator_epochs
         WHERE sidechain_epoch = ?1
         ORDER BY committee_seats DESC, sidechain_key ASC",
    )?;

    let rows = stmt.query_map(params![sidechain_epoch as i64], |row| {
        Ok(ValidatorEpochRecord {
            sidechain_epoch: row.get::<_, i64>(0)? as u64,
            sidechain_key: row.get(1)?,
            aura_key: row.get(2)?,
            committee_seats: row.get::<_, i64>(3)? as u32,
            committee_size: row.get::<_, i64>(4)? as u32,
            is_permissioned: row.get::<_, i32>(5)? != 0,
            stake_lovelace: row.get::<_, Option<i64>>(6)?.map(|s| s as u64),
            captured_at: row.get(7)?,
        })
    })?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

/// Get the latest validator epoch snapshot for each validator
pub fn get_latest_validator_epochs(conn: &Connection) -> Result<Vec<ValidatorEpochRecord>> {
    let mut stmt = conn.prepare(
        "SELECT ve.sidechain_epoch, ve.sidechain_key, ve.aura_key, ve.committee_seats,
                ve.committee_size, ve.is_permissioned, ve.stake_lovelace, ve.captured_at
         FROM validator_epochs ve
         INNER JOIN (
             SELECT sidechain_key, MAX(sidechain_epoch) as max_epoch
             FROM validator_epochs
             GROUP BY sidechain_key
         ) latest ON ve.sidechain_key = latest.sidechain_key
                  AND ve.sidechain_epoch = latest.max_epoch
         ORDER BY ve.committee_seats DESC, ve.sidechain_key ASC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ValidatorEpochRecord {
            sidechain_epoch: row.get::<_, i64>(0)? as u64,
            sidechain_key: row.get(1)?,
            aura_key: row.get(2)?,
            committee_seats: row.get::<_, i64>(3)? as u32,
            committee_size: row.get::<_, i64>(4)? as u32,
            is_permissioned: row.get::<_, i32>(5)? != 0,
            stake_lovelace: row.get::<_, Option<i64>>(6)?.map(|s| s as u64),
            captured_at: row.get(7)?,
        })
    })?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

/// Check if we have a validator epoch snapshot for a specific epoch
pub fn has_validator_epoch_snapshot(conn: &Connection, sidechain_epoch: u64) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM validator_epochs WHERE sidechain_epoch = ?1",
        params![sidechain_epoch as i64],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Validator epoch history record for drill-down view
#[derive(Debug, Clone)]
pub struct ValidatorEpochHistoryRecord {
    pub epoch: u64,
    pub seats: u32,
    pub committee_size: u32,
    pub blocks_produced: u64,
}

/// Get validator epoch-by-epoch history for drill-down view
///
/// Returns a list of epochs with the validator's seats and blocks produced,
/// ordered by epoch descending (most recent first).
pub fn get_validator_epoch_history(
    conn: &Connection,
    sidechain_key: &str,
    limit: usize,
) -> Result<Vec<ValidatorEpochHistoryRecord>> {
    // Query validator_epochs for seats info, then join with blocks count
    let sql = "
        SELECT
            ve.sidechain_epoch,
            ve.committee_seats,
            ve.committee_size,
            COALESCE(bc.block_count, 0) as blocks_produced
        FROM validator_epochs ve
        LEFT JOIN (
            SELECT sidechain_epoch, COUNT(*) as block_count
            FROM blocks
            WHERE author_key = ?1
            GROUP BY sidechain_epoch
        ) bc ON ve.sidechain_epoch = bc.sidechain_epoch
        WHERE ve.sidechain_key = ?1
        ORDER BY ve.sidechain_epoch DESC
        LIMIT ?2
    ";

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![sidechain_key, limit as i64], |row| {
        Ok(ValidatorEpochHistoryRecord {
            epoch: row.get::<_, i64>(0)? as u64,
            seats: row.get::<_, i64>(1)? as u32,
            committee_size: row.get::<_, i64>(2)? as u32,
            blocks_produced: row.get::<_, i64>(3)? as u64,
        })
    })?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

/// Get total committee seats for specified validators over the last N epochs
///
/// Returns the sum of committee_seats for all specified sidechain_keys
/// across epochs from (current_epoch - num_epochs + 1) to current_epoch inclusive.
pub fn get_total_seats_for_epochs(
    conn: &Connection,
    sidechain_keys: &[String],
    current_epoch: u64,
    num_epochs: usize,
) -> Result<u64> {
    if sidechain_keys.is_empty() || num_epochs == 0 {
        return Ok(0);
    }

    // Exclude current epoch (incomplete) - show num_epochs of *completed* epochs
    let end_epoch = current_epoch.saturating_sub(1);
    let start_epoch = end_epoch.saturating_sub(num_epochs as u64 - 1);

    // Build IN clause for multiple sidechain keys
    let placeholders: Vec<String> = (0..sidechain_keys.len())
        .map(|i| format!("?{}", i + 3))
        .collect();
    let in_clause = placeholders.join(", ");

    let query = format!(
        "SELECT COALESCE(SUM(committee_seats), 0) FROM validator_epochs
         WHERE sidechain_epoch >= ?1 AND sidechain_epoch <= ?2
         AND sidechain_key IN ({})",
        in_clause
    );

    let mut stmt = conn.prepare(&query)?;

    // Build params: start_epoch, end_epoch, then all sidechain_keys
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![
        Box::new(start_epoch as i64),
        Box::new(end_epoch as i64),
    ];
    for key in sidechain_keys {
        params.push(Box::new(key.clone()));
    }

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let total: i64 = stmt.query_row(param_refs.as_slice(), |row| row.get(0))?;

    Ok(total as u64)
}

/// Committee selection statistics for a validator
#[derive(Debug, Clone, Default)]
pub struct CommitteeSelectionStats {
    /// Total epochs tracked in database for this validator
    pub epochs_tracked: u64,
    /// Number of epochs where validator was selected (seats > 0)
    pub times_selected: u64,
    /// Total committee seats received across all epochs
    pub total_seats: u64,
    /// Most recent epoch where validator was selected
    pub last_selected_epoch: Option<u64>,
    /// Current epoch being tracked
    pub current_epoch: u64,
    /// Whether validator is currently in committee
    pub currently_in_committee: bool,
    /// Validator's stake rank among dynamic validators (1 = highest stake)
    pub stake_rank: Option<u32>,
    /// Total number of dynamic validators
    pub total_dynamic_validators: u32,
    /// Validator's stake as percentage of dynamic pool
    pub stake_share_percent: Option<f64>,
    /// Percentage of committee seats held by permissioned validators
    pub permissioned_seats_percent: f64,
}

impl CommitteeSelectionStats {
    /// Calculate selection rate as a string (e.g., "3 of 22 epochs")
    pub fn selection_rate_display(&self) -> String {
        format!("{} of {} epochs", self.times_selected, self.epochs_tracked)
    }

    /// Calculate average epochs between selections
    pub fn avg_epochs_between_selections(&self) -> Option<f64> {
        if self.times_selected <= 1 {
            None
        } else {
            Some(self.epochs_tracked as f64 / self.times_selected as f64)
        }
    }

    /// Calculate epochs since last selection
    pub fn epochs_since_selection(&self) -> Option<u64> {
        self.last_selected_epoch.map(|last| self.current_epoch.saturating_sub(last))
    }

    /// Calculate average seats per selected epoch
    pub fn avg_seats_when_selected(&self) -> Option<f64> {
        if self.times_selected == 0 {
            None
        } else {
            Some(self.total_seats as f64 / self.times_selected as f64)
        }
    }
}

/// Get committee selection statistics for a validator
pub fn get_committee_selection_stats(
    conn: &Connection,
    sidechain_key: &str,
    current_epoch: u64,
) -> Result<CommitteeSelectionStats> {
    // Query selection history summary
    let mut stmt = conn.prepare(
        "SELECT
            COUNT(*) as epochs_tracked,
            SUM(CASE WHEN committee_seats > 0 THEN 1 ELSE 0 END) as times_selected,
            COALESCE(SUM(committee_seats), 0) as total_seats,
            MAX(CASE WHEN committee_seats > 0 THEN sidechain_epoch ELSE NULL END) as last_selected
         FROM validator_epochs
         WHERE sidechain_key = ?1"
    )?;

    let (epochs_tracked, times_selected, total_seats, last_selected): (i64, i64, i64, Option<i64>) =
        stmt.query_row(params![sidechain_key], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?;

    // Check if currently in committee
    let currently_in_committee: bool = conn.query_row(
        "SELECT committee_seats > 0 FROM validator_epochs
         WHERE sidechain_key = ?1 AND sidechain_epoch = ?2",
        params![sidechain_key, current_epoch as i64],
        |row| row.get(0)
    ).unwrap_or(false);

    // Get stake rank and dynamic validator info
    let (stake_rank, total_dynamic, stake_share) = get_stake_rank_info(conn, sidechain_key, current_epoch)?;

    // Get committee structure (permissioned vs dynamic)
    let permissioned_percent = get_permissioned_seats_percent(conn, current_epoch)?;

    Ok(CommitteeSelectionStats {
        epochs_tracked: epochs_tracked as u64,
        times_selected: times_selected as u64,
        total_seats: total_seats as u64,
        last_selected_epoch: last_selected.map(|e| e as u64),
        current_epoch,
        currently_in_committee,
        stake_rank,
        total_dynamic_validators: total_dynamic,
        stake_share_percent: stake_share,
        permissioned_seats_percent: permissioned_percent,
    })
}

/// Get stake rank info for a validator among dynamic validators
fn get_stake_rank_info(
    conn: &Connection,
    sidechain_key: &str,
    current_epoch: u64,
) -> Result<(Option<u32>, u32, Option<f64>)> {
    // Get validator's stake
    let validator_stake: Option<i64> = conn.query_row(
        "SELECT stake_lovelace FROM validator_epochs
         WHERE sidechain_key = ?1 AND sidechain_epoch = ?2 AND is_permissioned = 0",
        params![sidechain_key, current_epoch as i64],
        |row| row.get(0)
    ).ok().flatten();

    // Get total dynamic validators and total stake
    let (total_dynamic, total_stake): (i64, i64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(stake_lovelace), 0) FROM validator_epochs
         WHERE sidechain_epoch = ?1 AND is_permissioned = 0 AND stake_lovelace IS NOT NULL",
        params![current_epoch as i64],
        |row| Ok((row.get(0)?, row.get(1)?))
    ).unwrap_or((0, 0));

    if let Some(stake) = validator_stake {
        // Calculate rank (how many have higher stake + 1)
        let higher_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM validator_epochs
             WHERE sidechain_epoch = ?1 AND is_permissioned = 0
             AND stake_lovelace > ?2 AND stake_lovelace IS NOT NULL",
            params![current_epoch as i64, stake],
            |row| row.get(0)
        ).unwrap_or(0);

        let rank = (higher_count + 1) as u32;
        let share = if total_stake > 0 {
            Some((stake as f64 / total_stake as f64) * 100.0)
        } else {
            None
        };

        Ok((Some(rank), total_dynamic as u32, share))
    } else {
        Ok((None, total_dynamic as u32, None))
    }
}

/// Get percentage of committee seats held by permissioned validators
fn get_permissioned_seats_percent(conn: &Connection, current_epoch: u64) -> Result<f64> {
    let result: (i64, i64) = conn.query_row(
        "SELECT
            COALESCE(SUM(CASE WHEN is_permissioned = 1 THEN committee_seats ELSE 0 END), 0),
            COALESCE(SUM(committee_seats), 0)
         FROM validator_epochs
         WHERE sidechain_epoch = ?1",
        params![current_epoch as i64],
        |row| Ok((row.get(0)?, row.get(1)?))
    ).unwrap_or((0, 0));

    let (permissioned_seats, total_seats) = result;
    if total_seats > 0 {
        Ok((permissioned_seats as f64 / total_seats as f64) * 100.0)
    } else {
        Ok(0.0)
    }
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

    #[test]
    fn test_validator_epoch_snapshot() {
        let conn = setup_db();

        // Store validator epoch data
        let record1 = ValidatorEpochRecord {
            sidechain_epoch: 100,
            sidechain_key: "0xsidechain1".to_string(),
            aura_key: "0xaura1".to_string(),
            committee_seats: 15,
            committee_size: 1200,
            is_permissioned: true,
            stake_lovelace: Some(1_000_000_000),
            captured_at: chrono::Utc::now().timestamp(),
        };

        let record2 = ValidatorEpochRecord {
            sidechain_epoch: 100,
            sidechain_key: "0xsidechain2".to_string(),
            aura_key: "0xaura2".to_string(),
            committee_seats: 10,
            committee_size: 1200,
            is_permissioned: false,
            stake_lovelace: Some(500_000_000),
            captured_at: chrono::Utc::now().timestamp(),
        };

        store_validator_epoch(&conn, &record1).unwrap();
        store_validator_epoch(&conn, &record2).unwrap();

        // Test get single validator epoch
        let retrieved = get_validator_epoch(&conn, 100, "0xsidechain1")
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.committee_seats, 15);
        assert!(retrieved.is_permissioned);
        assert_eq!(retrieved.stake_lovelace, Some(1_000_000_000));

        // Test get all validators for epoch
        let validators = get_validators_for_epoch(&conn, 100).unwrap();
        assert_eq!(validators.len(), 2);
        // Should be sorted by committee_seats descending
        assert_eq!(validators[0].sidechain_key, "0xsidechain1");
        assert_eq!(validators[1].sidechain_key, "0xsidechain2");

        // Test has_validator_epoch_snapshot
        assert!(has_validator_epoch_snapshot(&conn, 100).unwrap());
        assert!(!has_validator_epoch_snapshot(&conn, 101).unwrap());
    }

    #[test]
    fn test_latest_validator_epochs() {
        let conn = setup_db();

        // Store records across multiple epochs
        let record1_e100 = ValidatorEpochRecord {
            sidechain_epoch: 100,
            sidechain_key: "0xsidechain1".to_string(),
            aura_key: "0xaura1".to_string(),
            committee_seats: 10,
            committee_size: 1200,
            is_permissioned: true,
            stake_lovelace: Some(1_000_000_000),
            captured_at: 1000,
        };

        let record1_e101 = ValidatorEpochRecord {
            sidechain_epoch: 101,
            sidechain_key: "0xsidechain1".to_string(),
            aura_key: "0xaura1".to_string(),
            committee_seats: 12,
            committee_size: 1200,
            is_permissioned: true,
            stake_lovelace: Some(1_100_000_000),
            captured_at: 2000,
        };

        let record2_e100 = ValidatorEpochRecord {
            sidechain_epoch: 100,
            sidechain_key: "0xsidechain2".to_string(),
            aura_key: "0xaura2".to_string(),
            committee_seats: 5,
            committee_size: 1200,
            is_permissioned: false,
            stake_lovelace: Some(500_000_000),
            captured_at: 1000,
        };

        store_validator_epoch(&conn, &record1_e100).unwrap();
        store_validator_epoch(&conn, &record1_e101).unwrap();
        store_validator_epoch(&conn, &record2_e100).unwrap();

        // Get latest snapshots
        let latest = get_latest_validator_epochs(&conn).unwrap();
        assert_eq!(latest.len(), 2);

        // validator1 should have epoch 101 data
        let v1 = latest.iter().find(|v| v.sidechain_key == "0xsidechain1").unwrap();
        assert_eq!(v1.sidechain_epoch, 101);
        assert_eq!(v1.committee_seats, 12);

        // validator2 should have epoch 100 data (latest for that validator)
        let v2 = latest.iter().find(|v| v.sidechain_key == "0xsidechain2").unwrap();
        assert_eq!(v2.sidechain_epoch, 100);
        assert_eq!(v2.committee_seats, 5);
    }
}
