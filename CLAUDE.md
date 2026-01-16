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
```

## Architecture Overview

### Command-Based Structure

The application uses a modular command-based architecture where each major feature is implemented as a separate command module:

- `src/commands/status.rs` - Real-time validator monitoring with periodic health checks
- `src/commands/sync.rs` - Block synchronization engine with batch processing and polling
- `src/commands/query.rs` - Database query interface for blocks, stats, and gap detection
- `src/commands/keys.rs` - Session key verification and keystore management

### Core Systems

**RPC Client (`src/rpc/`)**
- Generic JSON-RPC 2.0 client with atomic request IDs
- Type-safe method calls with serde deserialization
- Handles both Substrate standard RPC methods and Midnight-specific sidechain methods

**Database Layer (`src/db/`)**
- SQLite-based persistence with three main tables: blocks, validators, sync_status
- Schema includes indexes on block_hash, slot_number, epoch, author_key, timestamp
- Block storage includes full header data, slot/epoch info, finalization status, and extrinsics count
- `sync_status` table is a singleton (id=1) that tracks synchronization progress

**Midnight-Specific Logic (`src/midnight/`)**
- `digest.rs` - Extracts AURA slot numbers from block digest logs (PreRuntime format: 0x06 + "aura" + slot_le_bytes)
- `keystore.rs` - Loads Substrate keystore files and validator keys (supports both keystore directories and JSON files)
- `registration.rs` - Checks validator registration status via `sidechain_getAriadneParameters` RPC call

### Key Data Flow

1. **Status Monitoring**: Polls RPC endpoints → Fetches health, sync state, block info, sidechain status → Verifies keys via `author_hasKey` → Checks registration via `sidechain_getAriadneParameters` → Displays formatted output

2. **Block Sync**: Determines sync start point → Fetches blocks in batches via `chain_getBlock` → Extracts slot from digest logs → Calculates epoch from slot → Stores in SQLite → Polls for new blocks at intervals

3. **Key Verification**: Loads keys from keystore directory (filenames: `<key_type_hex><public_key_hex>`) → Checks each key loaded via `author_hasKey` RPC → Checks registration in permissioned candidates or dynamic registrations

## Midnight Blockchain Specifics

### Key Types
- Sidechain: key type "crch" (63726368 in hex)
- Aura: key type "aura" (61757261 in hex)
- Grandpa: key type "gran" (6772616e in hex)

### Slot and Epoch Calculation
- Slots are extracted from AURA PreRuntime digest logs in blocks
- Epoch calculation is Midnight-specific (not standard Substrate)
- The relationship between slots and epochs is determined by the sidechain runtime

### Validator Registration
Two types of validators:
1. **Permissioned Candidates**: Static list in `permissionedCandidates` field
2. **Dynamic Registrations**: Runtime registrations in `candidateRegistrations` map, validated with `isValid` flag

### RPC Methods Used
Standard Substrate:
- `system_health`, `system_version`, `system_syncState`
- `chain_getHeader`, `chain_getBlock`, `chain_getBlockHash`, `chain_getFinalizedHead`
- `author_hasKey`

Midnight-specific:
- `sidechain_getStatus` - Returns epoch/slot information
- `sidechain_getAriadneParameters` - Returns validator registration data and permissioned candidates

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

### Metrics Parsing
The `metrics.rs` module parses Prometheus-format metrics from the node's `/metrics` endpoint. This is used for additional monitoring data like block production counts.
