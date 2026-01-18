# Release Notes: v0.6.1

**Release Date:** January 2026

## Overview

Version 0.6.1 is a bug fix release that addresses several issues:
- Fixed expected block prediction being 2x too high
- Fixed validator ownership status being lost after epoch changes
- Fixed TUI version display being hardcoded
- Added sidechain epoch tracking to blocks (displays correct epoch in TUI)

**Breaking Change:** Database schema updated - requires database recreation (see Upgrade Instructions).

## Bug Fixes

### Fixed: Recent Blocks Showing Mainchain Epoch Instead of Sidechain Epoch

The "Recent Blocks" panel in the TUI was displaying the mainchain epoch (24h cycles) instead of the sidechain epoch (2h cycles). Since sidechain epochs determine committee rotation and are more relevant for block production monitoring, the display now shows sidechain epoch.

**Changes:**
- Added `sidechain_epoch` column to blocks table
- Sync command now captures both mainchain and sidechain epochs per block
- TUI displays sidechain epoch in block listings

### Fixed: TUI Version Display Hardcoded

The version number in the TUI title bar was hardcoded to "v0.6.0" instead of reading from Cargo.toml.

**Fix:** Now uses `env!("CARGO_PKG_VERSION")` to display the correct version.

### Fixed: Validator "is_ours" Status Lost After Epoch Changes

Validators marked as "ours" via `mvm keys verify` would lose their ownership status when the sync command or TUI updated validator records during epoch changes. Users had to re-run `mvm keys verify` to restore the status.

**Root Cause:**
The `upsert_validator` SQL statement was overwriting the `is_ours` flag with `false` whenever the sync command updated a validator record (which it does with `is_ours: false` since it doesn't know which validators are ours).

**Fix:**
Changed the SQL from `is_ours = ?5` to `is_ours = MAX(is_ours, ?5)`. This ensures:
- Once a validator is marked as "ours", it stays marked
- The sync command can no longer accidentally unmark validators

### Fixed: Expected Block Prediction Was 2x Too High

The "expected blocks this epoch" calculation in the dashboard was showing values approximately twice as high as they should be due to incorrect timing constants.

**Root Cause:**
- Block time was incorrectly assumed to be 3 seconds (actually 6 seconds)
- Blocks per sidechain epoch was set to 2400 (should be 1200)
- Committee size was hardcoded instead of using the actual fetched value

**Changes:**
1. Corrected block time from 3 seconds to 6 seconds in comments
2. Fixed `BLOCKS_PER_SIDECHAIN_EPOCH` constant from 2400.0 to 1200.0
3. Changed hardcoded committee size divisor (1200.0) to use actual `committee_size` from network

**Before (incorrect):**
```
This Epoch: 1 blocks (expected: ~2.4)
```

**After (correct):**
```
This Epoch: 1 blocks (expected: ~1.2)
```

### Technical Details

The sidechain epoch timing is:
- Epoch duration: 2 hours (7200 seconds)
- Block time: 6 seconds
- Blocks per epoch: 7200 / 6 = **1200 blocks**
- Committee size: ~1199-1200 seats
- Expected blocks per seat per epoch: ~1.0

## Upgrade Instructions

This release includes a database schema change. You must recreate the database:

```bash
# Stop the sync daemon
sudo systemctl stop mvm-sync

# Remove the old database
rm /opt/midnight/mvm/data/mvm.db

# Rebuild the binary
cargo build --release
cp target/release/mvm /opt/midnight/mvm/bin/mvm

# Restart sync (will recreate database and re-sync)
sudo systemctl start mvm-sync

# Re-register your validator keys (after some blocks have synced)
mvm keys --keystore /path/to/keystore --db-path /opt/midnight/mvm/data/mvm.db verify
```

See `docs/COMPATIBILITY.md` for our pre-v1.0 compatibility policy.

## Files Changed

- `Cargo.toml` - Version bump to 0.6.1
- `src/tui/app.rs` - Fixed expected block calculation constants
- `src/tui/ui.rs` - Dynamic version display, show sidechain_epoch
- `src/db/validators.rs` - Fixed is_ours preservation in upsert
- `src/db/blocks.rs` - Added sidechain_epoch field to BlockRecord
- `src/db/schema.rs` - Added sidechain_epoch column to blocks table
- `src/commands/sync.rs` - Capture and store sidechain_epoch
- `docs/COMPATIBILITY.md` - New compatibility policy documentation

## Contributors

Built with Claude Code (Anthropic)
