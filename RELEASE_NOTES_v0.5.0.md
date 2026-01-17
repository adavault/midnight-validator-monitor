# Release Notes - v0.5.0

## Overview

v0.5.0 focuses on TUI improvements and enhanced monitoring visibility. This release adds critical features for understanding node sync status, validator identity, and improved layouts for wide terminals.

## Major Features

### 1. Node Sync Progress Display

The TUI now shows real-time node synchronization status:

**Synced Node:**
```
Node Sync:     ✓ Synced      Node: midnight-validator-01
```

**Syncing Node:**
```
Node Sync:     ━━━━━━━━━━━━░░░░░░░░ 65.3%  (1,234,567 blocks behind)
```

The sync progress bar provides visual feedback when your node is catching up to the network.

### 2. Configurable Node Name

Validator nodes can now display a custom name:

**Configuration (mvm.toml):**
```toml
[validator]
name = "midnight-validator-01"  # Optional, defaults to hostname
```

The node name appears in the dashboard, making it easy to identify which validator you're monitoring.

### 3. All Public Keys Display

The "Our Validators" panel now shows all three public keys for each validator:

```
★ Sidechain: 0x037764d2d83c269030fef6df5aeb4419c48762ada2cf20...809f4700
  AURA:      0xe05be3c28c72864efc49f4f12cb04f3bd6f20fdbc29750...273b3e1e
  Grandpa:   0xf5a39df9227f630754f78bbae43bd66a693612eeffa9ce...5f48d0e8
```

This makes it easier to verify your validator's identity across different key types.

### 4. Wide Screen Layout

For terminals wider than 160 columns, the dashboard now uses a side-by-side layout:

```
┌─ Network Status ─────────────┬─ Our Validators ──────────────────────────┐
│ ● Health: OK    Peers: 12    │ Committee: ✓ Elected (1 seat / 1200)      │
│ Block: #3372650 (finalized)  │ Count: 1   Total: 23 blocks   Share: 0.5% │
│ Node Sync: ✓ Synced          │ This Epoch: 0 blocks (expected: ~1.2)     │
│ Mainchain: e1180  Sidechain: │ ★ SC: 0x037764d2...809f4700               │
│ Epoch: ━━━━━━━━━━━━━━ 42.5%  │   AU: 0xe05be3c2...273b3e1e               │
├─ Recent Blocks ──────────────┴───────────────────────────────────────────┤
│ #3372650  slot 294893245  epoch 1180  ✓  author: 0x0203ae55...cdebc5     │
│ ...                                                                       │
└──────────────────────────────────────────────────────────────────────────┘
```

Smaller terminals continue to use the vertical stacked layout.

### 5. RPC Timeout Configuration

The `timeout_ms` configuration is now properly applied to all RPC requests:

```toml
[rpc]
url = "http://localhost:9944"
timeout_ms = 30000  # 30 second timeout (now enforced)
```

## Bug Fixes

- **Version display**: Title bar now shows correct v0.5.0 version
- **RPC timeout**: Configuration setting is now actually applied to HTTP client
- **Layout constraints**: Improved panel sizing for validator key display

## Configuration

### New Configuration Option

```toml
[validator]
# Display name for this validator node
# Defaults to system hostname if not specified
name = "midnight-validator-01"
```

### Example Full Configuration

```toml
[rpc]
url = "http://localhost:9944"
metrics_url = "http://localhost:9615/metrics"
timeout_ms = 30000

[database]
path = "/opt/midnight/mvm/data/mvm.db"

[validator]
keystore_path = "/var/lib/midnight/chains/testnet/keystore"
name = "my-validator"  # NEW in v0.5.0

[sync]
batch_size = 100
poll_interval_secs = 6

[view]
refresh_interval_ms = 6000
```

## Upgrade Instructions

1. **Stop the sync daemon** (if running):
   ```bash
   sudo systemctl stop mvm-sync
   ```

2. **Install new binary**:
   ```bash
   sudo cp target/release/mvm /opt/midnight/mvm/bin/mvm
   ```

3. **Optionally add node name** to config:
   ```bash
   sudo nano /opt/midnight/mvm/config/config.toml
   # Add under [validator]:
   # name = "my-validator"
   ```

4. **Restart services**:
   ```bash
   sudo systemctl start mvm-sync
   ```

## Known Limitations

1. **Block timestamps**: Stored blocks use sync time rather than actual block timestamp (planned for v0.6)
2. **Wide layout threshold**: Only activates on terminals wider than 160 columns
3. **Sync progress ETA**: Not yet implemented (shows percentage and blocks remaining)

## Files Changed

- `src/config.rs` - Added validator.name config option
- `src/tui/app.rs` - Added SyncProgress struct, node_name field
- `src/tui/ui.rs` - New sync progress bar, all keys display, wide layout rendering
- `src/tui/layout.rs` - Updated constraints for new panel sizes
- `src/rpc/client.rs` - Implemented timeout configuration
- `src/commands/view.rs` - Pass timeout to RPC client
- `src/commands/sync.rs` - Pass timeout to RPC client
- `src/commands/status.rs` - Pass timeout to RPC client
- `src/commands/keys.rs` - Pass timeout to RPC client
- `Cargo.toml` - Version bump to 0.5.0, added hostname dependency

## Testing

All 33 unit tests pass. Manual testing recommended for:

1. Sync progress display (on syncing vs synced node)
2. Node name display (with/without config)
3. Wide terminal layout (resize terminal to >160 columns)
4. Key display in different screen sizes

## Next Release (v0.6 Preview)

- Proper block timestamp extraction
- RPC retry logic with exponential backoff
- Prometheus metrics export
- Alert webhooks for missed blocks
