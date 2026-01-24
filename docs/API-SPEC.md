# MVM API Specification

**Status:** Draft
**Author:** CxO
**Date:** 2026-01-24
**Target:** v1.1.0 or next testnet (whichever comes first)

---

## Overview

This spec defines a central API that MVM can optionally connect to, providing:

1. **Pool ticker registry** — replaces local `validators.toml`
2. **Node stats telemetry** — optional push of validator health data
3. **Network dashboard** — aggregate view of all opted-in validators

### Design Principles

- **Opt-in by default** — new installs connect to API for better UX
- **Opt-out available** — privacy-conscious users can run fully local
- **Transparent** — users know exactly what's shared
- **Minimal data** — collect only what adds value

---

## API Endpoints

Base URL: `https://api.adavault.com/v1/midnight`

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

### Phase 1: Read-only (v1.1.0)

- `GET /tickers` endpoint live
- MVM fetches tickers from API
- No stats push yet
- Validates the integration pattern

### Phase 2: Telemetry (v1.2.0)

- `POST /stats` endpoint live
- MVM pushes node health
- `GET /network` returns aggregates
- Dashboard on adavault.com

### Phase 3: Enhanced (v1.3.0+)

- Historical data / trends
- Alerting integration
- Validator leaderboards (opt-in)
- API keys for rate limiting if needed

---

## API Infrastructure

**Hosting:** On-prem Express API (aligns with R1.1 website roadmap)

**Stack:**
- Express.js (Node)
- SQLite for storage (consistent with MVM philosophy)
- Reverse proxy via existing infrastructure

**Rate limits:**
- `GET` endpoints: 60 req/min per IP
- `POST /stats`: 1 req/min per IP (expected: 1 per 5 min)

---

## Security Considerations

1. **No auth required** for GET endpoints — public data
2. **POST endpoint** validates payload schema, rejects malformed
3. **No PII collected** — ticker is closest to identifying info
4. **Standard TLS** — HTTPS only
5. **IP not stored** with stats — only used for rate limiting

---

## Open Questions

1. **Ticker registry population** — manual curation or self-registration?
2. **Mainnet vs testnet** — same API with network param, or separate endpoints?
3. **Rate limiting strategy** — API keys for heavy users, or IP-based sufficient?
4. **Data retention** — how long to keep historical stats?

---

## Next Steps

1. CxO to review with CEO ✓
2. MVM team to assess integration complexity
3. Web team to scaffold API endpoints (R1.1 alignment)
4. Target Phase 1 for next testnet or v1.1.0

---

*Questions or feedback: raise in standup or file an issue.*
