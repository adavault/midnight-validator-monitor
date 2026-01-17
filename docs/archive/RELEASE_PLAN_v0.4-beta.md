# Release Plan: v0.4-beta

## Overview

v0.4-beta focuses on polish and production-readiness with emphasis on an enhanced, professional-grade TUI and critical operational features. This release transforms the basic v0.3 TUI into a comprehensive, production-ready monitoring dashboard.

**Status**: Planning Phase
**Target**: Beta Release (production-ready features, pending final testing)
**Focus**: TUI Excellence & Operational Features

## Goals for v0.4-beta

### Primary Objectives
1. **TUI Excellence**: Professional, polished terminal interface with dynamic scaling
2. **Enhanced Dashboard**: Comprehensive monitoring with epoch tracking and predictions
3. **Operational Features**: Critical missing features for production use
4. **Documentation & Spec**: Updated technical specification based on v0.3 learnings

### Success Criteria
- TUI adapts gracefully to all terminal sizes (80x24 to 200x60+)
- Dashboard provides all critical information at a glance
- Block prediction algorithm accurately forecasts validator performance
- Zero critical bugs or usability issues in TUI
- Complete technical specification aligned with implementation

---

## Feature Priorities

### CRITICAL - Block Attribution Fix 🔴

#### 0. Correct Validator Committee Implementation
**Goal**: Fix incorrect block author attribution discovered in v0.3.0-alpha

**Problem**: Current implementation uses 185 candidates instead of the actual 1200-seat committee for block author calculation, resulting in INCORRECT block attributions.

**Discovery**: See `VALIDATOR_COMMITTEE_DISCOVERY.md` for full details.

**Impact**:
- All block attributions in v0.3.0-alpha are incorrect
- Validator statistics are unreliable
- Performance rankings are inaccurate
- User trust is compromised

**Required Changes**:

1. **Update ValidatorSet structure** (`src/midnight/validators.rs`):
```rust
pub struct ValidatorSet {
    pub epoch: u64,
    pub candidates: Vec<Validator>,    // 185 candidates (for reference)
    pub committee: Vec<String>,         // ~1200 AURA keys (actual committee)
}
```

2. **Implement committee fetching**:
```rust
impl ValidatorSet {
    /// Fetch committee from AuraApi_authorities runtime call
    pub async fn fetch_with_committee(
        rpc: &RpcClient,
        epoch: u64,
    ) -> Result<Self> {
        let candidates = Self::fetch_candidates(rpc, epoch).await?;
        let committee = Self::fetch_aura_committee(rpc).await?;
        Ok(Self { epoch, candidates, committee })
    }

    /// Fetch current AURA authorities (committee)
    async fn fetch_aura_committee(rpc: &RpcClient) -> Result<Vec<String>> {
        // Call state_call("AuraApi_authorities", "0x")
        // Decode SCALE-encoded response
        // Parse 32-byte AURA keys
    }
}
```

3. **Fix author calculation**:
```rust
pub fn get_author(&self, slot_number: u64) -> Option<&Validator> {
    if self.committee.is_empty() {
        return None;
    }
    // CORRECT: Use committee size (~1200)
    let committee_index = (slot_number as usize) % self.committee.len();
    let aura_key = &self.committee[committee_index];

    // Find candidate by AURA key
    self.candidates.iter().find(|v| &v.aura_key == aura_key)
}
```

4. **Update prediction algorithm** to use committee size:
```rust
let expected_blocks = epoch_length_slots as f64 / committee_size as f64;
```

**Files to modify**:
- `src/midnight/validators.rs` - Add committee fetching and fix author calculation
- `src/commands/sync.rs` - Use new ValidatorSet::fetch_with_committee()
- `src/midnight/prediction.rs` - Update to use committee size for predictions (when created)
- `src/db/schema.rs` - Consider adding committee_snapshots table for historical tracking

**Files to create**:
- `src/midnight/scale.rs` - SCALE decoding utilities for AURA authorities response

