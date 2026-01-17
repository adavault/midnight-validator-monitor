# Release Plan - v0.5.0

## Overview

v0.5.0 focuses on TUI improvements, node sync visibility, and better validator identity display. This release addresses layout inefficiencies discovered in the v0.4 review and adds critical monitoring features.

## Security Status

**Confirmed SECURE** - Full security audit completed:
- No private key exposure anywhere in codebase
- Only public keys loaded, stored, and displayed
- Keys properly truncated based on screen size
- All RPC calls use public keys only

## Priority Features for v0.5

### 1. Node Sync Progress Bar (HIGH)
**Goal:** Show node synchronization status with visual progress indicator

**Implementation:**
- Query `system_syncState` RPC for `currentBlock`, `highestBlock`, `startingBlock`
- Calculate sync percentage: `(current - start) / (highest - start) * 100`
- Display progress bar in Network Status panel
- Show "Synced" indicator when current == highest
- Add estimated time remaining (based on sync rate)

**Display (Dashboard):**
```
Node Sync:    ━━━━━━━━━━━━━━━━░░░░ 78.5% (ETA: 2h 15m)
              Block 2,847,123 / 3,627,450
```

### 2. Configurable Node Name (HIGH)
**Goal:** Display validator/node identity with configurable name

**Implementation:**
- Add `[validator] name = "my-validator"` config option
- Default to system hostname if not specified
- Display in title bar and Dashboard
- Store in database for historical reference

**Config (mvm.toml):**
```toml
[validator]
name = "midnight-validator-01"  # Optional, defaults to hostname
```

**Display:**
```
┌─ Midnight Validator Monitor ─ midnight-validator-01 ─────────────────────┐
```

### 3. Display All Public Keys (HIGH)
**Goal:** Show all three validator public keys prominently

**Implementation:**
- Add dedicated "Node Identity" section to Dashboard
- Show Sidechain, AURA, and Grandpa public keys
- Configurable display mode: truncated (default) or full
- Add 'k' keyboard shortcut to toggle key display length

**Display (Dashboard):**
```
┌─ Node Identity ──────────────────────────────────────────────────────────┐
│ Sidechain: 0x037764d2d83c269030fef6df5aeb4419c48762ada2cf20...809f4700   │
│ AURA:      0xe05be3c28c72864efc49f4f12cb04f3bd6f20fdbc29750...273b3e1e   │
│ Grandpa:   0xf5a39df9227f630754f78bbae43bd66a693612eeffa9ce...5f48d0e8   │
└──────────────────────────────────────────────────────────────────────────┘
```

### 4. Improved Dashboard Grid Layout (MEDIUM)
**Goal:** Better utilize screen space, especially on wide terminals

**Implementation:**
- Enable side-by-side layout for large screens (width > 160)
- Use percentage-based panel heights instead of fixed
- Add configurable panel visibility

**Large Screen Layout (New):**
```
┌─ Network Status ─────────────┬─ Node Identity ──────────────────────────┐
│ Health: ● Synced             │ Sidechain: 0x037764d2...809f4700         │
│ Peers: 12                    │ AURA:      0xe05be3c2...273b3e1e         │
│ Epoch: 1180 (42.5%)          │ Grandpa:   0xf5a39df9...5f48d0e8         │
│ Sync: ━━━━━━━━━━━━━━ 100%    │ Committee: ✓ Elected (1 seat/1200)       │
├─ Our Validators ─────────────┴──────────────────────────────────────────┤
│ Count: 1    All-Time: 23 blocks    Share: 0.497%                        │
│ This Epoch: 0 blocks (expected: ~8.2)                                   │
│ ★ 0x037764d2...809f4700 - 23 blocks                                     │
├─ Recent Blocks ─────────────────────────────────────────────────────────┤
│ #3372650  slot 294893245  epoch 1180  ✓  author: 0x0203ae55...cdebc5    │
│ ...                                                                      │
└─────────────────────────────────────────────────────────────────────────┘
```

**Current Screen Layout (Vertical - keep for small/medium):**
```
┌─ Network Status ────────────────────────────────────────────────────────┐
├─ Our Validators ────────────────────────────────────────────────────────┤
├─ Recent Blocks ─────────────────────────────────────────────────────────┤
└─────────────────────────────────────────────────────────────────────────┘
```

## Bug Fixes for v0.5

### 5. Fix Block Timestamp Storage (HIGH)
**Issue:** Blocks stored with `Utc::now()` instead of actual block timestamp
**Location:** `src/commands/sync.rs:544`
**Fix:** Implement proper timestamp extraction from extrinsics or use slot-based calculation

### 6. Enforce RPC Timeout (MEDIUM)
**Issue:** `timeout_ms` config exists but is never used
**Location:** `src/rpc/client.rs`
**Fix:** Apply timeout to reqwest client

### 7. Add RPC Retry Logic (LOW)
**Issue:** Network hiccups cause sync failures
**Fix:** Add exponential backoff for transient errors

## File Changes

### New Files
- None (all changes to existing files)

### Modified Files
- `src/config.rs` - Add validator.name config
- `src/tui/app.rs` - Add sync progress state, node identity
- `src/tui/ui.rs` - New layout rendering, sync progress bar, key display
- `src/tui/layout.rs` - Enable wide layout, add Node Identity panel
- `src/rpc/client.rs` - Apply timeout configuration
- `src/commands/sync.rs` - Fix timestamp extraction

## Configuration Additions

```toml
[validator]
# Display name for this validator node
# Defaults to system hostname if not specified
name = "midnight-validator-01"

# Show full public keys in TUI (default: false = truncated)
show_full_keys = false
```

## Keyboard Shortcuts (New)

| Key | Action |
|-----|--------|
| k | Toggle key display (truncated/full) |
| n | Toggle node identity panel visibility |

## Testing Plan

1. Test sync progress display with:
   - Fully synced node (should show "Synced")
   - Syncing node (should show progress bar with ETA)
   - Node behind by few blocks (should show percentage)

2. Test node name:
   - With config set (should show config value)
   - Without config (should show hostname)

3. Test key display:
   - Small screen (truncated keys)
   - Large screen (full keys option)
   - Toggle with 'k' key

4. Test wide layout:
   - 80x24 terminal (vertical layout)
   - 200x50 terminal (side-by-side layout)

## Migration Notes

- No database changes required
- Config file additions are optional (backwards compatible)
- Existing installations will work without changes

## Timeline

- [ ] Node sync progress bar
- [ ] Configurable node name
- [ ] Display all public keys
- [ ] Improved dashboard layout
- [ ] Fix block timestamp
- [ ] Enforce RPC timeout
- [ ] Testing
- [ ] Release

## Known Limitations

1. **Sync ETA accuracy:** Based on recent sync rate, may fluctuate
2. **Wide layout:** Only activates on terminals > 160 columns wide
3. **Timestamp fix:** Historical blocks will retain incorrect timestamps; only new blocks will be correct

## Future Considerations (v0.6+)

- Prometheus metrics export
- Alert webhooks for missed blocks
- Block history sparkline from database
- Multi-validator support (monitor multiple nodes)
