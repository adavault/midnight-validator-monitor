# Deployment Guide

## Installation

### Quick Install

```bash
# Build release binary
cargo build --release

# Install (requires sudo)
sudo ./scripts/install.sh
```

The installation script will:
- Install to `/opt/midnight/mvm/`
- Create systemd services running as your user
- Create symlink at `/usr/local/bin/mvm`
- Set ownership to your user
- Configure automatic restarts

### Installation Structure

After installation:

| Resource | Path |
|----------|------|
| Binary | `/opt/midnight/mvm/bin/mvm` |
| Symlink | `/usr/local/bin/mvm` |
| Database | `/opt/midnight/mvm/data/mvm.db` |
| Config | `/opt/midnight/mvm/config/config.toml` |
| PID file | `/opt/midnight/mvm/data/mvm-sync.pid` |
| Systemd services | `/etc/systemd/system/mvm-*.service` |

**Key Points**:
- Everything owned by your user
- Services run as your user (not root)
- No dedicated service account needed
- No permission issues

### Configuration

Edit the config file at `/opt/midnight/mvm/config/config.toml`:

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
keystore_path = "/path/to/your/keystore"
label = "My Validator"

[sync]
batch_size = 100
poll_interval_secs = 6

[daemon]
pid_file = "/opt/midnight/mvm/data/mvm-sync.pid"
```

### Starting Services

```bash
# Start sync daemon (runs continuously)
sudo systemctl start mvm-sync
sudo systemctl enable mvm-sync

# Enable periodic health checks (every 5 minutes)
sudo systemctl start mvm-status.timer
sudo systemctl enable mvm-status.timer
```

### Monitoring

```bash
# Check service status
sudo systemctl status mvm-sync
sudo systemctl status mvm-status.timer

# View logs
sudo journalctl -u mvm-sync -f
sudo journalctl -u mvm-status

# Interactive TUI (uses config file by default)
mvm view

# The config file at /opt/midnight/mvm/config/config.toml
# already has the correct database path, so no extra flags needed!
```

### Managing the Daemon

```bash
# Stop services
sudo systemctl stop mvm-sync
sudo systemctl stop mvm-status.timer

# Restart after updates
sudo systemctl restart mvm-sync

# Check daemon health
sudo systemctl is-active mvm-sync

# Tail logs in real-time
sudo journalctl -u mvm-sync -f --since "5 minutes ago"
```

## Local Development

For local development without system installation:

```bash
# Build
cargo build --release

# Run sync locally (stores to ./mvm.db)
./target/release/mvm sync

# View with local database
./target/release/mvm view --db-path ./mvm.db

# Or set environment
export MVM_DB_PATH=./mvm.db
./target/release/mvm view
```

## Database Path Resolution

The `mvm` commands use the following priority for database path:

1. **CLI flag**: `--db-path /path/to/db` (highest priority)
2. **Environment variable**: `MVM_DB_PATH=/path/to/db`
3. **Config file**: First found in search path (see below)
4. **Default**: `./mvm.db` (current directory)

### Configuration File Search Path

MVM searches for config files in this order:
1. `./mvm.toml` (current directory)
2. `~/.config/mvm/config.toml` (user config)
3. `/opt/midnight/mvm/config/config.toml` (system install - created by install.sh)
4. `/etc/mvm/config.toml` (legacy location)

### Common Scenarios

**System installation (recommended):**
```bash
# After running ./scripts/install.sh, the config file is automatically created
# at /opt/midnight/mvm/config/config.toml with correct paths
mvm view  # Just works! Uses config from /opt/midnight/mvm/config/config.toml
```

**Override for specific command:**
```bash
# Use a different database temporarily
mvm view --db-path /path/to/other.db

# Or set environment variable for session
export MVM_DB_PATH=/path/to/other.db
mvm view
```

**Local development:**
```bash
# Everything in current directory
mvm sync  # Creates ./mvm.db
mvm view  # Uses ./mvm.db by default
```

## Permissions

### Database Access

The database at `/opt/midnight/mvm/data/mvm.db` is owned by the user who ran the installation (your user). All MVM commands run as your user, so no special permissions are needed:

```bash
# Just run the command - you already have access
mvm view
mvm query stats
mvm keys verify --keystore /path/to/keystore
```

### Keystore Access

If your keystore is in a protected location (e.g., owned by another user or requiring specific permissions):

```bash
# Ensure your user can read the keystore
sudo chmod 750 /path/to/keystore
sudo chown -R $USER:$USER /path/to/keystore

# Or use ACLs for more fine-grained control
sudo setfacl -R -m u:$USER:rx /path/to/keystore
```

## Uninstall

```bash
sudo ./scripts/uninstall.sh
```

This will:
- Stop and disable systemd services
- Remove service files
- Remove binary
- Optionally remove data directory (prompts for confirmation)

## Troubleshooting

### Database Permission Errors

**Error**: "Failed to open database... permission denied"

**Solutions**:
1. Use correct path: System installation → `/opt/midnight/mvm/data/mvm.db`
2. Check database file ownership: `ls -la /opt/midnight/mvm/data/`
3. Ensure directory exists: `sudo mkdir -p /opt/midnight/mvm/data && sudo chown $USER:$USER /opt/midnight/mvm/data`

### Terminal Corruption

**Error**: Terminal appears broken after error in `mvm view`

**Fix**:
```bash
reset
# Or
stty sane
```

The view command now properly restores terminal state on errors, but if you encounter this, these commands will fix it.

### Service Won't Start

Check the logs:
```bash
sudo journalctl -u mvm-sync -n 50
```

Common issues:
- Database path doesn't exist → Create directory or update config
- RPC endpoint unreachable → Check `[rpc]` url in config
- Keystore not readable → Check permissions

### View Command Shows Empty Data

1. Check database has data: `sqlite3 /opt/midnight/mvm/data/mvm.db "SELECT COUNT(*) FROM blocks;"`
2. Verify sync daemon is running: `sudo systemctl status mvm-sync`
3. Check sync logs: `sudo journalctl -u mvm-sync -f`

## Security Considerations

1. **User Permissions**: Services run as your user (not root) with minimal privileges
2. **File Permissions**: Data directory at `/opt/midnight/mvm/data/` is owned by your user
3. **RPC Access**: Configure firewall if RPC endpoint is network-accessible
4. **Database**: Contains public blockchain data, but limit write access to trusted users
5. **Keystore Security**: Ensure keystore files have appropriate permissions (750 or more restrictive)

## Backup

To backup your database:

```bash
# Stop sync first
sudo systemctl stop mvm-sync

# Backup database
sudo cp /opt/midnight/mvm/data/mvm.db /path/to/backup/mvm.db.$(date +%Y%m%d)

# Restart sync
sudo systemctl start mvm-sync
```

Or use SQLite backup while running:
```bash
sqlite3 /opt/midnight/mvm/data/mvm.db ".backup /path/to/backup/mvm.db.$(date +%Y%m%d)"
```

## Upgrading

```bash
# Stop services
sudo systemctl stop mvm-sync
sudo systemctl stop mvm-status.timer

# Build new version
cargo build --release

# Install new binary
sudo cp target/release/mvm /opt/midnight/mvm/bin/mvm

# Or run the install script (recommended - updates systemd files too)
sudo ./scripts/install.sh

# Restart services
sudo systemctl start mvm-sync
sudo systemctl start mvm-status.timer

# Verify
sudo systemctl status mvm-sync
mvm --version
```

No database migrations required - v0.3.0 is compatible with v0.2.0 databases.