**Testing Strategy**:
1. Verify committee size is ~1200 via RPC call
2. Validate author attribution matches block digest AURA keys
3. Resync 1000 blocks and compare results with old implementation
4. Unit tests for SCALE decoding
5. Integration test for committee fetch → author calculation flow

**Migration Considerations**:
- Existing block attributions in database are incorrect
- May want to add migration notice to users
- Consider adding `--reattribute` flag to sync command to recalculate authors
- Store committee snapshots per epoch for historical accuracy

**Timeline**: Must be completed in Phase 1 (Week 1) before other features

---

### HIGH PRIORITY - TUI Enhancements 🎨

#### 1. Dynamic Terminal Scaling
**Goal**: TUI adapts to terminal size changes in real-time

**Requirements**:
- Detect terminal resize events (SIGWINCH)
- Responsive layout that adjusts to available space
- Minimum size support: 80 columns × 24 rows
- Optimal size: 120 columns × 40 rows
- Large screen optimization: 200+ columns × 60+ rows

**Layout Breakpoints**:
- **Small** (80×24): Minimal view, single-column, essential data only
- **Medium** (120×40): Standard view, two-column where beneficial
- **Large** (160×50+): Full view, multi-column, additional details

**Implementation**:
```rust
// src/tui/layout.rs - New file
pub enum ScreenSize {
    Small,   // < 100 cols or < 30 rows
    Medium,  // 100-150 cols and 30-50 rows
    Large,   // > 150 cols or > 50 rows
}

pub struct ResponsiveLayout {
    size: ScreenSize,
    terminal_width: u16,
    terminal_height: u16,
}

impl ResponsiveLayout {
    pub fn from_terminal(width: u16, height: u16) -> Self;
    pub fn dashboard_layout(&self) -> Vec<Constraint>;
    pub fn blocks_layout(&self) -> Vec<Constraint>;
    // ... other view layouts
}
```

**Files to create**:
- `src/tui/layout.rs` - Responsive layout logic
- `src/tui/resize.rs` - Terminal resize handling

**Files to modify**:
- `src/tui/app.rs` - Add terminal size tracking
- `src/tui/ui.rs` - Use responsive layouts
- `src/tui/event.rs` - Handle resize events

#### 2. Enhanced Dashboard View
**Goal**: Comprehensive monitoring dashboard with all critical information

**Dashboard Layout** (4 main sections):

```
┌─────────────────────────────────────────────────────────────────┐
│ Network Status                       │ Our Validator            │
│ ─────────────────────────────────────│────────────────────────── │
│ ✓ Health: OK                         │ ✓ Registered & Valid     │
│ ✓ Syncing: 100%                      │   Rank: #1 / 185         │
│   Peers: 12                          │   Blocks: 23 (0.76%)     │
│   Block: #3363965 (finalized)        │   Label: My Validator    │
│   Epoch: 1179 (mainchain)            │                          │
│   Slot:  294763983                   │ Epoch Progress           │
│                                      │ ━━━━━━━━━━━━━━━━░░░░ 67% │
│ Health Checks  (last hour)           │ Est. Blocks This Epoch:  │
│ ✓✓✓✓✓✓✓✓✓✓✓✓ (12/12 passed)        │   Expected: 15-17        │
│                                      │   Actual:   11           │
├─────────────────────────────────────────────────────────────────┤
│ Block History (Our Validator - Last 24 hrs)                     │
│ ────────────────────────────────────────────────────────────────│
│ Hour  00 01 02 03 04 05 06 07 08 09 10 11 12 13 14 15 16 17 18 │
│ Blocks █  █  ▀  █  █  ▀  █  █  █  ▀  █  █  ▀  █  █  ▀  █  █  █ │
│                                                                  │
│ Recent Blocks:                                                   │
│   #3363965 (4 min ago) - Slot 294763983, Epoch 1179             │
│   #3363792 (31 min ago) - Slot 294763798, Epoch 1179            │
│   #3363612 (58 min ago) - Slot 294763613, Epoch 1179            │
└─────────────────────────────────────────────────────────────────┘
```

**Dashboard Components**:

1. **Network Status Panel** (top-left):
   - Health indicator (✓/✗)
   - Sync status with percentage
   - Peer count
   - Current block number (with finalized indicator)
   - Current epoch (mainchain)
   - Current slot number

