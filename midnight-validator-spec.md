# Midnight Validator Monitor (MVM) - Technical Specification

## 1. Executive Summary

**Midnight Validator Monitor (MVM)** is a Rust CLI tool for monitoring Midnight blockchain validator nodes. Inspired by cncli but adapted for Midnight's Substrate-based architecture, MVM provides block synchronization, validation tracking, and performance monitoring.

### Key Differences from cncli
- **No Leaderlog Prediction**: Midnight uses AURA/GRANDPA consensus, not VRF-based slot leader selection
- **Substrate RPC**: Uses Polkadot/Substrate JSON-RPC methods instead of Cardano mini-protocols
- **6-second blocks**: Fixed block time vs Cardano's 20-second average
- **Partner Chain Model**: Validator registration tracked via `sidechain_getAriadneParameters`
- **Finality tracking**: Monitors GRANDPA finality alongside best block

---

## 2. System Architecture

### 2.1 Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                  Midnight Validator Monitor                  │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │  SYNC    │  │  STATUS  │  │  QUERY   │  │   KEYS   │    │
│  │ Command  │  │ Command  │  │ Command  │  │ Command  │    │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘    │
│       │             │             │             │           │
│       └─────────────┴──────┬──────┴─────────────┘           │
│                            │                                 │
│              ┌─────────────┴─────────────┐                  │
│              │        RPC Client         │                  │
│              │     (HTTP + WebSocket)    │                  │
│              └─────────────┬─────────────┘                  │
│                            │                                 │
│              ┌─────────────┴─────────────┐                  │
│              │      SQLite Database      │                  │
│              └───────────────────────────┘                  │
└─────────────────────────────────────────────────────────────┘
                             │
                             ▼
             ┌───────────────────────────────┐
             │       Midnight Node           │
             │   HTTP: http://localhost:9944 │
             │   WS: ws://localhost:9944     │
             │   Metrics: :9615/metrics      │
             └───────────────────────────────┘
