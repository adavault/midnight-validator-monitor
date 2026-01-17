# Quick Start Guide

Get MVM running in 5 minutes.

## Prerequisites

- Rust 1.70+ (`rustup update stable`)
- A running Midnight node with RPC enabled (default: `http://localhost:9944`)

## Install

```bash
# Clone and build
git clone https://github.com/user/midnight-validator-monitor.git
cd midnight-validator-monitor
cargo build --release

# Copy binary to your PATH
sudo cp target/release/mvm /usr/local/bin/
```

## Basic Usage

### 1. Check Node Status

```bash
mvm status --once
```

If successful, you'll see your node's health, sync status, and peer count.

### 2. Start Syncing Blocks

```bash
mvm sync --db-path ./mvm.db
```

This creates a local database and starts syncing block data from your node. Keep this running (or use `--daemon` mode) to continuously sync.

### 3. Launch the TUI

In another terminal:

```bash
mvm view --db-path ./mvm.db
```

You'll see the interactive dashboard with:
- Network status and epoch progress
- Your validator info (if configured)
- Recent blocks and their authors

### TUI Controls

- `1-4` - Switch views (Dashboard, Blocks, Validators, Performance)
- `j/k` - Scroll up/down
- `t` - Toggle theme
- `q` - Quit

## Configure Your Validator

To track your validator's block production:

```bash
# Point to your keystore directory
mvm keys --keystore /path/to/your/keystore verify --db-path ./mvm.db
```

This marks your validator in the database so the TUI shows your blocks and performance.

## Production Setup

For 24/7 monitoring, use the systemd integration:

```bash
# Install as system service
sudo ./scripts/install.sh

# Start the sync daemon
sudo systemctl start mvm-sync
sudo systemctl enable mvm-sync
```

Then run `mvm view` anytime to see your validator status.

## Configuration File

Create `~/.config/mvm/config.toml`:

```toml
[rpc]
url = "http://localhost:9944"

[database]
path = "/opt/midnight/mvm/data/mvm.db"

[validator]
keystore_path = "/path/to/your/keystore"
name = "my-validator"
```

## Troubleshooting

**Can't connect to node:**
- Make sure your node is running and RPC is enabled
- Check the RPC URL (default port is 9944)
- Try: `curl http://localhost:9944 -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}'`

**Database not found:**
- Run `mvm sync` first to create the database
- Or specify the path: `mvm view --db-path /path/to/mvm.db`

**Keys not verified:**
- The node needs `--rpc-methods=unsafe` for key verification
- Registration checks work without this flag

## Next Steps

- Read `README.md` for full command reference
- Check `DEPLOYMENT.md` for detailed production setup
- Use `mvm config example` to see all configuration options
