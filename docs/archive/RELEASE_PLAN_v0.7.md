# Release Plan - v0.7.0

## Overview

v0.7.0 is a **stability and correctness** release focused on improving reliability, fixing data accuracy issues, and laying the groundwork for future prediction improvements through stake data collection.

**Theme**: Stability, Correctness, Data Collection

## Goals

1. **Reliability**: RPC retry logic to handle transient network failures
2. **Data Accuracy**: Store actual block timestamps instead of sync time
3. **Prediction Foundation**: Capture and store stake data for future analysis
4. **Correctness**: Use actual committee size in prediction calculations

## Feature Breakdown

### 1. RPC Retry Logic (Stability)

**Problem**: Network hiccups cause sync failures. A single failed RPC call can crash the sync daemon.

**Solution**: Implement exponential backoff retry for transient errors.

**Implementation**:
```rust
// src/rpc/client.rs
pub struct RetryConfig {
    pub max_retries: u32,        // Default: 3
    pub initial_delay_ms: u64,   // Default: 1000
    pub max_delay_ms: u64,       // Default: 30000
    pub backoff_multiplier: f64, // Default: 2.0
}

impl RpcClient {
    pub async fn call_with_retry<T, P>(&self, method: &str, params: P) -> Result<T>
    where
        T: DeserializeOwned,
        P: Serialize,
    {
        let mut delay = self.retry_config.initial_delay_ms;
        let mut attempts = 0;

        loop {
            match self.call(method, &params).await {
                Ok(result) => return Ok(result),
                Err(e) if is_retryable(&e) && attempts < self.retry_config.max_retries => {
                    attempts += 1;
                    tracing::warn!(
                        "RPC call {} failed (attempt {}/{}), retrying in {}ms: {}",
                        method, attempts, self.retry_config.max_retries, delay, e
                    );
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    delay = (delay as f64 * self.retry_config.backoff_multiplier) as u64;
                    delay = delay.min(self.retry_config.max_delay_ms);
                }
                Err(e) => return Err(e),
            }
        }
    }
}

fn is_retryable(error: &anyhow::Error) -> bool {
    // Retry on: connection refused, timeout, 502/503/504, connection reset
    // Don't retry on: 4xx errors, parse errors, method not found
}
```

**Configuration**:
```toml
[rpc]
max_retries = 3
retry_initial_delay_ms = 1000
retry_max_delay_ms = 30000
```

**Files to modify**:
- `src/rpc/client.rs` - Add retry logic
- `src/config.rs` - Add retry configuration options

---

### 2. Block Timestamp Fix (Correctness)

**Problem**: Blocks are stored with `Utc::now()` (sync time) instead of actual block timestamp. This affects time-based queries and historical analysis.

**Current (Incorrect)**:
```rust
// src/commands/sync.rs
let timestamp = Utc::now();  // Wrong: uses current time
```

**Solution**: Extract actual timestamp from block data.

**Approach Options**:

1. **Slot-based calculation** (Recommended):
   - Midnight uses 6-second block times
   - Calculate: `timestamp = genesis_time + (slot * 6 seconds)`
   - Requires knowing genesis timestamp (can fetch from chain or configure)

2. **Extrinsic extraction**:
   - Some chains include `Timestamp.set` extrinsic
   - Parse extrinsics to find timestamp
   - More complex, may not be present in all blocks

