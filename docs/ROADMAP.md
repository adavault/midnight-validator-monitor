# MVM Roadmap

This document outlines the long-term vision for the Midnight Validator Monitor.

## Vision

MVM aims to be the essential toolkit for the Midnight ecosystem - starting with node operators, expanding to developers, and ultimately providing infrastructure for the broader ecosystem.

## Version Strategy

### v1.x - Node Operations (SPO Focus)

**Target Users:** Stake Pool Operators, testnet validators, node operators

**Theme:** "Is my node healthy?"

**Goal:** Make it quick and easy to assess node performance, diagnose problems, and fix issues. Establish solid foundations that the dev community can rely on when working with Midnight testnets.

**Key Features:**
- Real-time node health monitoring
- Block production tracking with attribution
- Validator performance analytics
- Peer connectivity diagnostics (Prometheus-based)
- Interactive TUI with drill-down views
- Shell completions and polished CLI experience
- Comprehensive help and terminology glossary
- Stable, production-ready for mainnet launch

**Milestones:**
| Version | Focus |
|---------|-------|
| v0.7 | Sparkline, shell completions, UX polish |
| v0.8 | Help glossary, Prometheus peer metrics, resource monitoring |
| v0.9 | Block detail drill-down, validator epoch performance views |
| v1.0 | Documentation polish, mainnet readiness, stability |

---

### v2.x - Protocol Understanding (Developer Focus)

**Target Users:** dApp developers, protocol researchers, integration builders

**Theme:** "What's happening on-chain?"

**Goal:** Transform MVM into a feature-rich monitor that understands the Midnight protocol. Inspired by classic machine monitors that allowed breakpoints and debugging, v2 helps developers observe and understand on-chain activity.

**Key Features:**
- **Extrinsic Decoder**: Human-readable transaction details
  - Decode SCALE-encoded extrinsics
  - Show method calls, parameters, signers
  - Display events emitted by transactions

- **Transaction Watching**:
  - Filter transactions by type, sender, method
  - Set alerts on specific transaction patterns
  - Watch specific addresses for activity

- **Block Explorer Features**:
  - Detailed block inspection in TUI
  - Event log viewer
  - State changes per block

- **Developer Debugging**:
  - "Breakpoint" style alerts (pause and inspect when condition met)
  - Transaction tracing
  - Error analysis and decoding

**Technical Requirements:**
- Midnight-specific SCALE type definitions
- Metadata parsing for runtime calls
- Enhanced database schema for events/extrinsics
- More sophisticated query language

---

### v3.x - Ecosystem Integration (Platform Focus)

**Target Users:** Service builders, dashboard creators, bot developers, ecosystem tooling

**Theme:** "Programmatic access to everything"

**Goal:** Expose MVM's rich dataset via APIs and webhooks, becoming a swiss army knife for the Midnight ecosystem. Enable other tools and services to build on top of MVM's data.

**Key Features:**
- **REST API**:
  - Query blocks, transactions, validators
  - Performance statistics endpoints
  - Node health status
  - OpenAPI/Swagger documentation

- **WebSocket API**:
  - Real-time block notifications
  - Transaction stream with filters
  - Validator status changes
  - Peer connectivity events

- **Webhook System**:
  - Configurable alert triggers
  - HTTP callbacks for events
  - Integration with notification services (Slack, Discord, PagerDuty)

- **Data Export**:
  - CSV/JSON export for analysis
  - Prometheus metrics endpoint (for Grafana dashboards)
  - Time-series data for historical analysis

**Use Cases Enabled:**
- Custom monitoring dashboards
- Alerting bots (Telegram, Discord)
- Analytics platforms
- Portfolio trackers
- Automated failover systems

---

## Infrastructure & Deployment Strategy

**Decision Date:** 2026-01-23
**Status:** Approved

### v1.x: Manual Deployment

- Binary releases via GitHub Releases (CI already in place)
- Manual deployment to test/production nodes
- No containers - single static binary is sufficient
- Test environment: mdn90 (vdumdn90) with manual deploys

**Rationale:** MVM is a ~5MB static binary with no runtime dependencies. Containers add overhead without benefit at this stage.

### v2.x: Container-First with Sidecar Support

- Multi-arch container builds (linux/amd64, linux/arm64)
- Published to container registry (Docker Hub - decision pending on costs/uptake)
- Sidecar deployment model alongside midnight-node:

```
┌─────────────────────────────────────┐
│  Pod / Compose Stack                │
│  ┌─────────────┐  ┌──────────────┐  │
│  │ midnight-   │  │     mvm      │  │
│  │   node      │◄─│   (sidecar)  │  │
│  │  :9944      │  │              │  │
│  └─────────────┘  └──────────────┘  │
└─────────────────────────────────────┘
```

- Helm chart and/or docker-compose examples
- Kubernetes-native: health probes, resource limits, service discovery

**Rationale:** Operators running Midnight nodes in production expect containerized tooling. Sidecar model simplifies deployment and networking.

### Future Considerations

- **Container Registry:** Start with GHCR (free, tied to repo), evaluate Docker Hub based on adoption
- **CD Pipeline:** May add auto-deploy to staging (mdn90) in v1.x if manual deployment becomes friction

---

## Validator Registry Strategy

**Decision Date:** 2026-01-23
**Status:** Approved

MVM includes a public validator registry (`known_validators.toml`) that maps sidechain keys to Cardano stake pool tickers. This enables human-readable validator identification in the TUI.

### Goals

1. **Community engagement** - SPOs submit PRs to be listed, increasing repo visibility
2. **Competition** - Validators want to be recognized, driving adoption
3. **No PII concerns** - Pool tickers are intentionally public identifiers

### Verification Strategy

**Preview Testnet (v1.x):**
- Trust-based PR workflow
- Validator submits PR with sidechain key + ticker
- We verify key exists on-chain, then merge
- Low friction to bootstrap community

**PreProd / Mainnet (v2.x+):**
- Calidus signature verification required
- Validator signs with pool cold key using CIP-88v2
- Proves ownership of Cardano stake pool
- Ticker auto-collected from pool.json metadata

**Tools:** [cardano-signer](https://github.com/gitmachtl/cardano-signer) by Martin (gitmachtl)

### Rationale

Calidus verification for production networks:
- Prevents ticker squatting
- Cryptographically proves pool ownership
- Aligns with Cardano ecosystem standards
- Builds relationship with tooling maintainers (Martin)

Trust-based for preview:
- Lower friction for testnet experimentation
- Faster community onboarding
- Acceptable risk (testnet only)

---

## Design Principles

Throughout all versions, MVM adheres to these principles:

1. **Operator First**: Every feature should help someone running a node
2. **Terminal Native**: TUI is the primary interface, APIs extend reach
3. **Lightweight**: Minimal dependencies, fast startup, low resource usage
4. **Offline Capable**: Core features work with local database
5. **Secure by Default**: No unsafe operations, careful with credentials
6. **Well Documented**: Clear help, examples, and explanations

## Current Status

**Latest Release:** v0.9.3

**Next Up:** v1.0 (First production release - stability verification)

See [BACKLOG.md](BACKLOG.md) for detailed feature planning.

---

*This roadmap is a living document and will evolve as the Midnight ecosystem develops.*