2. **Our Validator Panel** (top-right):
   - Registration status
   - Performance rank
   - Total blocks produced
   - Validator label (from config)
   - Epoch progress bar
   - Block prediction for current epoch

3. **Health Check History** (middle-left):
   - Visual indicator of recent health checks (✓/✗)
   - Success rate
   - Time range (e.g., "last hour", "last 24h")

4. **Block History Visualization** (bottom):
   - Sparkline/bar chart of blocks produced over time
   - Configurable time range (1h, 6h, 24h, 7d)
   - Recent blocks list with timestamps
   - Slot and epoch information

**Implementation**:
```rust
// src/tui/dashboard.rs - New file
pub struct DashboardData {
    pub network_status: NetworkStatus,
    pub validator_status: ValidatorStatus,
    pub health_history: Vec<HealthCheck>,
    pub block_history: Vec<BlockProduction>,
    pub epoch_progress: EpochProgress,
}

pub struct EpochProgress {
    pub current_slot: u64,
    pub epoch_start_slot: u64,
    pub epoch_length_slots: u64,
    pub blocks_produced: u64,
    pub expected_blocks_min: u64,
    pub expected_blocks_max: u64,
}

pub struct BlockProduction {
    pub hour: u8,           // 0-23
    pub block_count: u32,
    pub expected_count: f64,
}
```

**Files to create**:
- `src/tui/dashboard.rs` - Dashboard data structures and rendering
- `src/tui/widgets/` - Custom widgets directory
  - `src/tui/widgets/sparkline.rs` - Block history sparkline
  - `src/tui/widgets/progress_bar.rs` - Epoch progress bar
  - `src/tui/widgets/health_indicator.rs` - Health check visualization

**Files to modify**:
- `src/tui/ui.rs` - Use new dashboard module
- `src/tui/app.rs` - Fetch and store dashboard data
- `src/commands/view.rs` - Pass dashboard data to UI

#### 3. Block Prediction Algorithm
**Goal**: Predict expected block production based on stake allocation

**Algorithm Overview**:
```
Expected Blocks = (Epoch Length in Slots) × (Our Stake / Total Stake)
```

**For AURA Consensus**:
- Each slot has one designated validator (round-robin)
- Validator's share = 1 / Total Active Validators
- Expected blocks per epoch = Slots per Epoch / Total Validators

**With Stake Weighting** (if applicable):
- If validators have different stakes, weight by relative stake
- Expected = (Epoch Slots) × (Validator Stake / Total Staked)

**Implementation**:
```rust
// src/midnight/prediction.rs - New file
pub struct BlockPrediction {
    pub expected_blocks: f64,
    pub confidence_interval_low: u64,
    pub confidence_interval_high: u64,
    pub actual_blocks: u64,
    pub performance_ratio: f64, // actual / expected
}

pub async fn predict_blocks_for_epoch(
    validator_key: &str,
    epoch: u64,
    rpc: &RpcClient,
    db: &Database,
) -> Result<BlockPrediction> {
    // 1. Get validator set for epoch
    // 2. Get validator's stake (if stake-weighted)
    // 3. Calculate total active stake
    // 4. Determine epoch length in slots
    // 5. Calculate expected blocks
    // 6. Get actual blocks produced so far
    // 7. Calculate confidence interval (±10% for variance)
}
```

**Data Requirements**:
- Epoch length in slots (from runtime metadata or config)
- Validator set size for epoch
- Validator stake allocation (from Ariadne parameters)
- Historical block production data (for confidence calculations)

**Files to create**:
- `src/midnight/prediction.rs` - Block prediction logic
- `src/midnight/stake.rs` - Stake calculation helpers

**Files to modify**:
- `src/midnight/mod.rs` - Export prediction module
- `src/tui/dashboard.rs` - Use predictions in dashboard

#### 4. Block History Tracking
**Goal**: Store and visualize historical block production

