# MVM Backlog

This document tracks future research items and feature ideas for the Midnight Validator Monitor.

## External IP Detection

The TUI relies on `system_unstable_networkState.externalAddresses` which includes all addresses (configured + peer-reported). This was resolved by fixing Docker port configuration - see `docs/EXTERNAL_IP_RESEARCH.md` for details.

### Future Research (Low Priority)

- [ ] **Investigate Substrate plans**: Check if Substrate/Polkadot has plans to expose configured addresses separately from discovered ones
- [ ] **libp2p address scoring**: libp2p maintains confidence scores for addresses - could help in edge cases

See `docs/EXTERNAL_IP_RESEARCH.md` for detailed research findings.

## Performance Improvements

- [ ] **Parallel RPC calls**: Some RPC calls in `fetch_rpc_data` could be made in parallel
- [ ] **Lazy committee fetching**: Only fetch committee when on Dashboard or Validators view
- [ ] **Cached block queries**: Consider caching recent blocks query results

## v1.1 Planned

- [ ] **Auto-install shell completions**: Add `mvm install --with-completions` flag that detects user's shell from `$SHELL`, installs completions to appropriate system path (`/etc/bash_completion.d/mvm` for bash, `~/.zsh/completions/_mvm` for zsh), and shows message about sourcing. Makes install experience smoother for new users.
- [ ] **File permissions documentation** (Issue #17): Document recommended multi-user setup - `midnight` user owns data, operators in `midnight` group, directory mode `775`, file mode `664`.
- [ ] **Troubleshooting guide** (Issue #18): Common issues and solutions.

## Feature Ideas

- [x] **Alert thresholds**: Configurable alerts when block production falls below expected rate *(v0.9.3 - AlertManager infrastructure added, config section ready)*
- [ ] **Historical performance graphs**: Show block production over time
- [ ] **Multiple node monitoring**: Support monitoring multiple validator nodes from single TUI
- [ ] **Export functionality**: Export performance data to CSV/JSON
- [ ] **Clipboard support**: Add keybinding (e.g., `y` to yank) to copy selected data to clipboard. Use `arboard` crate for cross-platform clipboard access. Useful for copying block hashes, validator keys, peer IDs, etc. Note: Mouse capture was removed in v0.9.2 to enable native terminal text selection as an interim solution.

## v0.8 Release Plan

### Help Screen Glossary
Add a glossary section to the help screen explaining Substrate and Midnight-specific terms:
- **Extrinsics** - transactions/calls submitted to the chain
- **Sidechain epoch** - committee rotation period (2h preview, TBD mainnet)
- **Mainchain epoch** - Cardano epoch alignment (24h preview, 5d mainnet)
- **Committee** - validators selected for block production each epoch
- **Seats** - weighted positions in the committee (stake-based)
- **AURA** - block authoring consensus mechanism
- **Grandpa** - block finalization protocol
- **Finalized** - irreversible blocks confirmed by 2/3+ validators
- **Slot** - 6-second time window for block production
- **State pruning** - removal of old blockchain state to save disk space

### Enhanced Peers View (Prometheus Metrics)
Replace/augment RPC-based peer data with richer Prometheus metrics from the node's `/metrics` endpoint.

**Known Issue (v0.6.1):** The current inbound peer count is unreliable. The `system_unstable_networkState.connectedPeers` RPC only shows peers with "dialing" endpoints (outbound connections we initiated). Inbound connections don't appear in this RPC response even when Prometheus confirms they exist. This will be fixed by using Prometheus metrics directly.

- [ ] **Connection counts by direction**: `substrate_sub_libp2p_connections_opened_total{direction="in|out"}` - accurate inbound/outbound tracking (fixes inbound count bug)
- [ ] **Connection close reasons**: `substrate_sub_libp2p_connections_closed_total{direction,reason}` - diagnose networking issues (transport-error, keep-alive-timeout, etc.)
- [ ] **Peer discovery**: `substrate_sub_libp2p_peerset_num_discovered` - total known peers in DHT
- [ ] **Pending connections**: `substrate_sub_libp2p_pending_connections` - connections being established
- [ ] **Request latency histograms**: `substrate_sub_libp2p_requests_in_success_total` / `requests_out_success_total` - sync request performance
- [ ] **Bandwidth stats**: `substrate_sub_libp2p_network_bytes_total{direction}` - already captured, could show rates

Benefits over current RPC approach:
- More accurate inbound/outbound detection (current method uses endpoint heuristics)
- Connection failure diagnostics (helps identify firewall/NAT issues)
- Performance metrics for network health assessment

### System Resource Monitoring (Requires node_exporter)
- [x] CPU, memory, disk usage display *(implemented)*
- [x] Integration with Prometheus node_exporter *(implemented)*
- [x] Alert thresholds for resource usage *(memory warning at 85%+, trend indicator)*
- [ ] CPU trend tracking (similar to memory)

### Other Candidates
- [ ] Notification system for missed blocks
- [ ] Web UI alternative to TUI
- [ ] REST API for external integrations

## v0.9.3 Completed (Discord Analysis Features)

Implemented based on Discord channel analysis of validator pain points:

### Phase 1 - High Impact (DONE)
- [x] **Registration health check enhancement**: Committee status, seats, selection probability, expected blocks, stake display
- [x] **Epoch countdown timers**: Shows time until next sidechain/mainchain epoch, highlights at 90%+

### Phase 2 - Block Production (DONE)
- [x] **Alert system infrastructure**: `AlertManager` with webhook support, `AlertConfig` in config.rs
- [x] **Committee selection stats**: Already existed, verified working

### Phase 3 - Robustness (DONE)
- [x] **Memory trend tracking**: Rising/stable/falling indicator with linear regression
- [x] **Memory warnings**: Warning at 85%+, critical coloring at 90%+
- [x] **Peer health visualization**: Health status header, diversity warnings, synced peer count

### Phase 4 - Documentation (DONE)
- [x] **Troubleshooting guide**: `mvm guide` command with topics: not-producing, registration, peers, memory, keys, setup
- [x] **Status explanation mode**: `mvm status --explain` flag for educational metric explanations

### Pending Integration
- [ ] **Alert integration with sync**: Wire AlertManager into sync command for real-time alerting
- [ ] **Add alerts section to config example**: Update `mvm config example` output

## Build Pipeline

Build VM provisioned at secondary site (vdumdn90):
- [x] Provision VM (vdumdn90: 32GB RAM, 4 CPUs, Ubuntu)
- [x] Install Rust toolchain (1.93.0) and build dependencies
- [x] Configure SSH access from vdumdn57
- [x] Create remote build script (`scripts/build-remote.sh`)
- [x] Deploy Midnight sync node for integration testing

**Build Script Usage:**
```bash
# Build only
./scripts/build-remote.sh

# Pull latest, build, and deploy
./scripts/build-remote.sh --pull --deploy

# Clean build
./scripts/build-remote.sh --clean
```

**Test Node (vdumdn90):**
- Sync-only node (no validator keys)
- Connects to partnerchains postgres on vdumds58
- RPC: `http://vdumdn90:9944`
- Metrics: `http://vdumdn90:9615`
- Compose file: `~/midnight-node/compose.yml`

### Future Enhancements
- [ ] Set up GitHub Actions self-hosted runner on vdumdn90
- [ ] Add cross-compilation for ARM64 (Mac M-series)
- [ ] Automated release builds with version tagging
- [ ] Integration test suite using test node RPC

## v0.9 Release Plan

### Block Detail Drill-Down
Add ability to select a block in Blocks view and see full details via modal popup.

**Interaction:**
- Navigate to block with j/k
- Press Enter to open detail popup
- Press Escape to close

**Detail popup shows:**
- Full block hash, parent hash
- State root, extrinsics root
- Slot number, epoch (sidechain + mainchain)
- Timestamp
- Author (sidechain key + label if known)
- Extrinsics count and list
- Finalization status

**Implementation:**
- Add `selected_index` to Blocks view state
- Add `DetailMode` enum to track popup state
- Render popup as overlay using ratatui layered rendering
- Query full block data from DB when opening

### Validator Performance Drill-Down
Add ability to select a validator in Performance view and drill into epoch-by-epoch performance.

**Interaction:**
- Navigate to validator with j/k
- Press Enter to push detail view onto view stack
- Press Escape/Backspace to pop back to Performance view

**Detail view shows:**
- Validator info (keys, label, registration status)
- Table of epochs with columns:
  - Epoch number
  - Seats allocated (from committee_snapshots)
  - Blocks produced
  - Expected vs actual ratio
- Scrollable list of epochs
- Summary stats (total blocks, avg per epoch, best/worst epochs)

**Implementation:**
- Add view stack to App state (`Vec<ViewMode>` or dedicated struct)
- Add `ValidatorDetail` view mode
- Query committee_snapshots for seat allocation per epoch
- Query blocks grouped by epoch for production counts
- May need new DB query: `get_validator_epoch_performance(sidechain_key)`

### State Management Refactor
Both features require enhanced state management:

```rust
struct AppState {
    // Existing...

    // Selection state per view
    selected_block: Option<usize>,
    selected_validator: Option<usize>,

    // Detail/popup state
    detail_mode: Option<DetailMode>,

    // View stack for drill-down navigation
    view_stack: Vec<ViewMode>,
}

enum DetailMode {
    BlockPopup(u64),           // block_number
    ValidatorDetail(String),   // sidechain_key
}
```
