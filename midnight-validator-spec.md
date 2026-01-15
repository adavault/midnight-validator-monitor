# Midnight Validator Monitor (MVM) - Technical Specification

## 1. Executive Summary

This specification defines the **Midnight Validator Monitor (MVM)**, a Rust-based CLI tool inspired by cncli but adapted for the Midnight blockchain. MVM focuses on block synchronization, validation tracking, and network monitoring for Midnight validators, replacing cncli's VRF-based leaderlog prediction with practical validator performance tracking.

### Key Differences from cncli
- **No Leaderlog Prediction**: Midnight uses different consensus (partner chain + AURA/GRANDPA) without VRF-based slot leader selection
- **Substrate-based RPC**: Uses Polkadot/Substrate JSON-RPC methods instead of Cardano mini-protocols
- **6-second blocks**: Fixed block time vs Cardano's 20-second average
- **Session key validation**: Tracks validator session keys via `author_hasKey` and `author_hasSessionKeys`
- **Finality tracking**: Monitors GRANDPA finality instead of just tip blocks

---

## 2. System Architecture

### 2.1 Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                  Midnight Validator Monitor                  │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐            │
│  │   SYNC     │  │  VALIDATE  │  │   HEALTH   │            │
│  │  Module    │  │   Module   │  │   Module   │            │
│  └────────────┘  └────────────┘  └────────────┘            │
│         │                │                │                  │
│         └────────────────┴────────────────┘                  │
│                       │                                       │
│              ┌────────▼────────┐                             │
│              │  RPC Client     │                             │
│              │  (JSON-RPC/WS)  │                             │
│              └────────┬────────┘                             │
│                       │                                       │
│              ┌────────▼────────┐                             │
│              │  SQLite Store   │                             │
│              └─────────────────┘                             │
└─────────────────────────────────────────────────────────────┘
                       │
                       ▼
        ┌──────────────────────────┐
        │  Midnight Node           │
        │  wss://rpc.endpoint      │
        │  Port: 9944 (WebSocket)  │
        └──────────────────────────┘
```

### 2.2 Technology Stack

- **Language**: Rust (edition 2021)
- **RPC Client**: `jsonrpsee` (WebSocket client for Substrate RPC)
- **Database**: `rusqlite` (SQLite for block/validation storage)
- **CLI**: `clap` v4 (command-line argument parsing)
- **Async Runtime**: `tokio` (async I/O)
- **Serialization**: `serde_json` (JSON handling)
- **Cryptography**: `sp-core`, `sp-runtime` (Substrate primitives)
- **Logging**: `tracing`, `tracing-subscriber`

---

## 3. Database Schema

### 3.1 SQLite Tables

```sql
-- Blocks table: stores synchronized block headers
CREATE TABLE IF NOT EXISTS blocks (
    block_number INTEGER PRIMARY KEY,
    block_hash TEXT NOT NULL UNIQUE,
    parent_hash TEXT NOT NULL,
    state_root TEXT NOT NULL,
    extrinsics_root TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    is_finalized BOOLEAN DEFAULT 0,
    author_id TEXT,                    -- Block author (validator)
    created_at INTEGER NOT NULL,
    INDEX idx_block_hash (block_hash),
    INDEX idx_timestamp (timestamp),
    INDEX idx_author (author_id)
);

-- Validators table: tracks validator information
CREATE TABLE IF NOT EXISTS validators (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    validator_id TEXT UNIQUE NOT NULL, -- Public key or SS58 address
    session_keys TEXT,                 -- Concatenated session keys (hex)
    aura_key TEXT,                     -- AURA session key
    grandpa_key TEXT,                  -- GRANDPA session key
    is_active BOOLEAN DEFAULT 1,
    first_seen_block INTEGER,
    last_validated_block INTEGER,
    total_blocks INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Validation events: records when our validator produced blocks
CREATE TABLE IF NOT EXISTS validation_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_number INTEGER NOT NULL,
    block_hash TEXT NOT NULL,
    validator_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    slot_number INTEGER,
    is_finalized BOOLEAN DEFAULT 0,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (block_number) REFERENCES blocks(block_number),
    INDEX idx_validator (validator_id),
    INDEX idx_timestamp (timestamp)
);

-- Sync status: tracks synchronization progress
CREATE TABLE IF NOT EXISTS sync_status (
    id INTEGER PRIMARY KEY CHECK (id = 1), -- Singleton table
    last_synced_block INTEGER NOT NULL,
    last_finalized_block INTEGER NOT NULL,
    tip_block INTEGER NOT NULL,
    is_syncing BOOLEAN DEFAULT 1,
    sync_started_at INTEGER,
    last_updated INTEGER NOT NULL
);

-- Network health: periodic health check snapshots
CREATE TABLE IF NOT EXISTS health_checks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    is_syncing BOOLEAN NOT NULL,
    peer_count INTEGER NOT NULL,
    best_block INTEGER NOT NULL,
    finalized_block INTEGER NOT NULL,
    should_have_peers BOOLEAN NOT NULL,
    response_time_ms INTEGER,
    INDEX idx_timestamp (timestamp)
);