**Database Schema Addition**:
```sql
-- Add to existing blocks table (already has what we need)
-- We'll query blocks table with author_key filter and time grouping

-- Optional: Add materialized view for performance
CREATE TABLE IF NOT EXISTS block_production_hourly (
    validator_key TEXT NOT NULL,
    hour_timestamp INTEGER NOT NULL, -- Unix timestamp rounded to hour
    block_count INTEGER NOT NULL DEFAULT 0,
    expected_blocks REAL,
    PRIMARY KEY (validator_key, hour_timestamp)
);

CREATE INDEX IF NOT EXISTS idx_production_validator ON block_production_hourly(validator_key);
CREATE INDEX IF NOT EXISTS idx_production_time ON block_production_hourly(hour_timestamp);
```

**Implementation**:
```rust
// src/db/analytics.rs - New file
pub struct BlockProductionHourly {
    pub validator_key: String,
    pub hour_timestamp: i64,
    pub block_count: u32,
    pub expected_blocks: f64,
}

impl Database {
    pub fn get_hourly_production(
        &self,
        validator_key: &str,
        start_time: i64,
        end_time: i64,
    ) -> Result<Vec<BlockProductionHourly>>;

    pub fn get_block_history_sparkline(
        &self,
        validator_key: &str,
        hours: u32,
    ) -> Result<Vec<u32>>; // block counts per hour
}
```

**Files to create**:
- `src/db/analytics.rs` - Analytics queries
- `src/tui/widgets/sparkline.rs` - Block history visualization

**Files to modify**:
- `src/db/mod.rs` - Export analytics module
- `src/db/blocks.rs` - Add timestamp-based queries

---

### MEDIUM PRIORITY - Core Monitoring Features 🔧

#### 5. Health Check History
**Goal**: Track and visualize node health over time

**Database Schema**:
```sql
CREATE TABLE IF NOT EXISTS health_checks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    is_healthy INTEGER NOT NULL, -- 1 = healthy, 0 = unhealthy
    is_syncing INTEGER NOT NULL,
    peer_count INTEGER NOT NULL,
    best_block INTEGER NOT NULL,
    finalized_block INTEGER NOT NULL,
    sync_percentage REAL,
    response_time_ms INTEGER
);

CREATE INDEX IF NOT EXISTS idx_health_timestamp ON health_checks(timestamp);
```

**Implementation**:
```rust
// src/db/health.rs - New file
pub struct HealthCheckRecord {
    pub timestamp: i64,
    pub is_healthy: bool,
    pub is_syncing: bool,
    pub peer_count: u32,
    pub best_block: u64,
    pub finalized_block: u64,
    pub sync_percentage: f64,
    pub response_time_ms: u32,
}

impl Database {
    pub fn insert_health_check(&self, record: &HealthCheckRecord) -> Result<()>;
    pub fn get_recent_health_checks(&self, hours: u32) -> Result<Vec<HealthCheckRecord>>;
    pub fn get_health_check_stats(&self, hours: u32) -> Result<HealthStats>;
}
```

**Files to create**:
- `src/db/health.rs` - Health check database operations

**Files to modify**:
- `src/commands/status.rs` - Store health checks to database
- `src/db/schema.rs` - Add health_checks table
- `src/tui/dashboard.rs` - Display health history

---

### LOW PRIORITY - Nice to Have ✨

#### 6. Prometheus Metrics Endpoint
**Goal**: Export metrics for Prometheus scraping

**Features**:
- HTTP server on configurable port (default: 9100)
- Standard Prometheus metrics format
- Validator performance metrics
- Sync status metrics
- Health check metrics

**Metrics to export**:
```
# Validator metrics
mvm_blocks_produced_total{validator="...",epoch="..."}
mvm_validator_rank{validator="..."}
mvm_expected_blocks{validator="...",epoch="..."}

# Node metrics
mvm_sync_status{status="syncing|synced"}
mvm_peer_count
mvm_best_block
mvm_finalized_block

# Health metrics
mvm_health_check_success_total
mvm_health_check_failure_total
mvm_health_check_response_time_ms
```

