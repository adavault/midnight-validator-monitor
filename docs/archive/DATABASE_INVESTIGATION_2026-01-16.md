# Database Investigation: Cardano db-sync Analysis

**Date**: 2026-01-16
**Database**: `cexplorer` on vdumds58.skynet (cardano-db-sync 13.6.0.4)
**Network**: Preview testnet
**Goal**: Find Midnight committee construction data

---

## Executive Summary

**Result**: NO Midnight-specific data found in Cardano db-sync database.

The db-sync database contains standard Cardano blockchain data but **NO information about**:
- Midnight validator registrations
- Midnight committee composition
- Midnight stake allocation
- Partner chain registration mappings

**Conclusion**: Midnight committee construction logic is entirely within the **Midnight node runtime**, not stored on Cardano chain.

---

## Database Schema Analysis

### Tables Examined

**Cardano Governance (Conway Era)**:
- `committee` - 41 Cardano governance committees
- `committee_member` - 246 Cardano governance committee members
- `committee_registration` / `committee_de_registration` - Governance registrations
- **NOT related to Midnight validator committee**

**Pool & Stake Tables**:
- `pool_hash` - Stake pool identifiers
- `pool_update` - Pool registration/update certificates
- `epoch_stake` - Stake distribution per epoch
- `tx_metadata` - Transaction metadata (various keys 0-378+)

**No Midnight/Partner Chain Tables Found**:
- No `partner_chain_*` tables
- No `midnight_*` tables
- No `sidechain_*` tables
- No extended schema for partner chains

---

## Our Pool's Cardano Stake Data

### Pool Information
- **Pool ID**: `pool1myvqymdmf9f26746d6uvfk34hqr7lq998n43xprgz4c27tpa6rd`
- **Pool ID (hex)**: `d918026dbb4952ad7aba6eb8c4da35b807ef80a53ceb1304681570af`

### Stake by Epoch (Recent)

| Epoch | Pool Stake (lovelace) | Network Total Stake | Stake % |
|-------|----------------------|---------------------|---------|
| 1180  | 1,258,876,254,658    | 1,056,955,452,403,806 | 0.119% |
| 1179  | 1,258,136,022,762    | 1,066,271,171,230,649 | 0.118% |
| 1178  | 1,257,819,155,887    | 1,065,821,281,504,980 | 0.118% |
| 1177  | 1,257,185,985,525    | 1,065,424,237,633,663 | 0.118% |
| 1176  | 1,256,765,190,290    | 1,063,615,796,248,187 | 0.118% |
| 1175  | 1,256,240,315,264    | 1,062,112,081,360,964 | 0.118% |

**Consistent ~0.118% stake** (~1.258 billion ADA)

### Expected vs Actual Midnight Committee Seats

**If stake-proportional**:
- Total committee: 1199 seats
- Our stake: 0.118%
- Expected seats: 1199 × 0.00118 = **1.41 seats** (~1-2 seats)

**Actual**:
- Seats in committee: **0** (not in committee at all)
- Validator not registered in Midnight (recent epochs)

**Discrepancy**: We have significant Cardano stake but NO Midnight presence.

---

## Transaction Metadata Investigation

### Metadata Keys Found
Examined all metadata keys in database (0 to 378+):
- Key 0-30: Common metadata (various uses)
- Key 100-140: Various protocols
- Key 378: 1313 entries with hex data (format: `{"0": "0x..."}`)
- No obvious Midnight/partner chain identifiers

### Our Pool Registration Transactions

Checked transaction metadata for our pool's registration updates:
- Transaction IDs: 5762358, 5073403, 5069594
- **Result**: NO metadata attached to these transactions
- **Conclusion**: Standard pool registrations, no Midnight extensions

### Midnight Validator Key Search

Searched for our Midnight sidechain key in all metadata:
- Sidechain key: `0x037764d2d83c269030fef6df5aeb4419c48762ada2cf20b0e4e6ede596809f4700`
- **Result**: NOT FOUND in any transaction metadata
- **Conclusion**: Midnight registrations not stored in Cardano tx metadata

---

## Findings & Conclusions

### 1. No Midnight Data in db-sync
The cardano-db-sync database is a standard Cardano blockchain database with NO Midnight-specific extensions or tables.

### 2. Committee Construction is Runtime Logic
Midnight's committee selection happens in the **Midnight node runtime**, not on-chain in Cardano. The node:
- Reads Ariadne parameters (registrations) from somewhere
- Applies committee construction algorithm
- Stores result in Midnight chain state
- This logic is **not visible in Cardano data**

