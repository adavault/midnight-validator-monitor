# Stake Allocation Research: Cardano ↔ Midnight Committee Composition

**Status**: Research Phase
**Date Started**: 2026-01-16
**Priority**: HIGH - Required for accurate block prediction algorithm

---

## Executive Summary

This document tracks our research into understanding how Cardano stake pool performance influences Midnight blockchain committee composition and block production rights.

**Key Hypothesis**: The 1200-seat committee is not equally distributed among 185 candidates, but rather **weighted by Cardano stake pool performance** with an epoch lag.

---

## Research Questions

### Primary Questions

1. **How is the 1200-seat committee constructed from 185 candidates?**
   - Is seat allocation proportional to Cardano stake?
   - Is it based on blocks produced by the associated Cardano pool?
   - Is there a minimum/maximum seat allocation per validator?
   - What is the exact formula?

2. **What is the epoch lag mechanism?**
   - Does Midnight epoch N use Cardano performance from epoch N-1, N-2, or another offset?
   - How do the epoch boundaries align between Cardano mainchain and Midnight sidechain?
   - Is the lag consistent or does it vary?

3. **Where is the allocation data stored?**
   - Which dbsync tables contain stake allocation data?
   - Can we query this from the Midnight node directly?
   - Is there an RPC method that exposes stake/allocation info?

4. **Can we predict future allocations?**
   - Given current Cardano stake/performance, can we predict next epoch's committee?
   - What's the prediction accuracy?
   - What variables affect the prediction?

### Secondary Questions

5. **Registration mechanism**: How are Cardano pool keys linked to Midnight validator keys?
6. **Dynamic changes**: How do stake changes affect allocation mid-epoch vs between epochs?
7. **Special cases**: Do permissioned validators (12 of them) have guaranteed seats?

---

## Data Sources

### 1. Midnight Partnerchain (vdumds58)

**Available Data**:
- Running Midnight node with RPC access
- dbsync instance (Cardano database sync)
- Validator registration data
- Committee history

**Access Method**:
```bash
ssh vdumds58
# Inspect partnerchain containers
docker ps | grep midnight
docker logs <container_id>

# Access dbsync database
docker exec -it <dbsync_container> psql -U postgres
```

**Key RPC Methods**:
- `sidechain_getAriadneParameters(epoch)` - Validator registrations and stakes (185 candidates)
- `state_call("AuraApi_authorities", "0x")` - Current committee (1200 AURA keys) ✅ **PRIMARY METHOD**
- `get_storage("Aura", "Authorities", None)` - Alternative: Storage query for committee
- `get_storage("SessionCommitteeManagement", "NextCommittee")` - Predict next epoch committee
- `sidechain_getEpochCommittee(epoch)` - Historical committees (if state permits)

### 2. Cardano Node (vducdn59)

**Available Data**:
- Cardano preview network node
- Stake pool performance metrics
- Registration transaction data
- Signing keys used for Midnight registration

**Access Method**: TBD (user to provide access details)

**Key Information**:
- Cardano pool ID(s) associated with Midnight validators
- Stake pool performance history
- Registration certificate details
- Signing key relationships

### 3. Existing Documentation

**Reference Files**:
- `VALIDATOR_COMMITTEE_DISCOVERY.md` - Committee structure (185 → 1200)
- `TECHNICAL_SPEC_v0.4.md` - Current understanding of AURA consensus
- Node logs showing "Selected committee of 1200 seats from 185 candidates"

---

## Investigation Plan

### Phase 1: Data Collection (Days 1-2)

**Midnight Committee Data** (PRIMARY FOCUS):
- [ ] Fetch committee for current epoch via `AuraApi_authorities` ✅ METHOD CONFIRMED
- [ ] Fetch committee for previous 5-10 epochs (if available via storage queries)
- [ ] **Count appearance frequency**: How many times each AURA key appears in the 1200 seats
- [ ] Fetch AriadneParameters for corresponding epochs (185 candidates + stakes)
- [ ] **Map AURA keys to candidates**: Match committee members to the 185 candidates
- [ ] **Calculate distribution**: Seats per candidate (min, max, average, variance)
- [ ] Document epoch numbers and timestamps

**Cardano Stake Data**:
- [ ] Identify Cardano pool ID(s) linked to Midnight validators
- [ ] Query stake pool performance for last 10 epochs
- [ ] Get active stake amounts per epoch
- [ ] Map pool IDs to Midnight validator keys via registration data
- [ ] Extract stake from AriadneParameters.candidateRegistrations

**dbsync Analysis**:
- [ ] Explore dbsync schema
- [ ] Identify tables containing stake allocation
- [ ] Query pool performance metrics
- [ ] Find registration transaction references
- [ ] Check for committee construction logs/data

