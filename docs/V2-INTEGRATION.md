# MVM v2.0 - Integration Strategy

**Status:** Planning document for future development
**Author:** ADAvault
**Created:** January 2026

---

## Context

MVM is part of a broader ADAvault ecosystem of SPO monitoring tools:

```
┌─────────────────────────────────────────────────────────────────────┐
│                         ADAvault Ecosystem                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────────────┐   │
│  │     MVM     │     │     CNM     │     │    adavault-web     │   │
│  │  Midnight   │     │   Cardano   │     │   Web frontend      │   │
│  │  Validator  │     │    Node     │     │   + Backend APIs    │   │
│  │  Monitor    │     │   Monitor   │     │                     │   │
│  └──────┬──────┘     └──────┬──────┘     └──────────┬──────────┘   │
│         │                   │                       │               │
│         └───────────┬───────┘                       │               │
│                     ▼                               │               │
│           ┌───────────────────┐                     │               │
│           │    spom-core      │◄────────────────────┘               │
│           │  Shared framework │      Datahub / API                  │
│           └───────────────────┘                                     │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

This document outlines how MVM v2.0 can integrate with this ecosystem while maintaining its standalone functionality.

---

## Design Principles

1. **Standalone first** - MVM must remain fully functional without any ecosystem dependencies
2. **Opt-in integration** - All ecosystem features are optional and disabled by default
3. **Standard protocols** - Use widely-adopted formats (JSON, Prometheus, SQLite)
4. **No breaking changes** - v2.0 integration features should not break v1.x workflows

---

## Integration Points

### 1. Data Export API

Expose MVM's SQLite data via an optional REST API for external consumption.

**Use cases:**
- Web dashboard pulling validator stats
- Central datahub aggregating multiple nodes
- Custom alerting systems

**Proposed endpoints:**
```
GET /api/v1/status          # Current node status (health, sync, epoch)
GET /api/v1/blocks          # Recent blocks with pagination
GET /api/v1/validators      # Validator list with performance stats
GET /api/v1/validators/ours # Our validators only
GET /api/v1/epochs/:id      # Epoch summary (blocks, performance)
GET /api/v1/health          # Simple health check for load balancers
```

**Implementation notes:**
- Lightweight HTTP server (axum or warp)
- Read-only access to existing SQLite database
- Optional authentication via API key
- Configurable bind address and port
- New command: `mvm api --port 8080`

**Configuration:**
```toml
[api]
enabled = false
bind = "127.0.0.1:8080"
api_key = ""  # Optional, empty = no auth
cors_origins = []  # For browser access
```

### 2. Prometheus Metrics Endpoint

Export metrics in Prometheus format for integration with existing monitoring stacks.

**Use cases:**
- Grafana dashboards
- AlertManager integration
- Central metrics aggregation

**Proposed metrics:**
```prometheus
# Node status
mvm_node_synced{chain="midnight"} 1
mvm_node_peers{chain="midnight"} 16
mvm_node_block_height{chain="midnight"} 1234567
mvm_node_finalized_height{chain="midnight"} 1234565

# Epoch progress
mvm_sidechain_epoch{chain="midnight"} 456
mvm_sidechain_epoch_progress{chain="midnight"} 0.45
mvm_mainchain_epoch{chain="midnight"} 123
mvm_mainchain_epoch_progress{chain="midnight"} 0.32

# Validator performance
mvm_validator_blocks_total{validator="0x123...",chain="midnight"} 150
mvm_validator_blocks_epoch{validator="0x123...",chain="midnight"} 3
mvm_validator_committee_seats{validator="0x123...",chain="midnight"} 5
mvm_validator_is_registered{validator="0x123...",chain="midnight"} 1

# Sync daemon
mvm_sync_last_block{chain="midnight"} 1234500
mvm_sync_blocks_behind{chain="midnight"} 67
mvm_sync_rate_bps{chain="midnight"} 145.2
```

**Implementation notes:**
- Endpoint at `/metrics` (standard Prometheus path)
- Can share HTTP server with Data Export API
- Minimal overhead - metrics computed on request

### 3. Webhook / Event Push

Push notifications for significant events to external systems.

**Use cases:**
- Slack/Discord alerts
- PagerDuty integration
- Custom notification systems
- Central event aggregation

**Proposed events:**
```json
{
  "event": "block_produced",
  "timestamp": "2026-01-21T12:34:56Z",
  "chain": "midnight",
  "validator": "0x123...",
  "block_number": 1234567,
  "slot": 789012,
  "epoch": 456
}

{
  "event": "validator_status_change",
  "timestamp": "2026-01-21T12:34:56Z",
  "chain": "midnight",
  "validator": "0x123...",
  "old_status": "registered",
  "new_status": "not_in_committee"
}