-- Session keys: tracks session key changes
CREATE TABLE IF NOT EXISTS session_key_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    validator_id TEXT NOT NULL,
    session_keys TEXT NOT NULL,
    set_at_block INTEGER,
    set_at_timestamp INTEGER NOT NULL,
    is_current BOOLEAN DEFAULT 1,
    INDEX idx_validator (validator_id)
);
```

---

## 4. RPC Methods Reference

### 4.1 Core Substrate RPC Methods

Based on Midnight's Substrate implementation:

#### Chain Methods
```rust
// Get block by hash or latest
chain_getBlock(hash?: Hash) -> SignedBlock

// Get block header
chain_getHeader(hash?: Hash) -> Header

// Get block hash by number
chain_getBlockHash(number?: BlockNumber) -> Hash

// Get finalized head
chain_getFinalizedHead() -> Hash

// Get current best head
chain_getHead() -> Hash

// Subscribe to new heads (WebSocket)
chain_subscribeNewHeads() -> Header

// Subscribe to finalized heads (WebSocket)
chain_subscribeFinalizedHeads() -> Header
```

#### System Methods
```rust
// Get node health
system_health() -> Health {
    isSyncing: bool,
    peers: u32,
    shouldHavePeers: bool
}

// Get chain name
system_chain() -> String

// Get sync state
system_syncState() -> SyncState {
    startingBlock: u64,
    currentBlock: u64,
    highestBlock: u64
}

// Get node version
system_version() -> String

// Get chain properties
system_properties() -> ChainProperties
```

#### Author Methods (Validator-specific)
```rust
// Check if node has a specific key
author_hasKey(publicKey: Bytes, keyType: String) -> bool

// Check if node has full session keys
author_hasSessionKeys(sessionKeys: Bytes) -> bool

// Rotate session keys (generates new keys)
author_rotateKeys() -> Bytes

// Get pending extrinsics
author_pendingExtrinsics() -> Vec<Extrinsic>
```

### 4.2 Block Structure

```rust
// Header structure from chain_getHeader
struct Header {
    parent_hash: Hash,
    number: BlockNumber,      // Block height
    state_root: Hash,
    extrinsics_root: Hash,
    digest: Digest {
        logs: Vec<DigestItem>  // Contains AURA slot, seal, etc.
    }
}

// Full block from chain_getBlock
struct SignedBlock {
    block: Block {
        header: Header,
        extrinsics: Vec<Extrinsic>
    },
    justifications: Option<Justifications>
}

// Digest log types (in digest.logs)
- PreRuntime: Contains slot number for AURA
- Seal: Block signature
- Consensus: Consensus messages
```

### 4.3 Extracting Block Author

The block author is embedded in the digest logs:

```rust
// AURA pre-runtime digest contains slot info
// Format: 0x06617572612[SLOT_NUMBER_HEX]
// "aura" = 0x61757261