**Implementation** (Slot-based):
```rust
// src/midnight/timing.rs
pub struct ChainTiming {
    pub genesis_timestamp_ms: u64,      // Network genesis time
    pub slot_duration_ms: u64,          // 6000ms for Midnight
    pub mainchain_epoch_ms: u64,        // 24h preview, 5 days mainnet
    pub sidechain_epoch_ms: u64,        // 2h preview, 10h mainnet
}

impl ChainTiming {
    /// Preview testnet timing (24h mainchain epochs)
    pub fn preview() -> Self {
        Self {
            genesis_timestamp_ms: 0,  // TBD
            slot_duration_ms: 6000,
            mainchain_epoch_ms: 24 * 60 * 60 * 1000,     // 24 hours
            sidechain_epoch_ms: 2 * 60 * 60 * 1000,      // 2 hours
        }
    }

    /// PreProd testnet timing (TBD - may differ from preview)
    pub fn preprod() -> Self {
        // TODO: Confirm preprod epoch durations
        // Assuming same as preview until confirmed otherwise
        Self {
            genesis_timestamp_ms: 0,  // TBD
            slot_duration_ms: 6000,
            mainchain_epoch_ms: 24 * 60 * 60 * 1000,     // TBD - assuming 24 hours
            sidechain_epoch_ms: 2 * 60 * 60 * 1000,      // TBD - assuming 2 hours
        }
    }

    /// Mainnet timing (5 day mainchain epochs)
    pub fn mainnet() -> Self {
        Self {
            genesis_timestamp_ms: 0,  // TBD
            slot_duration_ms: 6000,
            mainchain_epoch_ms: 5 * 24 * 60 * 60 * 1000, // 5 days
            sidechain_epoch_ms: 10 * 60 * 60 * 1000,     // 10 hours
        }
    }

    pub fn slot_to_timestamp(&self, slot: u64) -> DateTime<Utc> {
        let timestamp_ms = self.genesis_timestamp_ms + (slot * self.slot_duration_ms);
        DateTime::from_timestamp_millis(timestamp_ms as i64)
            .unwrap_or_else(Utc::now)
    }

    pub fn blocks_per_sidechain_epoch(&self) -> u64 {
        self.sidechain_epoch_ms / self.slot_duration_ms
    }
}

// Usage in sync.rs
let timestamp = chain_timing.slot_to_timestamp(slot_number);
```

**Note:** There are 12 sidechain epochs per mainchain epoch on both networks:
- Preview: 24h / 2h = 12
- Mainnet: 120h / 10h = 12

See `docs/EPOCH_TIMING.md` for detailed documentation on the epoch relationship.

**Configuration**:
```toml
[chain]
# Network preset: "preview", "preprod", or "mainnet"
network = "preview"

# Genesis timestamp for the network (milliseconds since Unix epoch)
# Testnet-02: TBD (need to determine from chain)
genesis_timestamp_ms = 1704067200000  # Example: 2024-01-01 00:00:00 UTC

# These can be omitted to use network preset defaults:
# slot_duration_ms = 6000           # 6 seconds per block
# mainchain_epoch_ms = 86400000     # 24 hours (preview) / 432000000 (mainnet - 5 days)
# sidechain_epoch_ms = 7200000      # 2 hours (preview) / 36000000 (mainnet - 10 hours)
```

**Investigation Required**:
- [ ] Determine testnet-02 (preview) genesis timestamp
- [ ] Verify slot duration is consistently 6 seconds
- [ ] Check if `sidechain_getStatus` or other RPC provides genesis info
- [ ] Confirm preprod epoch durations when Midnight launches on preprod
- [ ] Confirm mainnet epoch durations when available

**Files to modify**:
- `src/commands/sync.rs` - Use calculated timestamp
- `src/config.rs` - Add chain timing configuration
- `src/midnight/mod.rs` - Add timing module

**Migration Note**: Existing blocks will retain incorrect timestamps. Could add optional `--fix-timestamps` flag to recalculate historical blocks.

---

### 3. Stake Data Collection (Data Foundation)

**Problem**: We cannot predict future committee seat allocation without understanding the stake→seats relationship. Need to collect data first.

**Solution**: Capture `stakeDelegation` from AriadneParameters and store it historically.

**3a. Capture Stake from RPC**

**Current struct** (missing stake):
```rust
pub struct CandidateRegistration {
    pub sidechain_pub_key: String,
    pub aura_pub_key: String,
    pub grandpa_pub_key: String,
    pub is_valid: bool,
    pub invalid_reasons: Option<serde_json::Value>,
}
```

**Updated struct**:
```rust
pub struct CandidateRegistration {
    pub sidechain_pub_key: String,
    pub aura_pub_key: String,
    pub grandpa_pub_key: String,
    pub is_valid: bool,
    pub invalid_reasons: Option<serde_json::Value>,
    #[serde(default)]
    pub stake_delegation: Option<u128>,  // Lovelace (1 ADA = 1,000,000 lovelace)
}
```

