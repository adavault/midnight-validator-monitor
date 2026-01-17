# Midnight-blocklog Repository Analysis

**Repository**: https://github.com/btbf/Midnight-blocklog
**Author**: btbf
**License**: Apache 2.0
**Language**: Rust
**Last Updated**: 2026-01-10
**Status**: Beta (v0.3.2)

---

## Executive Summary

The Midnight-blocklog tool provides the **correct implementation** for block attribution that we need. They use the actual AURA authority set (the 1200-seat committee) fetched dynamically from chain state, not the 185 candidates.

**Key Finding**: They use `slot % authority_set.len()` where `authority_set` is fetched from `Aura.Authorities` storage - this gives them the ~1200 committee members.

---

## Technical Approach

### 1. Correct Block Attribution Algorithm

**Their Implementation**:
```rust
fn compute_my_slots(
    auths: &[sr25519::Public],      // The actual authority set (~1200 members)
    author_bytes: &[u8; 32],        // This validator's AURA key
    start_slot: u64,
    slots_to_scan: u64,
) -> Vec<u64> {
    let mut out = Vec::new();
    if auths.is_empty() {
        return out;
    }
    for i in 0..slots_to_scan {
        let slot = start_slot + i;
        let who = &auths[(slot as usize) % auths.len()];  // ✅ CORRECT
        if who.0 == *author_bytes {
            out.push(slot);
        }
    }
    out
}
```

**This is exactly what we need to implement**: `slot % auths.len()` where `auths` is the committee.

### 2. Fetching the Authority Set

**Their Method**:
```rust
fn fetch_authorities(
    api: &Api<DefaultRuntimeConfig, TungsteniteRpcClient>,
) -> anyhow::Result<Vec<sr25519::Public>> {
    let res: Option<Vec<sr25519::Public>> = api
        .get_storage("Aura", "Authorities", None)
        .map_err(|e| anyhow!("{e:?}"))?;
    Ok(res.unwrap_or_default())
}
```

**Key Points**:
- They fetch from `Aura.Authorities` storage (not `AriadneParameters`)
- This returns the actual committee (1200 members), not candidates (185)
- Uses `substrate-api-client` library for storage queries
- Returns `Vec<sr25519::Public>` - the AURA public keys

**Our Equivalent**: We should use `state_call("AuraApi_authorities", "0x")` which returns the same data via runtime call instead of storage query.

### 3. Committee vs Candidates Understanding

They **correctly distinguish** between:
- **Authority Set** (`Aura.Authorities`): The 1200-seat committee - used for block attribution
- **Candidates** (from Ariadne): The 185 validators - NOT used for slot calculations

This confirms our discovery in `VALIDATOR_COMMITTEE_DISCOVERY.md` is accurate.

### 4. Epoch Handling

**Epoch Detection**:
```rust
let epoch_idx = latest_slot / epoch_size;
let start_slot = epoch_idx * epoch_size;
let epoch_switched = prev_epoch.is_none() || prev_epoch.unwrap() != epoch_idx;

if epoch_switched {
    // Refresh authority set
    // Recalculate assigned slots
}
```

**Key Points**:
- Epoch size defaults to 1200 slots (matches committee size)
- On epoch transitions, they refetch the authority set
- Authority set can change each epoch (committee rotation)

### 5. Next Epoch Committee (Advanced Feature)

They implement **predictive scheduling** for the next epoch:

```rust
fn fetch_committee_info(
    api: &Api<DefaultRuntimeConfig, TungsteniteRpcClient>,
    pallet_name: &str,
    storage_name: &str,
) -> anyhow::Result<Option<(u64, Vec<(sr25519::Public, ed25519::Public)>)>> {
    // Fetches from SessionCommitteeManagement.NextCommittee storage
    // Returns explicit committee assignments for next epoch
}
```

**How it works**:
- For **current epoch**: Uses modulo calculation with current authorities
- For **next epoch**: Tries to fetch `NextCommittee` storage
- If `NextCommittee` available: Uses explicit seat assignments
- If unavailable: Falls back to modulo with projected authorities

