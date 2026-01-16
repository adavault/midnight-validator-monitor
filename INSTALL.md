# Installation Guide

## Quick Start

```bash
# Build the binary
cargo build --release

# Install (requires sudo)
sudo ./scripts/install.sh
```

That's it! The installation script will:
- Install to `/opt/midnight/mvm/`
- Create systemd services running as your user
- Create a symlink at `/usr/local/bin/mvm`
- Set up configuration with sensible defaults

## Installation Details

### What Gets Installed

**Location**: `/opt/midnight/mvm/`

```
/opt/midnight/mvm/
├── bin/
│   └── mvm              # Binary
├── config/
│   └── config.toml      # Configuration
└── data/
    └── mvm.db           # Database (created on first run)
```

**Systemd Services**: `/etc/systemd/system/`
- `mvm-sync.service` - Continuous block synchronization daemon
- `mvm-status.service` - One-shot status check
- `mvm-status.timer` - Periodic status checks (every 5 minutes)

**Symlink**: `/usr/local/bin/mvm` → `/opt/midnight/mvm/bin/mvm`

**Ownership**: All files owned by your user (the user running sudo)

**Services Run As**: Your user (not root, no dedicated user needed)

## Starting Services

After installation:

```bash
# Start the sync daemon
sudo systemctl start mvm-sync

# Enable auto-start on boot
sudo systemctl enable mvm-sync

# (Optional) Enable periodic health checks
sudo systemctl enable --now mvm-status.timer
```

## Using MVM

After installation, the `mvm` command is available system-wide:

```bash
# View interactive TUI (no sudo needed!)
mvm view

# Query data
mvm query stats
mvm query validators
mvm query blocks --limit 20

# Check validator keys
mvm keys --keystore /path/to/keystore verify

# View configuration
mvm config show
```

All commands automatically use `/opt/midnight/mvm/data/mvm.db` by default.

## Monitoring

```bash
# View sync logs in real-time
sudo journalctl -u mvm-sync -f

# Check service status
sudo systemctl status mvm-sync

# Check if services are running
systemctl is-active mvm-sync
systemctl is-active mvm-status.timer
```

## Configuration

Edit the configuration file:

```bash
sudo nano /opt/midnight/mvm/config/config.toml
```

Example configuration:
```toml
[rpc]
url = "http://localhost:9944"
metrics_url = "http://localhost:9615/metrics"

[database]
path = "/opt/midnight/mvm/data/mvm.db"

[validator]
keystore_path = "/path/to/keystore"
label = "My Validator"

[sync]
batch_size = 100
poll_interval_secs = 6
```

After changing config, restart the service:
```bash
sudo systemctl restart mvm-sync
```

## Upgrading

To upgrade to a newer version:

```bash
# Stop the service
sudo systemctl stop mvm-sync

# Build new version
cargo build --release

# Reinstall (preserves data and config)
sudo ./scripts/install.sh

# Services restart automatically
```

Your database and configuration are preserved during upgrades.

## Uninstallation

```bash
# Run uninstall script
sudo ./scripts/uninstall.sh
```

The script will:
1. Stop and disable services
2. Remove systemd service files
3. Remove the binary symlink
4. Prompt whether to remove data directory

You can choose to keep your database and configuration for later.

## Permissions

Since everything runs as your user:
- **No permission issues** accessing the database
- **No sudo needed** to run `mvm view` or queries
- **Easy access** to logs and config files

The installation uses sudo only to:
- Write to `/opt/midnight/mvm/`
- Install systemd services
- Create symlink in `/usr/local/bin/`

After installation, daily use doesn't require sudo.

## Troubleshooting

### Service Won't Start

Check the logs:
```bash
sudo journalctl -u mvm-sync -n 50
```

Common issues:
- RPC endpoint not accessible
- Database directory not writable (shouldn't happen with this installation)
- Binary not found (check symlink)

### Can't Connect to Node

Edit the config:
```bash
sudo nano /opt/midnight/mvm/config/config.toml
```

Change the RPC URL:
```toml
[rpc]
url = "http://your-node:9944"
```

Restart:
```bash
sudo systemctl restart mvm-sync
```

### View Command Shows Empty Data

Wait for sync to populate the database:
```bash
# Check sync progress
sudo journalctl -u mvm-sync -f

# Or check the TUI
mvm view
```

The dashboard will show sync progress and block counts.

## Development Workflow

For development, you can run commands directly without installation:

```bash
# Build
cargo build --release

# Run sync
./target/release/mvm sync

# In another terminal, run TUI
./target/release/mvm view --db-path ./mvm.db
```

This creates a local `./mvm.db` file for testing.

## Production Deployment

Recommended production setup:

1. **Install MVM**
   ```bash
   sudo ./scripts/install.sh
   ```

2. **Configure keystore** (if you have one)
   ```bash
   sudo nano /opt/midnight/mvm/config/config.toml
   # Add: keystore_path = "/path/to/keystore"
   ```

3. **Start services**
   ```bash
   sudo systemctl enable --now mvm-sync
   sudo systemctl enable --now mvm-status.timer
   ```

4. **Verify**
   ```bash
   # Check logs
   sudo journalctl -u mvm-sync -f

   # View TUI
   mvm view

   # Check validators (if you added keystore)
   mvm keys verify
   ```

5. **Monitor**
   - Use `mvm view` for interactive monitoring
   - Use `journalctl -u mvm-sync` for logs
   - Use `mvm query` commands for data analysis

## Backup

To backup your database:

```bash
# Stop sync first
sudo systemctl stop mvm-sync

# Backup
sudo cp /opt/midnight/mvm/data/mvm.db /path/to/backup/mvm.db.$(date +%Y%m%d)

# Restart
sudo systemctl start mvm-sync
```

Or use SQLite's online backup (without stopping):
```bash
sqlite3 /opt/midnight/mvm/data/mvm.db ".backup /path/to/backup/mvm.db.$(date +%Y%m%d)"
```

## Migration from Old Installation

If you had an old installation in a different location:

1. **Stop old services**
   ```bash
   sudo systemctl stop old-mvm-service
   ```

2. **Install new version**
   ```bash
   sudo ./scripts/install.sh
   ```

3. **Copy old database** (if you want to keep history)
   ```bash
   sudo systemctl stop mvm-sync
   sudo cp /old/path/mvm.db /opt/midnight/mvm/data/mvm.db
   sudo chown $USER:$USER /opt/midnight/mvm/data/mvm.db
   sudo systemctl start mvm-sync
   ```

## Next Steps

After installation:
- See **README.md** for feature overview
- See **DEPLOYMENT.md** for advanced deployment options
- See **CLAUDE.md** for architecture and development guide
- Run `mvm --help` for command reference
- Run `mvm view` and press `?` for TUI help
