# Research Findings: Committee Composition Investigation

**Date**: 2026-01-16
**Status**: CRITICAL DISCOVERY - Implementation Blocker
**Priority**: URGENT - Requires Midnight dev team clarification

---

## Executive Summary

We have discovered a **fundamental disconnect** between validator registrations and the actual committee used for block production. **ZERO registered validators appear in the current committee**, making it impossible to correlate Cardano stake with Midnight block production without understanding the committee construction mechanism.

**This is likely a TESTNET-SPECIFIC scenario** and may not reflect how mainnet operates.

---

## Key Findings

### 1. Committee Composition

**Current Committee (Epoch 245638)**:
- **Total seats**: 1199
- **Unique validators**: 325 AURA keys
- **Seat distribution**: 1 to 15 seats per validator (highly variable)
- **Average seats**: 3.69 per validator

### 2. Registered Validators

**AriadneParameters (Mainchain Epoch 1179 / Sidechain Epoch 245638)**:
- **Permissioned candidates**: 12 (all valid)
- **Registered candidates**: 182 (ALL invalid - isValid=false)
- **Total valid candidates**: 12

**Node logs say**: "173 registered candidates" (discrepancy with RPC: 182)

### 3. The Critical Problem

**ZERO OVERLAP**: Not a single AURA key from the 12 valid permissioned candidates appears in the 325-member committee.

```
Permissioned candidates' AURA keys: 12
Committee AURA keys: 325
Overlap: 0  ❌
```

**This means**:
- Current committee is NOT using current registrations
- Committee is either:
  - From very old/historical registrations
  - A hardcoded genesis/test committee
  - Derived/transformed from registrations in an undocumented way
  - A bug in testnet-02 configuration

---

## Investigation Steps Taken

### 1. Verified Node Logs ✅
- Logs confirm: "Selected committee of 1200 seats from 12 permissioned and 173 registered candidates"
- Committee rotates every epoch (2-hour intervals)
- Committee stored with input data hash

### 2. Checked Historical Epochs ✅
- Queried AriadneParameters for epochs 1170-1179 (mainchain)
- Checked epochs 245632-245639 (sidechain)
- Consistent: 12 permissioned + ~182 registered
- None of the historical registered AURA keys match committee

### 3. Analyzed Validation Status ✅
- All 12 permissioned: `isValid=true`
- All 182 registered: `isValid=false`
- This explains why only 12 are "valid" but not why committee has 325

### 4. Tested Cardano Correlation (BLOCKED) ❌
- Cannot correlate Cardano stake/blocks with Midnight committee
- Our validator NOT registered in any recent epoch
- False block attributions in database due to `slot % 185` bug

---

## Data Evidence

### Node Logs (Recent)
```
2026-01-16 20:00:00 💼 Selected committee of 1200 seats for epoch 245639
                       from 12 permissioned and 173 registered candidates
2026-01-16 20:00:00 Committee rotated: Returning 1200 validators, stored in epoch 245638
2026-01-16 20:00:00 💼 Storing committee of size 1200 for epoch 245639,
                       input data hash: 0x6f761aae00d5a89f2da3472aa7de5be163a3085f1c4b5bee34e729ae28f032e1
```

### Sample Committee AURA Keys
```
0x975ad49ff6df094823cbae0e55ec9240ed7e6e7a213d3e80c32c7fd84265c69f  (1 seat)
0xe2b67aed7f263090ea14c2176266efc829115476e74102d16084df74f21aec31  (15 seats)
0xfa7ea1cdda876fe5d1bfea35a952a9c1baa0db1ad0c115c915631c36757ec69f  (1 seat)
... (322 more unique keys)
```

### Sample Permissioned Candidates' AURA Keys (ALL not in committee)
```
0x4ea3a9a15869deb8cb7878f2ba9ca57ccbabb8e3fc71e4cadc27cda14d399b5c  ✗
0x7cfe0d72ce7dc70a0915f4c25b98e5af5e65c3815a49be2f12bb2f3ba1a7328b  ✗
0xd8f8975ad49ff6df094823cbae0e55ec9240ed7e6e7a213d3e80c32c7fd84265  ✗
... (9 more, all not in committee)
```

---

## Hypotheses

### Hypothesis A: Genesis/Test Committee
The 325 validators in the committee are a **hardcoded test set** that's independent of registrations. This would explain:
- Why they don't match any registrations
- Why there are 325 (not related to candidate count)
- Why committee persists despite registration changes

**Likelihood**: HIGH (for testnet)

### Hypothesis B: Historical Registrations
The committee uses registrations from very old epochs (pre-epoch 1170) that are no longer in AriadneParameters due to state pruning.

**Likelihood**: MEDIUM

### Hypothesis C: Key Derivation
AURA keys in the committee are derived/transformed from registered keys through an undocumented algorithm.

**Likelihood**: LOW (we checked byte ordering, no patterns found)

