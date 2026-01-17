# Critical Discovery: Validator Committee vs Candidates

**Date**: 2026-01-16
**Impact**: HIGH - Current block author attribution is INCORRECT
**Status**: Requires immediate fix in v0.3.1 or v0.4

---

## Executive Summary

Our current implementation of block author attribution is **fundamentally incorrect**. We're using a validator set of 185 candidates when Midnight actually uses a **committee of ~1200 seats** for block production.

**Current (Wrong)**:
```rust
author_index = slot % 185  // Using candidate count ❌
```

**Correct**:
```rust
author_index = slot % 1200  // Using committee size ✅
```

---

## Discovery Source

### Node Logs
```
2026-01-16 08:00:00 💼 Selected committee of 1200 seats for epoch 245633
                       from 12 permissioned and 173 registered candidates
2026-01-16 08:00:00 Committee rotated: Returning 1200 validators,
                       stored in epoch 245632
```

### RPC Verification
```bash
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "state_call", "params": ["AuraApi_authorities", "0x"]}' \
  http://localhost:9944
```

**Result**: 1199 AURA authorities (32-byte keys in SCALE-encoded array)

---

## The Problem

### What We Thought

- **185 validators** (12 permissioned + 173 registered)
- Validators ordered by AURA public key
- Block author = `validators[slot % 185]`

### What's Actually Happening

- **185 candidates** are selected for the committee
- **~1200 committee seats** are filled from these candidates
- Candidates appear **multiple times** in the committee (distributed rotation)
- Block author = `committee[slot % 1200]`

### Why This Matters

1. **Incorrect Attribution**: We're currently attributing blocks to the wrong validators
2. **Invalid Statistics**: Block production counts and rankings are unreliable
3. **Broken Predictions**: Our block prediction algorithm uses wrong denominator
4. **User Trust**: Operators relying on our stats are getting bad data

---

## How the Committee Works

### Committee Structure

```
185 Candidates → Committee Builder → 1200 Seats → Block Production

Candidates:
  - 12 permissioned
  - 173 registered (valid)

Committee Seats:
  - 1200 AURA authority slots
  - Filled by repeating candidates in a deterministic pattern
  - Changes each epoch (committee rotation)
```

### Committee Assignment

The 185 candidates are distributed across 1200 seats. This means:
- Each candidate appears approximately `1200 / 185 ≈ 6.5` times
- Some validators may appear 6 times, others 7 times
- Distribution is deterministic but algorithm is opaque
- Order is deterministic (based on AURA authorities)

### Epoch Rotation

- New committee is selected each epoch
- Committee is stored in state for the epoch
- `sidechain_getEpochCommittee(epoch)` may return the committee (state permitting)
- `AuraApi_authorities` runtime call returns CURRENT committee

---

## RPC Methods

### 1. AuraApi_authorities (Runtime Call)

**Purpose**: Get current committee (AURA authorities)

**Method**:
```bash
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "state_call",
       "params": ["AuraApi_authorities", "0x"]}' \
  http://localhost:9944
```

**Response**: SCALE-encoded array of 32-byte AURA public keys

**Decoding**:
```rust
// Response format (hex): 0x[count][key1][key2]...[keyN]
// - count: 4 bytes SCALE compact-encoded
// - keys: N × 32 bytes (AURA public keys)

// Example pseudo-code:
let hex_response = response.result; // "0xc1128c44..."
let bytes = hex::decode(&hex_response[2..])?; // Remove "0x"

// Skip SCALE count (first 4 bytes for counts > 64)
let key_data = &bytes[4..];

// Parse keys (32 bytes each)
let mut authorities = Vec::new();
for chunk in key_data.chunks(32) {
    let aura_key = hex::encode(chunk);
    authorities.push(aura_key);
}

// authorities.len() should be ~1200
```

**Pros**:
- Always returns current committee
- No state pruning issues
- Direct from runtime state

**Cons**:
- SCALE encoding complexity
- Only current epoch (no historical)
- Requires manual decoding

### 2. sidechain_getEpochCommittee (Might Work)

**Purpose**: Get committee for specific epoch

**Method**:
```bash
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "sidechain_getEpochCommittee",
       "params": [245637]}' \
  http://localhost:9944
```

**Status**: ❓ Unconfirmed - may return `UnknownBlock` for pruned states

**If it works**, would return JSON array of AURA keys directly (easier than SCALE decoding)

---

## Implementation Fix

### Required Changes

#### 1. Update ValidatorSet Structure

**File**: `src/midnight/validators.rs`

**Current**:
```rust
pub struct ValidatorSet {
    pub epoch: u64,
    pub validators: Vec<Validator>,  // 185 candidates
}

pub fn get_author(&self, slot_number: u64) -> Option<&Validator> {
    let author_index = (slot_number as usize) % self.validators.len(); // WRONG
    self.validators.get(author_index)
}
```