```

### 2.2 Technology Stack

| Component | Library | Notes |
|-----------|---------|-------|
| Language | Rust 2021 edition | |
| CLI | `clap` v4 | Command-line parsing |
| HTTP Client | `reqwest` | JSON-RPC over HTTP |
| WebSocket | `tokio-tungstenite` | Optional, for subscriptions |
| Database | `rusqlite` | SQLite with bundled build |
| Async Runtime | `tokio` | Full features |
| Serialization | `serde`, `serde_json` | JSON handling |
| Hex Encoding | `hex` | Digest parsing |
| Logging | `tracing`, `tracing-subscriber` | Structured logging |
| Errors | `anyhow`, `thiserror` | Error handling |
| Time | `chrono` | Timestamps |

**Note**: We deliberately avoid `sp-core` and `sp-runtime` (Substrate primitives) as they add ~100+ transitive dependencies. Manual digest parsing is simpler and sufficient.

---

## 3. Database Schema

### 3.1 SQLite Tables (MVP)

```sql
-- Synchronized block headers
CREATE TABLE IF NOT EXISTS blocks (
    block_number INTEGER PRIMARY KEY,
    block_hash TEXT NOT NULL UNIQUE,
    parent_hash TEXT NOT NULL,
    state_root TEXT NOT NULL,
    extrinsics_root TEXT NOT NULL,
    slot_number INTEGER NOT NULL,
    epoch INTEGER NOT NULL,
    timestamp INTEGER NOT NULL,
    is_finalized INTEGER DEFAULT 0,
    author_key TEXT,
    extrinsics_count INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_blocks_hash ON blocks(block_hash);
CREATE INDEX IF NOT EXISTS idx_blocks_slot ON blocks(slot_number);
CREATE INDEX IF NOT EXISTS idx_blocks_epoch ON blocks(epoch);
CREATE INDEX IF NOT EXISTS idx_blocks_author ON blocks(author_key);
CREATE INDEX IF NOT EXISTS idx_blocks_timestamp ON blocks(timestamp);

-- Tracked validators
CREATE TABLE IF NOT EXISTS validators (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sidechain_key TEXT UNIQUE NOT NULL,
    aura_key TEXT,
    grandpa_key TEXT,
    label TEXT,
    is_ours INTEGER DEFAULT 0,
    registration_status TEXT,
    first_seen_epoch INTEGER,
    total_blocks INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_validators_sidechain ON validators(sidechain_key);
CREATE INDEX IF NOT EXISTS idx_validators_ours ON validators(is_ours);

-- Sync progress (singleton row)
CREATE TABLE IF NOT EXISTS sync_status (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    last_synced_block INTEGER NOT NULL DEFAULT 0,
    last_finalized_block INTEGER NOT NULL DEFAULT 0,
    chain_tip_block INTEGER NOT NULL DEFAULT 0,
    current_epoch INTEGER NOT NULL DEFAULT 0,
    is_syncing INTEGER DEFAULT 1,
    last_updated INTEGER NOT NULL
);

-- Initialize singleton
INSERT OR IGNORE INTO sync_status (id, last_synced_block, last_finalized_block, chain_tip_block, current_epoch, last_updated)
VALUES (1, 0, 0, 0, 0, 0);
```

### 3.2 Future Tables (Post-MVP)

These tables will be added in Phase 2:

```sql
-- Health check history (Phase 2)
CREATE TABLE IF NOT EXISTS health_checks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    is_syncing INTEGER NOT NULL,
    peer_count INTEGER NOT NULL,
    best_block INTEGER NOT NULL,
    finalized_block INTEGER NOT NULL,
    response_time_ms INTEGER
);

CREATE INDEX IF NOT EXISTS idx_health_timestamp ON health_checks(timestamp);

-- Session key history (Phase 2)
CREATE TABLE IF NOT EXISTS session_key_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    validator_id INTEGER NOT NULL,
    aura_key TEXT NOT NULL,
    grandpa_key TEXT NOT NULL,
    set_at_block INTEGER,
    set_at_timestamp INTEGER NOT NULL,
    is_current INTEGER DEFAULT 1,
    FOREIGN KEY (validator_id) REFERENCES validators(id)
);

CREATE INDEX IF NOT EXISTS idx_session_validator ON session_key_history(validator_id);
```

---

## 4. RPC Methods Reference

### 4.1 Midnight-Specific Methods

These are custom to Midnight's partner chain implementation:

```rust
// Get current epoch and slot info
sidechain_getStatus() -> {
    sidechain: { epoch: u64, slot: u64, nextEpochTimestamp: Option<u64> },
    mainchain: { epoch: u64, slot: u64, nextEpochTimestamp: Option<u64> }
}

// Get validator registration data for an epoch
sidechain_getAriadneParameters(epoch: u64) -> {
    dParameter: { numPermissionedCandidates: u32, numRegisteredCandidates: u32 },
    permissionedCandidates: Vec<{
        sidechainPublicKey: String,
        auraPublicKey: String,
        grandpaPublicKey: String,
        isValid: bool
    }>,
    candidateRegistrations: HashMap<String, Vec<CandidateRegistration>>
}

// Get registrations (simpler alternative)
sidechain_getRegistrations(epoch: u64) -> Vec<Registration>
```

### 4.2 Standard Substrate Methods

```rust
// Chain methods
chain_getBlock(hash?: Hash) -> SignedBlock
chain_getHeader(hash?: Hash) -> Header
chain_getBlockHash(number?: BlockNumber) -> Hash
chain_getFinalizedHead() -> Hash
chain_subscribeNewHeads() -> Subscription<Header>  // WebSocket only
chain_subscribeFinalizedHeads() -> Subscription<Header>  // WebSocket only

// System methods
system_health() -> { peers: u32, isSyncing: bool, shouldHavePeers: bool }
system_syncState() -> { startingBlock: u64, currentBlock: u64, highestBlock: u64 }
system_version() -> String
system_chain() -> String

