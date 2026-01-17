# Midnight Validator Monitor - Technical Specification v0.4

**Version**: 0.4-beta (Planning)
**Current Release**: v0.3.0-alpha
**Last Updated**: 2026-01-16
**Status**: Production-Ready Alpha → Production-Ready Beta

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [System Architecture](#2-system-architecture)
3. [Database Schema](#3-database-schema)
4. [Command Specifications](#4-command-specifications)
5. [TUI Architecture](#5-tui-architecture)
6. [Block Prediction Algorithm](#6-block-prediction-algorithm)
7. [Data Flow](#7-data-flow)
8. [RPC API Specifications](#8-rpc-api-specifications)
9. [Performance Targets](#9-performance-targets)
10. [Security Considerations](#10-security-considerations)
11. [Deployment Architecture](#11-deployment-architecture)
12. [Testing Strategy](#12-testing-strategy)

---

## 1. Executive Summary

### 1.1 Project Overview

**Midnight Validator Monitor (MVM)** is a production-ready Rust CLI tool for monitoring Midnight blockchain validator nodes. It provides comprehensive block synchronization, real-time status monitoring, interactive visualization, and performance analytics.

### 1.2 Current State (v0.3.0-alpha)

**Implemented Features**:
- ✅ Block synchronization with author attribution (185+ validators tracked)
- ✅ Interactive TUI with 5 views (Dashboard, Blocks, Validators, Performance, Help)
- ✅ Systemd daemon support with graceful shutdown
- ✅ TOML configuration system with multi-source priority
- ✅ Session key verification and validator registration checking
- ✅ Performance rankings and statistics
- ✅ SQLite database with blocks and validators tables
- ✅ Installation automation via scripts

**Production Deployments**: Active on Midnight partner chains network

### 1.3 Vision for v0.4-beta

**Primary Goal**: TUI Excellence - Professional, dynamic monitoring dashboard

**Key Enhancements**:
- Dynamic terminal scaling (80×24 to 200×60+)
- Enhanced dashboard with epoch progress and block predictions
- Historical health check tracking
- Block production visualization (sparklines)
- Stake-based prediction algorithm

**Target Users**:
- Midnight validator operators
- Node infrastructure teams
- Blockchain monitoring services
- DevOps teams managing validator fleets

### 1.4 Key Differences from Similar Tools

| Feature | MVM | cncli (Cardano) | Polkadot Telemetry |
|---------|-----|-----------------|-------------------|
| Consensus | AURA/GRANDPA | Ouroboros Praos | BABE/GRANDPA |
| Block Prediction | Deterministic (round-robin) | VRF-based leaderlog | N/A |
| Interactive TUI | ✅ Full-featured | ❌ CLI only | ✅ Web dashboard |
| Daemon Mode | ✅ Systemd integration | ✅ Service mode | ✅ Always-on server |
| Local Database | ✅ SQLite | ✅ SQLite | ❌ Remote only |
| Configuration | ✅ TOML + Env vars | ❌ CLI flags only | ✅ Config files |

---

## 2. System Architecture

### 2.1 High-Level Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│                    Midnight Validator Monitor (MVM)                   │
├──────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │                      Command Layer                              │  │
│  ├────────┬────────┬────────┬────────┬────────┬─────────┬─────────┤  │
│  │ status │  sync  │  query │  keys  │  view  │ config  │ (future)│  │
│  └────┬───┴───┬────┴───┬────┴───┬────┴───┬────┴────┬────┴─────────┘  │
│       │       │        │        │        │         │                  │
│  ┌────┴───────┴────────┴────────┴────────┴─────────┴──────────────┐  │
│  │                     Core Services Layer                         │  │
│  ├─────────────┬──────────────┬──────────────┬────────────────────┤  │
│  │ RPC Client  │ Config Mgmt  │ Daemon Mgmt  │ TUI Engine         │  │
│  │ (HTTP/WS)   │ (TOML+Env)   │ (Signals)    │ (ratatui)          │  │
│  └─────┬───────┴──────┬───────┴──────┬───────┴────────┬───────────┘  │
│        │              │              │                │              │
│  ┌─────┴──────────────┴──────────────┴────────────────┴───────────┐  │
│  │                   Data Layer (SQLite)                           │  │
│  ├──────────────┬──────────────┬────────────────┬─────────────────┤  │
│  │ blocks       │ validators   │ sync_status    │ health_checks   │  │
│  │ (3M+ rows)   │ (185 rows)   │ (singleton)    │ (time-series)   │  │
│  └──────────────┴──────────────┴────────────────┴─────────────────┘  │
│                                                                        │
└────────────────────────────────┬───────────────────────────────────┬──┘
                                 │                                   │
                     ┌───────────┴──────────┐          ┌────────────┴─────────┐
                     │   Midnight Node      │          │  Terminal (User)     │
                     │  localhost:9944      │          │  80×24 to 200×60+    │
                     │  (RPC + Metrics)     │          │  (crossterm)         │
                     └──────────────────────┘          └──────────────────────┘
```

### 2.2 Component Responsibilities

#### Command Layer
- **status**: Real-time node monitoring with periodic health checks
- **sync**: Continuous block synchronization with validator attribution
- **query**: Database queries for blocks, validators, performance analytics
- **keys**: Session key verification and registration status checking
- **view**: Interactive TUI for real-time visualization
- **config**: Configuration management (show, validate, example, paths)

#### Core Services Layer

**RPC Client** (`src/rpc/`):
- JSON-RPC 2.0 over HTTP with atomic request IDs
- Type-safe method calls with serde deserialization
- Connection pooling (planned)
- Retry logic with exponential backoff
- Timeout handling (30s default)

**Configuration Management** (`src/config.rs`):
- TOML parsing with serde
- Multi-source priority: CLI > Env > Config File > Defaults
- Search paths: `./mvm.toml`, `~/.config/mvm/config.toml`, `/opt/midnight/mvm/config/config.toml`, `/etc/mvm/config.toml`
- Validation and example generation

**Daemon Management** (`src/daemon.rs`):
- PID file creation with atomic writes
- Signal handling (SIGTERM, SIGINT, SIGQUIT) via tokio select!
- Graceful shutdown with database flush
- Automatic PID cleanup via Drop trait

**TUI Engine** (`src/tui/`):
- Event-driven architecture with ratatui 0.26
- Crossterm for cross-platform terminal control
- Keyboard event handling (Vim-style navigation)
- View state management
- Dynamic layout system (v0.4)

#### Data Layer
- **SQLite 3.x** with bundled build (no system dependency)
- **Write-Ahead Logging (WAL)** mode for concurrency
- **Indexes** on all frequently queried columns
- **Transactions** for batch operations
- **Foreign keys** disabled (performance optimization)

### 2.3 Technology Stack

| Component | Library/Tool | Version | Purpose |
|-----------|--------------|---------|---------|
| Language | Rust | 2021 edition | Systems programming |
| CLI Framework | clap | 4.x | Command-line parsing |
| HTTP Client | reqwest | 0.11 | RPC communication |
| Async Runtime | tokio | 1.x | Async I/O |
| Database | rusqlite | 0.30 | SQLite interface |
| Serialization | serde + serde_json | 1.0 | JSON handling |
| Configuration | toml | 0.8 | TOML parsing |
| TUI Framework | ratatui | 0.26 | Terminal UI |
| Terminal Control | crossterm | 0.27 | Cross-platform terminal |
| Signal Handling | signal-hook + signal-hook-tokio | 0.3 | Unix signals |
| Logging | tracing + tracing-subscriber | 0.1 / 0.3 | Structured logging |
| Error Handling | anyhow + thiserror | 1.0 | Error management |
| Time | chrono | 0.4 | Timestamps |
| Hex Encoding | hex | 0.4 | Digest parsing |
| Directories | directories | 5.0 | XDG paths |
| Process Management | nix | 0.27 | PID files |

**Total Dependencies**: ~40 crates (including transitive)
**Binary Size**: ~8 MB (release, stripped)
**MSRV**: Rust 1.70+

---

## 3. Database Schema

### 3.1 Current Schema (v0.3.0-alpha)

#### blocks Table
```sql
CREATE TABLE IF NOT EXISTS blocks (
    block_number INTEGER PRIMARY KEY,
    block_hash TEXT NOT NULL UNIQUE,
    parent_hash TEXT NOT NULL,
    state_root TEXT NOT NULL,
    extrinsics_root TEXT NOT NULL,
    slot_number INTEGER NOT NULL,
    epoch INTEGER NOT NULL,              -- Sidechain epoch for metadata
    timestamp INTEGER NOT NULL,          -- Unix timestamp (seconds)
    is_finalized INTEGER DEFAULT 0,      -- Boolean: 0 = no, 1 = yes
    author_key TEXT,                     -- Sidechain public key of block author
    extrinsics_count INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL          -- When record was inserted
);

CREATE INDEX IF NOT EXISTS idx_blocks_hash ON blocks(block_hash);
CREATE INDEX IF NOT EXISTS idx_blocks_slot ON blocks(slot_number);
CREATE INDEX IF NOT EXISTS idx_blocks_epoch ON blocks(epoch);
CREATE INDEX IF NOT EXISTS idx_blocks_author ON blocks(author_key);
CREATE INDEX IF NOT EXISTS idx_blocks_timestamp ON blocks(timestamp);
```

**Size Estimates**:
- Row size: ~300 bytes average
- 1M blocks ≈ 300 MB
- 10M blocks ≈ 3 GB

#### validators Table
```sql
CREATE TABLE IF NOT EXISTS validators (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sidechain_key TEXT UNIQUE NOT NULL,  -- Primary identifier
    aura_key TEXT,                        -- AURA session key
    grandpa_key TEXT,                     -- GRANDPA session key
    label TEXT,                           -- User-friendly name
    is_ours INTEGER DEFAULT 0,            -- Boolean: 1 = our validator
    registration_status TEXT,             -- 'permissioned', 'registered', etc.
    first_seen_epoch INTEGER,             -- Mainchain epoch when first detected
    total_blocks INTEGER DEFAULT 0,       -- Blocks produced (all-time)
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_validators_sidechain ON validators(sidechain_key);
CREATE INDEX IF NOT EXISTS idx_validators_ours ON validators(is_ours);
```

**Current Data**:
- ~185 validators (12 permissioned + 173 registered)
- Updated during sync as blocks are attributed

#### sync_status Table (Singleton)
```sql
CREATE TABLE IF NOT EXISTS sync_status (
    id INTEGER PRIMARY KEY CHECK (id = 1),  -- Singleton constraint
    last_synced_block INTEGER NOT NULL DEFAULT 0,
    last_finalized_block INTEGER NOT NULL DEFAULT 0,
    chain_tip_block INTEGER NOT NULL DEFAULT 0,
    current_epoch INTEGER NOT NULL DEFAULT 0,  -- Mainchain epoch
    is_syncing INTEGER DEFAULT 1,
    last_updated INTEGER NOT NULL
);

-- Initialize singleton
INSERT OR IGNORE INTO sync_status (id, last_synced_block, last_finalized_block,
    chain_tip_block, current_epoch, last_updated)
VALUES (1, 0, 0, 0, 0, 0);
```

### 3.2 New Tables for v0.4-beta

#### health_checks Table
```sql
CREATE TABLE IF NOT EXISTS health_checks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,            -- Unix timestamp
    is_healthy INTEGER NOT NULL,           -- Boolean: overall health
    is_syncing INTEGER NOT NULL,
    peer_count INTEGER NOT NULL,
    best_block INTEGER NOT NULL,
    finalized_block INTEGER NOT NULL,
    sync_percentage REAL,                  -- 0.0 to 100.0
    response_time_ms INTEGER               -- RPC response time
);

CREATE INDEX IF NOT EXISTS idx_health_timestamp ON health_checks(timestamp);
```

**Retention Policy**: Keep last 30 days (configurable)
**Size Estimate**: ~100 bytes/row, ~200k rows/month ≈ 20 MB/month

#### block_production_hourly Table (Optional - Performance Optimization)
```sql
-- Materialized view for efficient historical queries
CREATE TABLE IF NOT EXISTS block_production_hourly (
    validator_key TEXT NOT NULL,
    hour_timestamp INTEGER NOT NULL,  -- Unix timestamp rounded to hour start
    block_count INTEGER NOT NULL DEFAULT 0,
    expected_blocks REAL,             -- Based on stake allocation
    PRIMARY KEY (validator_key, hour_timestamp)
);

CREATE INDEX IF NOT EXISTS idx_production_validator ON block_production_hourly(validator_key);
CREATE INDEX IF NOT EXISTS idx_production_time ON block_production_hourly(hour_timestamp);
```

**Rationale**: Pre-aggregated data for sparkline rendering without scanning millions of blocks
**Update Strategy**: Incremental updates during sync

### 3.3 Database Configuration

**SQLite Pragmas** (set on connection):
```sql
PRAGMA journal_mode = WAL;              -- Write-Ahead Logging for concurrency
PRAGMA synchronous = NORMAL;            -- Balance safety and performance
PRAGMA cache_size = -64000;             -- 64 MB cache
PRAGMA temp_store = MEMORY;             -- Temp tables in memory
PRAGMA mmap_size = 30000000000;         -- 30 GB memory-mapped I/O
PRAGMA page_size = 4096;                -- Standard page size
```

**Connection Pool** (planned for v0.4):
- Max connections: 5
- Idle timeout: 30s
- Connection timeout: 5s

---

## 4. Command Specifications

### 4.1 status Command

**Purpose**: Real-time node monitoring with health checks

**Syntax**:
```bash
mvm status [OPTIONS]
```

**Options**:
```
--rpc-url <URL>        RPC endpoint (default: from config)
--metrics-url <URL>    Metrics endpoint (default: from config)
--keystore <PATH>      Keystore directory (default: from config)
--interval <SECS>      Check interval (default: 60)
--once                 Run once and exit
```

**Output**:
```
INFO Health: ✓ | Syncing: ✓ | Peers: 12
INFO Block: 3363965 | Finalized: 3363963 | Sync: 100.00%
INFO Sidechain: epoch 1179 slot 294763983 | Mainchain: epoch 1179 slot 101838307
INFO Keys: sidechain ✓ | aura ✓ | grandpa ✓
INFO Registration: ✓ Registered (valid)
```

**Exit Codes**:
- 0: Healthy
- 1: Unhealthy (peers < 3, not syncing, or keys not loaded)
- 2: RPC error

### 4.2 sync Command

**Purpose**: Continuous block synchronization with validator attribution

**Syntax**:
```bash
mvm sync [OPTIONS]
```

**Options**:
```
--rpc-url <URL>         RPC endpoint
--db-path <PATH>        Database file
--start-block <N>       Start from block number (default: auto-detect)
--batch-size <N>        Blocks per batch (default: 100)
--poll-interval <SECS>  Poll for new blocks (default: 6)
--finalized-only        Only sync finalized blocks
--daemon                Run as background daemon
--pid-file <PATH>       PID file location (daemon mode)
```

**Behavior**:
1. **Initial Sync**: Batch fetch from start_block to chain tip
2. **Polling Mode**: Poll every 6s for new blocks
3. **Validator Attribution**: Fetch validator set per mainchain epoch, calculate author
4. **Error Recovery**: Continue on RPC errors (log warning)
5. **Graceful Shutdown**: On SIGTERM/SIGINT, finish current batch and exit

**Performance**:
- Batch sync: ~50-100 blocks/second (network dependent)
- Polling overhead: <1% CPU when idle

### 4.3 query Command

**Purpose**: Query stored block and validator data

**Subcommands**:

**stats** - Database statistics
```bash
mvm query stats [--db-path <PATH>]
```

**blocks** - List blocks
```bash
mvm query blocks [--from <N>] [--to <N>] [--limit <N>]
```

**validators** - List validators
```bash
mvm query validators [--ours] [--limit <N>]
```

**validator** - Show specific validator
```bash
mvm query validator <SIDECHAIN_KEY>
```

**performance** - Performance rankings
```bash
mvm query performance [--ours] [--limit <N>]
```

**gaps** - Find missing blocks
```bash
mvm query gaps
```

### 4.4 keys Command

**Purpose**: Session key verification and registration checking

**Subcommands**:

**show** - Display keys from keystore
```bash
mvm keys show --keystore <PATH>
```

**verify** - Verify keys and check registration
```bash
mvm keys verify --keystore <PATH> [--rpc-url <URL>] [--db-path <PATH>]
```

**Output** (verify):
```
Key Status:
  Sidechain: ✓ Loaded in keystore
  Aura:      ✓ Loaded in keystore
  Grandpa:   ✓ Loaded in keystore

Registration Status:
  ✓ Registered (valid)

Block Production Statistics:
  Total blocks produced: 17
  First seen in epoch:   1179
  Share of synced blocks: 0.56%
  Performance rank:       #1 of 185 validators

  Recent blocks (last 1000):
    Block #3363612 (slot 294763613, epoch 1179)
    Block #3363792 (slot 294763798, epoch 1179)
    Block #3363965 (slot 294763983, epoch 1179)
```

**Note**: Requires `--rpc-methods=unsafe` on node for key verification via `author_hasKey`

### 4.5 view Command

**Purpose**: Interactive TUI for real-time monitoring

**Syntax**:
```bash
mvm view [OPTIONS]
```

**Options**:
```
--rpc-url <URL>            RPC endpoint
--db-path <PATH>           Database file
--refresh-interval <MS>    Update interval (default: 2000)
```

**Views**:
1. **Dashboard** - Comprehensive overview (default)
2. **Blocks** - Scrollable block list
3. **Validators** - All validators with stats
4. **Performance** - Top validators ranked
5. **Help** - Keyboard shortcuts

**Keyboard Controls**:
```
1-4         Switch views
j/k         Scroll down/up (Vim-style)
f           Toggle "ours only" filter
r           Force refresh
q/Esc       Quit
```

### 4.6 config Command

**Purpose**: Configuration management

**Subcommands**:

**show** - Display effective configuration
```bash
mvm config show
```

**validate** - Validate config file
```bash
mvm config validate [--config <PATH>]
```

**example** - Generate example config
```bash
mvm config example > mvm.toml
```

**paths** - Show config search paths
```bash
mvm config paths
```

---

## 5. TUI Architecture

### 5.1 Component Structure

```
src/tui/
├── mod.rs              # Module exports
├── app.rs              # Application state
├── event.rs            # Event handling
├── ui.rs               # View rendering
├── layout.rs           # Responsive layouts (v0.4)
├── resize.rs           # Terminal resize handling (v0.4)
├── theme.rs            # Color schemes
├── dashboard.rs        # Dashboard data & rendering (v0.4)
└── widgets/            # Custom widgets (v0.4)
    ├── mod.rs
    ├── sparkline.rs    # Block history visualization
    ├── progress_bar.rs # Epoch progress
    └── health_indicator.rs  # Health check display
```

### 5.2 Application State

```rust
pub struct App {
    // View state
    pub view_mode: ViewMode,
    pub should_quit: bool,
    pub scroll_offset: usize,
    pub filter_ours_only: bool,

    // Terminal state (v0.4)
    pub terminal_width: u16,
    pub terminal_height: u16,
    pub screen_size: ScreenSize,

    // Data (refreshed periodically)
    pub network_status: Option<NetworkStatus>,
    pub validator_status: Option<ValidatorStatus>,
    pub blocks: Vec<BlockRecord>,
    pub validators: Vec<ValidatorRecord>,
    pub health_history: Vec<HealthCheck>,     // v0.4
    pub block_history: Vec<BlockProduction>,   // v0.4
    pub epoch_progress: Option<EpochProgress>, // v0.4

    // Metadata
    pub last_update: Instant,
    pub refresh_interval: Duration,
}

pub enum ViewMode {
    Dashboard,
    Blocks,
    Validators,
    Performance,
    Help,
}

pub enum ScreenSize {
    Small,   // < 100 cols or < 30 rows
    Medium,  // 100-150 cols and 30-50 rows
    Large,   // > 150 cols or > 50 rows
}
```

### 5.3 Event Loop

```rust
pub async fn run_tui(
    rpc_url: String,
    db_path: PathBuf,
    refresh_interval_ms: u64,
) -> Result<()> {
    // Initialize terminal
    let mut terminal = setup_terminal()?;
    let mut app = App::new(refresh_interval_ms);

    // Event channels
    let (tx, mut rx) = mpsc::channel(100);

    // Spawn event handler thread
    tokio::spawn(async move {
        event_handler(tx).await;
    });

    // Main loop
    loop {
        // Render current state
        terminal.draw(|f| ui::render(f, &app))?;

        // Handle events with timeout (refresh_interval)
        match tokio::time::timeout(app.refresh_interval, rx.recv()).await {
            Ok(Some(Event::Key(key))) => {
                if !handle_key_event(key, &mut app) {
                    break; // Quit requested
                }
            }
            Ok(Some(Event::Resize(width, height))) => {
                app.handle_resize(width, height);
            }
            Ok(Some(Event::Tick)) | Err(_) => {
                // Refresh data
                app.refresh_data(&rpc, &db).await?;
            }
            Ok(None) => break, // Channel closed
        }
    }

    // Restore terminal
    restore_terminal(terminal)?;
    Ok(())
}
```

### 5.4 Responsive Layout System (v0.4)

```rust
pub struct ResponsiveLayout {
    size: ScreenSize,
    terminal_width: u16,
    terminal_height: u16,
}

impl ResponsiveLayout {
    pub fn from_terminal(width: u16, height: u16) -> Self {
        let size = match (width, height) {
            (w, h) if w < 100 || h < 30 => ScreenSize::Small,
            (w, h) if w >= 150 || h >= 50 => ScreenSize::Large,
            _ => ScreenSize::Medium,
        };

        Self {
            size,
            terminal_width: width,
            terminal_height: height,
        }
    }

    pub fn dashboard_layout(&self) -> DashboardLayout {
        match self.size {
            ScreenSize::Small => DashboardLayout::SingleColumn,
            ScreenSize::Medium => DashboardLayout::TwoColumn,
            ScreenSize::Large => DashboardLayout::ThreeColumn,
        }
    }

    pub fn constraints_for_dashboard(&self) -> (Vec<Constraint>, Vec<Constraint>) {
        // Returns (vertical_constraints, horizontal_constraints)
        match self.size {
            ScreenSize::Small => {
                // Single column, minimal spacing
                (vec![
                    Constraint::Length(8),  // Network status
                    Constraint::Length(8),  // Validator status
                    Constraint::Min(10),    // Block history
                ], vec![Constraint::Percentage(100)])
            }
            ScreenSize::Medium => {
                // Two columns, moderate detail
                (vec![
                    Constraint::Length(12), // Top panels
                    Constraint::Length(3),  // Health checks
                    Constraint::Min(12),    // Block history
                ], vec![
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ])
            }
            ScreenSize::Large => {
                // Three columns or detailed two-column
                (vec![
                    Constraint::Length(15), // Top panels
                    Constraint::Length(5),  // Health checks
                    Constraint::Min(15),    // Block history
                ], vec![
                    Constraint::Percentage(40),
                    Constraint::Percentage(30),
                    Constraint::Percentage(30),
                ])
            }
        }
    }
}
```

### 5.5 Dashboard Layout (v0.4)

See Release Plan for detailed ASCII mockup. Key components:

**Top Row** (Split horizontally):
- Left: Network Status Panel
- Right: Validator Status Panel

**Middle Row**:
- Health Check History (sparkline/indicators)

**Bottom Section**:
- Block History Visualization (sparkline + table)

**Rendering**:
```rust
pub fn render_dashboard(
    f: &mut Frame,
    app: &App,
    layout: &ResponsiveLayout,
) {
    let (v_chunks, h_chunks) = layout.constraints_for_dashboard();

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints(v_chunks)
        .split(f.size());

    // Top row - Network + Validator status
    render_top_panels(f, app, vertical[0], &h_chunks);

    // Middle row - Health checks
    render_health_history(f, app, vertical[1]);

    // Bottom - Block history
    render_block_history(f, app, vertical[2]);
}
```

---

## 6. Block Prediction Algorithm

### 6.1 AURA Consensus Model

Midnight uses **AURA (Authority Round)** consensus with a committee-based system:
- **185 candidates** (12 permissioned + 173 registered) are selected each epoch
- These candidates fill a **committee of ~1200 seats**
- Each candidate appears approximately 6-7 times in the committee
- Validators take turns producing blocks in round-robin order through the committee
- Each slot (6 seconds) has exactly one designated committee seat
- Committee order is deterministic (AURA authorities list)

**Important**: Block author calculation uses the **1200-seat committee**, not the 185 candidates:
```rust
author_index = slot % 1200  // Committee size, NOT candidate count (185)
```

### 6.2 Prediction Formula

For **committee-based AURA** (Midnight's implementation):
```
Committee Slots per Validator = Committee Size / Candidate Count
Expected Blocks per Epoch = Epoch Length (slots) / Committee Size × Committee Slots

Example:
- Epoch length: 1800 slots (3 hours at 6s/slot)
- Committee size: 1200 seats
- Total candidates: 185
- Committee slots per validator: 1200 / 185 ≈ 6.49
- Expected blocks per slot: 1800 / 1200 = 1.5 blocks
- Expected blocks per validator: 1.5 × 6.49 ≈ 9.73 blocks per epoch
```

**Note**: Individual validators may appear 6 or 7 times in the committee, creating variance in expected blocks.

For **stake-weighted validators** (if applicable):
```
Expected Blocks = Epoch Length × (Validator Stake / Total Active Stake)

Example:
- Epoch length: 1800 slots
- Our stake: 500,000 units
- Total stake: 10,000,000 units
- Expected blocks: 1800 × (500,000 / 10,000,000) = 90 blocks
```

### 6.3 Implementation

```rust
// src/midnight/prediction.rs

pub struct BlockPrediction {
    pub expected_blocks: f64,
    pub confidence_interval_low: u64,
    pub confidence_interval_high: u64,
    pub actual_blocks: u64,
    pub performance_ratio: f64,  // actual / expected
    pub epoch: u64,
}

pub async fn predict_blocks_for_epoch(
    validator_key: &str,
    epoch: u64,
    epoch_length_slots: u64,
    rpc: &RpcClient,
    db: &Database,
) -> Result<BlockPrediction> {
    // 1. Fetch validator set for epoch
    let validator_set = ValidatorSet::fetch_for_epoch(rpc, epoch).await?;

    // 2. Calculate expected blocks
    let total_validators = validator_set.validators.len() as f64;
    let expected_blocks = epoch_length_slots as f64 / total_validators;

    // 3. Get actual blocks produced so far this epoch
    let actual_blocks = db.count_blocks_for_validator_in_epoch(
        validator_key,
        epoch,
    )?;

    // 4. Calculate confidence interval (±10% variance expected)
    let variance = expected_blocks * 0.1;
    let confidence_low = (expected_blocks - variance).max(0.0) as u64;
    let confidence_high = (expected_blocks + variance).ceil() as u64;

    // 5. Calculate performance ratio
    let performance_ratio = if expected_blocks > 0.0 {
        actual_blocks as f64 / expected_blocks
    } else {
        0.0
    };

    Ok(BlockPrediction {
        expected_blocks,
        confidence_interval_low: confidence_low,
        confidence_interval_high: confidence_high,
        actual_blocks: actual_blocks as u64,
        performance_ratio,
        epoch,
    })
}
```

### 6.4 Epoch Progress Calculation

```rust
pub struct EpochProgress {
    pub current_slot: u64,
    pub epoch_start_slot: u64,
    pub epoch_length_slots: u64,
    pub percentage: f64,
    pub slots_remaining: u64,
    pub time_remaining_secs: u64,
}

pub fn calculate_epoch_progress(
    current_slot: u64,
    epoch: u64,
    slots_per_epoch: u64,
) -> EpochProgress {
    let epoch_start_slot = epoch * slots_per_epoch;
    let slot_in_epoch = current_slot - epoch_start_slot;
    let percentage = (slot_in_epoch as f64 / slots_per_epoch as f64) * 100.0;
    let slots_remaining = slots_per_epoch - slot_in_epoch;
    let time_remaining_secs = slots_remaining * 6; // 6 seconds per slot

    EpochProgress {
        current_slot,
        epoch_start_slot,
        epoch_length_slots: slots_per_epoch,
        percentage,
        slots_remaining,
        time_remaining_secs,
    }
}
```

### 6.5 Historical Performance Analysis

```rust
// Calculate performance over last N epochs
pub async fn analyze_historical_performance(
    validator_key: &str,
    db: &Database,
    num_epochs: u32,
) -> Result<PerformanceStats> {
    let recent_epochs = db.get_recent_epochs(num_epochs)?;

    let mut stats = PerformanceStats::default();

    for epoch in recent_epochs {
        let blocks = db.count_blocks_for_validator_in_epoch(
            validator_key,
            epoch.epoch,
        )?;

        stats.epochs_analyzed += 1;
        stats.total_blocks += blocks;
        stats.blocks_per_epoch.push(blocks);
    }

    // Calculate statistics
    stats.average_blocks = stats.total_blocks as f64 / stats.epochs_analyzed as f64;
    stats.min_blocks = *stats.blocks_per_epoch.iter().min().unwrap_or(&0);
    stats.max_blocks = *stats.blocks_per_epoch.iter().max().unwrap_or(&0);

    // Calculate standard deviation
    let variance: f64 = stats.blocks_per_epoch
        .iter()
        .map(|&x| {
            let diff = x as f64 - stats.average_blocks;
            diff * diff
        })
        .sum::<f64>() / stats.epochs_analyzed as f64;
    stats.std_deviation = variance.sqrt();

    Ok(stats)
}
```

---

## 7. Data Flow

### 7.1 Sync Flow

```
┌─────────────┐
│   START     │
└──────┬──────┘
       │
       ▼
┌─────────────────────┐
│ Load Configuration  │
└──────┬──────────────┘
       │
       ▼
┌─────────────────────┐
│ Open Database       │
└──────┬──────────────┘
       │
       ▼
┌─────────────────────┐
│ Determine Start     │ ← Check sync_status table
│ Block Number        │   or use --start-block flag
└──────┬──────────────┘
       │
       ▼
┌─────────────────────────────────────┐
│ Initial Sync Loop (batch mode)      │
│                                      │
│  1. Fetch batch of blocks (RPC)     │
│  2. Extract slot from digest        │
│  3. Calculate epoch from slot       │
│  4. Fetch validator set (per epoch) │
│  5. Calculate block author          │
│  6. Insert blocks to database       │
│  7. Update validators table         │
│  8. Update sync_status              │
│                                      │
│  Repeat until chain tip reached     │
└──────┬──────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────┐
│ Polling Loop (continuous mode)      │
│                                      │
│  1. Wait for poll interval (6s)     │
│  2. Check for new blocks            │
│  3. Sync new blocks (same as above) │
│  4. Handle signals (SIGTERM, etc.)  │
│                                      │
│  Repeat until stopped               │
└──────┬──────────────────────────────┘
       │
       ▼
┌─────────────┐
│   STOP      │
└─────────────┘
```

### 7.2 TUI Data Flow

```
┌──────────────────┐
│ User opens TUI   │
└────────┬─────────┘
         │
         ▼
┌──────────────────────┐
│ Initialize App       │
│ - Connect to RPC     │
│ - Open Database      │
│ - Setup Terminal     │
└────────┬─────────────┘
         │
         ▼
┌────────────────────────────────────┐
│ Main Event Loop                    │
│                                     │
│ ┌─────────────────────────────┐   │
│ │ Render current view         │   │
│ └────────┬────────────────────┘   │
│          │                         │
│          ▼                         │
│ ┌─────────────────────────────┐   │
│ │ Wait for event (with         │   │
│ │ timeout = refresh_interval)  │   │
│ └────────┬────────────────────┘   │
│          │                         │
│          ├─► Keyboard Event        │
│          │   - Handle navigation   │
│          │   - Update app state    │
│          │                         │
│          ├─► Resize Event (v0.4)   │
│          │   - Recalculate layout  │
│          │   - Update screen size  │
│          │                         │
│          └─► Timeout/Tick Event    │
│              - Fetch fresh data:   │
│                • RPC: node status  │
│                • DB: blocks,       │
│                  validators, etc.  │
│              - Update app state    │
│                                     │
│ Loop until quit                    │
└────────────────────────────────────┘
         │
         ▼
┌──────────────────┐
│ Restore Terminal │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│   EXIT           │
└──────────────────┘
```

### 7.3 Configuration Loading Flow

```
┌──────────────────┐
│ Load Defaults    │
└────────┬─────────┘
         │
         ▼
┌────────────────────────────────────┐
│ Search for config file (in order): │
│ 1. ./mvm.toml                       │
│ 2. ~/.config/mvm/config.toml        │
│ 3. /opt/midnight/mvm/config/...     │
│ 4. /etc/mvm/config.toml             │
└────────┬───────────────────────────┘
         │
         ├─► Found: Parse TOML
         │   └─► Merge with defaults
         │
         └─► Not found: Use defaults
         │
         ▼
┌────────────────────────────────────┐
│ Apply environment variable          │
│ overrides (MVM_* prefix)            │
└────────┬───────────────────────────┘
         │
         ▼
┌────────────────────────────────────┐
│ Apply CLI flag overrides            │
│ (highest priority)                  │
└────────┬───────────────────────────┘
         │
         ▼
┌────────────────────────────────────┐
│ Validate configuration              │
│ - Check URL formats                 │
│ - Check numeric ranges              │
│ - Check file paths exist            │
└────────┬───────────────────────────┘
         │
         ▼
┌────────────────────────────────────┐
│ Return final configuration          │
└─────────────────────────────────────┘
```

---

## 8. RPC API Specifications

### 8.1 Substrate Standard Methods

#### system_health
```json
Request:
{
  "jsonrpc": "2.0",
  "method": "system_health",
  "params": [],
  "id": 1
}

Response:
{
  "jsonrpc": "2.0",
  "result": {
    "isSyncing": false,
    "peers": 12,
    "shouldHavePeers": true
  },
  "id": 1
}
```

#### chain_getHeader
```json
Request:
{
  "jsonrpc": "2.0",
  "method": "chain_getHeader",
  "params": [null],  // null = latest, or "0x..." for specific hash
  "id": 1
}

Response:
{
  "jsonrpc": "2.0",
  "result": {
    "parentHash": "0x...",
    "number": "0x335cbd",  // Hex-encoded block number
    "stateRoot": "0x...",
    "extrinsicsRoot": "0x...",
    "digest": {
      "logs": [
        "0x06617572610138e61100000000",  // PreRuntime AURA slot
        // ... more logs
      ]
    }
  },
  "id": 1
}
```

#### chain_getBlock
```json
Request:
{
  "jsonrpc": "2.0",
  "method": "chain_getBlock",
  "params": ["0x..."],  // Block hash
  "id": 1
}

Response:
{
  "jsonrpc": "2.0",
  "result": {
    "block": {
      "header": { /* same as chain_getHeader */ },
      "extrinsics": ["0x...", "0x...", /* ... */]
    },
    "justifications": null
  },
  "id": 1
}
```

#### author_hasKey
```json
Request:
{
  "jsonrpc": "2.0",
  "method": "author_hasKey",
  "params": [
    "0x...",  // Public key (hex)
    "aura"    // Key type: "aura", "gran", or "crch"
  ],
  "id": 1
}

Response:
{
  "jsonrpc": "2.0",
  "result": true,  // Boolean
  "id": 1
}
```

**Note**: Requires `--rpc-methods=unsafe` on node

### 8.2 Midnight-Specific Methods

#### state_call (AuraApi_authorities)

**Purpose**: Get current committee (AURA authorities) for block author attribution

```json
Request:
{
  "jsonrpc": "2.0",
  "method": "state_call",
  "params": ["AuraApi_authorities", "0x"],
  "id": 1
}

Response:
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": "0xc1128c44..." // SCALE-encoded array of ~1200 AURA keys (32 bytes each)
}
```

**Decoding**:
- Response is SCALE-encoded byte array
- First 4 bytes: compact-encoded count
- Remaining bytes: N × 32-byte AURA public keys
- Typical count: ~1200 authorities

**Usage**: Essential for correct block author attribution (use committee, not candidates)

#### sidechain_getStatus
```json
Request:
{
  "jsonrpc": "2.0",
  "method": "sidechain_getStatus",
  "params": [],
  "id": 1
}

Response:
{
  "jsonrpc": "2.0",
  "result": {
    "sidechain": {
      "epoch": 245624,
      "slot": 294763983
    },
    "mainchain": {
      "epoch": 1179,
      "slot": 101838307
    }
  },
  "id": 1
}
```

#### sidechain_getAriadneParameters
```json
Request:
{
  "jsonrpc": "2.0",
  "method": "sidechain_getAriadneParameters",
  "params": [1179],  // Mainchain epoch number
  "id": 1
}

Response:
{
  "jsonrpc": "2.0",
  "result": {
    "permissionedCandidates": [
      "0x030cba90c73fbc32159ba89a980744fb324bdae640a320068d88b560eed6d665f9",
      // ... 11 more
    ],
    "candidateRegistrations": {
      "0x037764d2bd5f81a7e79f2cac2a8f00b0b963c5e08c8f6bf92bbf8d4809f47000": {
        "sidechainPubKey": "0x037764d2bd5f81a7e79f2cac2a8f00b0b963c5e08c8f6bf92bbf8d4809f47000",
        "sidechainSignature": "0x...",
        "inputUtxo": "0x...",
        "auraPublicKey": "0x...",
        "grandpaPublicKey": "0x...",
        "isValid": true
      },
      // ... 172 more
    }
  },
  "id": 1
}
```

**Usage**: Fetch validator set for a given mainchain epoch

---

## 9. Performance Targets

### 9.1 Sync Performance

| Metric | Target | Current (v0.3) | v0.4 Goal |
|--------|--------|----------------|-----------|
| Initial sync speed | 50-100 blocks/s | ~75 blocks/s | 100+ blocks/s |
| Polling overhead (idle) | <1% CPU | ~0.5% CPU | <0.5% CPU |
| Memory usage (sync) | <100 MB | ~50 MB | <75 MB |
| Database write speed | 1000+ blocks/s | ~2000 blocks/s | Maintain |

### 9.2 Query Performance

| Query | Target | Note |
|-------|--------|------|
| stats | <10 ms | Indexed queries only |
| blocks (100) | <50 ms | With author attribution |
| validators (all) | <20 ms | ~185 rows |
| performance rankings | <30 ms | Sorted by total_blocks |
| gaps detection | <100 ms | Full table scan |

### 9.3 TUI Performance

| Metric | Target | Critical? |
|--------|--------|-----------|
| Dashboard render time | <100 ms | Yes |
| Frame rate | 30+ FPS | Yes |
| Input latency | <50 ms | Yes |
| Memory usage | <50 MB | No |
| Data refresh time | <200 ms | Yes |

**v0.4 Specific**:
- Sparkline rendering: <10 ms for 24 hours of data
- Epoch progress calculation: <1 ms
- Terminal resize handling: <50 ms

### 9.4 Database Size Projections

| Timeframe | Blocks | Database Size | Notes |
|-----------|--------|---------------|-------|
| 1 week | ~100k | ~30 MB | |
| 1 month | ~432k | ~130 MB | |
| 6 months | ~2.6M | ~780 MB | |
| 1 year | ~5.2M | ~1.6 GB | |
| 2 years | ~10.4M | ~3.2 GB | Still performant with indexes |

**Health Checks** (30-day retention):
- ~200k rows/month ≈ 20 MB

**Total Estimate (1 year)**: ~1.7 GB

---

## 10. Security Considerations

### 10.1 RPC Security

**Threat**: Malicious RPC endpoint
- **Mitigation**: Use HTTPS for remote nodes, validate TLS certificates
- **Impact**: Medium (data integrity)

**Threat**: RPC injection attacks
- **Mitigation**: Type-safe deserialization with serde, no string interpolation
- **Impact**: Low (client-side only)

**Threat**: Unsafe RPC methods exposure
- **Mitigation**: Document requirement for `--rpc-methods=unsafe`, warn users about security implications
- **Impact**: Medium (node security)

### 10.2 Database Security

**Threat**: SQL injection
- **Mitigation**: Parameterized queries exclusively (no string concatenation)
- **Impact**: N/A (not possible with rusqlite parameterized queries)

**Threat**: Database file tampering
- **Mitigation**: File permissions (600), integrity checks (optional)
- **Impact**: Low (local file access required)

**Threat**: Database corruption
- **Mitigation**: WAL mode, transaction integrity, backup recommendations
- **Impact**: Medium (data loss)

### 10.3 Keystore Security

**Threat**: Keystore file exposure
- **Mitigation**: File permission checks (warn if world-readable), no key storage in database
- **Impact**: High (validator keys)

**Threat**: Key material in logs
- **Mitigation**: Never log private keys, truncate public keys in logs
- **Impact**: Low (public keys only)

### 10.4 System Security

**Threat**: Privilege escalation
- **Mitigation**: Run as non-root user, drop privileges after binding ports (if applicable)
- **Impact**: Low (no privileged operations)

**Threat**: PID file manipulation
- **Mitigation**: Atomic PID file writes, permission checks, stale PID detection
- **Impact**: Low (daemon management only)

### 10.5 Configuration Security

**Threat**: Secrets in config files
- **Mitigation**: Environment variable support for sensitive values, file permission warnings
- **Impact**: Medium (depends on deployment)

**Threat**: Config file injection
- **Mitigation**: TOML parsing validation, strict type checking
- **Impact**: Low (local file access required)

---

## 11. Deployment Architecture

### 11.1 Installation Paths

**System Installation** (`/opt/midnight/mvm/`):
```
/opt/midnight/mvm/
├── bin/
│   └── mvm                    # Binary (owned by user)
├── config/
│   └── config.toml            # Configuration
└── data/
    ├── mvm.db                 # SQLite database
    └── mvm-sync.pid           # PID file (daemon mode)

/usr/local/bin/
└── mvm -> /opt/midnight/mvm/bin/mvm  # Symlink

/etc/systemd/system/
├── mvm-sync.service           # Sync daemon
├── mvm-status.service         # Health check (one-shot)
└── mvm-status.timer           # Periodic health check timer
```

**User Installation** (`~/.local/`):
```
~/.local/bin/
└── mvm                        # Binary

~/.config/mvm/
└── config.toml                # User config

~/.local/share/mvm/
└── mvm.db                     # Database
```

### 11.2 Systemd Services

**mvm-sync.service**:
```ini
[Unit]
Description=Midnight Validator Monitor - Block Sync Daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=%USER%
WorkingDirectory=/opt/midnight/mvm
Environment="MVM_DB_PATH=/opt/midnight/mvm/data/mvm.db"
ExecStart=/opt/midnight/mvm/bin/mvm sync --daemon --pid-file /opt/midnight/mvm/data/mvm-sync.pid
Restart=on-failure
RestartSec=10s
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

**mvm-status.timer**:
```ini
[Unit]
Description=Midnight Validator Monitor - Periodic Status Check

[Timer]
OnBootSec=1min
OnUnitActiveSec=5min
Persistent=true

[Install]
WantedBy=timers.target
```

### 11.3 Monitoring and Logging

**Journal Logs**:
```bash
# View sync daemon logs
sudo journalctl -u mvm-sync -f

# View status check logs
sudo journalctl -u mvm-status

# Filter by time
sudo journalctl -u mvm-sync --since "1 hour ago"

# Filter by priority
sudo journalctl -u mvm-sync -p err
```

**Log Levels**:
- `ERROR`: Critical failures (database corruption, RPC unreachable)
- `WARN`: Recoverable issues (RPC timeout, missing blocks)
- `INFO`: Normal operation (blocks synced, health checks passed)
- `DEBUG`: Detailed diagnostics (requires `--verbose` flag)

**Log Rotation**: Handled automatically by systemd journal

### 11.4 Backup and Recovery

**Database Backup**:
```bash
# Stop sync first
sudo systemctl stop mvm-sync

# Copy database
sudo cp /opt/midnight/mvm/data/mvm.db /backup/mvm.db.$(date +%Y%m%d)

# Or use SQLite backup (can do while running)
sqlite3 /opt/midnight/mvm/data/mvm.db ".backup /backup/mvm.db.$(date +%Y%m%d)"

# Restart sync
sudo systemctl start mvm-sync
```

**Recovery from Backup**:
```bash
sudo systemctl stop mvm-sync
sudo cp /backup/mvm.db.20260116 /opt/midnight/mvm/data/mvm.db
sudo chown midnight:midnight /opt/midnight/mvm/data/mvm.db
sudo systemctl start mvm-sync
```

**Resync from Scratch**:
```bash
sudo systemctl stop mvm-sync
sudo rm /opt/midnight/mvm/data/mvm.db
sudo systemctl start mvm-sync
# Sync will start from block 0
```

---

## 12. Testing Strategy

### 12.1 Unit Tests

**Coverage Target**: >70%

**Test Categories**:
- Digest parsing (slot extraction from AURA PreRuntime logs)
- Validator ordering and author calculation
- Configuration loading and priority
- Database CRUD operations
- Block prediction algorithm
- Epoch progress calculations

**Example**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_extraction() {
        let digest = "0x06617572610138e61100000000";
        let slot = extract_slot_from_digest(digest).unwrap();
        assert_eq!(slot, 294763000);
    }

    #[test]
    fn test_author_calculation() {
        let slot: u64 = 294763983;
        let validator_count = 185;
        let author_index = slot % validator_count;
        assert_eq!(author_index, 133);
    }
}
```

### 12.2 Integration Tests

**Scenarios**:
1. Full sync workflow (100 blocks)
2. Config file loading from multiple locations
3. TUI event handling and rendering
4. Database migration and schema validation
5. Daemon startup and signal handling

**Test Environment**:
- Mock RPC server (wiremock or similar)
- In-memory SQLite database
- Simulated terminal (for TUI tests)

### 12.3 Performance Tests

**Benchmarks**:
- Sync speed (blocks/second)
- Query response times
- TUI render times
- Memory usage under load

**Tools**:
- `criterion` for micro-benchmarks
- `valgrind` for memory profiling
- `perf` for CPU profiling

### 12.4 User Acceptance Testing

**Manual Test Scenarios**:
1. Fresh installation on clean system
2. Upgrade from v0.3.0-alpha
3. TUI navigation and interaction
4. Terminal resize handling (various sizes)
5. Long-running daemon stability (24+ hours)
6. Graceful shutdown and restart
7. Configuration changes and reloading

**Test Environments**:
- Ubuntu 22.04 LTS
- Debian 12
- macOS 13+
- Various terminal emulators (xterm, iTerm2, tmux, screen)

---

## Appendices

### A. Glossary

- **AURA**: Authority Round - Substrate consensus algorithm for block production
- **GRANDPA**: GHOST-based Recursive Ancestor Deriving Prefix Agreement - Finality gadget
- **Epoch**: Time period in Midnight blockchain (mainchain and sidechain have separate epochs)
- **Slot**: 6-second time unit for block production
- **Validator Set**: Collection of validators eligible to produce blocks in an epoch
- **Block Attribution**: Process of determining which validator produced a block
- **Session Keys**: Cryptographic keys used by validators (sidechain, aura, grandpa)
- **Ariadne Parameters**: Midnight-specific validator registration data

### B. References

- [Substrate Documentation](https://docs.substrate.io/)
- [AURA Consensus](https://docs.substrate.io/reference/glossary/#authority-round-aura)
- [GRANDPA Finality](https://docs.substrate.io/reference/glossary/#grandpa)
- [JSON-RPC Specification](https://www.jsonrpc.org/specification)
- [Midnight Partner Chains](https://docs.midnight.network/)

### C. Version History

| Version | Date | Changes |
|---------|------|---------|
| 0.4-beta | TBD | Dynamic TUI, block prediction, health tracking |
| 0.3.0-alpha | 2026-01-16 | TUI, daemon mode, configuration system |
| 0.2.0-alpha | 2026-01-15 | Block author attribution, validator tracking |
| 0.1.0 | 2026-01-14 | Initial release (status, sync, query, keys) |

---

**Document Status**: Complete
**Last Review**: 2026-01-16
**Next Review**: Before v0.4-beta implementation begins

