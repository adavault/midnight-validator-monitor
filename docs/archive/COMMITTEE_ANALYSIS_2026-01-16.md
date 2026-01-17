# Committee Composition Analysis - Epoch 1179

**Date**: 2026-01-16
**Epoch**: 1179 (mainchain)
**Network**: Testnet-02

---

## Executive Summary

**CRITICAL FINDING**: The committee structure is MORE COMPLEX than previously understood.

- **Committee size**: 1199 members
- **Unique validators**: 325 (NOT 185 or 194!)
- **Seat distribution**: Highly variable (1 to 15 seats per validator)
- **Average seats**: 3.69 per validator

This contradicts our previous assumption that "185 candidates → 1200 seats".

---

## Committee Composition

### Size Analysis

```
Total committee members: 1199
Unique AURA keys:        325
```

### Seat Allocation Distribution

```
Minimum seats per validator: 1
Maximum seats per validator: 15
Average seats per validator: 3.69
Median seats:               ~3 (estimated)
```

### Top 10 Validators by Seat Count

| Rank | AURA Key (prefix) | Seats | Percentage |
|------|-------------------|-------|------------|
| 1    | e2b67aed7f263090  | 15    | 1.25%      |
| 2    | e2b67aed7f263090  | 13    | 1.08%      |
| 3    | b215b1b779e2d142  | 13    | 1.08%      |
| 4    | af737ecacfc30213  | 13    | 1.08%      |
| 5    | af737ecacfc30213  | 13    | 1.08%      |
| 6    | 2173467843c471b4  | 12    | 1.00%      |
| 7    | 0d72ce7dc70a0915  | 12    | 1.00%      |
| 8    | d084859b99078c28  | 12    | 1.00%      |
| 9    | d084859b99078c28  | 12    | 1.00%      |
| 10   | 2173467843c471b4  | 12    | 1.00%      |

### Bottom Validators

Many validators have only **1 seat** (0.08% of committee).

---

## Candidate Registration Data

### AriadneParameters for Epoch 1179

```
Permissioned candidates: 12
Registered candidates:   182
Total candidates:        194
```

### The Discrepancy

**Expected**: 194 candidates → 1199 committee seats
**Reality**: 325 unique validators in committee

**Gap**: 325 - 194 = **131 additional validators**

---

## Possible Explanations

### Hypothesis 1: Multi-Epoch Committee
The committee may include validators from multiple recent epochs, not just current registrations. This would create a "rolling window" of validators.

### Hypothesis 2: Historical Registrations
Validators who were registered in previous epochs but are no longer in current AriadneParameters might still be in the committee for stability.

### Hypothesis 3: Different Registration Types
There may be additional registration types beyond permissioned and candidateRegistrations that we're not querying.

### Hypothesis 4: Committee Construction Algorithm
The runtime may use a more complex algorithm that creates the committee from historical snapshots or multiple data sources.

---

## Stake Allocation Hypothesis

### If Stake-Weighted

Given the wide range (1-15 seats), stake weighting is likely:
- High-stake validators get 10-15 seats (1.0-1.25% of committee)
- Medium-stake validators get 3-7 seats (0.25-0.58%)
- Low-stake validators get 1-2 seats (0.08-0.17%)

This would explain the distribution pattern.

### Required Data

To test this, we need:
1. Stake amount for each AURA key in the committee
2. Map AURA keys → Sidechain keys → Cardano pool IDs
3. Cardano stake for each pool

---

## Implications for Block Prediction

### Current Approach (Wrong)

```rust
expected_blocks = epoch_slots / 185  // WRONG on multiple levels
```

### Correct Approach (Based on Findings)

```rust
// If validator has N seats in 1199-member committee:
expected_blocks = epoch_slots * (seats / 1199)

// Example:
// - Validator with 12 seats: 1800 * (12/1199) ≈ 18 blocks
// - Validator with 3 seats:  1800 * (3/1199) ≈ 4.5 blocks
// - Validator with 1 seat:   1800 * (1/1199) ≈ 1.5 blocks
```

### To Predict Future Performance

Need to determine:
1. How many seats will validator get next epoch?
2. Based on current stake? Past performance? Registration timing?

---

## Next Steps for Research

### Critical Questions

1. **Why 325 validators instead of 194?**
   - Investigate SessionCommitteeManagement storage
   - Check if there's a multi-epoch rolling window
   - Query historical AriadneParameters

2. **What determines seat allocation (1-15 range)?**
   - Get stake data for sample of validators
   - Correlate seat count with Cardano stake
   - Test if it's proportional or has other factors

3. **Can we predict seat allocation?**
   - Analyze correlation between stake and seats
   - Test if formula is deterministic
   - Check epoch-to-epoch stability

### Technical Investigation

```rust
// Queries needed:
1. SessionCommitteeManagement.Committee
2. SessionCommitteeManagement.NextCommittee
3. Historical AriadneParameters (epoch 1178, 1177, 1176)
4. Stake snapshots from Cardano for active validators
```

### Data Collection Plan

1. Export current committee with seat counts
2. For top 50 validators by seats:
   - Find their sidechain keys (if possible)
   - Get Cardano pool IDs
   - Query Cardano stake amounts
3. Calculate correlation: stake ↔ seats
4. Derive formula if pattern exists

---

## Impact on Our Tool

### Week 1 Fix (Still Valid)

The committee fix is still correct:
- Use `slot % 1199` (committee size)
- Fetch committee from `AuraApi_authorities`
- Store committee snapshots per epoch

### Week 2 Research (Now More Complex)

Additional questions:
- Why 325 validators?
- What's the seat allocation formula?
- Can we predict future allocations?

### Prediction Algorithm (Needs Revision)

Cannot use simple `1200/185` formula. Must:
1. Fetch actual committee for current epoch
2. Count validator's seats in committee
3. Calculate: `expected = epoch_slots * (seats/committee_size)`

For future prediction:
- Need to understand seat allocation mechanism
- May require querying `NextCommittee` if available
- Stake correlation analysis essential

---

## Cardano Correlation Data

### Our Test Pool

**Cardano Pool** (vducdn59):
- Pool ID: `pool1myvqymdmf9f26746d6uvfk34hqr7lq998n43xprgz4c27tpa6rd`
- Stake: ~1.258M ADA (0.12% of network)
- Epoch: 1179

**Midnight Validator** (vdumds58):
- NOT in committee (0 seats)
- False attribution: 23 blocks (due to bug)
- Status: Inactive or not registered in epoch 1179

**Cannot test correlation** with this validator since it's not active.

---

## Recommendations

### Immediate (Week 1)
1. ✅ Fix committee attribution bug (use 1199, not 185)
2. ✅ Store committee snapshots
3. ✅ Implement correct author calculation

### Research Phase (Week 2)
1. Investigate 325 vs 194 discrepancy
2. Analyze seat allocation vs stake correlation
3. Find a validator with known stake to test formula
4. Query historical AriadneParameters
5. Check SessionCommitteeManagement storage

### Future (Week 5-6)
1. Implement accurate prediction based on actual seat count
2. Add confidence intervals based on variance
3. Display seat allocation in TUI
4. Show validator's percentage of committee

---

## References

- **Node**: vdumds58 (testnet-02)
- **RPC Method**: `state_call("AuraApi_authorities", "0x")`
- **Epoch**: 1179
- **Analysis Date**: 2026-01-16

## Data Export

Committee analysis performed with Python script parsing SCALE-encoded AURA authorities.

---

**Status**: Major discrepancy discovered - 325 unique validators vs 194 candidates
**Impact**: Seat allocation formula more complex than assumed
**Next**: Deep dive into committee construction mechanism
