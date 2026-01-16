# Midnight Validator Monitor (MVM)

A Rust CLI tool for monitoring and managing Midnight blockchain validator nodes.

## Features

- **Status Monitoring**: Node health, sync status, peer count, block production
- **Block Sync**: Synchronize blocks to local SQLite database with real-time polling
- **Data Queries**: Query synced blocks, statistics, and detect gaps
- **Key Management**: Verify keystore loading and registration status

## Installation

Requires Rust 1.70+

```bash
cargo build --release
```

## Commands

### status - Monitor validator node

Display current validator node status with health checks and key verification.

```bash
# Basic monitoring (runs every 60s)
mvm status --keystore /path/to/keystore

# Single check
mvm status --once --keystore /path/to/keystore

# Custom endpoints
mvm status \
  --rpc-url http://localhost:9944 \
  --metrics-url http://localhost:9615/metrics \
  --keystore /path/to/keystore \
  --interval 30
```

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--rpc-url` | `-r` | Node RPC endpoint | `http://localhost:9944` |
| `--metrics-url` | `-M` | Prometheus metrics endpoint | `http://localhost:9615/metrics` |
| `--keystore` | `-K` | Path to Substrate keystore directory | - |
| `--keys-file` | `-k` | Path to keys JSON file (alternative) | - |
| `--interval` | `-i` | Monitoring interval in seconds | `60` |
| `--once` | - | Run once and exit | `false` |

### sync - Synchronize blocks to database

Fetch blocks from the node and store in a local SQLite database.

```bash
# Sync blocks to database
mvm sync --db-path ./mvm.db

# Sync from specific block
mvm sync --db-path ./mvm.db --start-block 1000000

# Custom batch size and poll interval
mvm sync --db-path ./mvm.db --batch-size 50 --poll-interval 10

# Only sync finalized blocks
mvm sync --db-path ./mvm.db --finalized-only
```

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--rpc-url` | `-r` | Node RPC endpoint | `http://localhost:9944` |
| `--db-path` | `-d` | SQLite database path | `./mvm.db` |
| `--start-block` | `-s` | Block number to start from | auto |
| `--batch-size` | `-b` | Blocks per batch | `100` |
| `--finalized-only` | - | Only sync finalized blocks | `false` |
| `--poll-interval` | - | Seconds between new block checks | `6` |

### query - Query stored block data

Query the synced database for statistics, blocks, and gaps.

```bash
# Show database statistics
mvm query --db-path ./mvm.db stats

# List recent blocks
mvm query --db-path ./mvm.db blocks --limit 20

# List blocks in range
mvm query --db-path ./mvm.db blocks --from 1000000 --to 1000100

# Find gaps in synced data
mvm query --db-path ./mvm.db gaps
```

| Subcommand | Description |
|------------|-------------|
| `stats` | Show database statistics (total blocks, finalized, gaps) |
| `blocks` | List blocks with slot, epoch, extrinsics count |
| `gaps` | Find missing blocks in the synced range |

### keys - Verify session keys

Display and verify validator session keys from the keystore.

```bash
# Show keys from keystore
mvm keys --keystore /path/to/keystore show

# Verify keys are loaded and registered
mvm keys --keystore /path/to/keystore verify
```

| Subcommand | Description |
|------------|-------------|
| `show` | Display sidechain, aura, and grandpa public keys |
| `verify` | Check keys are loaded in node and registration status |

## Output Examples

### status command

```
INFO Health: ✓ | Syncing: ✓ | Peers: 12
INFO Block: 3349667 | Finalized: 3349665 | Sync: 100.00%
INFO Blocks produced: 1
INFO Sidechain: epoch 245624 slot 294749055 | Mainchain: epoch 1178 slot 101838307
INFO Keys: sidechain ✓ | aura ✓ | grandpa ✓
INFO Registration: ✓ Registered (valid)
```

### sync command

```
INFO Starting block synchronization
INFO Chain tip: 3352077, finalized: 3352075
INFO Starting sync from block 3351077
INFO Synced blocks 3351077-3351176 (100 blocks)
INFO Initial sync complete. 1001 blocks in database
INFO Watching for new blocks (poll interval: 6s)
INFO New block: 3352078-3352078 (1 synced)
```

### query stats

```
INFO Database Statistics
INFO Total blocks:     1003
INFO Finalized blocks: 1001
INFO Unfinalized:      2
INFO Block range:      3351077 - 3352079
INFO Gaps:             None (continuous)
```

### keys verify

```
INFO Key Status:
INFO   Sidechain: ✓ Loaded in keystore
INFO   Aura:      ✓ Loaded in keystore
INFO   Grandpa:   ✓ Loaded in keystore
INFO Registration Status:
INFO   ✓ Registered (valid)
INFO Summary: ✓ All keys loaded and registered
```

## Registration Status Types

| Status | Meaning |
|--------|---------|
| `✓ Permissioned candidate` | In the static permissioned validators list |
| `✓ Registered (valid)` | Dynamically registered with valid stake |
| `⚠ Registered but INVALID` | Registered but stake/signature validation failed |
| `✗ Not registered` | Not found in any registration list |

## Architecture

```
src/
├── main.rs              # CLI entry point with subcommands
├── commands/
│   ├── status.rs        # Status monitoring command
│   ├── sync.rs          # Block synchronization command
│   ├── query.rs         # Database query command
│   └── keys.rs          # Key verification command
├── rpc/
│   ├── client.rs        # JSON-RPC 2.0 client
│   └── types.rs         # Response data structures
├── db/
│   ├── schema.rs        # SQLite schema definitions
│   └── blocks.rs        # Block CRUD operations
├── midnight/
│   ├── digest.rs        # AURA slot extraction from block digest
│   ├── keystore.rs      # Substrate keystore loading
│   └── registration.rs  # Validator registration checks
└── metrics.rs           # Prometheus metrics parser
```

## Database Schema

The sync command creates a SQLite database with:

- **blocks**: Block number, hash, slot, epoch, extrinsics count, finalization status
- **validators**: Validator public keys and metadata (future use)
- **sync_status**: Current sync progress and chain state

## RPC Methods Used

- `system_health` - Node health and peer count
- `system_version` - Node version
- `system_syncState` - Sync progress
- `chain_getHeader` - Current block header
- `chain_getBlock` - Full block with extrinsics
- `chain_getBlockHash` - Block hash by number
- `chain_getFinalizedHead` - Finalized block hash
- `sidechain_getStatus` - Epoch/slot info for both chains
- `sidechain_getAriadneParameters(mainchain_epoch)` - Validator registration data (requires mainchain epoch)
- `author_hasKey` - Check if key is in keystore

## License

MIT