**Files to modify**:
- `src/midnight/validators.rs` - Add stake field to structs
- `src/midnight/registration.rs` - Add stake field

**3b. Store Stake Historically**

**New database table**:
```sql
CREATE TABLE IF NOT EXISTS validator_epochs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    epoch INTEGER NOT NULL,
    sidechain_key TEXT NOT NULL,
    aura_key TEXT NOT NULL,
    stake_lovelace INTEGER,           -- Stake in lovelace (NULL if unknown)
    committee_seats INTEGER NOT NULL, -- Seats in committee this epoch
    committee_size INTEGER NOT NULL,  -- Total committee size
    is_permissioned INTEGER NOT NULL, -- 1 if permissioned candidate
    captured_at TEXT NOT NULL,        -- When this snapshot was taken
    UNIQUE(epoch, sidechain_key)
);

CREATE INDEX idx_validator_epochs_epoch ON validator_epochs(epoch);
CREATE INDEX idx_validator_epochs_key ON validator_epochs(sidechain_key);
```

**Data capture flow**:
1. On epoch change (detected in sync or view command)
2. Fetch AriadneParameters (includes stake)
3. Fetch committee (AuraApi_authorities)
4. Count seats per validator
5. Store snapshot in `validator_epochs`

**Files to modify**:
- `src/db/schema.rs` - Add new table
- `src/db/mod.rs` - Add snapshot storage functions
- `src/commands/sync.rs` - Capture epoch snapshots during sync
- `src/tui/app.rs` - Capture snapshots during TUI refresh

**3c. Display Stake in TUI**

Add stake display to Validators view:

```
┌─ Validators ──────────────────────────────────────────────────────────────┐
│ Key (Sidechain)              Stake (ADA)    Seats   Blocks   Share        │
│ ─────────────────────────────────────────────────────────────────────────│
│ ★ 0x037764d2...809f4700      1,258,876      3       23       0.050%      │
│   0x0203ae55...cdebc500      5,432,100      12      156      0.340%      │
│   0x1a2b3c4d...5e6f7890      892,340        2       45       0.098%      │
└───────────────────────────────────────────────────────────────────────────┘
```

**Files to modify**:
- `src/tui/ui.rs` - Add stake column to validators view
- `src/tui/app.rs` - Fetch and store stake data

---

### 4. Fix Prediction Calculation (Correctness)

**Problem**: Current prediction has two issues:
1. Uses hardcoded `1200.0` instead of actual committee size
2. Uses incorrect values: 2400 blocks/epoch and 3-second block time (should be 1200 blocks and 6 seconds)

**Current (Incorrect)** - `src/tui/app.rs:598-604`:
```rust
// ~2400 blocks per 2h sidechain epoch (1 block per 3 seconds)  // WRONG!
const BLOCKS_PER_SIDECHAIN_EPOCH: f64 = 2400.0;                 // WRONG!
let expected_per_seat = BLOCKS_PER_SIDECHAIN_EPOCH / 1200.0;    // Hardcoded!
```

**Fixed**:
```rust
let committee_size = self.state.committee_size as f64;
if committee_size > 0.0 {
    // ~1200 blocks per 2h sidechain epoch (1 block per 6 seconds)
    // Each committee member gets 1 slot per epoch rotation
    const BLOCKS_PER_SIDECHAIN_EPOCH: f64 = 1200.0;
    let expected_per_seat = BLOCKS_PER_SIDECHAIN_EPOCH / committee_size;
    self.state.epoch_progress.expected_blocks =
        epoch_progress_ratio * expected_per_seat * self.state.committee_seats as f64;
}
```

**Note**: With committee_size ≈ 1200 and BLOCKS_PER_EPOCH ≈ 1200, expected_per_seat ≈ 1.0 block per seat per epoch. This aligns with the round-robin model where each committee position produces exactly one block per epoch.

**Files to modify**:
- `src/tui/app.rs` - Fix constants and use actual committee_size

---

## Database Schema Changes

### New Table: validator_epochs