**Implementation**:
```rust
// src/metrics/prometheus.rs - New file
use hyper::{Body, Request, Response, Server};

pub async fn start_metrics_server(
    bind_addr: String,
    db: Arc<Database>,
) -> Result<()> {
    // Start HTTP server
    // Expose /metrics endpoint
    // Query database for current stats
    // Format as Prometheus metrics
}
```

**Dependencies to add**:
```toml
hyper = { version = "0.14", features = ["server", "http1"] }
```

**Files to create**:
- `src/metrics/prometheus.rs` - Prometheus metrics server
- `src/commands/metrics.rs` - Metrics command (start server)

#### 7. Alert Webhooks
**Goal**: Send alerts on critical events

**Events to alert on**:
- Node becomes unhealthy
- Sync falls behind (>100 blocks)
- No blocks produced for >2 hours (when expected)
- Session key rotation detected
- Validator becomes unregistered

**Configuration**:
```toml
[alerts]
enable_webhooks = true
webhook_url = "https://hooks.slack.com/services/..."
alert_on_unhealthy = true
alert_on_sync_behind = true
alert_on_no_blocks = true
alert_threshold_hours = 2
```

**Implementation**:
```rust
// src/alerts/webhook.rs - New file
pub struct AlertWebhook {
    url: String,
    client: reqwest::Client,
}

impl AlertWebhook {
    pub async fn send_alert(&self, alert: &Alert) -> Result<()>;
}

pub enum Alert {
    NodeUnhealthy { reason: String },
    SyncBehind { blocks_behind: u64 },
    NoBlocksProduced { hours: u32 },
    KeyRotation { key_type: String },
    Unregistered,
}
```

**Files to create**:
- `src/alerts/webhook.rs` - Webhook alerting
- `src/alerts/mod.rs` - Alert management

---

## Implementation Plan

### Phase 1: Critical Fixes & Research (Week 1-2)
**Goal**: Fix block attribution bug and understand stake allocation mechanism

**Week 1 - Critical Fix**:
- [ ] Implement SCALE decoding for AURA authorities
- [ ] Add committee fetching to ValidatorSet
- [ ] Fix block author calculation to use committee
- [ ] Update sync command to fetch committee per epoch
- [ ] Add committee_snapshots table for historical tracking
- [ ] Test committee fetch and author attribution accuracy
- [ ] Document migration path for existing users

**Week 2 - Stake Allocation Research**:
- [ ] Analyze dbsync data on vdumds58 (partnerchain containers)
- [ ] Examine Cardano stake pool registration and performance data
- [ ] Map Cardano pool IDs to Midnight validator keys
- [ ] Fetch and analyze multiple epoch committees
- [ ] Count validator appearance frequency in committee (1200 seats)
- [ ] Correlate committee seats with Cardano stake allocation
- [ ] Verify epoch lag hypothesis (N-1 or N-2)
- [ ] Document mathematical formula for seat allocation
- [ ] Create STAKE_ALLOCATION_RESEARCH.md with findings
- [ ] Update technical specification with accurate model

### Phase 2: TUI Foundation (Week 3-4)
**Goal**: Dynamic layout and responsive design
- [ ] Implement terminal resize detection
- [ ] Create responsive layout system
- [ ] Add screen size detection and breakpoints
- [ ] Test on various terminal sizes (xterm, iTerm2, tmux, screen)
- [ ] Update all views to use responsive layouts
- [ ] Implement graceful degradation for minimum size (80×24)
- [ ] Test with large screens (200×60+)

### Phase 3: Enhanced Dashboard (Week 5-6)
**Goal**: Comprehensive monitoring dashboard with accurate predictions
- [ ] Design dashboard layout and components
- [ ] Implement network status panel
- [ ] Implement validator status panel
- [ ] Add epoch progress bar
- [ ] Implement block prediction algorithm (using research findings)
- [ ] Add stake-weighted calculation based on Cardano performance
- [ ] Implement epoch lag handling
- [ ] Create block history sparkline widget
- [ ] Implement recent blocks list
- [ ] Display confidence intervals for predictions