**Fixed**:
```rust
pub struct ValidatorSet {
    pub epoch: u64,
    pub candidates: Vec<Validator>,    // 185 candidates (for reference)
    pub committee: Vec<String>,         // 1200 AURA keys (actual authority set)
}

pub fn get_author(&self, slot_number: u64) -> Option<&Validator> {
    if self.committee.is_empty() {
        return None;
    }

    // Get committee position
    let committee_index = (slot_number as usize) % self.committee.len(); // CORRECT (% 1200)
    let aura_key = &self.committee[committee_index];

    // Find candidate by AURA key
    self.candidates
        .iter()
        .find(|v| &v.aura_key == aura_key)
}
```

#### 2. Add Committee Fetching

**New Method**:
```rust
impl ValidatorSet {
    /// Fetch committee from AuraApi_authorities runtime call
    pub async fn fetch_with_committee(
        rpc: &RpcClient,
        epoch: u64,
    ) -> Result<Self> {
        // 1. Fetch candidates from AriadneParameters (existing code)
        let candidates = Self::fetch_candidates(rpc, epoch).await?;

        // 2. Fetch committee from AURA runtime
        let committee = Self::fetch_aura_committee(rpc).await?;

        Ok(Self {
            epoch,
            candidates,
            committee,
        })
    }

    /// Fetch current AURA authorities (committee)
    async fn fetch_aura_committee(rpc: &RpcClient) -> Result<Vec<String>> {
        // Call AuraApi_authorities
        let result: String = rpc
            .call("state_call", vec!["AuraApi_authorities", "0x"])
            .await?;

        // Decode SCALE-encoded response
        let hex = result.trim_start_matches("0x");
        let bytes = hex::decode(hex)?;

        // Skip first 4 bytes (SCALE compact count)
        let key_data = &bytes[4..];

        // Parse 32-byte chunks as AURA keys
        let mut authorities = Vec::new();
        for chunk in key_data.chunks(32) {
            authorities.push(format!("0x{}", hex::encode(chunk)));
        }

        Ok(authorities)
    }

    /// Fetch candidates from AriadneParameters
    async fn fetch_candidates(
        rpc: &RpcClient,
        epoch: u64,
    ) -> Result<Vec<Validator>> {
        // Existing implementation from fetch()
        // Returns 185 candidates
        todo!("Move existing fetch() logic here")
    }
}
```

#### 3. Update Block Prediction

**File**: `src/midnight/prediction.rs` (or relevant file)

**Current**:
```rust
let expected_blocks = epoch_length_slots as f64 / total_validators; // Wrong: uses 185
```

**Fixed**:
```rust
let expected_blocks = epoch_length_slots as f64 / committee_size as f64; // Correct: uses 1200
```

**Impact**:
- Old: `1800 slots / 185 validators ≈ 9.7 blocks per validator`
- New: `1800 slots / 1200 committee ≈ 1.5 blocks per committee slot`

But since validators appear ~6.5 times in committee:
- Actual: `1.5 × 6.5 ≈ 9.75 blocks per validator` (similar to old calculation)

The key difference is the **distribution** - not all validators appear exactly 6 or 7 times, so individual predictions will vary.

---

## Testing Strategy

### 1. Verification Test

```bash
# Get current slot
SLOT=$(curl -s http://localhost:9944 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"sidechain_getStatus","id":1}' \
  | jq -r '.result.sidechain.slot')

# Get current block
BLOCK=$(curl -s http://localhost:9944 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"chain_getHeader","id":1}' \
  | jq -r '.result.number')

# Get committee
COMMITTEE=$(curl -s http://localhost:9944 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"state_call",
       "params":["AuraApi_authorities","0x"],"id":1}' \
  | jq -r '.result')

# Decode committee and calculate author
# author_index = slot % committee_size
# Extract AURA key from block digest
# Verify: committee[author_index] == block's AURA key
```

### 2. Historical Validation

- Resync 1000 blocks with new algorithm
- Compare author attribution with old results
- Verify committee size is stable (~1200)
- Check validator appearance frequency (~6-7 times each)

### 3. Unit Tests

```rust
#[test]
fn test_committee_size() {
    let validator_set = ValidatorSet::fetch_with_committee(rpc, epoch).await?;
    assert_eq!(validator_set.committee.len(), 1200, "Committee should have ~1200 seats");
    assert_eq!(validator_set.candidates.len(), 185, "Should have 185 candidates");
}

#[test]
fn test_author_attribution_uses_committee() {
    let validator_set = ValidatorSet { /* ... */ };
    let author = validator_set.get_author(slot)?;

    // Verify we're using committee, not candidates
    let expected_index = slot % 1200;  // Not % 185
    assert_eq!(/* author matches committee[expected_index] */);
}
```

---

## Migration Plan

### Option A: Quick Fix (v0.3.1 patch)

