# Release Notes - v0.4.0-alpha

## Overview

This release fixes critical issues with block author attribution and adds support for syncing on non-archive nodes.

## Major Changes

### Fixed: Block Author Attribution

**Problem**: Blocks were being attributed to incorrect validators because the sync was using the candidate list (from `sidechain_getAriadneParameters`) instead of the actual committee (from `AuraApi_authorities`).

**Root Cause**:
- Candidates (~185 validators) are registered validators who MAY produce blocks
- Committee (~1200 seats, ~50-60 unique validators) is the actual block production schedule
- Block author = `committee[slot % committee.len()]`, not `candidates[slot % candidates.len()]`

**Fix**:
- Added `fetch_with_committee()` to query the AURA authorities via `state_call`
- Implemented SCALE decoder for the committee response (`src/midnight/scale.rs`)
- Updated sync to use committee for author attribution
- Store committee snapshots in database for each epoch

### Fixed: Historical Committee Queries

**Problem**: When syncing historical blocks, the code was querying the current committee instead of the committee at the time each block was produced.

**Root Cause**: Committees change each epoch (~2 hours). Using the current committee for historical blocks results in incorrect attribution.

**Fix**:
- Modified `fetch_with_committee()` to accept an optional `block_hash` parameter
- Pass the block hash to `state_call` to query historical state
- Cache committees per epoch during sync

### New: Fallback for Pruned Nodes

**Problem**: Non-archive nodes prune historical state (typically keeping only ~256 blocks). Syncing historical blocks fails when state is unavailable.

**Solution**:
- Added `fetch_with_committee_or_fallback()` method
- When historical state is pruned, falls back to current committee
- Logs warning that attribution may be inaccurate for pruned epochs
- Allows syncing entire chain history on pruned nodes (with caveats)

## Technical Details

### New Files
- `src/midnight/scale.rs` - SCALE decoder for AURA authorities

### Modified Files
- `src/midnight/validators.rs` - Added committee fetching, historical queries, and fallback logic
- `src/midnight/mod.rs` - Export new `decode_aura_authorities` function
- `src/commands/sync.rs` - Use committee for author attribution with fallback
- `src/db/schema.rs` - Added `committee_snapshots` table
- `src/db/blocks.rs` - Added committee snapshot storage/retrieval functions

### New Database Table
```sql
CREATE TABLE committee_snapshots (
    epoch INTEGER NOT NULL,
    position INTEGER NOT NULL,
    aura_key TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (epoch, position)
);
```

### Key Methods

```rust
// Fetch committee at historical block (preferred)
ValidatorSet::fetch_with_committee(rpc, epoch, Some(&block_hash)).await

// Fetch with fallback for pruned state
ValidatorSet::fetch_with_committee_or_fallback(rpc, epoch, &block_hash).await
// Returns (ValidatorSet, used_fallback: bool)

// Get block author from committee
let aura_key = committee[slot % committee.len()];
```

## Committee Selection Insights

Research during this release revealed important details about Midnight's committee selection:

1. **Not purely stake-based**: Validators with lower stake can be selected while higher-stake validators are not
2. **Probabilistic**: Selection appears to be stake-weighted random, similar to Cardano's slot leader election
3. **Epoch-based rotation**: Committee changes each epoch
4. **Permissioned validators included**: ~12 IOG/Midnight team validators are always in committee (no stake required)

## Upgrade Notes

### Database Migration

The sync will automatically create the new `committee_snapshots` table. However, for accurate author attribution:

1. **Recommended**: Delete existing database and resync
2. **Alternative**: Keep existing data but note that historical blocks may have incorrect attribution

### For Archive Nodes

If running an archive node (full historical state), author attribution will be accurate for all blocks.

### For Pruned Nodes

- Recent blocks (~256) will have accurate attribution
- Older blocks will use current committee (may be inaccurate if committee changed)
- Warning logged when fallback is used

## Known Limitations

1. **Non-archive nodes**: Historical block attribution may be inaccurate when syncing from before the node's state retention window
2. **Committee visibility**: The exact committee selection algorithm is not publicly documented

## Testing

Verified on Midnight testnet-02:
- Sync from block 3,000,000 with fallback working correctly
- Recent blocks show accurate author attribution
- Committee snapshots stored correctly (1200 seats per epoch)
