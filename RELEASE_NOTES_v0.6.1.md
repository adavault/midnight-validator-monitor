# Release Notes: v0.6.1

**Release Date:** January 2026

## Overview

Version 0.6.1 is a bug fix release that corrects the expected block prediction calculation and fixes a critical bug where validator ownership status was lost after epoch changes.

## Bug Fixes

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

Simply rebuild and restart:

```bash
cargo build --release
sudo systemctl restart mvm-sync  # if running as daemon
```

No database migrations required.

## Files Changed

- `Cargo.toml` - Version bump to 0.6.1
- `src/tui/app.rs` - Fixed expected block calculation constants
- `src/db/validators.rs` - Fixed is_ours preservation in upsert

## Contributors

Built with Claude Code (Anthropic)
