# Release Plan: v0.2.0

## Overview

v0.2.0 focuses on completing the Phase 1 MVP features from the specification, particularly improving block author attribution, validator tracking, and configuration management.

## Status: v0.1-alpha → v0.2.0

### ✓ Implemented in v0.1-alpha

- [x] Basic SYNC command with SQLite persistence
- [x] STATUS command with health checks and key verification
- [x] QUERY command (stats, blocks, gaps)
- [x] KEYS command (show, verify)
- [x] Database schema (blocks, validators, sync_status)
- [x] RPC integration (Substrate + Midnight methods)
- [x] Registration tracking with mainchain epoch
- [x] Metrics parsing (Prometheus)
- [x] Slot and epoch extraction from block digests

### 🎯 Goals for v0.2.0

#### 1. Block Author Attribution (HIGH PRIORITY)
**Problem**: Spec notes "author_index = slot_number % validator_count" but validator ordering is undocumented.

**Solution**:
- Implement `sidechain_getAriadneParameters` caching for validator set
- Calculate author index from slot number
- Store author_key in blocks table
- Add query subcommand to show blocks by author
- **Research needed**: Verify validator ordering matches AURA implementation

**Files to modify**:
- `src/commands/sync.rs` - Add author calculation
- `src/midnight/mod.rs` - Add validator set fetching and ordering
- `src/commands/query.rs` - Add author filtering

#### 2. Validator Tracking and Population (HIGH PRIORITY)
**Problem**: Validators table exists but is not populated.

**Solution**:
- Auto-populate validators table from `sidechain_getAriadneParameters`
- Track validator registration status changes over time
- Mark "our" validators based on keystore
- Update total_blocks counter when blocks are synced
- Add query subcommand to show validator stats

**Files to modify**:
- `src/db/blocks.rs` - Add validator CRUD operations
- `src/commands/sync.rs` - Populate validators during sync
- `src/commands/query.rs` - Add validator stats query

#### 3. Configuration File Support (MEDIUM PRIORITY)
**Problem**: All configuration via CLI flags, no persistent config.

**Solution**:
- Implement `mvm.toml` configuration file support
- Support environment variables as fallback
- Priority: CLI flags > Environment > Config file > Defaults
- Add `mvm config` command to show/validate config

**Files to create**:
- `src/config.rs` - Configuration loading and parsing

**Example mvm.toml**:
```toml
[rpc]
url = "http://localhost:9944"
metrics_url = "http://localhost:9615/metrics"

[database]
path = "./mvm.db"

[validator]
keystore_path = "/path/to/keystore"
label = "MyValidator"

[sync]
batch_size = 100
poll_interval = 6
finalized_only = false
```

#### 4. Enhanced Error Handling and Recovery (MEDIUM PRIORITY)
**Problem**: Sync stops on errors, limited retry logic.

**Solution**:
- Implement exponential backoff for RPC failures
- Add connection pooling/keepalive for HTTP client
- Better error messages with actionable guidance
- Sync resume from last successful block on restart

**Files to modify**:
- `src/rpc/client.rs` - Add retry logic and better error handling
- `src/commands/sync.rs` - Improve error recovery

#### 5. Deployment Support (LOW PRIORITY)
**Problem**: No systemd service templates or deployment docs.

**Solution**:
- Add `systemd/mvm-sync.service` template
- Add `systemd/mvm-status.service` and timer for alerts
- Add installation script `install.sh`
- Document deployment in README

**Files to create**:
- `systemd/mvm-sync.service`
- `systemd/mvm-status.timer`
- `install.sh`

#### 6. Enhanced Query Features (LOW PRIORITY)
**Problem**: Limited query capabilities.

**Solution**:
- Add `query validator` subcommand for per-validator stats
- Add `query performance` for blocks per hour/day
- Add `query authorship` for expected vs actual blocks
- Export queries to JSON/CSV format

**Files to modify**:
- `src/commands/query.rs` - Add new subcommands

## Testing Requirements

### Unit Tests
- [ ] Validator set ordering and author calculation
- [ ] Configuration file parsing
- [ ] Database validator operations

### Integration Tests
- [ ] Full sync with author attribution
- [ ] Validator table population
- [ ] Config file loading from multiple sources

### Manual Testing
- [ ] Sync 1000+ blocks and verify authors
- [ ] Test with multiple keystores
- [ ] Verify validator stats accuracy

## Documentation Updates

- [ ] Update README with configuration file usage
- [ ] Add DEPLOYMENT.md with systemd setup
- [ ] Update CLAUDE.md with new features
- [ ] Add examples/ directory with sample configs

## Breaking Changes

None planned - v0.2.0 should be fully backward compatible with v0.1-alpha databases.

## Timeline Estimate

- **High Priority Features**: 2-3 days
- **Medium Priority Features**: 2-3 days
- **Low Priority Features**: 1-2 days
- **Testing & Documentation**: 1-2 days

**Total**: ~1-2 weeks of development

## Success Criteria

v0.2.0 is ready for release when:

1. ✓ Block author attribution works and is verified accurate
2. ✓ Validators table is fully populated and tracked
3. ✓ Configuration file support works with all commands
4. ✓ Sync can recover from common RPC errors
5. ✓ All tests pass
6. ✓ Documentation is updated

## Future: v0.3.0 (Phase 2)

Deferred to v0.3.0:
- Health check history tracking
- Session key rotation detection
- Historical performance metrics
- Alert webhooks
- WebSocket support for real-time updates

## Notes

**Author Attribution Challenge**: The spec notes that validator ordering is undocumented. This will require either:
1. Examining Midnight's AURA implementation source code
2. Empirical testing by observing blocks and matching authors
3. Consulting Midnight documentation/team

This is the highest risk item for v0.2.0 and may need to be marked as experimental until verified.