{
  "event": "sync_alert",
  "timestamp": "2026-01-21T12:34:56Z",
  "chain": "midnight",
  "alert_type": "falling_behind",
  "blocks_behind": 100
}
```

**Configuration:**
```toml
[webhooks]
enabled = false
endpoints = [
  { url = "https://hooks.slack.com/...", events = ["block_produced", "validator_status_change"] },
  { url = "https://datahub.adavault.com/events", events = ["*"], auth = "Bearer xxx" }
]
retry_attempts = 3
retry_delay_ms = 1000
```

### 4. Database Schema Alignment

Ensure MVM's schema patterns align with CNM for easier datahub integration.

**Current MVM tables:**
- `blocks` - Block data with author attribution
- `validators` - Validator registry with performance stats
- `committee_snapshots` - Committee composition per epoch
- `validator_epochs` - Per-epoch validator performance
- `sync_status` - Sync progress tracking

**Alignment considerations:**
- Add `chain` column to all tables (default: "midnight")
- Standardize timestamp formats (Unix ms)
- Consistent key naming (`sidechain_key` vs `pool_id`)
- Schema version tracking for migrations

**Proposed unified schema prefix:**
```sql
-- Future: Tables could be prefixed or namespaced
-- mvm_blocks, cnm_blocks, or chain column approach
-- Decision deferred until CNM schema is defined
```

### 5. Remote Database Access

Allow the sync daemon to write to a remote database for centralized data collection.

**Use cases:**
- Central datahub collecting from multiple nodes
- Web backend with direct database access
- Backup/replication scenarios

**Implementation options:**

1. **SQLite over network (not recommended)**
   - SQLite doesn't handle concurrent remote writes well

2. **PostgreSQL support (v2.0+)**
   - Add optional PostgreSQL backend
   - Same schema, different driver
   - Better for multi-writer scenarios

3. **Database sync/replication**
   - Keep local SQLite, periodically push to central DB
   - One-way sync, local is source of truth

**Configuration:**
```toml
[database]
# Local SQLite (default, always available)
path = "./mvm.db"

# Optional: Push to central database
[database.remote]
enabled = false
type = "postgresql"  # or "sqlite_replica"
url = "postgresql://user:pass@datahub.adavault.com/mvm"
sync_interval_secs = 60
```

---

## Shared Core Extraction (spom-core)

Components that could be extracted to `spom-core` for sharing with CNM:

| Component | MVM Location | Extraction Priority |
|-----------|--------------|---------------------|
| TUI framework | `src/tui/` | High - identical patterns |
| Theme system | `src/tui/theme.rs` | High - reusable |
| Config system | `src/config.rs` | High - same TOML/env/CLI layering |
| Daemon utilities | `src/daemon.rs` | High - PID, signals, systemd |
| Database patterns | `src/db/` | Medium - schema differs |
| RPC client | `src/rpc/` | Low - protocol differs |

**Extraction criteria:**
- Code is stable and unlikely to change
- Patterns proven in both MVM and CNM
- Clear abstraction boundaries
- No chain-specific logic leaking through

---

## Configuration Example (v2.0)

```toml
# mvm.toml - v2.0 with integration features

[rpc]
url = "http://localhost:9944"
metrics_url = "http://localhost:9615/metrics"
timeout_ms = 30000

[database]
path = "./mvm.db"

[validator]
keystore_path = "/opt/midnight/keystore"

[sync]
batch_size = 1000
poll_interval_secs = 6
finalized_only = false

[view]
theme = "midnight"
refresh_interval_secs = 2

# === v2.0 Integration Features ===

[api]
enabled = false
bind = "127.0.0.1:8080"
api_key = ""
metrics_enabled = true  # /metrics endpoint

[webhooks]
enabled = false
endpoints = []

[integration]
chain_id = "midnight"  # For multi-chain datahub
instance_name = "mdn57-validator"  # Human-readable identifier
```

---

## Implementation Phases

### Phase 1: Foundation (v1.x maintenance)
- Stabilize current features
- Gather SPO feedback
- Document schema and patterns

### Phase 2: API Layer (v2.0)
- Add optional REST API
- Add Prometheus metrics endpoint
- Maintain backwards compatibility

### Phase 3: Event System (v2.1)
- Webhook support
- Event definitions
- Retry logic

### Phase 4: Core Extraction (v2.2+)
- Coordinate with CNM development
- Extract proven shared components
- Create spom-core crate

---

## Open Questions

1. **API authentication** - Simple API key, or JWT/OAuth for web integration?
2. **Event batching** - Push immediately or batch events for efficiency?
3. **PostgreSQL** - Add as optional backend, or stay SQLite-only?
4. **Binary size** - How much do integration features add? Feature flags?
5. **Versioning** - API versioning strategy for breaking changes?

---

## References

- [SPO Monitor Vision](https://github.com/adavault/spo-monitor/blob/main/docs/VISION.md) (private)
- [MVM CLAUDE.md](./CLAUDE.md)
- [Prometheus Exposition Format](https://prometheus.io/docs/instrumenting/exposition_formats/)
- [JSON:API Specification](https://jsonapi.org/)

---

*This document captures strategic thinking for v2.0. Implementation details will be refined as development progresses.*