**This answers one of our research questions**: There IS a way to predict next epoch's committee!

### 6. Stake Integration

**Informational Only**:
```rust
fn fetch_registration_status(...) -> anyhow::Result<(u128, bool)> {
    // Fetches stake from Ariadne testnet
    let stake = entry.get("stakeDelegation")
        .and_then(parse_lovelace)
        .unwrap_or(0);

    // Display: "ADA Stake: 2816841.654532 ADA"
}
```

**Key Finding**: Stake is **displayed but NOT used** in slot assignment calculations.

**Implication**: Either:
1. Stake affects committee composition (185 → 1200 mapping) upstream
2. Stake weighting happens in the runtime when building the committee
3. Equal distribution is used (each of 185 candidates gets ~6.5 seats)

We still need to research **how** the 1200 seats are allocated among 185 candidates.

---

## Data Storage

### SQLite Schema

**epoch_info table**:
```rust
CREATE TABLE IF NOT EXISTS epoch_info (
    epoch INTEGER PRIMARY KEY,
    start_slot INTEGER NOT NULL,
    end_slot INTEGER NOT NULL,
    authority_set_hash TEXT NOT NULL,
    authority_set_len INTEGER NOT NULL,
    created_at_utc TEXT NOT NULL
)
```

**blocks table**:
```rust
CREATE TABLE IF NOT EXISTS blocks (
    slot INTEGER PRIMARY KEY,
    epoch INTEGER NOT NULL,
    planned_time_utc TEXT NOT NULL,
    block_number INTEGER,
    block_hash TEXT,
    produced_time_utc TEXT,
    status TEXT NOT NULL  -- 'schedule' | 'mint' | 'finality'
)
```

**Key Design**:
- Stores authority set metadata per epoch
- Tracks block status transitions: scheduled → minted → finalized
- Uses slot as primary key (unique per network)

---

## Dependencies

**Key Libraries** (from `Cargo.toml`):
```toml
substrate-api-client = "1.20.0"    # Substrate RPC interaction
sp-core = "39.0.0"                  # Cryptographic primitives
sp-runtime = "44.0.0"               # Runtime types
scale-value = "0.18.1"              # SCALE codec
rusqlite = "0.32.1"                 # SQLite database
reqwest = "0.12.12"                 # HTTP client (for Ariadne)
tokio = "1.49.0"                    # Async runtime
chrono = "0.4.42"                   # Timestamps
clap = "4.5.54"                     # CLI parsing
```

**Notable**: They use `substrate-api-client` for RPC, we use our custom `RpcClient`. Both approaches are valid.

---

## Comparison with Our Implementation

### What They Do Right

✅ **Correct committee size**: Uses actual authority set (~1200), not candidates (185)
✅ **Correct attribution formula**: `slot % authority_set.len()`
✅ **Dynamic fetching**: Queries chain state, doesn't hardcode values
✅ **Epoch awareness**: Refreshes authority set on epoch changes
✅ **Future prediction**: Attempts to fetch next epoch's committee

### What We Need to Fix

❌ **Our bug**: We use `slot % 185` (candidates) instead of `slot % 1200` (committee)
❌ **Static approach**: We don't refetch authority set per epoch
❌ **No committee storage**: We don't cache authority sets

### What We Do Better

✅ **Comprehensive monitoring**: Full TUI, health checks, analytics
✅ **Configuration system**: TOML config, environment variables
✅ **Systemd integration**: Daemon mode, service files
✅ **Validator tracking**: Mark and filter "ours", performance rankings
✅ **Documentation**: Extensive docs and specifications

---

## Key Insights for Our Research

### 1. Authority Set is the Source of Truth

The `Aura.Authorities` storage contains the actual 1200-seat committee. This is what we should use, not `AriadneParameters`.

**Action**: Fetch via `state_call("AuraApi_authorities", "0x")` (our approach) or `get_storage("Aura", "Authorities")` (their approach).

### 2. Stake is NOT Used in Slot Calculation