### Hypothesis D: Configuration Bug
Testnet-02 has a misconfiguration where committee and registrations are out of sync.

**Likelihood**: MEDIUM

---

## Implications for Our Tool

### Can We Fix the Block Attribution Bug?
**YES** - We can still implement the correct fix:
```rust
author_index = slot % 1199  // Use actual committee size
```

This will give us **correct slot-to-position mapping** regardless of who the validators are.

### Can We Predict Block Production?
**PARTIALLY**:
- **For actual block attribution**: YES (use committee size)
- **For individual validators**: NO (without understanding committee construction)
- **For stake correlation**: NO (committee doesn't reflect current registrations)

### Can We Track "Our" Validator?
**YES, BUT**:
- We can detect if our AURA key is in the committee
- We can count our seats in the committee
- We can calculate expected blocks: `(our_seats / 1199) * epoch_slots`
- We CANNOT predict future committee membership

---

## Recommended Next Steps

### Immediate (Week 1) - Block Attribution Fix ✅
**Proceed with this regardless** of committee mystery:

1. Implement committee fetching from `AuraApi_authorities`
2. Fix author calculation: `slot % committee.len()`
3. Store committee snapshots per epoch
4. This gives us CORRECT block attribution

**Status**: Can implement now - not blocked

### Short-term (Week 2) - Gather More Data
**Before** implementing prediction algorithm:

1. **Contact Midnight dev team** with our findings
2. Ask for clarification on committee construction
3. Request documentation on:
   - How candidates → committee mapping works
   - Whether testnet uses hardcoded committee
   - If/when registrations affect committee
   - Stake weighting mechanism (if any)

### Medium-term (Post-clarification)
**After** understanding committee:

1. Implement stake-aware predictions (if applicable)
2. Add committee composition analysis
3. Track validator seat allocation over time
4. Correlate with Cardano data (if relevant)

---

## What We CANNOT Do Right Now

❌ **Correlate Cardano stake with Midnight blocks** - Committee doesn't use current registrations
❌ **Predict future committee membership** - Algorithm unknown
❌ **Calculate expected blocks from stake** - No stake correlation visible
❌ **Validate registration → committee flow** - Flow is broken/unknown

---

## What We CAN Do Right Now

✅ **Fix block attribution bug** - Use actual committee size
✅ **Detect our validator in committee** - Check AURA key presence
✅ **Count our seats** - Frequency analysis of committee
✅ **Calculate expected blocks** - From seat count, not stake
✅ **Track committee changes** - Store snapshots per epoch

---

## Questions for Midnight Team

1. **Is testnet-02 using a hardcoded/genesis committee?**
   - The 325 validators don't match any registrations

2. **When do validator registrations affect the committee?**
   - Current registrations have zero representation

3. **How is the committee constructed from candidates?**
   - 12 valid candidates → 1199-seat committee with 325 unique validators
   - What's the algorithm?

4. **Does Cardano stake affect committee composition?**
   - If so, how? We see no correlation in current data

5. **Is isValid=false for all 182 registered candidates expected?**
   - What makes a registration invalid?

6. **What is the "input data hash" in committee storage logs?**
   - Can this help us understand committee construction?

---

## Cardano Pool Data (For Reference)

**Pool ID**: `pool1myvqymdmf9f26746d6uvfk34hqr7lq998n43xprgz4c27tpa6rd`
**Stake**: ~1.258M ADA (0.12% of network)
**Mainchain Epoch**: 1179

**Recent Cardano Block Production**:
```
Epoch 1179: 1 block
Epoch 1178: 5 blocks
Epoch 1177: 7 blocks
Epoch 1176: 3 blocks
Epoch 1175: 6 blocks
```

**Midnight Validator**:
- Sidechain key: `0x037764d2d83c269030fef6df5aeb4419c48762ada2cf20b0e4e6ede596809f4700`
- AURA key: `0xe05be3c28c72864efc49f4f12cb04f3bd6f20fdbc297501aa71f8590273b3e1e`
- Status: NOT registered in any recent epoch
- NOT in committee

---

## Conclusion

We have hit a **fundamental knowledge gap** about how Midnight's committee system works. The current committee appears completely disconnected from validator registrations, suggesting either:
- A testnet-specific configuration
- Undocumented committee construction logic
- State from genesis/historical data we can't access

**We can proceed with the block attribution fix** (Week 1) which will give us correct slot→validator mapping using the actual committee.

**We cannot proceed with stake-based predictions** (Week 2) until we understand:
- How the committee is constructed
- Whether/how Cardano stake influences it
- Why current registrations don't appear in the committee

**Recommendation**: Implement Week 1 fix immediately, then seek clarification from Midnight development team before attempting Week 2 research.

---

**Status**: Investigation complete, blocker identified
**Next**: Contact Midnight team OR proceed with attribution fix only
**Decision**: User input required
