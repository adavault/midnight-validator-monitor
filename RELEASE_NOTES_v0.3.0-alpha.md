# Release Notes: v0.3.0-alpha

**Release Date**: 2026-01-16
**Status**: Alpha Release

## Overview

v0.3.0-alpha transforms MVM from a CLI tool into a production-ready monitoring solution with daemon support, an interactive TUI, and comprehensive configuration management. This release is ready for production testing.

## Major New Features

### 1. Interactive TUI Dashboard 🎨

A full-featured terminal UI for real-time monitoring with 5 views:

- **Dashboard**: Network overview, our validators, recent blocks
- **Blocks**: Scrollable block list with author attribution
- **Validators**: Complete validator list with performance stats
- **Performance**: Top validators ranked by block production
- **Help**: Keyboard shortcuts reference

**Usage:**
```bash
mvm view
```

**Keyboard Controls:**
- `1-4` - Switch views
- `j/k` - Scroll (Vim-style)
- `f` - Toggle "ours only" filter
- `r` - Force refresh
- `q/Esc` - Quit

### 2. Systemd Daemon Support 🔧

Continuous block synchronization with systemd integration:

- **Graceful shutdown** on SIGTERM/SIGINT/SIGQUIT
- **PID file management** with automatic cleanup
- **Auto-restart** on failure
- **Journal logging** for easy monitoring
- **Installation scripts** for one-command setup

**Installation:**
```bash
sudo ./scripts/install.sh
sudo systemctl enable --now mvm-sync
```

**Service Files:**
- `mvm-sync.service` - Continuous sync daemon
- `mvm-status.service` - One-shot health check
- `mvm-status.timer` - Periodic health checks (5 min)

### 3. Configuration File System ⚙️

TOML-based configuration with multi-source priority:

**Priority Order:**
1. CLI flags (highest)
2. Environment variables (`MVM_*`)
3. Config file (first found in search path)
4. Defaults (lowest)

**Search Path:**
1. `./mvm.toml` (current directory)
2. `~/.config/mvm/config.toml` (user config)
3. `/opt/midnight/mvm/config/config.toml` (system install)
4. `/etc/mvm/config.toml` (legacy)

**New Command:**
```bash
mvm config show      # Show effective configuration
mvm config validate  # Validate config file
mvm config example   # Generate example config
mvm config paths     # Show search paths
```

### 4. Enhanced Validator Tracking 📊

The `keys verify` command now provides comprehensive validator information:

- ✅ Key loading verification
- ✅ Registration status checking
- ✅ Automatic "ours" marking in database
- ✅ Block production statistics
- ✅ Performance ranking
- ✅ Recent block history

**Example Output:**
```
Block Production Statistics:
  Total blocks produced: 17
  First seen in epoch:   1179
  Share of synced blocks: 0.56%
  Performance rank:       #1 of 185 validators

  Recent blocks (last 1000):
    Block #3363612 (slot 294763613, epoch 1179)
    Block #3363792 (slot 294763798, epoch 1179)
    Block #3363965 (slot 294763983, epoch 1179)
```

## Critical Bug Fixes

### Configuration System Not Working

**Fixed**: All commands had hardcoded defaults in clap arguments, completely bypassing the config file system.

**Impact**: Configuration files, environment variables, and the entire config priority system were non-functional in earlier builds.

**Resolution**:
- Changed all CLI arguments to `Option<T>` types
- Implemented proper fallback pattern: `args.field.unwrap_or(config.field)`
- Added config file path to search locations
- Added logging to show config loading and resolution

This was a showstopper bug that would have made v0.3.0 unusable. The config system now works correctly across all commands.

## New Commands

### view - Interactive TUI
```bash
mvm view [--rpc-url <URL>] [--db-path <PATH>] [--refresh-interval <MS>]
```

### config - Configuration Management
```bash
mvm config <show|validate|example|paths>
```

### query enhancements
New subcommands:
- `validators [--ours] [--limit N]` - List validators with stats
- `validator <KEY>` - Show specific validator details
- `performance [--ours] [--limit N]` - Performance rankings

## Installation Changes

### New Installation Method (Recommended)

```bash
# Build and install
cargo build --release
sudo ./scripts/install.sh
```