Their code shows stake is informational only. This suggests:
- Stake weighting happens when **building** the committee (185 → 1200)
- The committee itself is then used for **equal round-robin** assignment
- We need to research the committee construction algorithm

### 3. Epoch Lag Question Remains Open

They don't show how Cardano stake from epoch N-1 or N-2 affects Midnight committee at epoch N. This is still a research question.

### 4. NextCommittee Storage Exists

The `SessionCommitteeManagement.NextCommittee` storage provides predictive scheduling. We should investigate if this is available and reliable.

---

## Implementation Recommendations

### Immediate (Week 1 - Critical Fix)

1. **Update ValidatorSet fetching**:
   ```rust
   // Replace AriadneParameters with AuraApi_authorities
   let committee = fetch_aura_authorities(rpc).await?;  // Returns ~1200 keys
   ```

2. **Fix author calculation**:
   ```rust
   // Change from:
   let author_index = slot % self.validators.len();  // Wrong: 185

   // To:
   let committee_index = slot % self.committee.len();  // Correct: 1200
   let aura_key = &self.committee[committee_index];
   ```

3. **Store committee snapshots**:
   ```sql
   CREATE TABLE committee_snapshots (
       epoch INTEGER,
       position INTEGER,
       aura_key TEXT,
       PRIMARY KEY (epoch, position)
   );
   ```

### Research Phase (Week 2)

1. **Investigate committee construction**:
   - Compare authority set composition across epochs
   - Count how many times each candidate appears
   - Correlate with Cardano stake data

2. **Test NextCommittee storage**:
   ```rust
   let next_committee = rpc.get_storage(
       "SessionCommitteeManagement",
       "NextCommittee",
       None
   ).await?;
   ```

3. **Analyze epoch lag**:
   - Compare Midnight epoch N committee with Cardano epoch N, N-1, N-2 stakes
   - Find correlation

---

## Open Questions Remaining

1. **Committee Construction**: How are the 1200 seats allocated among 185 candidates?
   - Equal distribution (6-7 seats each)?
   - Stake-weighted?
   - Block production weighted?

2. **Epoch Lag**: What is the exact relationship between Cardano epoch and Midnight committee?

3. **NextCommittee Reliability**: Can we depend on `NextCommittee` storage for predictions?

4. **Stake Role**: If not used in slot calculation, how does stake affect the system?

---

## References

- **Repository**: https://github.com/btbf/Midnight-blocklog
- **Key File**: `src/bin/mblog.rs` (68 KB)
- **Dependencies**: Uses `substrate-api-client` 1.20.0
- **Our Discovery**: `VALIDATOR_COMMITTEE_DISCOVERY.md`
- **Research Plan**: `STAKE_ALLOCATION_RESEARCH.md`

---

## Actionable Takeaways

### For v0.4-beta Week 1 (Critical Fix)

1. ✅ **Confirmed**: We must use the authority set (1200 members), not candidates (185)
2. ✅ **Method**: Fetch via `AuraApi_authorities` runtime call (our approach is correct)
3. ✅ **Formula**: `slot % authority_set.len()` (validated by working implementation)
4. ✅ **Caching**: Store authority sets per epoch for historical accuracy

### For v0.4-beta Week 2 (Research)

1. ❓ **Investigate**: How 185 candidates map to 1200 seats
2. ❓ **Test**: Can we reliably fetch `NextCommittee` for predictions?
3. ❓ **Analyze**: Correlation between Cardano stake and committee composition
4. ❓ **Document**: Mathematical formula for seat allocation

### For v0.4-beta Week 5-6 (Prediction Algorithm)

1. Use authority set size (1200) for baseline predictions
2. If research reveals stake weighting, incorporate it
3. Use `NextCommittee` if available and reliable
4. Provide confidence intervals based on historical variance

---

**Status**: Analysis complete. Their implementation validates our discovery and provides a working reference for our critical fix.

**Next Step**: Implement committee-based attribution in Week 1, then research committee construction in Week 2.
