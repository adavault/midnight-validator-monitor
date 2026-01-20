# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Midnight Validator Monitor (MVM) is a Rust CLI tool for monitoring and managing Midnight blockchain validator nodes. It provides real-time status monitoring, block synchronization to SQLite, data queries, and validator key management.

## Build and Development Commands

```bash
# Build release binary
cargo build --release

# Build debug binary
cargo build

# Run with verbose logging
cargo run -- --verbose <command>

# Run tests
cargo test

# Run specific test
cargo test <test_name>

# Run with backtrace on error
RUST_BACKTRACE=1 cargo run -- <command>
```

The binary name is `mvm`.

## Command Usage

**Note:** Key verification via `author_hasKey` RPC requires the node to be started with `--rpc-methods=unsafe`. Without this, key status will show `?` (unable to verify) but registration checks will still work.

### status - Monitor validator node
```bash
mvm status --keystore /path/to/keystore
mvm status --once --keystore /path/to/keystore
mvm status --rpc-url http://localhost:9944 --metrics-url http://localhost:9615/metrics
```

### sync - Synchronize blocks to database
```bash
mvm sync --db-path ./mvm.db
mvm sync --db-path ./mvm.db --start-block 1000000
mvm sync --db-path ./mvm.db --finalized-only
mvm sync --daemon --pid-file /opt/midnight/mvm/data/mvm-sync.pid
```

### query - Query stored block data
```bash
mvm query --db-path ./mvm.db stats
mvm query --db-path ./mvm.db blocks --limit 20
mvm query --db-path ./mvm.db gaps
```

### keys - Verify session keys
```bash
mvm keys --keystore /path/to/keystore show
mvm keys --keystore /path/to/keystore verify
mvm keys --keystore /path/to/keystore --db-path ./mvm.db verify
```

### view - Interactive TUI monitoring
```bash
mvm view --db-path ./mvm.db --rpc-url http://localhost:9944
```

### config - Configuration management
```bash
mvm config show      # Show current configuration
mvm config validate  # Validate configuration file
mvm config example   # Print example configuration
mvm config paths     # Show config file search paths
```

## Architecture Overview

### Command-Based Structure

The application uses a modular command-based architecture where each major feature is implemented as a separate command module:

- `src/commands/status.rs` - Real-time validator monitoring with periodic health checks
- `src/commands/sync.rs` - Block synchronization engine with batch processing, polling, and daemon mode
- `src/commands/query.rs` - Database query interface for blocks, validators, stats, and performance metrics
- `src/commands/keys.rs` - Session key verification, keystore management, and validator registration
- `src/commands/view.rs` - Interactive TUI for real-time monitoring with multiple views
- `src/commands/config.rs` - Configuration management and validation

### Core Systems

**RPC Client (`src/rpc/`)**
- Generic JSON-RPC 2.0 client with atomic request IDs
- Type-safe method calls with serde deserialization
- Handles both Substrate standard RPC methods and Midnight-specific sidechain methods

**Database Layer (`src/db/`)**
- SQLite-based persistence with four main tables: blocks, validators, committee_snapshots, sync_status
- Schema includes indexes on block_hash, slot_number, epoch, author_key, timestamp
- Block storage includes full header data, slot/epoch info, finalization status, and extrinsics count
- `committee_snapshots` table stores the full committee (AURA keys by position) for each epoch
- `sync_status` table is a singleton (id=1) that tracks synchronization progress

**Midnight-Specific Logic (`src/midnight/`)**
- `digest.rs` - Extracts AURA slot numbers from block digest logs (PreRuntime format: 0x06 + "aura" + slot_le_bytes)
- `keystore.rs` - Loads Substrate keystore files and validator keys (supports both keystore directories and JSON files)
- `registration.rs` - Checks validator registration status via `sidechain_getAriadneParameters` RPC call
- `scale.rs` - SCALE decoder for AURA authorities response (committee member list)
- `validators.rs` - Validator set management with committee fetching, historical state queries, and fallback logic for pruned nodes

**Configuration System (`src/config.rs`)**
- TOML-based configuration with three-tier priority: CLI flags > Environment variables > Config file > Defaults
- Multiple config file locations searched in order: `./mvm.toml`, `~/.config/mvm/config.toml`, `/opt/midnight/mvm/config/config.toml`, `/etc/mvm/config.toml`
- Environment variable overrides using `MVM_` prefix (e.g., `MVM_RPC_URL`, `MVM_DB_PATH`)
- Validation and example generation via `config` command
- Sections: rpc, database, validator, sync, view, daemon