**Reference Implementation Study**:
- [x] ✅ Analyzed Midnight-blocklog implementation
- [x] ✅ Confirmed: `slot % authority_set.len()` is correct
- [x] ✅ Confirmed: Stake NOT used in slot calculation
- [ ] Test `NextCommittee` storage availability and reliability

### Phase 2: Correlation Analysis (Days 3-4)

**Committee Composition Analysis** (CORE RESEARCH QUESTION):
- [ ] Create dataset: Epoch → Candidate → Seat Count in Committee
- [ ] Calculate each candidate's percentage of 1200 seats
- [ ] Compare with candidate's Cardano stake percentage
- [ ] Compare with candidate's Cardano block production
- [ ] **Test Hypothesis 1**: Equal distribution (~6.5 seats per candidate)
- [ ] **Test Hypothesis 2**: Stake-proportional distribution
- [ ] **Test Hypothesis 3**: Block-production-proportional distribution

**Epoch Lag Identification**:
- [ ] Align Midnight epochs with Cardano epochs (epoch number mapping)
- [ ] For Midnight epoch N, extract committee composition
- [ ] Compare with Cardano stake from epochs N, N-1, N-2, N-3
- [ ] Test correlation for each lag hypothesis
- [ ] Identify which Cardano epoch's data best predicts Midnight committee at epoch N

**Statistical Analysis**:
- [ ] Calculate correlation coefficient: Cardano stake (various lags) ↔ Midnight seats
- [ ] Calculate correlation: Cardano blocks (various lags) ↔ Midnight seats
- [ ] Identify permissioned validators (special case: guaranteed seats?)
- [ ] Identify newly registered validators (first epoch behavior?)
- [ ] Document confidence intervals and prediction accuracy
- [ ] Test if distribution is deterministic or has variance

**Key Question to Answer**:
> **If a validator has X% of total Cardano stake in epoch N-k, how many seats (out of 1200) do they get in Midnight epoch N?**

### Phase 3: Formula Derivation (Day 5)

**Mathematical Model**:
- [ ] Derive formula for expected seats based on stake
- [ ] Account for minimum/maximum seat constraints
- [ ] Include epoch lag in formula
- [ ] Validate formula against historical data
- [ ] Calculate prediction accuracy

**Edge Cases**:
- [ ] Zero-stake validators (permissioned?)
- [ ] Newly registered validators
- [ ] Deregistered validators
- [ ] Validators with changing stake

---

## Data Collection Template

### Committee Snapshot

```
Epoch: _______
Timestamp: _______
Total Seats: 1200
Total Candidates: 185

Validator Appearances:
┌────────────────────────────────┬──────────┬─────────┬──────────────┐
│ Validator (Sidechain Key)      │ Seats    │ % of    │ Cardano Pool │
│                                 │          │ Total   │ ID (if known)│
├────────────────────────────────┼──────────┼─────────┼──────────────┤
│ 0x1234...                       │ 8        │ 0.67%   │ pool1xyz...  │
│ 0x5678...                       │ 12       │ 1.00%   │ pool1abc...  │
│ ...                             │ ...      │ ...     │ ...          │
└────────────────────────────────┴──────────┴─────────┴──────────────┘
```

### Cardano Stake Data

```
Epoch: _______
Pool ID: pool1xyz...

Performance:
- Active Stake: _______ ADA
- Pool Percentage: _______ %
- Blocks Produced: _______
- Expected Blocks: _______
- Performance Ratio: _______

Midnight Mapping:
- Sidechain Key: 0x1234...
- Committee Seats: _______
- Expected Seats (if equal): 6.49 (1200/185)
- Actual vs Expected: _______ %
```

---

## Preliminary Findings

### Observation 1: Committee Size Confirmed
- ✅ Committee has ~1200 seats (1199 observed via RPC)
- ✅ 185 candidates fill these seats
- ✅ Each candidate appears multiple times (avg: 6.49 times)

### Observation 2: Unequal Distribution (Hypothesis)
- ❓ Not all validators appear exactly 6-7 times
- ❓ Variation suggests stake-weighted allocation
- ⏳ Requires data collection to confirm

### Observation 3: Epoch Lag (Hypothesis)
- ❓ User reports "at least one epoch" lag
- ❓ Cardano performance affects future Midnight allocation
- ⏳ Requires cross-chain epoch alignment analysis

### Observation 4: dbsync Integration
- ❓ Data collected from dbsync in partnerchain containers
- ❓ Suggests Midnight node queries Cardano state directly
- ⏳ Requires container inspection

