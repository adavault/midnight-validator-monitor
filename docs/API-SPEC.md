# MVM API Specification

**Status:** Draft (Updated for Decentralized Model)
**Author:** CxO
**Date:** 2026-01-25
**Target:** v2.0 (Registry), v2.1 (API Nodes)

---

## Strategic Context

> **"Our competitive edge will come in other ways, not by hoarding central control points."**

This spec has been updated to reflect a **decentralized architecture**. Instead of a central ADAvault API, MVM enables a network of SPO-operated nodes that collectively provide data infrastructure.

---

## Overview

MVM provides decentralized data infrastructure through:

1. **Pool ticker registry** — Calidus-verified, multi-source, community-maintained
2. **API nodes** — SPO-operated endpoints serving local chain data
3. **Network aggregation** — Cross-node queries for network-wide statistics

### Design Principles

- **Decentralization first** — No single point of failure or control
- **Community-owned** — SPOs operate infrastructure, not ADAvault
- **Opt-in participation** — SPOs choose to run API nodes
- **Progressive enhancement** — Start simple (registry), add complexity (API, aggregation)
- **Transparent** — Open source, auditable, forkable

---

## Architecture

### Phase 1: Git-based Registry (v2.0)

```
┌──────────────────────────────────────────────────┐
│              GitHub Repository                    │
│  ┌────────────────────────────────────────────┐  │
│  │  known_validators.toml (GPG-signed)        │  │
│  │  + Calidus proofs per entry                │  │
│  └────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        ▼               ▼               ▼
   GitHub Raw      Mirror 1        Mirror 2
   (primary)      (SPO-hosted)   (SPO-hosted)
        │               │               │
        └───────────────┼───────────────┘
                        ▼
              ┌─────────────────┐
              │    MVM Client   │
              │ (fetches from   │
              │  multiple URLs) │
              └─────────────────┘
```

### Phase 2: API Nodes (v2.1)

```
┌─────────────────────────────────────────────────────┐
│                   Client Apps                        │
│         (MVM TUI, dashboards, bots, dApps)          │
└─────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────┐
│              Bootstrap Endpoint List                 │
│   [node1.example.com, node2.example.com, ...]       │
└─────────────────────────────────────────────────────┘
                          │
          ┌───────────────┼───────────────┐
          ▼               ▼               ▼
    ┌──────────┐    ┌──────────┐    ┌──────────┐
    │ MVM Node │    │ MVM Node │    │ MVM Node │
    │  (ADV)   │    │  (ATADA) │    │  (APEX)  │
    │ mvm serve│    │ mvm serve│    │ mvm serve│
    └──────────┘    └──────────┘    └──────────┘
```

---

## API Endpoints

Each API node exposes the same endpoints. Clients can use any node.

Base URL: `http://<any-api-node>:8080/v1`

### 1. Pool Tickers

```
GET /tickers
```

Returns list of known Midnight validator pool tickers.

**Response:**
```json
{
  "tickers": [
    {
      "ticker": "ADV",
      "name": "AdaVault",
      "website": "https://adavault.com",
      "updated_at": "2026-01-24T08:00:00Z"
    },
    {
      "ticker": "APEX",
      "name": "Apex Staking",
      "website": "https://apex.example",
      "updated_at": "2026-01-23T12:00:00Z"
    }
  ],
  "count": 2,
  "last_updated": "2026-01-24T08:00:00Z"
}
```

**MVM Usage:**
- Called on startup (with cache)
- Replaces `validators.toml` for pool identification
- User can still override locally if needed

**Cache:** MVM should cache response for 1 hour to reduce API load.

---

### 2. Node Stats (Telemetry)

```
POST /stats
```

Optional push of validator node health data.

**Request:**
```json
{
  "mvm_version": "1.1.0",
  "node": {
    "ticker": "ADV",
    "sync_progress": 99.8,
    "sync_state": "synced",
    "block_height": 1234567,
    "peer_count": 42,
    "uptime_seconds": 86400
  },
  "submitted_at": "2026-01-24T08:30:00Z"
}
```

**Response:**
```json
{
  "accepted": true,
  "network_rank": 15,
  "network_total": 87
}
```

**Fields explained:**

| Field | Description | Required |
|-------|-------------|----------|
| `mvm_version` | MVM version string | Yes |
| `ticker` | Pool ticker (if configured) | No |
| `sync_progress` | Sync percentage | Yes |
| `sync_state` | syncing/synced/unknown | Yes |
| `block_height` | Current block height | Yes |
| `peer_count` | Connected peers | Yes |
| `uptime_seconds` | Node uptime | No |

