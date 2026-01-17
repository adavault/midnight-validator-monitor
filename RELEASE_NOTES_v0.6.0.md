# Release Notes: v0.6.0

**Release Date:** January 2026

## Overview

Version 0.6.0 focuses on TUI enhancements with a new Peers view, improved epoch tracking, and better network visibility. The dashboard now displays comprehensive node and network information including external IP, peer ID, and proper sidechain epoch tracking for block production metrics.

## New Features

### Peers View (Key 5)
- New dedicated view showing all connected peers
- Displays peer ID, best block number, and sync status indicator
- Shows external IP:port for each peer (IPv4 and IPv6 support)
- Peers sorted by sync status (most synced first)
- Scrollable list with keyboard navigation

### Enhanced Dashboard
- **External IP & Peer ID**: Shows your node's external IP address and abbreviated peer ID
- **Node Version**: Displays the Midnight node version in the "Our Validator" panel
- **Grandpa Voter Status**: Shows whether your node is participating in block finalization
- **Separate Epoch Progress Bars**:
  - Sidechain epoch (2-hour cycle) - determines committee election
  - Mainchain epoch (24-hour Cardano cycle)
- **Accurate "This Epoch" Tracking**: Block count now correctly reflects the current sidechain epoch using timestamp-based queries

### Scrollable Help Screen
- Help screen now supports scrolling for smaller terminal sizes
- Added comprehensive field explanations
- Documents all keyboard shortcuts and view descriptions

## Improvements

### UI Polish
- Progress bars now properly align between sidechain and mainchain rows
- Improved label clarity: "Synced" → "Connected", "Sync:" → "Node:"
- Better spacing and alignment throughout all views
- Consistent column widths in help screen

### Code Quality
- Applied clippy suggestions throughout codebase
- Replaced `.min().max()` patterns with `.clamp()` for clarity
- Removed unused imports and dead code
- Improved type annotations

### Database
- New `count_blocks_by_author_since()` function for timestamp-based queries
- Enables accurate sidechain epoch block counting

## Technical Details

### New RPC Methods Used
- `system_peers` - Connected peers with sync status
- `system_unstable_networkState` - External IPs and local peer ID (requires `--rpc-methods=unsafe`)

### Epoch Handling
- **Sidechain epochs**: 2 hours - committee rotation, block production eligibility
- **Mainchain epochs**: 24 hours - Cardano-style epochs
- Block counting now uses timestamp-based queries matching the 2-hour sidechain epoch cycle

### Keyboard Navigation
- `1` - Dashboard
- `2` - Blocks
- `3` - Validators
- `4` - Performance
- `5` - Peers (NEW)
- `?/h/F1` - Help (scrollable)
- `j/k` or `↑/↓` - Scroll
- `f` - Filter (ours only)
- `t` - Toggle theme
- `q/Esc` - Quit

## Upgrade Notes

This release is backwards compatible. Simply rebuild and restart:

```bash
cargo build --release
sudo systemctl restart mvm-sync
```

No database migrations required.

## Known Limitations

- External IP display requires `--rpc-methods=unsafe` on the node
- Some peers may show blank IP addresses (typically those using non-standard transport)
- Committee election status updates every data refresh cycle (configurable interval)

## Contributors

Built with Claude Code (Anthropic)
