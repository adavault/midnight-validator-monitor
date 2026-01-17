# Epoch Timing: Sidechain and Mainchain Relationship

## Overview

Midnight has two epoch systems that run in parallel:

1. **Mainchain Epoch** - Inherited from Cardano, determines stake snapshots and rewards
2. **Sidechain Epoch** - Midnight-specific, determines committee rotation and block production eligibility

## Epoch Relationship

The sidechain epoch is a subdivision of the mainchain epoch. There are **12 sidechain epochs per mainchain epoch**.

```
┌─────────────────────────── Mainchain Epoch ───────────────────────────┐
│  SC   │  SC   │  SC   │  SC   │  SC   │  SC   │  SC   │  SC   │ ...  │
│ Ep 1  │ Ep 2  │ Ep 3  │ Ep 4  │ Ep 5  │ Ep 6  │ Ep 7  │ Ep 8  │ x12  │
└───────┴───────┴───────┴───────┴───────┴───────┴───────┴───────┴──────┘
```

### Mathematical Relationship

Given the mainchain epoch progress (0.0 to 1.0):

```
sidechain_epoch_index = floor(mainchain_progress × 12)
sidechain_progress = (mainchain_progress × 12) mod 1
```

**Example:** If mainchain is at 91.5%:
- `0.915 × 12 = 10.98`
- Sidechain epoch index: 10 (11th epoch, 0-indexed)
- Sidechain progress: `10.98 - 10 = 0.98 = 98%`

## Network-Specific Durations

### Cardano Preview Testnet (Current)

| Epoch Type | Duration | Blocks (approx) |
|------------|----------|-----------------|
| Mainchain  | 24 hours | - |
| Sidechain  | 2 hours  | ~1200 |
| Block time | 6 seconds | - |

### Cardano PreProd Testnet

PreProd is a longer-running testnet that more closely mirrors mainnet conditions. Epoch durations TBD.

| Epoch Type | Duration | Blocks (approx) |
|------------|----------|-----------------|
| Mainchain  | TBD | - |
| Sidechain  | TBD | TBD |
| Block time | 6 seconds | - |

**Note:** PreProd timing to be confirmed when Midnight launches on this network.

### Cardano Mainnet (Expected)

Cardano mainnet epochs are **5 days** (120 hours). If the 12:1 ratio is preserved:

| Epoch Type | Duration | Blocks (approx) |
|------------|----------|-----------------|
| Mainchain  | 5 days (120 hours) | - |
| Sidechain  | 10 hours | ~6000 |
| Block time | 6 seconds | - |

**Note:** The actual mainnet durations are not yet confirmed. The ratio may or may not be preserved.

## Code Abstraction Requirements

The current codebase has **hardcoded durations** that will need to be made configurable for mainnet:

### Current Hardcoded Values

**`src/tui/app.rs`:**
```rust
const SIDECHAIN_EPOCH_DURATION_MS: u64 = 2 * 60 * 60 * 1000; // 2 hours in ms
const MAINCHAIN_EPOCH_DURATION_MS: u64 = 24 * 60 * 60 * 1000; // 24 hours in ms
const SIDECHAIN_EPOCH_DURATION_SECS: i64 = 2 * 60 * 60; // 2 hours in seconds
const BLOCKS_PER_SIDECHAIN_EPOCH: f64 = 1200.0; // Should be ~1200 for 2h with 6s blocks
```

### Recommended Configuration Structure

```toml
[chain]
# Network identifier for preset selection
network = "preview"  # "preview", "preprod", or "mainnet"

# Override epoch durations if needed (otherwise use network presets)
# mainchain_epoch_hours = 24
# sidechain_epoch_hours = 2
# block_time_seconds = 6
```

### Network Presets

The code should support network presets:

```rust
pub struct ChainTiming {
    pub mainchain_epoch_ms: u64,
    pub sidechain_epoch_ms: u64,
    pub block_time_ms: u64,
    pub epochs_per_mainchain: u64,  // Should always be 12 if ratio preserved
}

impl ChainTiming {
    pub fn preview() -> Self {
        Self {
            mainchain_epoch_ms: 24 * 60 * 60 * 1000,      // 24 hours
            sidechain_epoch_ms: 2 * 60 * 60 * 1000,       // 2 hours
            block_time_ms: 6000,                           // 6 seconds
            epochs_per_mainchain: 12,
        }
    }

    pub fn preprod() -> Self {
        // TODO: Confirm preprod timing when available
        Self {
            mainchain_epoch_ms: 24 * 60 * 60 * 1000,      // TBD - assuming 24 hours
            sidechain_epoch_ms: 2 * 60 * 60 * 1000,       // TBD - assuming 2 hours
            block_time_ms: 6000,                           // 6 seconds
            epochs_per_mainchain: 12,
        }
    }

    pub fn mainnet() -> Self {
        Self {
            mainchain_epoch_ms: 5 * 24 * 60 * 60 * 1000,  // 5 days
            sidechain_epoch_ms: 10 * 60 * 60 * 1000,      // 10 hours (if ratio preserved)
            block_time_ms: 6000,                           // 6 seconds
            epochs_per_mainchain: 12,
        }
    }

    pub fn blocks_per_sidechain_epoch(&self) -> u64 {
        self.sidechain_epoch_ms / self.block_time_ms
    }
}
```

## Impact on Block Prediction

The expected blocks calculation depends on epoch duration:

```rust
let blocks_per_epoch = chain_timing.blocks_per_sidechain_epoch();
let expected_per_seat = blocks_per_epoch as f64 / committee_size as f64;
let expected_blocks = epoch_progress_ratio * expected_per_seat * our_seats as f64;
```

| Network | Sidechain Epoch | Block Time | Blocks/Epoch | Expected/Seat |
|---------|-----------------|------------|--------------|---------------|
| Preview | 2 hours | 6 sec | 1,200 | ~1.0 |
| PreProd | TBD | 6 sec | TBD | TBD |
| Mainnet | 10 hours | 6 sec | 6,000 | ~5.0 |

## RPC Data Source

Epoch timing comes from `sidechain_getStatus` RPC:

```json
{
  "sidechain": {
    "epoch": 245639,
    "slot": 46627312,
    "nextEpochTimestamp": 1705432800000
  },
  "mainchain": {
    "epoch": 1179,
    "slot": 12345678,
    "nextEpochTimestamp": 1705449600000
  }
}
```

The `nextEpochTimestamp` (milliseconds since Unix epoch) is used to calculate progress:

```rust
let time_remaining_ms = next_epoch_timestamp - now_ms;
let time_elapsed_ms = epoch_duration_ms - time_remaining_ms;
let progress = time_elapsed_ms / epoch_duration_ms;
```

## Committee Rotation

The committee rotates at **sidechain epoch boundaries**:

- At each sidechain epoch transition, a new committee is selected
- Committee selection is stake-weighted (more stake = more seats)
- Committee size is ~1200 seats (may vary slightly)
- Each seat produces 1 block per epoch in round-robin order

## Future Considerations

1. **Mainnet Launch**: Verify actual epoch durations before mainnet deployment
2. **Configuration**: Make durations configurable rather than hardcoded
3. **Auto-detection**: Consider detecting network from RPC and auto-selecting presets
4. **Ratio Changes**: Monitor whether the 12:1 ratio is preserved on mainnet

## References

- `src/tui/app.rs` - Epoch progress calculations
- `src/rpc/types.rs` - SidechainStatus struct definition
- CLAUDE.md - Midnight-specific timing documentation