1. Update `validators.rs` with committee fetching
2. Update block attribution to use committee
3. Add deprecation notice for old stats
4. **Do NOT resync** - old data remains wrong but clearly marked
5. Release as v0.3.1-alpha with warning in release notes

**Timeline**: 1-2 days

### Option B: Full Fix (Include in v0.4-beta)

1. Implement all changes above
2. Add database migration to clear old block attributions
3. Recommend full resync for accurate statistics
4. Update prediction algorithm
5. Comprehensive testing

**Timeline**: Include in 7-week v0.4-beta schedule

### Recommendation

**Option A for users**, **Option B for development**:
- Release v0.3.1-alpha patch ASAP with fix + warning
- Include comprehensive fix in v0.4-beta
- Document known inaccuracy for v0.3.0-alpha users

---

## Impact Assessment

### Data Accuracy

**Current v0.3.0-alpha**:
- Block attributions: **INCORRECT** (using 185 instead of 1200)
- Validator statistics: **UNRELIABLE**
- Performance rankings: **INACCURATE**
- Predictions: **WRONG DENOMINATOR** (but may be close due to distribution)

**After Fix**:
- Block attributions: **CORRECT**
- Validator statistics: **ACCURATE**
- Performance rankings: **RELIABLE**
- Predictions: **PRECISE**

### User Impact

**Low Impact** (surprisingly):
- Prediction error is small due to coincidence (9.7 vs 9.75 expected blocks)
- Rankings are relative, so top performers still top (just different exact counts)
- Most users use tool for monitoring, not precise analytics

**High Impact** (for precise use cases):
- Research/analysis requiring exact attribution
- Validation of specific block authorship
- Audit trails
- Slashing/rewards calculation (if ever implemented)

---

## Open Questions

### Q1: How is the 1200-seat committee constructed from 185 candidates?

**Status**: Unknown - algorithm not documented

**Hypotheses**:
- Weighted by stake? (some validators appear more if higher stake)
- Equal distribution with remainder? (some get 6, some get 7)
- Randomized selection? (unlikely - deterministic logs suggest otherwise)

**Investigation**:
- Analyze committee composition over multiple epochs
- Compare validator appearance frequencies
- Look for patterns in AURA key distribution

### Q2: Does committee change every epoch?

**Status**: Yes (confirmed by "Committee rotated" logs)

**Impact**:
- Must fetch fresh committee per epoch
- Cannot cache committee across epochs
- Need epoch-aware caching strategy

### Q3: Can we get historical committees?

**Status**: Unclear

**Methods to try**:
- `sidechain_getEpochCommittee(historical_epoch)` - may fail with "state pruned"
- Archive node might keep historical state
- May need to track committee changes in our database

**Workaround**: Store committee snapshot when we sync each epoch

---

## Recommendations

### Immediate Actions

1. ✅ **Document this discovery** (this file)
2. 🔄 **Update technical specification** with committee information
3. 🔄 **Create GitHub issue** to track fix
4. ⏳ **Plan v0.3.1 patch release** with committee support

### Medium-Term (v0.4-beta)

1. Store committee snapshots in database
2. Add committee analysis commands
3. Visualize validator distribution in committee
4. Track committee rotation patterns

### Long-Term

1. Research committee construction algorithm
2. Predict committee composition (if deterministic)
3. Optimize committee caching
4. Archive historical committees

---

## References

### Node Logs
```
💼 Selected committee of 1200 seats for epoch 245633 from 12 permissioned and 173 registered candidates
Committee rotated: Returning 1200 validators, stored in epoch 245632
```

### RPC Methods
- `state_call("AuraApi_authorities", "0x")` - Get current committee
- `get_storage("Aura", "Authorities", None)` - Alternative storage query method
- `sidechain_getEpochCommittee(epoch)` - Get epoch committee (state permitting)
- `sidechain_getAriadneParameters(epoch)` - Get candidates
- `get_storage("SessionCommitteeManagement", "NextCommittee")` - Predict next epoch committee

### External References
- **Midnight-blocklog**: https://github.com/btbf/Midnight-blocklog
  - Working implementation that correctly uses 1200-seat committee
  - Uses `slot % authority_set.len()` for attribution
  - Fetches from `Aura.Authorities` storage
  - See `MIDNIGHT_BLOCKLOG_ANALYSIS.md` for detailed analysis

### Code Files to Update
- `src/midnight/validators.rs` - Add committee support
- `src/commands/sync.rs` - Use committee for attribution
- `src/midnight/prediction.rs` - Update prediction algorithm
- `TECHNICAL_SPEC_v0.4.md` - Document committee architecture
- `RELEASE_PLAN_v0.4-beta.md` - Add committee as a fix item

---

**Status**: Discovery documented, awaiting implementation
**Priority**: HIGH - affects core functionality
**Assigned**: To be addressed in v0.3.1-alpha patch or v0.4-beta