**Daemon Mode (`src/daemon.rs`)**
- PID file management with Drop trait for automatic cleanup
- Signal handling (SIGTERM, SIGINT, SIGQUIT) for graceful shutdown
- Systemd service files for sync and status commands
- Installation scripts for system deployment

**TUI System (`src/tui/`)**
- Event-driven architecture with ratatui and crossterm
- Six views: Dashboard, Blocks, Validators, Performance, Peers, Help
- Keyboard navigation (1-5 for views, j/k for scrolling, f for filtering, t for theme, q to quit)
- Components: `app.rs` (state), `event.rs` (input handling), `ui.rs` (rendering), `layout.rs` (responsive sizing), `theme.rs` (Midnight/Midday themes)

### Key Data Flow

1. **Status Monitoring**: Polls RPC endpoints → Fetches health, sync state, block info, sidechain status → Verifies keys via `author_hasKey` → Checks registration via `sidechain_getAriadneParameters` → Displays formatted output

2. **Block Sync**: Determines sync start point → Fetches blocks in batches via `chain_getBlock` → Extracts slot from digest logs → Calculates epoch from slot → Attributes block to author → Stores in SQLite → Polls for new blocks at intervals → Handles signals for graceful shutdown in daemon mode

3. **Key Verification**: Loads keys from keystore directory (filenames: `<key_type_hex><public_key_hex>`) → Checks each key loaded via `author_hasKey` RPC → Checks registration in permissioned candidates or dynamic registrations → Marks validators as "ours" in database

4. **Interactive TUI**: Loads config → Connects to RPC and database → Enters event loop → Handles keyboard input → Fetches fresh data on intervals → Renders views (Dashboard/Blocks/Validators/Performance/Peers/Help) → Updates display → Repeats until quit

## Midnight Blockchain Specifics

### Key Types
- Sidechain: key type "crch" (63726368 in hex)
- Aura: key type "aura" (61757261 in hex)
- Grandpa: key type "gran" (6772616e in hex)

### Slot and Epoch Calculation
- Slots are extracted from AURA PreRuntime digest logs in blocks
- Epoch calculation is Midnight-specific (not standard Substrate)
- **Sidechain epochs**: 2-hour cycles that determine committee rotation and block production eligibility
- **Mainchain epochs**: 24-hour Cardano-style epochs
- The `sidechain_getStatus` RPC returns `nextEpochTimestamp` for both chains
- "This Epoch" block counting in TUI uses sidechain epoch (timestamp-based query)

### Validator Registration
Two types of validators:
1. **Permissioned Candidates**: Static list in `permissionedCandidates` field (IOG/Midnight team validators, no stake required)
2. **Dynamic Registrations**: Runtime registrations in `candidateRegistrations` map, validated with `isValid` flag

### Committee vs Candidates (Critical Distinction)

**Candidates** (~185 validators): Registered validators from `sidechain_getAriadneParameters`. These are validators that MAY produce blocks.

**Committee** (variable size, ~1200 seats on preview): The actual block production schedule from `AuraApi_authorities`. This is an ordered list of AURA keys where:
- Block author = `committee[slot % committee.len()]`
- Validators can have multiple seats (stake-weighted)
- Committee changes each **sidechain epoch** (2h preview, 10h mainnet)
- Selection is stake-weighted random (not purely top-N by stake)
- Committee size may differ between networks (preview vs mainnet)

**Important**: A validator being registered with `isValid: true` does NOT guarantee they are in the committee. Committee selection is probabilistic based on stake.

### Block Author Attribution

The sync command attributes blocks to validators using:
1. Extract slot number from AURA PreRuntime digest
2. Fetch committee via `state_call("AuraApi_authorities", "0x", block_hash)`
3. Calculate: `author_aura_key = committee[slot % committee.len()]`
4. Look up validator by AURA key to get sidechain key
5. Store sidechain key as `author_key` in blocks table

**Historical State Queries**: When syncing historical blocks, the committee must be queried at that block's hash to get the correct historical committee (since committees change each **sidechain epoch**).

