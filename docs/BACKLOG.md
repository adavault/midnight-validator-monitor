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

## Feature Ideas

- [ ] **Alert thresholds**: Configurable alerts when block production falls below expected rate
- [ ] **Historical performance graphs**: Show block production over time
- [ ] **Multiple node monitoring**: Support monitoring multiple validator nodes from single TUI
- [ ] **Export functionality**: Export performance data to CSV/JSON