### Observation 5: Reference Implementation Found (2026-01-16)
- ✅ **Midnight-blocklog** repository provides working implementation
- ✅ Confirmed: Use `Aura.Authorities` storage (1200 members) for block attribution
- ✅ Confirmed: Correct formula is `slot % authority_set.len()`
- ✅ Confirmed: Authority set changes per epoch (committee rotation)
- ❗ **KEY FINDING**: Stake is displayed but NOT used in slot calculation
- ❓ **Implication**: Stake weighting must happen during committee construction (185 → 1200)
- ✅ `NextCommittee` storage exists for predicting next epoch
- 📄 See `MIDNIGHT_BLOCKLOG_ANALYSIS.md` for detailed analysis

### Observation 6: Slot Assignment is Round-Robin
- ✅ Once committee is constructed, slots are assigned equally via round-robin
- ✅ No per-slot stake weighting after committee is built
- ❓ **Critical Question**: How does stake affect the 185 → 1200 mapping?
  - Equal distribution (~6.5 seats per candidate)?
  - Proportional to stake (high stake = more seats)?
  - Based on Cardano block production?

---

## Expected Outcomes

### Success Criteria

1. **Formula Derivation**: Mathematical formula to calculate expected committee seats from Cardano stake
2. **Epoch Lag**: Precise epoch offset (e.g., "Midnight epoch N uses Cardano epoch N-2 data")
3. **Prediction Accuracy**: >90% accuracy in predicting seat allocation
4. **Documentation**: Complete technical specification update

### Deliverables

1. **This Document**: Complete findings and data
2. **Updated TECHNICAL_SPEC_v0.4.md**: Accurate stake allocation model
3. **Implementation**: `src/midnight/stake.rs` with prediction logic
4. **Tests**: Validation against historical data

---

## Implementation Impact

### Block Prediction Algorithm Update

**Current (Incorrect) Assumption**:
```rust
let expected_blocks = epoch_slots / total_validators;  // Assumes equal distribution
```

**With Stake Weighting** (Expected):
```rust
let expected_seats = calculate_expected_seats(
    validator_stake,
    total_stake,
    committee_size,  // 1200
);
let expected_blocks = epoch_slots * (expected_seats / committee_size);
```

### Code Changes Required

**New Files**:
- `src/midnight/stake.rs` - Stake calculation and seat prediction
- `src/cardano/dbsync.rs` - Query Cardano stake data (if needed)
- `src/midnight/epoch_lag.rs` - Handle epoch alignment

**Modified Files**:
- `src/midnight/prediction.rs` - Use stake-weighted predictions
- `src/midnight/validators.rs` - Store stake information
- `src/db/schema.rs` - Add stake tracking tables

---

## Access Requirements Checklist

- [ ] SSH access to vducdn59 (Cardano node)
- [ ] Cardano node RPC endpoint URL and credentials
- [ ] Cardano signing keys used for Midnight registration
- [ ] Documentation on stake allocation (if available)
- [ ] Pool ID(s) for test validators

**Note**: vdumds58 access already available

---

## Timeline

**Week 2 of v0.4-beta** (Following committee fix in Week 1):

- **Day 1-2**: Data collection from both networks
- **Day 3-4**: Correlation analysis and pattern identification
- **Day 5**: Formula derivation and validation
- **Day 6**: Documentation and specification update
- **Day 7**: Buffer for unexpected findings

---

## Key Learnings from Midnight-blocklog

**Repository**: https://github.com/btbf/Midnight-blocklog (analyzed 2026-01-16)

### Confirmed Facts

1. ✅ **Authority Set = Committee**: The `Aura.Authorities` storage contains ~1200 AURA public keys
2. ✅ **Correct Formula**: `slot % authority_set.len()` (where len ≈ 1200)
3. ✅ **Per-Epoch Refresh**: Authority set changes each epoch (committee rotation)
4. ✅ **Round-Robin Assignment**: Once committee is built, slots are assigned equally
5. ✅ **Stake is Informational**: Stake is displayed but NOT used in slot-level calculations

### Critical Insight

**The 1200-seat committee IS the source of truth for block attribution.**

**The open question is**: How do 185 candidates get mapped to 1200 committee seats?

### Possible Mechanisms

**Hypothesis A: Equal Distribution**
- Each of 185 candidates gets 1200/185 ≈ 6.486 seats
- 108 candidates get 7 seats, 77 get 6 seats (1200 total)
- Distribution algorithm: deterministic rounding or random selection

**Hypothesis B: Stake-Proportional**
- Candidates with higher stake get more seats in the 1200
- Example: 2% stake → ~24 seats (2% × 1200)
- Minimum/maximum seat constraints may apply

**Hypothesis C: Block-Production-Proportional**
- Based on Cardano pool's recent block production
- Rewards active/performing validators with more slots
- Correlation with past performance

**Hypothesis D: Hybrid**
- Minimum guaranteed seats for all registered validators
- Additional seats allocated proportionally by stake
- Example: Each gets 4 base + proportional bonus

### What We Need to Determine