// Author methods (validator key checks)
author_hasKey(publicKey: Bytes, keyType: String) -> bool
author_hasSessionKeys(sessionKeys: Bytes) -> bool
author_rotateKeys() -> Bytes  // Requires unsafe RPC access
```

### 4.3 Block Structure

Actual response from `chain_getBlock()`:

```json
{
  "block": {
    "header": {
      "parentHash": "0x55907f0ca903e0e16164cebeaf53d31b49c9a7895347cf57b1089ac9d9f896ea",
      "number": "0x332534",
      "stateRoot": "0x36777d330a059e80b6256277ff6fe02924da22cd1a0e626b43684e30b6ed3636",
      "extrinsicsRoot": "0xfc1e9fb03c071b2781f0aa9deb6f5e61411e6001465f0d333130d5c417f23046",
      "digest": {
        "logs": [
          "0x066175726120778c911100000000",
          "0x066d637368804404db62c3e40b047c638c2cc3ae2d45678b65b2fc57b748c4d1a9576bf4bc8c",
          "0x044d4e535610e02e0000",
          "0x05617572610101f2a177943e6df53c0ae1a610ea380bae752fc98886ba61b484fbf3c56f18a51671c8021e00b8d363e95ebd10e65346dabf632cf99e627605ab8b729416c7a48b"
        ]
      }
    },
    "extrinsics": ["0x280401000b10254cc39b01", "..."]
  },
  "justifications": null
}
```

**Digest Log Types**:
| Prefix | Type | Content |
|--------|------|---------|
| `0x0661757261` | PreRuntime AURA | Slot number (8 bytes LE) |
| `0x066d637368` | PreRuntime "mcsh" | Midnight chain specific data |
| `0x044d4e5356` | Consensus "MNSV" | Midnight node version |
| `0x0561757261` | Seal AURA | Block signature |

---

## 5. Block Author Extraction

### 5.1 Slot Extraction (Verified Working)

```rust
/// Extract AURA slot number from block digest logs
fn extract_slot_from_digest(logs: &[String]) -> Option<u64> {
    for log in logs {
        // PreRuntime AURA format: 0x06 + "aura"(61757261) + slot_le_bytes
        if log.starts_with("0x0661757261") && log.len() >= 30 {
            let slot_hex = &log[14..30]; // 8 bytes = 16 hex chars
            let bytes = hex::decode(slot_hex).ok()?;
            let arr: [u8; 8] = bytes.try_into().ok()?;
            return Some(u64::from_le_bytes(arr));
        }
    }
    None
}
```

### 5.2 Author Determination

**Theory**: In AURA consensus, block author is determined by:
```
author_index = slot_number % validator_count
author = validators[author_index]
```

**Challenge**: The validator ordering is not documented. The `sidechain_getAriadneParameters` returns:
- `permissionedCandidates`: ~1100 static validators
- `candidateRegistrations`: ~100 dynamic registrations

**Current Approach**: For MVP, we track:
1. Slot number per block (store in DB)
2. Our validator's registration status
3. Defer full author attribution until ordering is verified

**Verification Method**: To determine ordering:
1. Sync several blocks with slot numbers
2. Query `sidechain_getAriadneParameters` for validator list
3. For known blocks where we produced, check if `slot % count` matches our position

---

## 6. Command Specifications

### 6.1 Command Overview

| Command | Purpose | Mode |
|---------|---------|------|
| `mvm sync` | Synchronize blocks to SQLite | Daemon |
| `mvm status` | Display current node status | One-shot or loop |
| `mvm query` | Query stored block data | One-shot |
| `mvm keys` | Verify/manage session keys | One-shot |

### 6.2 `mvm sync`

**Purpose**: Continuously synchronize block headers from Midnight node to local SQLite database.

**Usage**:
```bash
mvm sync [OPTIONS]
```

**Options**:
```
-r, --rpc-url <URL>        RPC endpoint [default: http://localhost:9944]
-d, --db-path <PATH>       SQLite database path [default: ./mvm.db]
-s, --start-block <N>      Start sync from block N [default: 0]
-b, --batch-size <N>       Blocks per batch [default: 100]
    --finalized-only       Only sync finalized blocks
    --track                Track our validator's block production
-K, --keystore <PATH>      Keystore path (for --track)
-v, --verbose              Debug logging
```

**Behavior**:
1. Initialize SQLite database with schema
2. Check `sync_status` for last synced block
3. Fetch current chain tip and finalized head
4. Batch-fetch missing blocks via `chain_getBlock`
5. Extract slot, epoch, timestamp from each block
6. Store in `blocks` table
7. Update `sync_status` periodically
8. Subscribe to new heads (WebSocket) or poll (HTTP)
9. Mark blocks as finalized when finalized head advances

**Output**:
```
2026-01-15 10:23:45 INFO  Starting sync from block 3350000
2026-01-15 10:23:45 INFO  Chain tip: 3351234, finalized: 3351232
2026-01-15 10:23:50 INFO  Synced 3350000-3350100 (100 blocks, 4.2s)
2026-01-15 10:24:00 INFO  Synced 3350100-3350200 (100 blocks, 4.1s)
...
2026-01-15 10:30:00 INFO  Caught up to tip. Waiting for new blocks...
2026-01-15 10:30:06 INFO  New block #3351235 slot=294751400 epoch=245626
```

### 6.3 `mvm status`

**Purpose**: Display current validator node status (health, sync, keys, registration).

**Usage**:
```bash
mvm status [OPTIONS]
```

**Options**:
```
-r, --rpc-url <URL>        RPC endpoint [default: http://localhost:9944]
-M, --metrics-url <URL>    Prometheus metrics [default: http://localhost:9615/metrics]
-K, --keystore <PATH>      Keystore path for key detection
-k, --keys-file <PATH>     Keys JSON file (alternative to keystore)
-i, --interval <SECS>      Loop interval [default: 60]
    --once                 Run once and exit
-v, --verbose              Debug logging
```

**Behavior**: (Current implementation - preserve this)
1. Fetch `system_health`, `system_syncState`, `chain_getHeader`
2. Fetch `chain_getFinalizedHead` for finality info
3. Fetch `sidechain_getStatus` for epoch/slot
4. Parse Prometheus metrics for blocks produced
5. If keystore provided, check registration via `sidechain_getAriadneParameters`
6. Display formatted status
7. Repeat every `--interval` seconds unless `--once`

**Output**:
```
INFO ─────────────────────────────────────────
INFO Health: ✓ | Syncing: ✓ | Peers: 12
INFO Block: 3351234 | Finalized: 3351232 | Sync: 100.00%
INFO Blocks produced: 42
INFO Sidechain: epoch 245626 slot 294751400 | Mainchain: epoch 1178 slot 101838500
INFO Keys: sidechain ? | aura ? | grandpa ?
INFO Registration: ✓ Registered (valid)
```

### 6.4 `mvm query`

**Purpose**: Query block and statistics from local SQLite database.

**Usage**:
```bash
mvm query <SUBCOMMAND> [OPTIONS]
```

**Subcommands**:

#### `mvm query blocks`
```bash
mvm query blocks [OPTIONS]
  --from <N>           Start block number
  --to <N>             End block number
  --epoch <N>          Filter by epoch
  --author <KEY>       Filter by author sidechain key
  --limit <N>          Max results [default: 100]
  --format <FMT>       Output format: table|json|csv [default: table]
```

#### `mvm query stats`
```bash
mvm query stats [OPTIONS]
  --period <PERIOD>    Time period: 1h|24h|7d|30d [default: 24h]
  --validator <KEY>    Stats for specific validator
```

Output:
```
Block Statistics (last 24h)
├─ Total blocks: 14,400
├─ Finalized: 14,398
├─ Avg block time: 6.0s
├─ Our blocks: 12 (0.08%)
└─ Epochs covered: 245620-245626
```

#### `mvm query gaps`
```bash
mvm query gaps [OPTIONS]
  --auto-fill          Fetch missing blocks
```

Find gaps in synced block sequence.

### 6.5 `mvm keys`

**Purpose**: Verify and manage validator session keys.

**Usage**:
```bash
mvm keys <SUBCOMMAND> [OPTIONS]
```

**Subcommands**:

#### `mvm keys verify`
```bash
mvm keys verify [OPTIONS]
  -r, --rpc-url <URL>      RPC endpoint
  -K, --keystore <PATH>    Keystore path
  -k, --keys-file <PATH>   Keys JSON file
```

Verify that session keys are loaded in the node's keystore.

**Output**:
```
Verifying session keys...
  Sidechain (crch): 0x037764d2d8... ✓ Loaded
  AURA:             0xe05be3c28c... ? (RPC restricted)
  GRANDPA:          0xf5a39df922... ? (RPC restricted)

Registration Status: ✓ Registered (valid)
```

#### `mvm keys show`
```bash
mvm keys show -K <PATH>
```

Display keys from keystore without RPC check.

---

## 7. Project Structure

```
midnight-validator-monitor/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point, argument parsing
│   ├── lib.rs               # Library exports
│   │
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── sync.rs          # Block synchronization
│   │   ├── status.rs        # Node status display
│   │   ├── query.rs         # Database queries
│   │   └── keys.rs          # Key verification
│   │
│   ├── rpc/
│   │   ├── mod.rs
│   │   ├── client.rs        # HTTP JSON-RPC client
│   │   ├── types.rs         # RPC response types
│   │   └── substrate.rs     # Substrate-specific methods
│   │
│   ├── db/
│   │   ├── mod.rs
│   │   ├── schema.rs        # Table creation, migrations
│   │   ├── blocks.rs        # Block CRUD operations
│   │   └── validators.rs    # Validator CRUD operations
│   │
│   ├── midnight/
│   │   ├── mod.rs
│   │   ├── digest.rs        # Block digest parsing
│   │   ├── registration.rs  # Validator registration checks
│   │   └── keystore.rs      # Substrate keystore loading
│   │
│   └── metrics/
│       ├── mod.rs
│       └── prometheus.rs    # Prometheus metrics parsing
│
├── tests/
│   ├── integration_test.rs
│   └── rpc_mock.rs
│
└── README.md
```

---

## 8. Dependencies (Cargo.toml)

```toml
[package]
name = "midnight-validator-monitor"
version = "0.2.0"
edition = "2021"
description = "Monitoring tool for Midnight blockchain validators"
license = "MIT"

[dependencies]
# CLI
clap = { version = "4", features = ["derive"] }

# Async runtime
tokio = { version = "1", features = ["full"] }

# HTTP client
reqwest = { version = "0.11", features = ["json"] }

# WebSocket (optional, for subscriptions)
tokio-tungstenite = { version = "0.21", optional = true }
futures-util = { version = "0.3", optional = true }

# Database
rusqlite = { version = "0.30", features = ["bundled"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
hex = "0.4"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Time
chrono = { version = "0.4", features = ["serde"] }

[features]
default = []
websocket = ["tokio-tungstenite", "futures-util"]
```

---

## 9. Configuration File

### 9.1 `mvm.toml` (Optional)

```toml
[network]
rpc_url = "http://localhost:9944"
metrics_url = "http://localhost:9615/metrics"

[database]
path = "./mvm.db"

[validator]
keystore_path = "/path/to/keystore"
# Or specify keys directly:
# sidechain_key = "0x037764d2d83c269030fef6df5aeb4419c48762ada2cf20b0e4e6ede596809f4700"
# aura_key = "0xe05be3c28c72864efc49f4f12cb04f3bd6f20fdbc297501aa71f8590273b3e1e"
# grandpa_key = "0xf5a39df9227f630754f78bbae43bd66a693612eeffa9ceec5681f6c05f48d0e8"

[sync]
batch_size = 100
poll_interval = 6
finalized_only = false

[logging]
level = "info"  # trace, debug, info, warn, error
```

### 9.2 Environment Variables

```bash
MVM_RPC_URL="http://localhost:9944"
MVM_METRICS_URL="http://localhost:9615/metrics"
MVM_DB_PATH="./mvm.db"
MVM_KEYSTORE_PATH="/path/to/keystore"
MVM_LOG_LEVEL="info"
RUST_LOG="midnight_validator_monitor=info"
```

---

## 10. Error Handling

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MvmError {
    #[error("RPC error: {0}")]
    Rpc(#[from] reqwest::Error),

    #[error("RPC response error {code}: {message}")]
    RpcResponse { code: i64, message: String },

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Block not found: {0}")]
    BlockNotFound(u64),

    #[error("Invalid digest format: {0}")]
    InvalidDigest(String),

    #[error("Keystore error: {0}")]
    Keystore(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, MvmError>;
```

---

## 11. Deployment

### 11.1 Build and Install

```bash
# Build release binary
cargo build --release

# Install to /usr/local/bin
sudo cp target/release/mvm /usr/local/bin/

# Or install via cargo
cargo install --path .
```

### 11.2 Systemd Service (Sync Daemon)

```ini
# /etc/systemd/system/mvm-sync.service
[Unit]
Description=Midnight Validator Monitor - Sync
After=network.target

[Service]
Type=simple
User=midnight
ExecStart=/usr/local/bin/mvm sync \
    --rpc-url http://localhost:9944 \
    --db-path /var/lib/mvm/mvm.db \
    --track \
    --keystore /path/to/keystore
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

### 11.3 Cron for Status Alerts

```bash
# /etc/cron.d/mvm-status
*/5 * * * * midnight /usr/local/bin/mvm status --once --rpc-url http://localhost:9944 2>&1 | logger -t mvm
```

---

## 12. Testing Strategy

### 12.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_extraction() {
        let logs = vec![
            "0x066175726120778c911100000000".to_string(),
            "0x066d637368...".to_string(),
        ];
        let slot = extract_slot_from_digest(&logs);
        assert_eq!(slot, Some(294751351));
    }

    #[test]
    fn test_block_number_parsing() {
        assert_eq!(parse_hex_number("0x332534"), Some(3351860));
        assert_eq!(parse_hex_number("0x0"), Some(0));
    }

    #[tokio::test]
    async fn test_database_operations() {
        let db = Database::open(":memory:").unwrap();
        db.create_schema().unwrap();

        let block = Block {
            number: 1000,
            hash: "0x123".to_string(),
            slot: 100000,
            epoch: 100,
            // ...
        };
        db.insert_block(&block).unwrap();

        let retrieved = db.get_block(1000).unwrap();
        assert_eq!(retrieved.hash, "0x123");
    }
}
```

### 12.2 Integration Tests

```rust
#[tokio::test]
#[ignore] // Requires running node
async fn test_live_rpc() {
    let client = RpcClient::new("http://localhost:9944");
    let health = client.system_health().await.unwrap();
    assert!(health.peers > 0);
}
```

---

## 13. Future Enhancements (Phase 2+)

These features are explicitly deferred from MVP:

### Phase 2
- Health check history storage and trending
- Session key change tracking
- Prometheus metrics export (`/metrics` endpoint)
- Email/webhook alerting on issues
- Block author attribution (requires validator ordering research)

### Phase 3
- Web dashboard (real-time status UI)
- Multi-validator tracking
- Historical analytics and graphing
- Automated node restart on issues
- GraphQL API for external integrations

---

## 14. Open Questions

1. **Validator Ordering**: How does AURA determine slot-to-validator mapping? Is it:
   - Index in `permissionedCandidates` array?
   - Sorted by public key?
   - Combined with `candidateRegistrations` somehow?

2. **Epoch Transitions**: Do validator sets change at epoch boundaries? Need to cache validator list per epoch.

3. **Block Time Variance**: Is 6 seconds guaranteed or can it vary? Affects sync polling interval.

4. **RPC Rate Limits**: Are there rate limits on the RPC endpoints? Need backoff strategy.

---

## 15. References

### Midnight Resources
- Midnight Documentation: https://docs.midnight.network
- Midnight Node: https://github.com/midnightntwrk/midnight-node
- Ankr RPC: https://www.ankr.com/docs/rpc-service/chains/chains-api/midnight/

### Technical References
- Substrate RPC: https://polkadot.js.org/docs/substrate/rpc/
- AURA Consensus: https://docs.substrate.io/reference/glossary/#aura
- GRANDPA Finality: https://docs.substrate.io/reference/glossary/#grandpa

### Similar Tools
- cncli (Cardano): https://github.com/cardano-community/cncli
- Substrate Telemetry: https://github.com/paritytech/substrate-telemetry

---

## Appendix A: Keystore File Format

Substrate keystores use filenames to encode key type and public key:

```
<key_type_hex><public_key_hex>
```

| Key Type | Hex | Example Filename |
|----------|-----|------------------|
| AURA | `61757261` | `61757261e05be3c28c72864efc49f4f12cb04f3bd6f20fdbc297501aa71f8590273b3e1e` |
| GRANDPA | `6772616e` | `6772616ef5a39df9227f630754f78bbae43bd66a693612eeffa9ceec5681f6c05f48d0e8` |
| Sidechain | `63726368` | `63726368037764d2d83c269030fef6df5aeb4419c48762ada2cf20b0e4e6ede596809f4700` |

---

## Appendix B: SQL Query Examples

```sql
-- Blocks in last 24 hours
SELECT COUNT(*) FROM blocks
WHERE timestamp >= strftime('%s', 'now', '-24 hours');

-- Blocks by our validator
SELECT COUNT(*) FROM blocks
WHERE author_key = '0x037764d2d83c269030fef6df5aeb4419c48762ada2cf20b0e4e6ede596809f4700';

-- Find gaps in block sequence
SELECT b1.block_number + 1 AS gap_start,
       MIN(b2.block_number) - 1 AS gap_end
FROM blocks b1
LEFT JOIN blocks b2 ON b1.block_number < b2.block_number
WHERE NOT EXISTS (
    SELECT 1 FROM blocks c WHERE c.block_number = b1.block_number + 1
)
AND b2.block_number IS NOT NULL
GROUP BY b1.block_number;

-- Average block time (last 1000 blocks)
SELECT AVG(b2.timestamp - b1.timestamp) AS avg_seconds
FROM blocks b1
JOIN blocks b2 ON b2.block_number = b1.block_number + 1
WHERE b1.block_number >= (SELECT MAX(block_number) - 1000 FROM blocks);

-- Blocks per epoch
SELECT epoch, COUNT(*) AS block_count
FROM blocks
GROUP BY epoch
ORDER BY epoch DESC
LIMIT 10;
```

---

## Appendix C: Quick Start

```bash
# 1. Build
cargo build --release

# 2. Check node status
./target/release/mvm status --once \
    --rpc-url http://localhost:9944 \
    --keystore ~/midnight-node-docker/data/chains/partner_chains_template/keystore/

# 3. Start syncing blocks
./target/release/mvm sync \
    --rpc-url http://localhost:9944 \
    --db-path ./mvm.db

# 4. Query synced data
./target/release/mvm query stats --period 24h
./target/release/mvm query blocks --limit 10

# 5. Verify keys
./target/release/mvm keys verify \
    --keystore ~/midnight-node-docker/data/chains/partner_chains_template/keystore/
```

---

**END OF SPECIFICATION**