### Phase 4: Health & Analytics (Week 7)
**Goal**: Historical tracking and visualization
- [ ] Create health_checks table
- [ ] Implement health check storage
- [ ] Create analytics queries for block production
- [ ] Add hourly aggregation
- [ ] Implement health history visualization
- [ ] Add time-range filtering
- [ ] Performance optimize queries for large datasets

### Phase 5: Polish & Testing (Week 8)
**Goal**: Production-ready release
- [ ] Performance optimization and profiling
- [ ] Memory leak detection and fix
- [ ] Comprehensive testing (unit + integration)
- [ ] Documentation updates (README, technical spec, deployment guide)
- [ ] User testing and feedback incorporation
- [ ] Bug fixes and refinements
- [ ] Optional: Implement Prometheus metrics endpoint if time permits
- [ ] Optional: Implement alert webhooks if time permits

---

## Technical Specification Updates

See `TECHNICAL_SPEC_v0.4.md` for:
- Updated architecture diagrams
- Detailed database schema changes
- API specifications for new features
- Performance benchmarks and targets
- Security considerations

---

## Dependencies to Add

```toml
# Metrics server (optional - LOW PRIORITY)
hyper = { version = "0.14", features = ["server", "http1"] }

# Potential additions
# sysinfo = "0.30" - For system resource monitoring
```

---

## Success Metrics

### User Experience
- **Block attribution is 100% accurate** (critical fix from v0.3.0)
- TUI works seamlessly on terminals from 80×24 to 200×60+
- Dashboard provides comprehensive status at-a-glance
- Block prediction accuracy within ±10% of actual
- Zero crashes or freezes during normal operation
- Intuitive navigation and information hierarchy

### Performance
- Dashboard refresh under 100ms with 100k+ blocks
- Memory usage stable under 50MB for TUI
- Database queries complete in <10ms
- Export of 10k blocks completes in <1 second

### Completeness
- All HIGH priority features implemented
- All MEDIUM priority features implemented
- At least 1-2 LOW priority features implemented (stretch goal)
- Comprehensive test coverage (>70%)
- Complete documentation

---

## Risk Mitigation

### Technical Risks
1. **Terminal resize edge cases**
   - Mitigation: Extensive testing on multiple terminals (xterm, iTerm2, tmux, screen)
   - Fallback: Graceful degradation to minimum size

2. **Block prediction accuracy**
   - Mitigation: Validate algorithm against historical data
   - Fallback: Show prediction as range rather than exact number

3. **Performance with large datasets**
   - Mitigation: Database indexing and query optimization
   - Fallback: Pagination and time-range limits

### Schedule Risks
1. **Scope creep**
   - Mitigation: Strict priority adherence, defer LOW priority features if needed

2. **Testing time underestimated**
   - Mitigation: Allocate 2 weeks for testing and polish

---

## Deferred to v0.5

The following features are valuable but not critical for beta:
- **Export functionality** (CSV/JSON export of blocks, validators, performance data)
- **Session key rotation detection** (Track and alert on key changes)
- Multi-validator support (tracking multiple validators simultaneously)
- Docker container and Kubernetes manifests
- GraphQL API for external integrations
- WebSocket real-time updates
- Advanced alerting (email, SMS, PagerDuty)
- Mobile app / web dashboard

---

## Version Naming

- **v0.4-beta**: Production-ready with all critical features
- **v0.4**: Final release after beta testing period
- **v0.5+**: Future enhancements and ecosystem integrations

---

## Next Steps

1. **Update Technical Specification** - Comprehensive spec update based on v0.3 learnings
2. **Create Detailed Task Breakdown** - Break each feature into implementable tasks
3. **Set up Milestones** - Weekly milestones with deliverables
4. **Begin Phase 1 Implementation** - Start with TUI foundation

---

**Planning Complete**: Ready to begin implementation of v0.4-beta
**Estimated Timeline**: 8 weeks to beta release
**Target Release**: Mid March 2026

**Core Focus**:
- **Week 1**: Fix critical block attribution bug
- **Week 2**: Research stake allocation mechanism
- **Weeks 3-8**: TUI Excellence - Dynamic scaling, enhanced dashboard, accurate block prediction, and health monitoring
