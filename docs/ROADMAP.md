# MVM Roadmap

This document outlines the long-term vision for the Midnight Validator Monitor.

## Vision

MVM aims to be the essential toolkit for the Midnight ecosystem - starting with node operators, building community-owned data infrastructure, and ultimately providing developer tooling for the broader ecosystem.

## Strategic Principle

> **"Our competitive edge will come in other ways, not by hoarding central control points."**

Cardano's ethos is true decentralization. MVM embodies this - designed for community ownership, resilience, and neutrality from the start. ADAvault leads development but doesn't control the infrastructure.

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

### v2.x - Decentralized Datahub (Community Infrastructure)

**Target Users:** SPOs, ecosystem builders, data consumers

**Theme:** "Community-owned network data"

**Goal:** Build decentralized data infrastructure for Midnight - a koios-style indexer owned and operated by the validator community, not a single entity.

**Key Features:**

- **Decentralized Registry (v2.0)**:
  - Calidus-verified pool ticker registry
  - Multi-source fetch with fallback (GitHub + SPO mirrors)
  - GPG-signed registry files
  - Cryptographic proof of pool ownership

- **API Nodes (v2.1)**:
  - `mvm serve` command for SPOs to run API endpoints
  - Local chain data: blocks, validators, performance
  - Bootstrap discovery from community-maintained list
  - No single point of failure

- **Data Aggregation (v2.2+)**:
  - Network-wide health metrics
  - Cross-node validator statistics
  - Historical performance data
  - Block and transaction indexing

**Incentive Model:**
- Visibility: Contributing SPOs listed in TUI, docs, dashboards
- Intrinsic: SPOs want healthy ecosystem
- Access: Contributors get premium features (future)
- Fees/Tokens: Query fees distributed to operators (long-term goal)

**Design Goals:**
1. Resilience - No single point of failure
2. Neutrality - Community-owned, not ADAvault-dependent
3. Cost sharing - Distributed hosting across SPOs
4. Decentralization ethos - Aligned with Cardano values

---

### v3.x - Developer Tooling (Protocol Understanding)

**Target Users:** dApp developers, protocol researchers, integration builders

**Theme:** "What's happening on-chain?"

**Goal:** Transform MVM into a feature-rich monitor that understands the Midnight protocol. Deferred from original v2.x to prioritize community infrastructure.

**Key Features:**
- **Extrinsic Decoder**: Human-readable transaction details
- **Transaction Watching**: Filter and alert on tx patterns
- **Block Explorer Features**: Detailed inspection, event logs
- **Developer Debugging**: Breakpoints, tracing, error analysis

**Note:** This was originally v2.x scope. Deferred to allow focus on decentralized datahub, which has a window of opportunity in the Midnight ecosystem.

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

## CI/CD Maturity

**Decision Date:** 2026-01-23
**Status:** In Progress

### Current State (v1.0)

| Job | Purpose | Status |
|-----|---------|--------|
| Build | Compile release binary | Active |
| Test | Run cargo test suite | Active |
| Format | Enforce cargo fmt | Active |
| Clippy | Lint with warnings as errors | Active |
| Security Audit | rustsec dependency scan | Active |
| PII Check | Detect leaked IPs in config files | Active |
| Release | Tag-triggered binary releases | Active |

### Release Automation

Triggered on `v*` tags, the release workflow:
1. Builds binaries for linux/amd64 and linux/arm64
2. Creates tarball with SHA256 checksums
3. Publishes GitHub Release with assets
4. Uses `docs/RELEASE_NOTES_vX.Y.Z.md` if present, otherwise auto-generates notes
5. Marks alpha/beta/rc tags as pre-release

**Release process:**
```bash
# 1. Update version in Cargo.toml
# 2. Create release notes: docs/RELEASE_NOTES_v1.0.0.md
# 3. Commit and tag
git tag v1.0.0
git push origin v1.0.0
# 4. CI builds and publishes release automatically
```

### v2.0 Planned: Integration Testing

**Goal:** Catch RPC compatibility regressions before release

**Approach:**
- Nightly scheduled workflow (not on every PR - too slow/expensive)
- Connect to Midnight preview testnet endpoint
- Run smoke tests:
  - `system_health` - node reachable
  - `system_version` - parse version response
  - `chain_getHeader` - fetch and decode block header
  - `sidechain_getStatus` - Midnight-specific RPC works
- Store test results for trend analysis
- Alert on failure (GitHub issue or notification)

**Infrastructure options:**
1. **Public testnet endpoint** - simplest, depends on Midnight providing stable endpoint
2. **Self-hosted testnet node** - more control, higher maintenance
3. **Mock RPC server** - fastest, but doesn't catch real compatibility issues

**Decision:** Start with public endpoint if available, fall back to mock for CI reliability.

### Future Considerations

| Enhancement | Priority | Target |
|-------------|----------|--------|
| MSRV check | Medium | v1.1 |
| Binary size tracking | Low | v2.0 |
| Changelog enforcement | Low | v2.0 |
| Signed releases (sigstore) | Medium | v2.0+ (post-mainnet) |
| Auto-deploy to staging | Low | If manual becomes friction |

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

**Latest Release:** v1.0.0

**Next Up:**
- v1.1 - UX improvements, documentation (Issues #17, #23, #27-31)
- v2.0 - Decentralized registry MVP (Calidus verification, multi-source fetch)

See [BACKLOG.md](BACKLOG.md) for detailed feature planning.

---

## Implementation Phases

| Phase | Version | Focus | Key Deliverables |
|-------|---------|-------|------------------|
| Current | v1.x | SPO Operations | Node health, block production, TUI |
| Next | v2.0 | Registry | Calidus verification, multi-source fetch, signed files |
| Then | v2.1 | API | `mvm serve` command, REST endpoints, discovery |
| Later | v2.2 | Data | Block/tx endpoints, performance history, aggregation |
| Future | v3.0 | Developer | Extrinsic decoder, tx watching |

---

*This roadmap is a living document and will evolve as the Midnight ecosystem develops.*
