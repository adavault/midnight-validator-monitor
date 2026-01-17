# Changelog

All notable changes to Midnight Validator Monitor will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0-alpha] - 2026-01-16

### Added

#### Block Author Attribution
- Validator set management with epoch-based fetching from `sidechain_getAriadneParameters`
- Automatic block author calculation using `slot_number % validator_count`
- Author public key storage in blocks table (`author_key` field)
- Validator ordering by AURA public key (matching AURA consensus)
- Support for both permissioned and registered validators (185 total)
- Automatic validator record creation and block count tracking
- Database schema with `validators` table for tracking validator statistics

#### Query Commands
- `query validators [--ours] [--limit N]` - List all validators with registration status and block counts
- `query validator <key>` - Show detailed validator information with recent block history
- `query performance [--ours] [--limit N]` - Display performance rankings with percentage shares
- Enhanced `query stats` to show validator counts and attribution percentages
- Support for filtering queries by "our" validators using `--ours` flag

#### Keys Command Enhancements
- Database integration for automatic validator marking (`--db-path` parameter)
- Automatic marking of validators as `is_ours = true` during key verification
- Block production statistics display after verification
- Performance rank calculation among all validators
- Recent blocks display for our validators
- Automatic validator record creation if not exists

#### Developer Features
- Comprehensive development checkpoint documentation (CHECKPOINT_v0.2.0.md)
- Release planning documentation (RELEASE_PLAN_v0.2.md)
- Epoch tracking with mainchain/sidechain distinction
- Batch validator set caching for sync efficiency
- Graceful error handling when validator set fetch fails

### Changed
- Sync command now uses mainchain epoch for validator set lookups (fixes registration validation)
- Continuous polling loop refreshes epoch to handle epoch transitions
- Database tracks both mainchain and sidechain epochs
- Improved logging with validator counts and attribution details

### Fixed
- Fixed `is_ours` field not being updated in validator upsert ON CONFLICT clause
- Corrected epoch parameter naming throughout sync command (mainchain_epoch)
- Fixed key verification to properly mark existing validators as "ours"

### Technical Details
- Validator set fetched once per batch (not per block) for performance
- Block author calculation: `author_index = slot_number % validator_count`
- Registration status tracked as "permissioned" or "registered"
- First seen epoch recorded for each validator
- All 185 validators tracked (12 permissioned + 173 registered)

### Testing
- Verified on 1700+ synced blocks with complete author attribution
- All 185 validators tracked with accurate block counts
- Query commands tested with various filters and limits
- Keys verification tested with database integration

## [0.1.0] - 2026-01-14

### Added
- Initial release with basic monitoring features
- `status` command for real-time validator node monitoring
- `sync` command for block synchronization to SQLite database
- `query` command for blocks, stats, and gap detection
- `keys` command for session key verification
- Support for Midnight-specific RPC methods
- Block digest parsing for AURA slot extraction
- Validator registration checking via `sidechain_getAriadneParameters`
- SQLite database with blocks and sync_status tables
- Keystore file loading and key verification
- Health, sync state, and metrics monitoring

### Technical
- JSON-RPC 2.0 client implementation
- Substrate keystore format support
- AURA PreRuntime digest log parsing
- Midnight sidechain integration
- WAL mode SQLite for better performance

---

[0.2.0-alpha]: https://github.com/adavault/midnight-validator-monitor/compare/v0.1.0...v0.2.0-alpha
[0.1.0]: https://github.com/adavault/midnight-validator-monitor/releases/tag/v0.1.0