**Privacy notes:**
- No IP logging beyond standard web server logs
- No validator keys or sensitive data
- Ticker is optional — anonymous stats accepted

**Push frequency:** Every 5 minutes when opted-in.

---

### 3. Network Dashboard

```
GET /network
```

Returns aggregate network health (public, no auth).

**Response:**
```json
{
  "network": "testnet",
  "validators": {
    "reporting": 87,
    "synced": 82,
    "syncing": 5
  },
  "blocks": {
    "latest": 1234567,
    "median_height": 1234565
  },
  "health": {
    "sync_rate": 94.2,
    "avg_peers": 38.5
  },
  "last_updated": "2026-01-24T08:35:00Z"
}
```

**Use cases:**
- MVM can display network health in dashboard
- Public dashboard on adavault.com
- Community monitoring tools

---

## MVM Integration

### Configuration

Add to MVM config:

```toml
[api]
enabled = true                          # default: true
endpoint = "https://api.adavault.com"   # default
push_stats = true                       # default: true
push_interval = 300                     # seconds, default: 300
ticker = "ADV"                          # optional, for stats attribution
```

### Opt-out modes

| Mode | `enabled` | `push_stats` | Behaviour |
|------|-----------|--------------|-----------|
| Full | true | true | Fetch tickers, push stats, show network |
| Read-only | true | false | Fetch tickers, no push, show network |
| Local | false | - | No API calls, requires validators.toml |

### First-run UX

On first run, MVM should:

1. Explain what the API provides
2. Ask user to confirm opt-in (default: yes)
3. Optionally ask for their ticker (for stats attribution)
4. Save preference to config

```
MVM connects to api.adavault.com to:
  - Fetch pool ticker registry (replaces validators.toml)
  - Share anonymous node health stats (optional)
  - Display network-wide dashboard

No validator keys or sensitive data are shared.

Enable API connection? [Y/n]:
Share node stats? [Y/n]:
Your pool ticker (optional):
```

---

## Implementation Phases

### Phase 1: Decentralized Registry (v2.0)

- Calidus-verified entries in known_validators.toml
- Multi-source fetch (GitHub + SPO mirrors)
- GPG-signed registry files
- MVM verifies signatures and proofs

### Phase 2: API Nodes (v2.1)

- `mvm serve` command exposes REST API
- SPOs run alongside their Midnight nodes
- Bootstrap list in config (community-maintained)
- Local chain data + registry data

### Phase 3: Aggregation (v2.2+)

- Cross-node queries for network statistics
- Client-side or coordinator-based aggregation
- Historical data and trends
- Full chain indexer capability

### Future: Fee-based Sustainability

- Query fees distributed to node operators
- Integration with Midnight native tokens
- Long-term goal, not v2.x scope

---

## API Infrastructure

**Hosting:** Decentralized - each SPO runs their own node

**Stack per node:**
- MVM binary with `serve` command
- SQLite for local storage
- Optional reverse proxy (nginx, caddy)

**No central infrastructure required.**

**Rate limits (per node, configurable):**
- `GET` endpoints: 60 req/min per IP (default)
- Node operators can adjust based on their resources

---

## Security Considerations

1. **No auth required** for GET endpoints — public data
2. **POST endpoint** validates payload schema, rejects malformed
3. **No PII collected** — ticker is closest to identifying info
4. **Standard TLS** — HTTPS only
5. **IP not stored** with stats — only used for rate limiting

---

## Open Questions

1. **Calidus key rotation** — How to handle expired proofs when SPO rotates keys?
2. **Mirror discovery** — How do new nodes find existing mirrors?
3. **Data consistency** — How to handle nodes with different sync states?
4. **Incentive bootstrap** — How to recruit first 5 mirror operators?

---

## Next Steps

1. ✅ Strategic direction validated (decentralized model)
2. Design Calidus verification flow (coordinate with Martin/ATADA)
3. Implement multi-source registry fetch
4. Create mirror hosting documentation
5. Recruit initial mirror operators
6. Implement `mvm serve` command (v2.1)

---

## Success Metrics

- 5+ SPOs hosting registry mirrors by v2.0 launch
- 90%+ of registered validators Calidus-verified
- Zero single points of failure for registry access
- Community contributions (not just ADAvault merging PRs)

---

*Questions or feedback: raise in standup or file an issue.*