**Installs to:**
- Binary: `/opt/midnight/mvm/bin/mvm`
- Symlink: `/usr/local/bin/mvm`
- Database: `/opt/midnight/mvm/data/mvm.db`
- Config: `/opt/midnight/mvm/config/config.toml`
- PID file: `/opt/midnight/mvm/data/mvm-sync.pid`

**Permissions:**
- All files owned by your user (not root)
- Services run as your user
- No dedicated service account needed

### Upgrading from v0.2.0-alpha

```bash
# Stop existing sync if running
sudo systemctl stop mvm-sync 2>/dev/null || true

# Build new version
cargo build --release

# Install (updates binary and systemd files)
sudo ./scripts/install.sh

# Start services
sudo systemctl start mvm-sync
sudo systemctl enable mvm-sync

# Verify
mvm --version
sudo systemctl status mvm-sync
```

**Database Migration:** None required - v0.3.0 is fully compatible with v0.2.0 databases.

## Breaking Changes

**None**. v0.3.0-alpha is fully backward compatible with v0.2.0-alpha.

However, note that:
- Default paths now come from config files
- Install location changed to `/opt/midnight/mvm/`
- Services run as your user instead of a dedicated `mvm` user

## Dependencies Added

```toml
# Signal handling
signal-hook = "0.3"
signal-hook-tokio = { version = "0.3", features = ["futures-v0_3"] }

# PID file management
nix = { version = "0.27", features = ["fs", "signal", "process"] }

# TUI
ratatui = "0.26"
crossterm = "0.27"

# Configuration
toml = "0.8"
directories = "5.0"
```

## Files Added

### Source Code
- `src/daemon.rs` - PID file management
- `src/config.rs` - Configuration system
- `src/commands/view.rs` - TUI command
- `src/commands/config.rs` - Config management
- `src/tui/` - TUI module (app.rs, event.rs, ui.rs)

### Scripts and Services
- `scripts/install.sh` - Installation automation
- `scripts/uninstall.sh` - Clean uninstallation
- `systemd/mvm-sync.service` - Sync daemon
- `systemd/mvm-status.service` - Health check
- `systemd/mvm-status.timer` - Periodic timer

### Documentation
- `DEPLOYMENT.md` - Deployment guide
- `mvm.toml.example` - Example configuration
- Updated `CLAUDE.md`, `README.md`

## Known Issues

**None critical.**

Minor warnings for unused utility functions (reserved for future features).

## Testing Notes

### Tested ✅
- Configuration loading from all sources
- Environment variable overrides
- Systemd service installation
- Daemon signal handling
- TUI rendering and keyboard navigation
- Validator tracking and "ours" marking
- Block production statistics
- Config file search paths
- Database compatibility with v0.2.0

### Pending Production Testing ⏳
- Multi-day daemon stability
- TUI performance with large datasets (10,000+ blocks)
- Memory usage over extended periods
- Performance under high load

## Documentation

- `README.md` - Updated for v0.3.0 with new features
- `DEPLOYMENT.md` - Complete systemd deployment guide
- `CLAUDE.md` - Technical architecture documentation
- `CHECKPOINT_v0.3.0.md` - Development checkpoint with all implementation details

## Recommendations

**For Production Use:**
1. Use systemd for process management
2. Configure log rotation (systemd journal handles this automatically)
3. Set up monitoring alerts for service failures
4. Use the configuration file for consistency
5. Run services as your user account
6. Test the TUI in a production environment before relying on it

**For Upgrades:**
1. Stop existing sync processes before upgrading
2. Run `./scripts/install.sh` to update both binary and systemd files
3. Verify configuration file is correct at `/opt/midnight/mvm/config/config.toml`
4. Test with `mvm config show` before starting services

## Next Steps

### For v0.3.0 Final (Post-Alpha)
- Extended production testing (7+ days)
- Performance tuning based on real-world usage
- User feedback incorporation
- Memory optimization
- Enhanced logging options

### Future Features (v0.4.0+)
- Alert webhooks and notifications
- WebSocket real-time updates
- Historical performance trends
- Session key rotation detection
- CSV/JSON export
- Prometheus metrics endpoint
- Docker container
- Kubernetes manifests

## Credits

Developed and tested on Midnight blockchain partner chains network.

---

**Full Changelog**: v0.2.0-alpha...v0.3.0-alpha
**Installation**: `sudo ./scripts/install.sh`
**Documentation**: See README.md and DEPLOYMENT.md