### 3. Stake Correlation Cannot Be Determined
Without access to:
- Committee construction algorithm
- Midnight validator registration mappings
- Historical committee data

We **cannot correlate** Cardano stake percentage with Midnight committee seats.

### 4. Our Validator Status
- **Cardano**: Active pool with 0.118% stake, producing blocks regularly
- **Midnight**: NOT registered (or registration is invalid/expired)
- **Committee**: NOT present (0 seats)
- **Blocks**: False attributions in our database (due to `slot % 185` bug)

---

## Where Committee Data Likely Lives

### Option A: Midnight Node Runtime State
- Committee stored in Substrate runtime storage
- Accessible via: `AuraApi_authorities` (current committee)
- Accessible via: `SessionCommitteeManagement` pallet storage (maybe)
- **NOT accessible via Cardano RPC or db-sync**

### Option B: Ariadne Smart Contract
- Registrations might be in Cardano smart contract
- Contract state not directly visible in db-sync
- Would need to query contract state on Cardano node
- Committee construction might happen in contract logic

### Option C: Off-Chain Oracle/Service
- Midnight node might query external service
- Service combines Cardano + Midnight data
- Committee constructed off-chain, then stored on Midnight
- Would explain disconnect between registrations and committee

---

## Next Steps

### What We CAN Do (Immediate)
✅ **Implement Week 1 Fix**:
- Use actual committee size (1199) from `AuraApi_authorities`
- Fix block attribution: `slot % committee.len()`
- Store committee snapshots per epoch
- **This works regardless of committee construction mystery**

### What We CANNOT Do (Blocked)
❌ Predict committee membership from Cardano stake
❌ Calculate expected blocks from stake percentage
❌ Validate registration → committee flow
❌ Research stake-weighted allocation mechanism

### Recommendation
**Proceed with Week 1 block attribution fix** - it solves the immediate bug and doesn't require understanding committee construction.

**Defer Week 2 research** - requires either:
1. Midnight dev team documentation
2. Source code access to see committee construction
3. Finding an active, registered validator to study

---

## Data Quality Assessment

**Cardano Data**: ✅ Complete and accurate
- Pool registrations tracked
- Stake per epoch available
- Block production logged (in CNCLI, not db-sync)

**Midnight Data**: ❌ Not in db-sync
- No validator registrations
- No committee composition
- No stake allocation rules

**Integration**: ❌ Not visible
- No bridge between Cardano pools and Midnight validators
- Mapping happens in Midnight node, not on-chain
- Cannot be discovered from Cardano data alone

---

## Useful Queries

### Get Pool Stake History
```sql
SELECT
  es.epoch_no,
  SUM(es.amount) as pool_stake,
  (SELECT SUM(amount) FROM epoch_stake WHERE epoch_no = es.epoch_no) as total_stake,
  ROUND((SUM(es.amount)::numeric /
         (SELECT SUM(amount) FROM epoch_stake WHERE epoch_no = es.epoch_no)::numeric) * 100, 4)
    as stake_percent
FROM epoch_stake es
JOIN pool_hash ph ON ph.id = es.pool_id
WHERE ph.view = 'pool1myvqymdmf9f26746d6uvfk34hqr7lq998n43xprgz4c27tpa6rd'
  AND es.epoch_no >= 1170
GROUP BY es.epoch_no
ORDER BY es.epoch_no DESC;
```

### Find Pool Registration Transactions
```sql
SELECT
  ph.view,
  pu.registered_tx_id,
  t.hash,
  t.block_id,
  b.epoch_no
FROM pool_update pu
JOIN pool_hash ph ON ph.id = pu.hash_id
JOIN tx t ON t.id = pu.registered_tx_id
JOIN block b ON b.id = t.block_id
WHERE ph.view = 'pool1myvqymdmf9f26746d6uvfk34hqr7lq998n43xprgz4c27tpa6rd'
ORDER BY pu.registered_tx_id DESC;
```

---

## Conclusion

The db-sync database provides valuable **Cardano stake data** but contains **NO Midnight committee information**. Committee construction is a Midnight runtime operation that:

1. Reads registrations (from somewhere - not clear where)
2. Applies unknown algorithm
3. Produces 1199-seat committee with 325 unique validators
4. Stores in Midnight chain state only

**We have exhausted database investigation** - no more insights available from db-sync.

**Recommendation**: Proceed with block attribution fix, defer stake correlation research.

---

**Status**: Database investigation complete - no Midnight data found
**Next**: Implement Week 1 fix (not blocked by this)
**Blocked**: Week 2 research (requires Midnight team input or source code)
