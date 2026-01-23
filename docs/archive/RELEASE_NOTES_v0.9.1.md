# Release Notes: v0.9.1

**Release Date:** January 2026

## Overview

Version 0.9.1 adds a self-installing binary and includes bug fixes.

## New Feature: Self-Installing Binary

Users can now download just the `mvm` binary and run `sudo mvm install` to set up everything:

```bash
# Download from GitHub releases
gh release download --pattern mvm -R adavault/midnight-validator-monitor

# Install (one command does everything)
chmod +x mvm
sudo ./mvm install
```

### What `mvm install` Does

1. **Creates directories**
   - `/opt/midnight/mvm/bin` - Binary location
   - `/opt/midnight/mvm/data` - Database and PID files
   - `/opt/midnight/mvm/config` - Configuration

2. **Installs binary**
   - Copies itself to `/opt/midnight/mvm/bin/mvm`
   - Creates symlink at `/usr/local/bin/mvm`

3. **Creates systemd services**
   - `mvm-sync.service` - Continuous block sync daemon
   - `mvm-status.service` - One-shot health check
   - `mvm-status.timer` - Periodic health check (every 5 min)

4. **Generates default configuration**
   - `/opt/midnight/mvm/config/config.toml`

### Uninstall

```bash
# Remove services and symlink (keeps data)
sudo mvm install uninstall

# Remove everything including database
sudo mvm install uninstall --remove-data
```

## Bug Fixes

- **Fixed Performance view popup showing wrong validator** - The popup was using unsorted validator list while the display was sorted by total blocks
- Refactored Validators view sorting to use shared helper function

## Upgrade from v0.9.0

```bash
# Download new binary
gh release download v0.9.1 --pattern mvm --clobber -R adavault/midnight-validator-monitor

# Reinstall (stops services, updates binary, restarts)
sudo systemctl stop mvm-sync
chmod +x mvm
sudo ./mvm install
sudo systemctl start mvm-sync
```

## Files Changed

- `src/commands/install.rs` - New install/uninstall command (350 lines)
- `src/commands/mod.rs` - Export InstallArgs
- `src/main.rs` - Add Install command variant
- `Cargo.toml` - Version bump, add libc dependency
- `README.md` - Updated installation instructions
