# Release Notes: v0.9.3

**Release Date:** January 22, 2026

## Summary

Bug fix release that improves sparkline accuracy by aligning block and seat counting to use the same epoch-based boundaries.

## Changes

### Bug Fixes

- **Sparkline epoch alignment** (Issue #15): Changed sparkline from timestamp-based to epoch-based block counting
  - Previously, blocks were counted by timestamp (last 48h) while seats were counted by epoch number
  - Now both use sidechain epoch boundaries, ensuring perfect alignment
  - Label updated from "Last 48h" to "24 Epochs" to reflect the change

### Issue Cleanup

- **Issue #4**: Closed as "cannot reproduce" - code analysis confirmed `is_ours` flag preservation works correctly
- **Issue #11**: Responded with Claude workflow documentation for community member
- **Issue #14**: Deferred style guide alignment to v2.0 (spom-core extraction phase)
- **Issue #15**: Fixed and closed

## Files Changed

- `src/db/blocks.rs` - Added `get_block_counts_by_epoch()` function
- `src/db/mod.rs` - Exposed new epoch-based counting function
- `src/tui/app.rs` - Use epoch-based counting for sparkline
- `src/tui/ui.rs` - Updated label from "Last 48h" to "24 Epochs"
- `docs/RELEASE_PLAN_v1.0.md` - Updated issue tracking and status

## Upgrade Notes

This is a drop-in replacement for v0.9.2. No database migration required.

## Installation

### From Binary

```bash
# Download and extract
tar xzf mvm-v0.9.3-linux-x86_64.tar.gz
sudo mv mvm /usr/local/bin/

# Or use the install command
sudo mvm install
```

### From Source

```bash
cargo build --release
sudo cp target/release/mvm /usr/local/bin/
```

---

*Full changelog: v0.9.2...v0.9.3*
