# Release Notes: v0.2.0-alpha

**Release Date**: 2026-01-16

## 🎉 Major Features

### Block Author Attribution System
Track which validator authored each block with complete attribution tracking:
- Automatic block author calculation from slot numbers
- Support for 185+ validators (permissioned + registered)
- Validator set management with epoch-based updates
- Block count tracking per validator

### Comprehensive Validator Queries
New query commands for validator insights and performance analysis:
```bash
# List all validators with stats
mvm query validators --limit 20

# Show specific validator details
mvm query validator <key>

# Performance rankings
mvm query performance --limit 10

# Filter by our validators
mvm query validators --ours
mvm query performance --ours
```

### Enhanced Keys Command
Automatic validator tracking and statistics:
- Marks validators as "ours" during verification
- Displays block production statistics
- Shows performance rank among all validators
- Lists recent blocks produced

## 📊 What You Can Do Now

1. **Track Block Production**
   - See which validators are producing blocks
   - Monitor your validator's performance rank
   - Calculate block share percentages

2. **Query Validator Data**
   - List all validators with registration status
   - Filter by permissioned vs registered
   - View detailed validator information

3. **Monitor Performance**
   - Rankings by block production
   - Performance share calculations
   - Filter to show only your validators

## 🔧 Technical Improvements

- Validator ordering by AURA public key (matches consensus)
- Batch validator set caching for sync efficiency
- Mainchain/sidechain epoch tracking
- Graceful error handling for validator fetching
- Database schema with validators table

## 🐛 Bug Fixes

- Fixed `is_ours` field not updating during validator upsert
- Corrected epoch parameter usage (mainchain vs sidechain)
- Fixed key verification validator marking

## 📈 Testing Results

- ✅ Tested on 1700+ blocks with complete attribution
- ✅ All 185 validators tracked accurately
- ✅ Query commands verified with various filters
- ✅ Keys verification with database integration tested

## 🚀 Quick Start

```bash
# Download and build
git clone https://github.com/adavault/midnight-validator-monitor.git
cd midnight-validator-monitor
git checkout v0.2.0-alpha
cargo build --release

# Sync blocks with author attribution
./target/release/mvm sync --db-path ./mvm.db

# View validator statistics
./target/release/mvm query stats

# Check your validator performance
./target/release/mvm keys --keystore /path/to/keystore verify

# View performance rankings
./target/release/mvm query performance --limit 10
```

## 📚 Documentation

- See [CHANGELOG.md](CHANGELOG.md) for detailed changes
- See [CHECKPOINT_v0.2.0.md](CHECKPOINT_v0.2.0.md) for development notes
- See [CLAUDE.md](CLAUDE.md) for project overview and commands

## ⚠️ Alpha Release Notes

This is an alpha release for testing and feedback. All HIGH and MEDIUM priority v0.2.0 features are complete and tested. Please report any issues on GitHub.

## 🙏 Credits

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>

---

**Full Changelog**: [v0.1.0...v0.2.0-alpha](https://github.com/adavault/midnight-validator-monitor/compare/v0.1.0...v0.2.0-alpha)