**State Pruning Fallback**: Non-archive nodes prune historical state (typically keeping only ~256 blocks). When historical state is unavailable, the sync falls back to using the current committee with a warning that attribution may be inaccurate for blocks from different epochs.

**Committee Caching**: The sync caches committees by **sidechain epoch** (not mainchain epoch) since that's when committees rotate. Within a single mainchain epoch (24h preview), there are ~12 sidechain epochs with different committees.

### RPC Methods Used
Standard Substrate:
- `system_health`, `system_version`, `system_syncState`, `system_chain`
- `chain_getHeader`, `chain_getBlock`, `chain_getBlockHash`, `chain_getFinalizedHead`
- `author_hasKey`
- `system_peers` - Connected peers with sync status
- `system_unstable_networkState` - Network state including external IPs and peer ID (requires `--rpc-methods=unsafe`)
- `state_call("AuraApi_authorities", "0x", [optional_block_hash])` - Returns SCALE-encoded committee (requires historical state for past blocks)

Midnight-specific:
- `sidechain_getStatus` - Returns epoch/slot information with `nextEpochTimestamp` for both chains
- `sidechain_getAriadneParameters(mainchain_epoch)` - Returns validator registration data and permissioned candidates. **Note: Takes mainchain epoch number as parameter, not sidechain epoch.**

## Error Handling Patterns

The codebase uses `anyhow::Result<T>` for most error handling with context via `.context()`. Key patterns:

- RPC errors are converted to `anyhow::Error` with descriptive messages
- File I/O errors include the file path in context
- Missing keys in keystore return context-rich errors
- Database errors propagate with rusqlite error types

## Testing

Tests are located in the same files as the code they test using `#[cfg(test)]` modules. Key test patterns:

- RPC client tests would require mocking (currently minimal)
- Digest extraction tests use real block data examples
- Database schema tests use in-memory SQLite
- Key normalization tests verify hex string handling

## Important Implementation Details

### Keystore File Format
Substrate keystore files have no extension and are named: `<8_char_hex_key_type><64_char_hex_pubkey>`

Example: `63726368a1b2c3d4e5f6...` = crch (sidechain) key type + public key

### Block Digest Log Parsing
AURA PreRuntime digest structure:
- Byte 0: 0x06 (PreRuntime)
- Bytes 1-4: "aura" (61757261)
- Bytes 5-12: slot number as little-endian u64

Must check length >= 30 hex chars (0x + 28 chars) before parsing.

### Database Sync Logic
The sync command maintains a singleton row in `sync_status` (id=1) to track:
- `last_synced_block` - highest block number stored
- `last_finalized_block` - highest finalized block
- `chain_tip_block` - current chain head
- `current_epoch` - latest epoch seen

Initial sync processes batches of blocks, then enters polling mode for new blocks.

### Daemon Mode and Systemd
The sync command supports daemon mode with:
- `--daemon` flag enables background operation
- `--pid-file` creates a PID file for process management
- Signal handling (SIGTERM, SIGINT, SIGQUIT) for graceful shutdown using tokio select! macro
- PidFile struct with Drop trait ensures automatic cleanup

Systemd integration:
- `systemd/mvm-sync.service` - Continuous sync daemon (Type=simple, Restart=on-failure)
- `systemd/mvm-status.service` - One-shot health check (Type=oneshot)
- `systemd/mvm-status.timer` - Periodic health checks every 5 minutes
- `scripts/install.sh` - Automated installation and user/directory setup
- `scripts/uninstall.sh` - Clean removal of services and files

### Configuration File Support
Configuration priority (highest to lowest):
1. **CLI flags** - Explicitly provided command-line arguments
2. **Environment variables** - Variables prefixed with `MVM_` (e.g., `MVM_RPC_URL`)
3. **Config file** - First found in search path: `./mvm.toml`, `~/.config/mvm/config.toml`, `/opt/midnight/mvm/config/config.toml`, `/etc/mvm/config.toml`
4. **Defaults** - Built-in default values

The `config` command provides:
- `show` - Display current effective configuration
- `validate` - Check configuration file syntax and values
- `example` - Generate example configuration with comments
- `paths` - List config file search locations with existence status

### Metrics Parsing
The `metrics.rs` module parses Prometheus-format metrics from the node's `/metrics` endpoint. This is used for additional monitoring data like block production counts.