// To find author:
// 1. Parse digest logs for PreRuntime AURA entry
// 2. Extract slot number
// 3. Query runtime: chain.getBlock() -> extrinsics[0] 
//    (first extrinsic is usually inherent with author)
// 4. Or use state_getStorage to query session validators
```

---

## 5. Command Specifications

### 5.1 `mvm sync`

**Purpose**: Continuously synchronize blocks from Midnight node to local database

**Usage**:
```bash
mvm sync --rpc-url <URL> [OPTIONS]
```

**Options**:
```
--rpc-url <URL>           WebSocket RPC endpoint (default: wss://rpc.testnet-02.midnight.network)
--http-rpc <URL>          HTTP RPC endpoint for queries (default: https://rpc.ankr.com/midnight_testnet/)
--start-block <NUMBER>    Block number to start sync from (default: 0)
--batch-size <SIZE>       Blocks to fetch per batch (default: 100)
--db-path <PATH>          SQLite database path (default: ./mvm.db)
--finalized-only          Only sync finalized blocks
--poll-interval <SECS>    Seconds between new block checks (default: 6)
--prune-older-than <DAYS> Delete blocks older than N days (default: never)
--log-level <LEVEL>       Logging level (default: info)
```

**Behavior**:
1. Connect to Midnight node via WebSocket
2. Check `sync_status` table for last synced block
3. Subscribe to `chain_subscribeNewHeads()` for real-time updates
4. Fetch missing blocks in batches using `chain_getBlock()`
5. Parse block headers and extract:
   - Block number, hash, parent hash
   - Timestamp (from extrinsics or system time)
   - Block author (from digest logs)
   - Extrinsics count
6. Store blocks in `blocks` table
7. Update `sync_status` periodically
8. Subscribe to `chain_subscribeFinalizedHeads()` to mark finalized blocks
9. Log progress every 100 blocks

**Output**:
```
2026-01-15 10:23:45 INFO  Starting sync from block 1000000
2026-01-15 10:23:45 INFO  Connected to wss://rpc.testnet-02.midnight.network
2026-01-15 10:23:46 INFO  Current tip: 1050000, finalized: 1049800
2026-01-15 10:23:50 INFO  Synced blocks 1000000-1000100 (100 blocks in 4.2s)
2026-01-15 10:24:00 INFO  Synced blocks 1000100-1000200 (100 blocks in 4.1s)
...
2026-01-15 10:30:00 INFO  Sync complete. Tip: 1050000, DB blocks: 50000
2026-01-15 10:30:00 INFO  Waiting for new blocks...
2026-01-15 10:30:06 INFO  New block: 1050001 (hash: 0x1a2b3c..., author: 5GrwD...)
```

### 5.2 `mvm validate`

**Purpose**: Validate that blocks in database exist on-chain

**Usage**:
```bash
mvm validate --rpc-url <URL> [OPTIONS]
```

**Options**:
```
--rpc-url <URL>          WebSocket RPC endpoint
--db-path <PATH>         SQLite database path (default: ./mvm.db)
--start-block <NUMBER>   Start validation from block N
--end-block <NUMBER>     End validation at block N
--batch-size <SIZE>      Validate N blocks per batch (default: 100)
--fix-mismatches         Update DB if on-chain data differs
--check-finality         Verify finalized status
--validator-id <ID>      Only validate blocks from specific validator
```

**Behavior**:
1. Query `blocks` table for blocks in range
2. For each block:
   - Call `chain_getBlockHash(number)` to get on-chain hash
   - Compare with stored hash
   - If `--check-finality`: verify against `chain_getFinalizedHead()`
3. Report mismatches (orphaned blocks, reorgs, etc.)
4. If `--fix-mismatches`: update or delete incorrect blocks

**Output**:
```
2026-01-15 10:35:00 INFO  Validating blocks 1000000-1050000
2026-01-15 10:35:10 INFO  Validated 10000 blocks
2026-01-15 10:35:20 WARN  Block 1020304 hash mismatch!
                          DB:    0x1a2b3c4d...
                          Chain: 0x5e6f7g8h...
2026-01-15 10:35:20 INFO  Block 1020304 was orphaned (reorg detected)
2026-01-15 10:35:30 INFO  Validation complete: 50000 blocks checked
                          - Valid: 49999
                          - Mismatches: 1 (orphaned)
                          - Finality verified: 49800
```

### 5.3 `mvm health`

**Purpose**: Monitor Midnight node health and connectivity

**Usage**:
```bash
mvm health --rpc-url <URL> [OPTIONS]
```

**Options**:
```
--rpc-url <URL>           WebSocket RPC endpoint
--http-rpc <URL>          HTTP RPC endpoint
--interval <SECS>         Check interval in seconds (default: 30)
--db-path <PATH>          Store health checks in DB (optional)
--alert-on-sync           Alert if node falls behind
--alert-threshold <N>     Alert if behind by N blocks (default: 100)
--format <FORMAT>         Output format: text|json (default: text)
```

**Behavior**:
1. Call `system_health()` to get node status
2. Call `system_syncState()` to check sync progress
3. Call `chain_getFinalizedHead()` and `chain_getHead()` for tip info
4. Measure response time for each call
5. Store in `health_checks` table if `--db-path` provided
6. Alert if node is syncing or behind
7. Repeat every `--interval` seconds

**Output**:
```
2026-01-15 10:40:00 INFO  Node Health Check
├─ Status: Healthy ✓
├─ Syncing: No
├─ Peers: 47
├─ Best Block: 1050234
├─ Finalized Block: 1050200 (34 blocks behind)
├─ Response Time: 45ms
└─ Timestamp: 2026-01-15 10:40:00 UTC

2026-01-15 10:40:30 WARN  Node Health Check
├─ Status: Syncing ⚠
├─ Syncing: Yes (49850 / 50000)
├─ Peers: 12
├─ Best Block: 1049850
├─ Finalized Block: 1049800 (50 blocks behind)
├─ Response Time: 120ms
└─ Alert: Node is 384 blocks behind!
```

### 5.4 `mvm track`

**Purpose**: Track validator performance and block production

**Usage**:
```bash
mvm track --validator-id <ID> [OPTIONS]
```

**Options**:
```
--validator-id <ID>       Validator public key or SS58 address (required)
--rpc-url <URL>           WebSocket RPC endpoint
--db-path <PATH>          SQLite database path (default: ./mvm.db)
--session-keys <KEYS>     Hex-encoded session keys to track
--check-keys              Verify node has validator keys via author_hasKey
--alert-on-miss           Alert if expected blocks are missed
--stats-interval <SECS>   Print stats every N seconds (default: 300)
--export-csv <FILE>       Export performance data to CSV
```

**Behavior**:
1. Subscribe to `chain_subscribeNewHeads()`
2. For each new block:
   - Parse block author from digest logs
   - If author matches `--validator-id`, record in `validation_events`
   - Update `validators` table with latest stats
3. If `--check-keys`: periodically call `author_hasKey()` for each session key
4. Calculate metrics:
   - Blocks produced in last 1h/24h/7d
   - Average time between blocks
   - Miss rate (if expected frequency known)
5. Print periodic stats report

**Output**:
```
2026-01-15 10:45:00 INFO  Tracking validator: 5GrwDkfLkfT3J...
2026-01-15 10:45:00 INFO  Session keys verified: AURA ✓, GRANDPA ✓

2026-01-15 10:45:06 INFO  Block produced: #1050300
2026-01-15 10:45:42 INFO  Block produced: #1050306
2026-01-15 10:46:18 INFO  Block produced: #1050312

--- Validator Performance (last 1 hour) ---
Blocks Produced:     58
Expected Blocks:     60 (estimate)
Success Rate:        96.7%
Average Block Time:  6.2s
Last Block:          #1050312 (18s ago)
Status:              Active ✓
-------------------------------------------
```

### 5.5 `mvm query`

**Purpose**: Query block and validator data from local database

**Usage**:
```bash
mvm query <SUBCOMMAND> [OPTIONS]
```

**Subcommands**:

#### `mvm query blocks`
```bash
mvm query blocks --from <N> --to <N> [--validator <ID>] [--format json]
```
List blocks in range, optionally filtered by validator

#### `mvm query validator`
```bash
mvm query validator --id <ID> [--stats] [--history]
```
Show validator information and statistics

#### `mvm query stats`
```bash
mvm query stats [--period <1h|24h|7d|30d>]
```
Display aggregate statistics (blocks per hour, validator participation, etc.)

#### `mvm query gaps`
```bash
mvm query gaps [--auto-fill]
```
Find missing blocks in database (gaps in block numbers)

**Output Examples**:

```
# mvm query blocks --from 1050000 --to 1050010

Block Range: 1050000 - 1050010
┌───────────┬──────────────────┬──────────────────┬────────────────┐
│ Number    │ Hash             │ Author           │ Timestamp      │
├───────────┼──────────────────┼──────────────────┼────────────────┤
│ 1050000   │ 0x1a2b3c4d...    │ 5GrwDkfLkfT...   │ 10:00:00       │
│ 1050001   │ 0x2b3c4d5e...    │ 5HpG9w8EBL...    │ 10:00:06       │
│ 1050002   │ 0x3c4d5e6f...    │ 5GrwDkfLkfT...   │ 10:00:12       │
│ ...       │ ...              │ ...              │ ...            │
└───────────┴──────────────────┴──────────────────┴────────────────┘
```

```
# mvm query validator --id 5GrwDkfLkfT3J... --stats

Validator: 5GrwDkfLkfT3J...
Session Keys: 0x04ab5c3d... (AURA), 0x7f8e9d... (GRANDPA)
Status: Active

Statistics:
├─ Total Blocks:     12,456
├─ First Block:      #850000 (2025-12-01)
├─ Last Block:       #1050234 (18s ago)
├─ Avg Block Time:   6.1s
│
├─ Last 1 Hour:      58 blocks (96.7% rate)
├─ Last 24 Hours:    1,392 blocks (97.2% rate)
├─ Last 7 Days:      9,745 blocks (96.8% rate)
└─ All Time:         12,456 blocks
```

### 5.6 `mvm session-keys`

**Purpose**: Manage and verify validator session keys

**Usage**:
```bash
mvm session-keys <SUBCOMMAND> [OPTIONS]
```

**Subcommands**:

#### `mvm session-keys verify`
```bash
mvm session-keys verify --validator-id <ID> --session-keys <KEYS>
```
Verify node has the specified session keys using `author_hasKey()`

#### `mvm session-keys rotate`
```bash
mvm session-keys rotate --rpc-url <URL>
```
Generate new session keys via `author_rotateKeys()` (requires unsafe RPC)

#### `mvm session-keys track`
```bash
mvm session-keys track --validator-id <ID>
```
Monitor session key changes over time (from `session_key_history` table)

**Output**:
```
# mvm session-keys verify --validator-id 5GrwD... --session-keys 0x04ab5c...

Verifying session keys for validator 5GrwD...

AURA Key:    0x04ab5c3d... ✓ Present in keystore
GRANDPA Key: 0x7f8e9d... ✓ Present in keystore

Status: All session keys verified successfully
```

---

## 6. Implementation Details

### 6.1 Project Structure

```
mvm/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library exports
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── sync.rs          # Sync command
│   │   ├── validate.rs      # Validate command
│   │   ├── health.rs        # Health command
│   │   ├── track.rs         # Track command
│   │   ├── query.rs         # Query command
│   │   └── session_keys.rs  # Session keys command
│   ├── rpc/
│   │   ├── mod.rs
│   │   ├── client.rs        # JSON-RPC client
│   │   ├── types.rs         # RPC type definitions
│   │   └── methods.rs       # RPC method wrappers
│   ├── db/
│   │   ├── mod.rs
│   │   ├── schema.rs        # Database schema & migrations
│   │   ├── blocks.rs        # Block operations
│   │   ├── validators.rs    # Validator operations
│   │   └── queries.rs       # Query helpers
│   ├── types/
│   │   ├── mod.rs
│   │   ├── block.rs         # Block types
│   │   ├── validator.rs     # Validator types
│   │   └── health.rs        # Health types
│   └── utils/
│       ├── mod.rs
│       ├── logger.rs        # Logging setup
│       ├── crypto.rs        # Cryptographic helpers
│       └── time.rs          # Time utilities
├── tests/
│   ├── integration_test.rs
│   └── rpc_test.rs
└── README.md
```

### 6.2 Key Dependencies (Cargo.toml)

```toml
[package]
name = "midnight-validator-monitor"
version = "0.1.0"
edition = "2021"

[dependencies]
# CLI
clap = { version = "4.4", features = ["derive", "cargo"] }

# Async runtime
tokio = { version = "1.35", features = ["full"] }

# RPC client
jsonrpsee = { version = "0.21", features = ["ws-client", "http-client"] }

# Database
rusqlite = { version = "0.30", features = ["bundled"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
hex = "0.4"

# Substrate/Polkadot types
sp-core = "28.0"
sp-runtime = "31.0"
scale-codec = { package = "parity-scale-codec", version = "3.6", features = ["derive"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Time
chrono = "0.4"

# Utilities
futures = "0.3"
```

### 6.3 RPC Client Implementation

```rust
// src/rpc/client.rs
use jsonrpsee::{
    core::client::ClientT,
    ws_client::{WsClient, WsClientBuilder},
    http_client::{HttpClient, HttpClientBuilder},
};
use anyhow::Result;

pub struct MidnightRpcClient {
    ws_client: WsClient,
    http_client: Option<HttpClient>,
}

impl MidnightRpcClient {
    pub async fn new(ws_url: &str, http_url: Option<&str>) -> Result<Self> {
        let ws_client = WsClientBuilder::default()
            .build(ws_url)
            .await?;
        
        let http_client = if let Some(url) = http_url {
            Some(HttpClientBuilder::default().build(url)?)
        } else {
            None
        };
        
        Ok(Self { ws_client, http_client })
    }
    
    // Chain methods
    pub async fn get_block(&self, hash: Option<Hash>) -> Result<SignedBlock> {
        self.ws_client.request("chain_getBlock", rpc_params![hash]).await
    }
    
    pub async fn get_block_hash(&self, number: Option<u64>) -> Result<Hash> {
        self.ws_client.request("chain_getBlockHash", rpc_params![number]).await
    }
    
    pub async fn get_finalized_head(&self) -> Result<Hash> {
        self.ws_client.request("chain_getFinalizedHead", rpc_params![]).await
    }
    
    pub async fn subscribe_new_heads(&self) -> Result<Subscription<Header>> {
        self.ws_client.subscribe(
            "chain_subscribeNewHeads",
            rpc_params![],
            "chain_unsubscribeNewHeads"
        ).await
    }
    
    // System methods
    pub async fn system_health(&self) -> Result<Health> {
        self.ws_client.request("system_health", rpc_params![]).await
    }
    
    pub async fn system_sync_state(&self) -> Result<SyncState> {
        self.ws_client.request("system_syncState", rpc_params![]).await
    }
    
    // Author methods (validator)
    pub async fn author_has_key(&self, public_key: &str, key_type: &str) -> Result<bool> {
        self.ws_client.request("author_hasKey", rpc_params![public_key, key_type]).await
    }
    
    pub async fn author_has_session_keys(&self, session_keys: &str) -> Result<bool> {
        self.ws_client.request("author_hasSessionKeys", rpc_params![session_keys]).await
    }
}
```

### 6.4 Block Author Extraction

```rust
// src/utils/block.rs
use sp_runtime::generic::DigestItem;

pub fn extract_block_author(header: &Header) -> Option<String> {
    // Iterate through digest logs
    for log in &header.digest.logs {
        match log {
            DigestItem::PreRuntime(engine_id, data) => {
                // Check for AURA consensus (0x61757261 = "aura")
                if engine_id == b"aura" {
                    // Parse AURA pre-digest
                    // Format: [SLOT_NUMBER (8 bytes)]
                    let slot = u64::from_le_bytes(data[0..8].try_into().ok()?);
                    
                    // Note: Actual author extraction requires:
                    // 1. Query current epoch validators
                    // 2. Calculate: author_index = slot % validator_count
                    // 3. Return validators[author_index]
                    
                    // For now, return slot info
                    return Some(format!("slot_{}", slot));
                }
            }
            DigestItem::Seal(engine_id, signature) => {
                // AURA seal contains signature
                // Could extract public key from signature
            }
            _ => continue,
        }
    }
    None
}

// More robust: query runtime storage
pub async fn get_block_author_from_runtime(
    client: &MidnightRpcClient,
    block_hash: Hash
) -> Result<Option<String>> {
    // Query state_getStorage for author at specific block
    // Storage key: System.Events or Session.Validators
    // This requires understanding Midnight's runtime storage layout
    
    // Fallback: parse first extrinsic (timestamp inherent usually includes author)
    let block = client.get_block(Some(block_hash)).await?;
    
    // First extrinsic is usually set_timestamp inherent
    // Author may be in second or third extrinsic
    // Need to decode extrinsics to find author info
    
    Ok(None) // Placeholder
}
```

### 6.5 Database Operations

```rust
// src/db/blocks.rs
use rusqlite::{Connection, params};
use anyhow::Result;

pub struct BlockStore {
    conn: Connection,
}

impl BlockStore {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        Self::create_tables(&conn)?;
        Ok(Self { conn })
    }
    
    fn create_tables(conn: &Connection) -> Result<()> {
        conn.execute_batch(include_str!("../../schema.sql"))?;
        Ok(())
    }
    
    pub fn insert_block(&self, block: &Block) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO blocks 
             (block_number, block_hash, parent_hash, state_root, extrinsics_root, 
              timestamp, is_finalized, author_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                block.number,
                block.hash,
                block.parent_hash,
                block.state_root,
                block.extrinsics_root,
                block.timestamp,
                block.is_finalized,
                block.author_id,
                chrono::Utc::now().timestamp()
            ]
        )?;
        Ok(())
    }
    
    pub fn get_last_synced_block(&self) -> Result<Option<u64>> {
        let mut stmt = self.conn.prepare(
            "SELECT last_synced_block FROM sync_status WHERE id = 1"
        )?;
        let result = stmt.query_row([], |row| row.get(0)).optional()?;
        Ok(result)
    }
    
    pub fn update_sync_status(&self, last_synced: u64, tip: u64, finalized: u64) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO sync_status 
             (id, last_synced_block, last_finalized_block, tip_block, last_updated)
             VALUES (1, ?1, ?2, ?3, ?4)",
            params![last_synced, finalized, tip, chrono::Utc::now().timestamp()]
        )?;
        Ok(())
    }
    
    pub fn mark_finalized(&self, block_number: u64) -> Result<()> {
        self.conn.execute(
            "UPDATE blocks SET is_finalized = 1 WHERE block_number <= ?1",
            params![block_number]
        )?;
        Ok(())
    }
}
```

---

## 7. Configuration File

### 7.1 `mvm.toml` (Optional configuration)

```toml
# Midnight Validator Monitor Configuration

[network]
# Default RPC endpoints
ws_rpc_url = "wss://rpc.testnet-02.midnight.network"
http_rpc_url = "https://rpc.ankr.com/midnight_testnet/"

# Fallback endpoints
fallback_urls = [
    "wss://midnight-testnet.blockdaemon.com",
    "wss://midnight.ankr.com/ws"
]

[sync]
# Sync configuration
start_block = 0              # Start from genesis
batch_size = 100             # Blocks per batch
poll_interval = 6            # Seconds between checks (1 block time)
finalized_only = false       # Sync all blocks or only finalized
prune_older_than_days = 0    # Disable pruning (0 = never)

[database]
# Database settings
path = "./mvm.db"
max_connections = 10
wal_mode = true              # Write-Ahead Logging for performance

[validator]
# Your validator configuration
validator_id = ""            # Your validator SS58 address
session_keys = ""            # Hex-encoded session keys
check_keys_interval = 300    # Check session keys every 5 minutes

[health]
# Health monitoring
check_interval = 30          # Seconds between health checks
alert_threshold = 100        # Alert if behind by N blocks
store_history = true         # Store health checks in DB

[logging]
# Logging configuration
level = "info"               # trace, debug, info, warn, error
format = "text"              # text or json
file = ""                    # Log to file (empty = stdout only)
max_file_size_mb = 100
max_backups = 10

[metrics]
# Prometheus metrics (future feature)
enable = false
listen_addr = "127.0.0.1:9615"
```

---

## 8. Error Handling

### 8.1 Error Types

```rust
// src/types/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MvmError {
    #[error("RPC error: {0}")]
    Rpc(#[from] jsonrpsee::core::Error),
    
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    
    #[error("Block not found: {0}")]
    BlockNotFound(String),
    
    #[error("Sync error: {0}")]
    SyncError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Node unreachable: {0}")]
    NodeUnreachable(String),
    
    #[error("Invalid configuration: {0}")]
    Config(String),
    
    #[error("Invalid session keys: {0}")]
    InvalidSessionKeys(String),
}

pub type Result<T> = std::result::Result<T, MvmError>;
```

### 8.2 Retry Logic

```rust
// src/utils/retry.rs
use tokio::time::{sleep, Duration};
use anyhow::Result;

pub async fn retry_with_backoff<F, T, E>(
    mut f: F,
    max_retries: u32,
    initial_backoff: Duration,
) -> Result<T>
where
    F: FnMut() -> Result<T, E>,
    E: std::fmt::Display,
{
    let mut retries = 0;
    let mut backoff = initial_backoff;
    
    loop {
        match f() {
            Ok(result) => return Ok(result),
            Err(err) if retries < max_retries => {
                tracing::warn!("Attempt {} failed: {}. Retrying in {:?}...", 
                              retries + 1, err, backoff);
                sleep(backoff).await;
                retries += 1;
                backoff *= 2; // Exponential backoff
            }
            Err(err) => {
                anyhow::bail!("Failed after {} retries: {}", max_retries, err);
            }
        }
    }
}
```

---

## 9. Testing Strategy

### 9.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_block_author_extraction() {
        // Mock header with AURA digest
        let header = Header {
            digest: Digest {
                logs: vec![
                    DigestItem::PreRuntime(b"aura".to_vec(), vec![0u8; 8])
                ]
            },
            // ... other fields
        };
        
        let author = extract_block_author(&header);
        assert!(author.is_some());
    }
    
    #[tokio::test]
    async fn test_database_operations() {
        let store = BlockStore::new(":memory:").unwrap();
        
        let block = Block {
            number: 1000,
            hash: "0x123...".to_string(),
            // ... other fields
        };
        
        store.insert_block(&block).unwrap();
        
        let retrieved = store.get_block(1000).unwrap();
        assert_eq!(retrieved.hash, block.hash);
    }
}
```

### 9.2 Integration Tests

```rust
// tests/integration_test.rs
use midnight_validator_monitor::*;

#[tokio::test]
async fn test_sync_workflow() {
    // Test full sync workflow with mock RPC
    // 1. Connect to test node
    // 2. Sync blocks
    // 3. Verify storage
}

#[tokio::test]
async fn test_validation_workflow() {
    // Test validation with known blocks
}
```

### 9.3 Mock RPC Server

For testing without live node:

```rust
// tests/mock_rpc.rs
use jsonrpsee::server::{ServerBuilder, ServerHandle};
use std::net::SocketAddr;

pub async fn start_mock_rpc_server() -> (ServerHandle, SocketAddr) {
    let server = ServerBuilder::default()
        .build("127.0.0.1:0")
        .await
        .unwrap();
    
    let addr = server.local_addr().unwrap();
    
    let mut module = RpcModule::new(());
    module.register_method("chain_getBlock", |_, _| {
        // Return mock block
        Ok(mock_block())
    }).unwrap();
    
    let handle = server.start(module).unwrap();
    (handle, addr)
}
```

---

## 10. Deployment & Operations

### 10.1 Installation

```bash
# From source
git clone https://github.com/your-org/midnight-validator-monitor
cd midnight-validator-monitor
cargo build --release
sudo cp target/release/mvm /usr/local/bin/

# Or via cargo
cargo install midnight-validator-monitor
```

### 10.2 Systemd Service for Sync

```ini
# /etc/systemd/system/mvm-sync.service
[Unit]
Description=Midnight Validator Monitor - Sync Service
After=network.target

[Service]
Type=simple
User=midnight
ExecStart=/usr/local/bin/mvm sync \
    --rpc-url wss://rpc.testnet-02.midnight.network \
    --db-path /var/lib/mvm/mvm.db \
    --log-level info
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

### 10.3 Systemd Service for Health Monitoring

```ini
# /etc/systemd/system/mvm-health.service
[Unit]
Description=Midnight Validator Monitor - Health Check
After=network.target

[Service]
Type=simple
User=midnight
ExecStart=/usr/local/bin/mvm health \
    --rpc-url wss://rpc.testnet-02.midnight.network \
    --interval 30 \
    --db-path /var/lib/mvm/mvm.db \
    --alert-threshold 100
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

### 10.4 Cron Job for Validation

```bash
# /etc/cron.d/mvm-validate
# Validate blocks daily at 2 AM
0 2 * * * midnight /usr/local/bin/mvm validate --rpc-url wss://rpc.testnet-02.midnight.network --db-path /var/lib/mvm/mvm.db >> /var/log/mvm/validate.log 2>&1
```

---

## 11. Performance Considerations

### 11.1 Optimization Strategies

1. **Batch Processing**: Fetch blocks in batches (default 100)
2. **Concurrent Requests**: Use tokio to parallelize RPC calls
3. **Database Indexing**: Ensure proper indexes on `block_number`, `block_hash`, `timestamp`
4. **WAL Mode**: Enable Write-Ahead Logging in SQLite for better concurrent writes
5. **Connection Pooling**: Reuse RPC connections
6. **Pruning**: Optionally delete old blocks to limit database size

### 11.2 Expected Performance

- **Sync Speed**: ~500-1000 blocks/second (network dependent)
- **Database Size**: ~100KB per 1000 blocks (header only)
- **Memory Usage**: <100MB typical
- **CPU Usage**: <5% on modern hardware

---

## 12. Future Enhancements

### 12.1 Phase 2 Features (Post-MVP)

1. **Prometheus Metrics**: Export metrics for Grafana dashboards
2. **Alerting System**: Email/Telegram/Slack notifications
3. **Web Dashboard**: Real-time web UI for monitoring
4. **Historical Analytics**: Block time trends, validator performance graphs
5. **Reorg Detection**: Automated detection and handling of chain reorganizations
6. **Multi-Validator Support**: Track multiple validators simultaneously
7. **GraphQL API**: Query interface for third-party integrations

### 12.2 Phase 3 Features (Advanced)

1. **Predictive Analytics**: ML-based performance prediction
2. **Automated Actions**: Auto-restart node on issues
3. **Cross-Chain Integration**: Monitor Cardano SPO status alongside Midnight
4. **Plugin System**: Extensible architecture for custom modules
5. **HA Clustering**: Multi-instance coordination for high availability

---

## 13. Security Considerations

1. **RPC Credentials**: Support authentication for private RPC endpoints
2. **Database Encryption**: Optional SQLCipher for encrypted database
3. **Secure Key Storage**: Never log or store private keys
4. **Rate Limiting**: Respect RPC provider rate limits
5. **Input Validation**: Sanitize all user inputs and RPC responses

---

## 14. Documentation Requirements

### 14.1 User Documentation

- Installation guide (Linux, macOS, Windows)
- Configuration reference
- Command reference with examples
- Troubleshooting guide
- FAQ

### 14.2 Developer Documentation

- Architecture overview
- API reference (generated from code)
- Contributing guidelines
- Database schema documentation
- RPC methods reference

---

## 15. Success Criteria

### 15.1 MVP Requirements

- [x] Sync blocks from Midnight node to SQLite
- [x] Validate blocks against on-chain data
- [x] Monitor node health
- [x] Track validator block production
- [x] Query block/validator data
- [x] Verify session keys

### 15.2 Quality Metrics

- 95%+ test coverage for core modules
- <100ms P95 latency for database queries
- <1% sync drift vs. chain tip
- Zero data loss during normal operation
- Successful recovery from network interruptions

---

## 16. Timeline Estimate

### Phase 1: MVP (8-10 weeks)
- Week 1-2: Project setup, RPC client, database schema
- Week 3-4: Sync command implementation
- Week 5-6: Validate, health, track commands
- Week 7-8: Query command, testing
- Week 9-10: Documentation, bug fixes, release

### Phase 2: Enhancements (4-6 weeks)
- Prometheus metrics
- Alerting system
- Performance optimizations

### Phase 3: Advanced Features (6-8 weeks)
- Web dashboard
- Analytics
- Multi-validator support

---

## 17. References

### 17.1 Midnight Resources
- Midnight Documentation: https://docs.midnight.network
- Midnight Node GitHub: https://github.com/midnightntwrk/midnight-node
- Midnight RPC Endpoints: https://www.ankr.com/docs/rpc-service/chains/chains-api/midnight/

### 17.2 Technical References
- Substrate RPC: https://polkadot.js.org/docs/substrate/rpc/
- Polkadot Types: https://polkadot.js.org/docs/types
- AURA Consensus: https://paritytech.github.io/substrate/master/sc_consensus_aura/
- GRANDPA Finality: https://github.com/w3f/consensus

### 17.3 Similar Tools
- cncli (Cardano): https://github.com/cardano-community/cncli
- Substrate Archive: https://github.com/paritytech/substrate-archive
- Polkadot Telemetry: https://github.com/paritytech/substrate-telemetry

---

## Appendix A: Example SQL Queries

```sql
-- Get blocks produced by validator in last 24 hours
SELECT COUNT(*) as block_count
FROM blocks
WHERE author_id = '5GrwDkfLkfT3J...'
  AND timestamp >= strftime('%s', 'now', '-24 hours');

-- Find gaps in block sequence
SELECT a.block_number + 1 AS gap_start,
       MIN(b.block_number) - 1 AS gap_end
FROM blocks a
LEFT JOIN blocks b ON a.block_number < b.block_number
WHERE NOT EXISTS (
    SELECT 1 FROM blocks c
    WHERE c.block_number = a.block_number + 1
)
GROUP BY a.block_number
HAVING gap_end >= gap_start;

-- Validator performance by hour
SELECT 
    strftime('%Y-%m-%d %H:00', datetime(timestamp, 'unixepoch')) as hour,
    validator_id,
    COUNT(*) as blocks_produced
FROM validation_events
WHERE timestamp >= strftime('%s', 'now', '-7 days')
GROUP BY hour, validator_id
ORDER BY hour DESC, blocks_produced DESC;

-- Average block time
SELECT 
    AVG(b2.timestamp - b1.timestamp) as avg_block_time_seconds
FROM blocks b1
JOIN blocks b2 ON b2.block_number = b1.block_number + 1
WHERE b1.block_number >= (SELECT MAX(block_number) - 1000 FROM blocks);
```

---

## Appendix B: Environment Variables

```bash
# RPC endpoints
export MVM_WS_RPC_URL="wss://rpc.testnet-02.midnight.network"
export MVM_HTTP_RPC_URL="https://rpc.ankr.com/midnight_testnet/"

# Database
export MVM_DB_PATH="/var/lib/mvm/mvm.db"

# Validator
export MVM_VALIDATOR_ID="5GrwDkfLkfT3J..."
export MVM_SESSION_KEYS="0x04ab5c3d..."

# Logging
export MVM_LOG_LEVEL="info"
export RUST_LOG="midnight_validator_monitor=info"

# Credentials (if needed)
export MVM_RPC_USERNAME="admin"
export MVM_RPC_PASSWORD="secret"
```

---

## Appendix C: Quick Start Guide

```bash
# 1. Install MVM
cargo install midnight-validator-monitor

# 2. Initialize database
mvm init --db-path ./mvm.db

# 3. Start syncing blocks
mvm sync \
  --rpc-url wss://rpc.testnet-02.midnight.network \
  --db-path ./mvm.db

# 4. In another terminal, monitor health
mvm health \
  --rpc-url wss://rpc.testnet-02.midnight.network \
  --interval 30

# 5. Track your validator
mvm track \
  --validator-id YOUR_VALIDATOR_ID \
  --session-keys YOUR_SESSION_KEYS \
  --db-path ./mvm.db

# 6. Query statistics
mvm query stats --period 24h
mvm query validator --id YOUR_VALIDATOR_ID --stats
```

---

**END OF SPECIFICATION**

---

## Notes for Implementation

This specification provides a comprehensive blueprint for building MVM. Key implementation priorities:

1. **Start with core RPC client** - Get `chain_getBlock`, `chain_getHeader`, and subscriptions working first
2. **Build database layer** - Implement schema and basic CRUD operations
3. **Implement sync command** - This is the foundation for all other features
4. **Add validation and query** - Leverage the synced data
5. **Implement health and track** - These build on the sync infrastructure
6. **Polish and document** - Add error handling, logging, tests, and documentation

The specification deliberately avoids VRF-based leaderlog prediction since Midnight's consensus mechanism is fundamentally different from Cardano. Instead, it focuses on what's practical and valuable: accurate block tracking, validation, and performance monitoring for Midnight validators.