# Release Notes: v0.7.0

**Release Date:** January 2026

## Overview

Version 0.7.0 includes bug fixes, new features, and UX improvements:

### Bug Fixes
- Fixed expected block prediction being 2x too high
- Fixed validator ownership status being lost after epoch changes
- Fixed TUI version display being hardcoded
- Fixed block author attribution on pruned nodes (was attributing incorrectly)
- Fixed block timestamps (now calculated from slot number instead of sync time)
- Fixed external IP display showing incorrect peer-reported addresses

### New Features
- **Sparkline visualization**: Dashboard shows block production history as a sparkline graph
- **Shell completions**: Support for bash, zsh, fish, powershell, and elvish
- **Page scrolling**: J/K (uppercase) or PageUp/PageDown for faster navigation
- **Help screen scrollbar**: Visual indicator when help content extends beyond screen

### Improvements
- Improved `mvm keys verify` messaging when validator not yet in database
- Updated config file search paths and example configuration
- Added sidechain epoch tracking to blocks

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

### Fixed: Block Author Attribution on Pruned Nodes

When syncing historical blocks on a pruned node (non-archive), the sync command was incorrectly attributing blocks to validators using the current committee instead of the historical committee. This caused wrong validators to be credited with blocks they didn't produce.

**Root Cause:**
Non-archive nodes prune historical state (typically keeping ~256 blocks). When historical state is unavailable, the code fell back to the current committee, but committees change each sidechain epoch (2 hours). Using the wrong committee means `slot % committee.len()` points to a different validator.

**Fix:**
- When historical state is pruned, blocks are now stored with `author_key = NULL` instead of incorrect attribution
- Added safe start block detection with binary search to find oldest block with available state
- Added startup warning when syncing from blocks older than available state
- See `docs/BLOCK_ATTRIBUTION.md` for full design documentation

**Before (incorrect):**
```
Block 3357362 authored by validator 0xabc123... (wrong validator)
```

**After (correct):**
```
WARN Historical state pruned for epoch 1170 - blocks will be stored without author attribution
Block 3357362 stored without author (state pruned)
```

### Improved: `mvm keys verify` Messaging

When running `mvm keys verify` and the validator is not yet in the database, the command now:
- Creates the validator record automatically (if registered)
- Shows clear messaging about what happened
- No longer tells users to "run mvm sync" when sync won't help

### Fixed: External IP Display Showing Incorrect Addresses

The TUI's external IP display was showing peer-reported relay addresses instead of the node's actual external IP. This happened because libp2p's `externalAddresses` includes all discovered addresses (configured + peer-reported).

**Fix:**
Added `view.expected_ip` config option to filter external addresses:

```toml
[view]
expected_ip = "203.0.113.10"  # Your actual external IP
```

Or via environment variable:
```bash
MVM_EXPECTED_IP="203.0.113.10" mvm view
```

See `docs/EXTERNAL_IP_RESEARCH.md` for detailed findings on this issue.

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

## Known Limitations

### Inbound Peer Count Unreliable
The TUI's inbound peer count may show 0 even when inbound connections exist. This is due to a limitation in the `system_unstable_networkState` RPC which only reports peers with "dialing" endpoints (outbound connections). Prometheus metrics confirm inbound connections are working, but this data isn't exposed via the RPC we use.

**Workaround:** Check Prometheus directly:
```bash
curl -s "http://localhost:9615/metrics" | grep "connections_opened_total"
```

**Fix planned for v0.8:** Replace RPC-based peer detection with Prometheus metrics for accurate inbound/outbound counts.

## Security Review

A comprehensive security review was conducted for this release with no issues found:

| Area | Status | Notes |
|------|--------|-------|
| SQL Injection | Safe | All queries use parameterized `params![]` |
| Command Injection | Safe | No shell/subprocess execution |
| File Path Traversal | Safe | Paths from config/CLI only |
| Panic/Unwrap | Safe | All `unwrap()` in test code only |
| Unsafe Code | Safe | No `unsafe` blocks |
| Integer Overflow | Low risk | Casts bounded to block numbers/counts |
| Memory Exhaustion | Protected | SCALE decoder validates data length |
| Error Disclosure | Appropriate | No sensitive info in errors |
| Dependencies | Clean | No known vulnerabilities |

## Files Changed

- `Cargo.toml` - Version bump to 0.7.0, added clap_complete dependency
- `src/config.rs` - Added `view.expected_ip` config option
- `src/tui/app.rs` - Fixed expected block calculation, external IP filtering
- `src/tui/ui.rs` - Dynamic version display, show sidechain_epoch
- `src/db/validators.rs` - Fixed is_ours preservation in upsert
- `src/db/blocks.rs` - Added sidechain_epoch field to BlockRecord
- `src/db/schema.rs` - Added sidechain_epoch column to blocks table
- `src/commands/sync.rs` - Capture sidechain_epoch, safe attribution, pruning detection
- `src/commands/view.rs` - Pass expected_ip config to TUI
- `src/commands/keys.rs` - Improved validator creation and messaging
- `docs/COMPATIBILITY.md` - New compatibility policy documentation
- `docs/BLOCK_ATTRIBUTION.md` - New block attribution design documentation
- `docs/EXTERNAL_IP_RESEARCH.md` - External IP detection research and solution
- `docs/BACKLOG.md` - Future research items, v0.8 release plan
- `src/main.rs` - Added shell completions command
- `src/tui/ui.rs` - Added sparkline visualization, help screen scrollbar
- `src/tui/event.rs` - Added page up/down key bindings (J/K, PageUp/PageDown)
- `src/tui/app.rs` - Added sparkline state, page scroll methods
- `src/tui/layout.rs` - Adjusted validator panel height for sparkline
- `src/db/blocks.rs` - Added bucketed block counting for sparkline
- `src/commands/config.rs` - Updated example config with detailed documentation

## Contributors

Built with Claude Code (Anthropic)