1. **Seat Allocation Formula**: How are 1200 seats divided among 185 candidates?
2. **Stake Influence**: Does stake affect allocation, and if so, how?
3. **Epoch Lag**: Which Cardano epoch's data determines Midnight epoch N committee?
4. **Predictability**: Can we accurately predict future committee composition?

### Technical Approach Validated

Their implementation confirms our approach is sound:
- Use `state_call("AuraApi_authorities")` to get committee ✅
- Use SCALE decoding for the response ✅
- Store committee snapshots per epoch ✅
- Calculate author as `committee[slot % committee.len()]` ✅

See `MIDNIGHT_BLOCKLOG_ANALYSIS.md` for full technical details.

---

## Questions for User

1. Can you provide SSH/RPC access to vducdn59 (Cardano node)?
2. Which Cardano pool ID(s) are associated with your Midnight validator?
3. Do you have the signing keys used for registration transactions?
4. Is there any existing documentation on the stake allocation mechanism?
5. Should we focus on preview network or mainnet data?
6. **NEW**: Do you know if the committee allocation is stake-weighted or equal distribution?

---

## Notes and Observations

*(To be filled during research)*

### 2026-01-16 - Initial Research

**Cardano Node Access Established** (vducdn59):
- Network: Preview testnet (testnet-magic 2)
- Current epoch: 1179
- Pool ID: `pool1myvqymdmf9f26746d6uvfk34hqr7lq998n43xprgz4c27tpa6rd`
- Pool ID (hex): `d918026dbb4952ad7aba6eb8c4da35b807ef80a53ceb1304681570af`
- Pool name: ADAvault_Preview
- Current stake: ~1.258M ADA (0.12% of network)
- Socket: `/opt/cardano/cnode/sockets/node.socket`

**Stake Snapshots** (lovelace):
- stakeGo: 1,257,819,155,887 (future epoch)
- stakeMark: 1,258,876,254,658 (2 epochs ahead)
- stakeSet: 1,258,136,022,762 (next epoch)

**Network Total Stake**: ~1.066T ADA

**Midnight Validator Keys** (vdumds58):
- AURA key: `0xe05be3c28c72864efc49f4f12cb04f3bd6f20fdbc297501aa71f8590273b3e1e`
- Sidechain key: `0x037764d2d83c269030fef6df5aeb4419c48762ada2cf20b0e4e6ede596809f4700`
- GRANDPA key: `0xf5a39df9227f630754f78bbae43bd66a693612eeffa9ceec5681f6c05f48d0e8`
- Keystore: `/home/midnight/midnight-node-docker/data/chains/partner_chains_template/keystore/`

**Cardano Registration Keys** (used from vdumds58 ~/priv):
- Payment vkey: `daa28e3f127bafe647f883e1a1d2c95de31ec1e56bf15ef498af5d6d532ab8e3`
- Used to register Midnight validator with Cardano pool

**Current Committee Analysis** (Epoch 1179):
- Committee size: **1199 members** (expected ~1200) ✅
- Our AURA key appearances: **0** (not in current committee)
- Reason: Validator appears to NOT be registered in epoch 1179

**Critical Discovery - Bug Demonstration**:
- Database shows our validator produced **23 blocks** in epoch 1179
- BUT: Validator is NOT in the AriadneParameters.candidateRegistrations for epoch 1179
- BUT: Validator AURA key is NOT in the current 1199-member committee
- **Conclusion**: These 23 blocks are **FALSE ATTRIBUTIONS** due to our `slot % 185` bug!

**Proof of Bug Impact**:
```
Database blocks attributed to us (WRONG):
  Block 3361844, slot 294761763, epoch 1179
  Block 3361664, slot 294761578, epoch 1179
  Block 3361487, slot 294761393, epoch 1179
  (and 20 more...)

Reality check:
  - Our validator NOT in committee → Cannot produce blocks
  - Blocks ARE being produced → By different validators
  - Wrong attribution due to: slot % 185 instead of slot % 1200
```

**This demonstrates**:
1. The bug causes incorrect block attributions even when validator is inactive
2. Statistics are completely unreliable with current implementation
3. Critical fix is essential before any analysis can proceed

**Next Steps**:
- ~~Connect to vdumds58~~ ✅ DONE
- ~~Find Midnight validator keys~~ ✅ DONE
- ~~Link Cardano pool to validator~~ ✅ DONE
- ~~Fetch current committee~~ ✅ DONE - 1199 members confirmed
- **PRIORITY**: Implement committee fix (Week 1) before continuing research
- **After fix**: Re-sync blocks with correct attributions
- **Then**: Analyze actual committee composition and stake correlation

---

**Status**: Awaiting access to Cardano node and initial data collection
**Next Action**: Begin dbsync analysis on vdumds58
**Blocking**: Cardano node access for complete correlation analysis
