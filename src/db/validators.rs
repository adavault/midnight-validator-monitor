use anyhow::Result;
use rusqlite::{params, Connection};

/// Validator record for database storage
#[derive(Debug, Clone)]
pub struct ValidatorRecord {
    pub sidechain_key: String,
    pub aura_key: Option<String>,
    pub grandpa_key: Option<String>,
    pub label: Option<String>,
    pub is_ours: bool,
    pub registration_status: Option<String>,
    pub first_seen_epoch: Option<u64>,
    pub total_blocks: u64,
}

/// Insert or update a validator
pub fn upsert_validator(conn: &Connection, validator: &ValidatorRecord) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    conn.execute(
        "INSERT INTO validators
         (sidechain_key, aura_key, grandpa_key, label, is_ours, registration_status,
          first_seen_epoch, total_blocks, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(sidechain_key) DO UPDATE SET
           aura_key = ?2,
           grandpa_key = ?3,
           label = COALESCE(excluded.label, label),
           is_ours = MAX(is_ours, ?5),
           registration_status = ?6,
           updated_at = ?10",
        params![
            &validator.sidechain_key,
            &validator.aura_key,
            &validator.grandpa_key,
            &validator.label,
            validator.is_ours as i32,
            &validator.registration_status,
            validator.first_seen_epoch.map(|e| e as i64),
            validator.total_blocks as i64,
            now,
            now,
        ],
    )?;
    Ok(())
}

/// Get a validator by sidechain key
pub fn get_validator(conn: &Connection, sidechain_key: &str) -> Result<Option<ValidatorRecord>> {
    let mut stmt = conn.prepare(
        "SELECT sidechain_key, aura_key, grandpa_key, label, is_ours, registration_status,
                first_seen_epoch, total_blocks
         FROM validators
         WHERE sidechain_key = ?1",
    )?;

    let mut rows = stmt.query(params![sidechain_key])?;

    if let Some(row) = rows.next()? {
        Ok(Some(ValidatorRecord {
            sidechain_key: row.get(0)?,
            aura_key: row.get(1)?,
            grandpa_key: row.get(2)?,
            label: row.get(3)?,
            is_ours: row.get::<_, i32>(4)? != 0,
            registration_status: row.get(5)?,
            first_seen_epoch: row.get::<_, Option<i64>>(6)?.map(|e| e as u64),
            total_blocks: row.get::<_, i64>(7)? as u64,
        }))
    } else {
        Ok(None)
    }
}

/// Get all validators
pub fn get_all_validators(conn: &Connection) -> Result<Vec<ValidatorRecord>> {
    let mut stmt = conn.prepare(
        "SELECT sidechain_key, aura_key, grandpa_key, label, is_ours, registration_status,
                first_seen_epoch, total_blocks
         FROM validators
         ORDER BY is_ours DESC, total_blocks DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ValidatorRecord {
            sidechain_key: row.get(0)?,
            aura_key: row.get(1)?,
            grandpa_key: row.get(2)?,
            label: row.get(3)?,
            is_ours: row.get::<_, i32>(4)? != 0,
            registration_status: row.get(5)?,
            first_seen_epoch: row.get::<_, Option<i64>>(6)?.map(|e| e as u64),
            total_blocks: row.get::<_, i64>(7)? as u64,
        })
    })?;

    let mut validators = Vec::new();
    for row in rows {
        validators.push(row?);
    }
    Ok(validators)
}

/// Get our validators only
pub fn get_our_validators(conn: &Connection) -> Result<Vec<ValidatorRecord>> {
    let mut stmt = conn.prepare(
        "SELECT sidechain_key, aura_key, grandpa_key, label, is_ours, registration_status,
                first_seen_epoch, total_blocks
         FROM validators
         WHERE is_ours = 1
         ORDER BY total_blocks DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ValidatorRecord {
            sidechain_key: row.get(0)?,
            aura_key: row.get(1)?,
            grandpa_key: row.get(2)?,
            label: row.get(3)?,
            is_ours: row.get::<_, i32>(4)? != 0,
            registration_status: row.get(5)?,
            first_seen_epoch: row.get::<_, Option<i64>>(6)?.map(|e| e as u64),
            total_blocks: row.get::<_, i64>(7)? as u64,
        })
    })?;

    let mut validators = Vec::new();
    for row in rows {
        validators.push(row?);
    }
    Ok(validators)
}

/// Increment block count for a validator
pub fn increment_block_count(conn: &Connection, sidechain_key: &str) -> Result<()> {
    conn.execute(
        "UPDATE validators
         SET total_blocks = total_blocks + 1,
             updated_at = ?2
         WHERE sidechain_key = ?1",
        params![sidechain_key, chrono::Utc::now().timestamp()],
    )?;
    Ok(())
}

/// Count total validators
pub fn count_validators(conn: &Connection) -> Result<u64> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM validators", [], |row| row.get(0))?;
    Ok(count as u64)
}

/// Count our validators
pub fn count_our_validators(conn: &Connection) -> Result<u64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM validators WHERE is_ours = 1",
        [],
        |row| row.get(0),
    )?;
    Ok(count as u64)
}
