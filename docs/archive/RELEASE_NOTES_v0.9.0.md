# Release Notes: v0.9.0

**Release Date:** January 2026

## Overview

Version 0.9.0 is a major UI/UX release focused on drill-down detail views and dashboard improvements. This is the final feature release before v1.0.

### New Features
- **Drill-down detail popups** for all list views (press Enter)
- **Dashboard reorganization** with improved field placement
- **Theme icons** in status bar (☽ Moon / ☀ Sun)
- **Validators view sorting** by type and seats

### Improvements
- Per-view selection state preserved when switching views
- MVM sync status and node version moved to status bar
- GRANDPA voter status moved to Our Validator panel
- New Finalized field showing lag from chain tip

## New Features

### Drill-Down Detail Popups

Press **Enter** on any list view to open a detail popup with comprehensive information:

#### Block Detail (Blocks View)
- Block number and full block hash
- Parent hash, state root, extrinsics root
- Slot number, sidechain epoch, mainchain epoch
- Timestamp (formatted)
- Author key with label (if known)
- Extrinsics count and finalization status

#### Validator Identity Card (Validators View)
- Registration status (Permissioned/Registered)
- All three public keys (Sidechain, AURA, GRANDPA)
- Stake amount (if available from RPC)
- Current epoch seats and committee percentage
- Blocks produced vs expected this epoch
- Total blocks produced (all time)

#### Epoch History Table (Performance View)
- Scrollable table with epoch-by-epoch history
- Columns: Epoch, Seats, Blocks, Expected, Ratio%
- Color-coded performance (green ≥90%, warning 70-90%, error <70%)
- Navigate with j/k or arrow keys within popup

#### Peer Detail (Peers View)
- Full Peer ID
- Remote address
- Best block number and hash
- Roles (Full/Light/Authority)

All popups dismiss with **Esc** or **q**.

### Dashboard Reorganization

The Network Status panel has been reorganized for better logical grouping:

**Before:**
```
Block:        #1234567              Uptime:       2d 5h
Node:         ✓ Synced              MVM:          12345 blocks
```

**After:**
```
Node:         ✓ Synced              Uptime:       2d 5h
Block:        #1234567              Finalized:    #1234564 (-3)
```

Changes:
- **Node sync** moved to top row (most important status)
- **Block + Finalized** now together (related chain state)
- **MVM sync** moved to status bar (app metadata, not network status)
- **Node version** moved to status bar
- **GRANDPA voter** moved to Our Validator panel (validator participation status)

### Our Validator Panel Changes

Row 1 now shows both participation statuses together:
```
Committee:    ✓ Elected (5 / 1200)  GRANDPA:      ✓ Voting
```

This groups related information: both fields indicate whether your validator is actively participating in consensus.

### Theme Icons

The status bar now includes thematic icons:
- **Midnight theme:** ☽ Midnight
- **Midday theme:** ☀ Midday

### Validators View Sorting

The Validators view is now sorted for easier navigation:
1. **Permissioned validators** first (at top)
2. **Registered validators** below
3. Within each group: sorted by **seats descending** (most active first)

## Technical Changes

### State Management Refactor

- Per-view selection state using `HashMap<ViewMode, usize>`
- Popup overlay system with `PopupContent` enum
- Selection preserved when switching between views

### New Database Query

Added `get_validator_epoch_history()` query for the Performance drill-down:
```rust
pub struct ValidatorEpochHistoryRecord {
    pub epoch: u64,
    pub seats: u32,
    pub committee_size: u32,
    pub blocks_produced: u64,
}
```

### No Schema Changes

This release does not modify the database schema. Existing databases are fully compatible.

## Keyboard Shortcuts Summary

| Key | Context | Action |
|-----|---------|--------|
| Enter | Blocks view | Open block detail popup |
| Enter | Validators view | Open identity card popup |
| Enter | Performance view | Open epoch history popup |
| Enter | Peers view | Open peer detail popup |
| Esc | Popup open | Close popup |
| j/k | Popup (scrollable) | Scroll up/down |
| q | Any view | Quit application |

## Upgrade Instructions

1. Stop any running MVM services:
   ```bash
   sudo systemctl stop mvm-sync
   ```

2. Build and install the new version:
   ```bash
   cargo build --release
   sudo cp target/release/mvm /opt/midnight/mvm/bin/
   ```

3. Restart services:
   ```bash
   sudo systemctl start mvm-sync
   ```

4. Verify the upgrade:
   ```bash
   mvm --version
   # Should show: mvm 0.9.0
   ```

No database migration required.

## What's Next: v1.0

Version 1.0 will focus on stability and polish:
- Comprehensive testing
- Documentation review
- Performance optimization
- Bug fixes from v0.9 feedback

## Files Changed

- `src/tui/app.rs` - State management, popup content, view stack
- `src/tui/ui.rs` - Popup rendering, dashboard reorganization
- `src/tui/event.rs` - Enter/Escape handling for popups
- `src/db/blocks.rs` - Epoch history query
- `Cargo.toml` - Version bump to 0.9.0
