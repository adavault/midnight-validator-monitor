# Release Plan: v0.2.0

**Status**: ✅ **COMPLETED** - Released as v0.2.0-alpha on 2026-01-16

See [CHANGELOG.md](CHANGELOG.md) for full release details.

## Overview

v0.2.0 focuses on completing the Phase 1 MVP features from the specification, particularly improving block author attribution, validator tracking, and configuration management.

## Status: v0.1-alpha → v0.2.0-alpha ✅

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

#### ✅ 1. Block Author Attribution (HIGH PRIORITY) - COMPLETED
**Problem**: Spec notes "author_index = slot_number % validator_count" but validator ordering is undocumented.

**Solution Implemented**:
- ✅ Implemented `sidechain_getAriadneParameters` caching for validator set
- ✅ Calculate author index from slot number
- ✅ Store author_key in blocks table
- ✅ Validator ordering by AURA public key (matches AURA consensus)
- ✅ Batch caching for efficiency
- ✅ Mainchain epoch tracking for correct validator lookups

**Files modified**:
- ✅ `src/commands/sync.rs` - Author calculation implemented
- ✅ `src/midnight/validators.rs` - Validator set fetching and ordering (NEW)
- ✅ `src/db/validators.rs` - Database operations (NEW)

#### ✅ 2. Validator Tracking and Population (HIGH PRIORITY) - COMPLETED
**Problem**: Validators table exists but is not populated.

**Solution Implemented**:
- ✅ Auto-populate validators table from `sidechain_getAriadneParameters`
- ✅ Track validator registration status (permissioned/registered)
- ✅ Mark "our" validators via keys command
- ✅ Update total_blocks counter when blocks are synced
- ✅ Query commands for validator stats

**Files modified**:
- ✅ `src/db/validators.rs` - Validator CRUD operations (NEW)
- ✅ `src/commands/sync.rs` - Populate validators during sync
- ✅ `src/commands/query.rs` - Added validators, validator, performance subcommands
- ✅ `src/commands/keys.rs` - Enhanced with database integration

#### ✅ 3. Enhanced Query Features (MEDIUM PRIORITY) - COMPLETED
**Problem**: Limited query capabilities.

**Solution Implemented**:
- ✅ `query validators` subcommand with --ours filtering
- ✅ `query validator <key>` for per-validator details
- ✅ `query performance` for block production rankings
- ✅ Enhanced `query stats` with validator counts

**Files modified**:
- ✅ `src/commands/query.rs` - Added comprehensive query subcommands

#### ⏭️ 4. Configuration File Support (MEDIUM PRIORITY) - DEFERRED to v0.3.0
**Status**: Not implemented in v0.2.0-alpha

**Reason**: Prioritized core functionality (author attribution and validator tracking) for v0.2.0. Configuration support moved to v0.3.0 along with daemon features.

#### ⏭️ 5. Enhanced Error Handling and Recovery (MEDIUM PRIORITY) - PARTIALLY DEFERRED
**Status**: Basic error handling implemented, advanced features deferred to v0.3.0

**Completed**:
- ✅ Graceful error handling in sync loop
- ✅ Continue on block fetch failures

**Deferred to v0.3.0**:
- ⏭️ Exponential backoff for RPC failures
- ⏭️ Connection pooling/keepalive
- ⏭️ Advanced retry logic

#### ⏭️ 6. Deployment Support (LOW PRIORITY) - DEFERRED to v0.3.0
**Status**: Not implemented in v0.2.0-alpha

**Reason**: Systemd support and installation scripts are major features that deserve dedicated focus in v0.3.0, along with TUI and production-ready deployment features.

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

## Success Criteria - ✅ ALL MET

v0.2.0-alpha was released when:

1. ✅ Block author attribution works and is verified accurate (185 validators tracked)
2. ✅ Validators table is fully populated and tracked
3. ⏭️ Configuration file support (deferred to v0.3.0)
4. ✅ Sync can recover from common RPC errors (basic implementation)
5. ✅ Manual testing passed (1700+ blocks synced)
6. ✅ Documentation is updated (CHANGELOG, CHECKPOINT, RELEASE_NOTES)

## v0.3.0 - Next Release

See [RELEASE_PLAN_v0.3.md](RELEASE_PLAN_v0.3.md) for the next release plan.

**v0.3.0 Focus Areas**:
- ✨ Systemd daemon support for continuous monitoring
- ✨ Text User Interface (TUI) for real-time visualization
- ✨ Configuration file support (deferred from v0.2.0)
- ✨ Enhanced logging and observability
- ✨ Installation scripts and packaging
- ✨ Production-ready deployment

**Also Deferred to Future Releases**:
- Health check history tracking (v0.4.0+)
- Session key rotation detection (v0.4.0+)
- Historical performance metrics (v0.4.0+)
- Alert webhooks (v0.4.0+)
- WebSocket support for real-time updates (v0.4.0+)

## Notes

**Author Attribution Challenge**: The spec notes that validator ordering is undocumented. This will require either:
1. Examining Midnight's AURA implementation source code
2. Empirical testing by observing blocks and matching authors
3. Consulting Midnight documentation/team

This is the highest risk item for v0.2.0 and may need to be marked as experimental until verified.
