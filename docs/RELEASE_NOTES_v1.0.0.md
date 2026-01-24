# Release Notes: v1.0.0

**Release Date:** January 2026

## First Production Release

MVM v1.0.0 is the first production-ready release of the Midnight Validator Monitor. After extensive development through the v0.x series, this release marks MVM as stable and ready for PreProd/Mainnet validators.

## What is MVM?

Midnight Validator Monitor is a CLI tool for Midnight blockchain node operators. It provides:

- **Real-time node monitoring** - Health checks, sync status, peer connectivity
- **Block production tracking** - Per-validator attribution with historical data
- **Interactive TUI** - Dashboard, block explorer, validator list, performance metrics
- **Validator key management** - Session key verification and registration status
- **Daemon mode** - Background sync service with systemd integration

## Highlights

### Stability Verified

- 24+ hour daemon stability test passed
- No memory leaks or crashes under continuous operation
- Accurate block attribution across epoch boundaries

### Production Ready

- All known issues resolved or triaged for post-v1.0
- Comprehensive error handling and recovery
- Configuration validation with helpful error messages

### CI/CD Pipeline

- Automated builds and tests on push
- GitHub Releases with pre-built binaries on tag
- Quality gates: `cargo test`, `cargo fmt`, `cargo clippy`

## Changes Since v0.9.3

### Improvements

- **Config error messages** (Issue #16): When no configuration file is found, MVM now displays searched paths and helpful suggestions:
  ```
  No configuration file found.

  Searched locations:
    - ./mvm.toml
    - ~/.config/mvm/config.toml
    - /opt/midnight/mvm/config/config.toml

  To see search paths: mvm config paths
  To create config:    mvm config example > ./mvm.toml
  ```

### Infrastructure

- Added CI workflow for automated testing
- Added release workflow for binary builds
- Updated gitignore and fixed clippy warnings

## Feature Summary (v0.7 - v1.0)

For users upgrading from earlier versions, here's what's been added:

| Version | Key Features |
|---------|--------------|
| v0.7 | Sparklines, shell completions, UX polish |
| v0.8 | Help glossary, Prometheus metrics, resource monitoring |
| v0.9 | Drill-down popups, dashboard reorganization, validator sorting |
| v0.9.1-3 | Bug fixes, epoch alignment, performance tuning |
| v1.0 | Stability verification, improved error messages, CI/CD |

## System Requirements

- **OS:** Linux (x86_64)
- **Midnight Node:** v0.12.0 or later
- **Disk:** ~100MB for database (grows with chain history)
- **Memory:** ~50MB runtime

## Installation

### From Binary (Recommended)

```bash
# Download from GitHub Releases
tar xzf mvm-v1.0.0-linux-x86_64.tar.gz
sudo mv mvm /usr/local/bin/

# Or use the self-installer
sudo mvm install
```

### From Source

```bash
git clone https://github.com/adavault/midnight-validator-monitor.git
cd midnight-validator-monitor
cargo build --release
sudo cp target/release/mvm /usr/local/bin/
```

## Quick Start

```bash
# Check node status
mvm status --once

# Start block sync daemon
mvm sync --daemon --db-path /opt/midnight/mvm/data/mvm.db

# Launch interactive TUI
mvm view --db-path /opt/midnight/mvm/data/mvm.db

# View configuration
mvm config show
```

## Upgrade Notes

This is a drop-in replacement for any v0.9.x release. No database migration required.

If upgrading from v0.8.x or earlier, the database schema will auto-migrate on first run.

### Systemd Services

If using systemd, restart services after upgrade:
```bash
sudo systemctl restart mvm-sync
```

## Known Limitations

- Historical state queries require archive node (non-archive nodes keep ~256 blocks of state)
- Some RPC methods require `--rpc-methods=unsafe` on the Midnight node
- Currently Linux x86_64 only (macOS/ARM64 builds planned for v1.1)

## What's Next

See [ROADMAP.md](docs/ROADMAP.md) for the long-term vision.

**v1.1 (planned):**
- File permissions documentation (Issue #17)
- Troubleshooting guide (Issue #18)
- macOS/ARM64 binary releases

**v2.0 (future):**
- Extrinsic decoder for human-readable transactions
- Container images with sidecar deployment support
- Enhanced developer debugging tools

## Acknowledgments

Thanks to the Midnight Discord community for feedback and testing throughout the v0.x development cycle.

---

*Full changelog: v0.9.3...v1.0.0*
