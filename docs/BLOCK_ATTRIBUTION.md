# Block Attribution Design

## Overview

Block attribution is the process of determining which validator produced each block. This is essential for tracking validator performance and block production statistics.

## How Block Attribution Works

1. **Extract slot number** from block's AURA PreRuntime digest
2. **Fetch committee** via `state_call("AuraApi_authorities")` at the block's hash
3. **Calculate author**: `author_aura_key = committee[slot % committee.len()]`
4. **Look up validator** by AURA key to get sidechain key
5. **Store** sidechain key as `author_key` in blocks table

## State Pruning Challenge

Non-archive Midnight nodes prune historical state to save disk space. Typically only the most recent ~256 blocks retain full state. This means:

- **Block data** (hash, number, parent, extrinsics) is always available via `chain_getBlock`
- **Runtime state** (committee list) is only available for recent blocks

When syncing historical blocks on a pruned node, we cannot fetch the committee that was active when that block was produced.

## Design Principles

### 1. Never Create Block Gaps

Block data should always be synced, regardless of whether attribution is possible. A complete block history is valuable even without author information.

### 2. Accurate Attribution or None

**Incorrect attribution is worse than no attribution.**

If we use the current committee to attribute historical blocks:
- Wrong validators get credited for blocks they didn't produce
- Correct validators don't get credit for their blocks
- Validator statistics become meaningless

Therefore: when historical state is pruned, set `author_key = NULL` rather than attributing incorrectly.

### 3. Two Types of Gaps

| Gap Type | Description | Acceptable? |
|----------|-------------|-------------|
| Block gaps | Missing blocks in database | No - always sync blocks |
| Author gaps | Blocks exist, `author_key` is NULL | Yes - when state unavailable |

## Implementation

### Sync Behavior

When syncing a block:

```
1. Fetch block data via chain_getBlock
2. Try to fetch committee at block hash
3. If committee available:
   - Calculate author from slot
   - Store block with author_key
4. If state pruned (committee unavailable):
   - Log warning (once per epoch)
   - Store block with author_key = NULL
5. Continue to next block
```

### Safe Start Detection

On sync startup:

1. Query current finalized block
2. Binary search backwards to find oldest block with available state
3. If `start_block` config is older than this:
   - Warn user that author attribution won't be available for older blocks
   - Continue sync (blocks will have NULL author_key)

### Restart After Downtime

When mvm-sync daemon restarts after node downtime:

1. Resume from `last_synced_block + 1`
2. For blocks where state is pruned: sync with `author_key = NULL`
3. For recent blocks where state is available: sync with accurate `author_key`
4. Validator statistics remain accurate for attributed blocks

## Database Schema

The `blocks` table `author_key` column is nullable:

```sql
author_key TEXT  -- NULL when attribution not possible
```

Queries should handle NULL appropriately:
- `COUNT(*) WHERE author_key = ?` - counts attributed blocks
- `COUNT(*) WHERE author_key IS NULL` - counts unattributed blocks

## Recommendations

### For Complete History

Use an **archive node** (`--state-pruning archive`) if you need:
- Complete block attribution for all historical blocks
- Accurate validator statistics from genesis

### For Typical Usage

Standard pruned nodes work well if you:
- Start mvm-sync when the node starts
- Keep mvm-sync running continuously
- Accept that blocks during extended downtime may lack attribution

## Logging

The sync command provides clear logging:

```
INFO  Syncing blocks 3357362 to 3387000...
WARN  Historical state pruned for epoch 1170 - blocks will be stored without author attribution
INFO  Block 3386750: state available, resuming author attribution
INFO  Synced 29638 blocks (28500 without attribution, 1138 with attribution)
```

## Future Enhancements

1. **Attribution backfill**: If an archive node becomes available, backfill NULL author_keys
2. **Peer attribution**: Query archive node peers for historical committee data
3. **Committee snapshots**: Store committee snapshots to help with attribution after restarts