```sql
CREATE TABLE IF NOT EXISTS validator_epochs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    epoch INTEGER NOT NULL,
    sidechain_key TEXT NOT NULL,
    aura_key TEXT NOT NULL,
    stake_lovelace INTEGER,
    committee_seats INTEGER NOT NULL,
    committee_size INTEGER NOT NULL,
    is_permissioned INTEGER NOT NULL,
    captured_at TEXT NOT NULL,
    UNIQUE(epoch, sidechain_key)
);
```

### Migration

No breaking changes. New table is additive. Existing installations will automatically get the new table on first run.

---

## Configuration Additions

```toml
[rpc]
# Retry configuration for transient failures
max_retries = 3
retry_initial_delay_ms = 1000
retry_max_delay_ms = 30000

[chain]
# Chain timing for timestamp calculation
# These values are network-specific
genesis_timestamp_ms = 0  # TBD: Determine from chain
slot_duration_ms = 6000   # 6 seconds per block
```

---

## Testing Plan

### RPC Retry Logic
- [ ] Test with simulated network failures (disconnect during sync)
- [ ] Verify exponential backoff timing
- [ ] Confirm non-retryable errors fail immediately
- [ ] Test max retry limit is respected

### Block Timestamp
- [ ] Verify calculated timestamps match expected values
- [ ] Compare with block explorer timestamps (if available)
- [ ] Test with blocks from different epochs

### Stake Data Collection
- [ ] Verify stake field is captured from RPC
- [ ] Confirm validator_epochs records are created on epoch change
- [ ] Check seat counting matches manual verification
- [ ] Test TUI displays stake correctly

### Prediction Fix
- [ ] Verify committee_size is used (not hardcoded 1200)
- [ ] Test prediction accuracy against actual block production

---

## Implementation Order

1. **Week 1**: RPC retry logic
   - Implement RetryConfig and call_with_retry
   - Add configuration options
   - Update sync command to use retry

2. **Week 2**: Block timestamp fix
   - Research genesis timestamp for testnet-02
   - Implement ChainTiming module
   - Update sync to use calculated timestamps

3. **Week 3**: Stake data collection
   - Add stake field to structs
   - Create validator_epochs table
   - Implement epoch snapshot capture
   - Add TUI stake display

4. **Week 4**: Prediction fix + Testing + Release
   - Fix hardcoded committee size
   - Comprehensive testing
   - Documentation updates
   - Release

---

## Files Changed Summary

### New Files
- `src/midnight/timing.rs` - Chain timing calculations

### Modified Files
- `src/rpc/client.rs` - Retry logic
- `src/config.rs` - New configuration options
- `src/db/schema.rs` - New validator_epochs table
- `src/db/mod.rs` - Epoch snapshot functions
- `src/midnight/validators.rs` - Add stake field
- `src/midnight/registration.rs` - Add stake field
- `src/commands/sync.rs` - Retry, timestamps, epoch snapshots
- `src/tui/app.rs` - Epoch snapshots, fix prediction
- `src/tui/ui.rs` - Display stake in validators view

---

## Future Work (v0.8+)

This release sets up data collection for future prediction improvements:

1. **Analyze stake→seats correlation** using collected validator_epochs data
2. **Derive prediction formula** from empirical data
3. **Implement `NextCommittee` lookup** for next-epoch prediction
4. **Add confidence intervals** to predictions

---

## Success Criteria

- [ ] Sync survives transient network failures (auto-retry)
- [ ] Block timestamps are accurate (within slot duration)
- [ ] Stake data captured for all validators each epoch
- [ ] TUI shows stake in Validators view
- [ ] Prediction uses actual committee size
- [ ] All existing tests pass
- [ ] No regressions in TUI functionality

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Genesis timestamp unknown | Research via RPC or configure per-network |
| Stake field missing from some registrations | Use `Option<u128>` with default None |
| Large validator_epochs table over time | Add periodic cleanup or archival (future) |
| Retry logic masks real errors | Log all retries, fail fast on non-transient errors |

---

## Release Checklist

- [ ] All features implemented
- [ ] Tests pass
- [ ] Documentation updated (CLAUDE.md, README if needed)
- [ ] Version bumped in Cargo.toml
- [ ] Release notes drafted
- [ ] Binary built and tested on target system

---

**Target**: v0.7.0
**Theme**: Stability, Correctness, Data Collection
**Estimated Duration**: 4 weeks
