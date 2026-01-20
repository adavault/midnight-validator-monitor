# Release Plan: v1.0

**Target:** Mainnet-ready, production stable release

**Theme:** "Is my node healthy?" - Polish, stability, and bug fixes

---

## GitHub Issues to Fix

### Bug Fixes (High Priority)

#### Issue #4: Validator flag for 'Our validator' expires at end of epoch
**Status:** Unconfirmed
**Symptom:** Validator no longer shows as "ours" in dashboard after epoch change. Running `mvm keys verify` fixes it.
**Likely Cause:** The `is_ours` flag in the database may be getting reset when validator data is refreshed at epoch boundaries.
**Proposed Fix:**
- Investigate `fetch_db_data()` and `update()` to see if `is_ours` is being overwritten
- The flag should persist in DB and not be affected by epoch changes
- Add test case for epoch transition
**Effort:** Medium

#### Issue #6: Dashboard sparkline can show more blocks than seats
**Version:** v0.9.1
**Symptom:** Sparkline showing 4 blocks but only 3 seats over 48h period.
**Likely Cause:** Timing mismatch when feature was implemented - historical seat data may be incomplete or blocks attributed to wrong epochs.
**Proposed Fix:**
- Audit `get_block_counts_bucketed()` and `get_total_seats_for_epochs()` queries
- Ensure both use consistent epoch boundaries
- May need to cap display at seats count or show warning indicator
**Effort:** Medium

#### Issue #8: Validator showing more blocks than seats in epoch
**Symptom:** Validator showing 200% (2 blocks / 1 seat) in Epoch 245670.
**Likely Cause:** Either block attribution error or seat counting issue for specific epoch.
**Proposed Fix:**
- Cross-reference with on-chain data for that epoch
- Check if committee snapshot was correctly captured
- May be related to #6 (same root cause)
**Effort:** Medium (investigate with #6)

### UI Polish (Low Priority)

#### Issue #7: Stake shown as tDUST but should be tADA
**Symptom:** Validator identity popup shows stake as "tDUST" but mainchain stake is in tADA.
**Proposed Fix:**
- Change label from "tDUST" to "tADA" in `open_validator_identity_popup()`
- Update formatting to use ADA decimal places (6)
**Effort:** Small (5 mins)

#### Issue #9: Validator popup text justification
**Symptom:** Text not correctly justified in validator popup.
**Proposed Fix:**
- Adjust padding/spacing in `render_validator_identity_popup()`
- Move text left one space
**Effort:** Small (5 mins)

#### Issue #10: Block popup text justification
**Symptom:** Text not correctly justified in block popup.
**Proposed Fix:**
- Adjust padding/spacing in `render_block_detail_popup()`
- Move text back one space
**Effort:** Small (5 mins)

### New Features (Medium Priority)

#### Committee Selection Statistics in Popups
**Source:** Troubleshooting session 2026-01-20 - analyzing "GRANDPA Not voting" status
**Context:** When a validator shows "Not voting" for GRANDPA, it's often because they weren't selected for the current epoch's committee. Users need visibility into their historical selection patterns to understand this is normal behavior.

**Feature:** Add committee selection statistics to validator identity popup and/or performance view popup.

**Statistics to Display:**
1. **Committee Membership Summary**
   - Epochs tracked (from database)
   - Times selected for committee
   - Total seats received
   - Selection rate (e.g., "Selected 3 of 22 epochs")

2. **Selection Rate Analysis**
   - Expected seats/epoch (based on stake proportion)
   - Actual seats/epoch (from historical data)
   - Performance vs expected (e.g., "10x better than stake proportion")
   - Average epochs between selections

3. **Stake Context**
   - Stake rank among dynamic validators (e.g., "46th of 172")
   - Share of dynamic validator stake pool (e.g., "0.30%")
   - Committee structure note (e.g., "~91% permissioned, ~9% dynamic")

4. **Current Status**
   - Last selected epoch
   - Epochs since last selection
   - Current committee status (In Committee / Not Selected)

**SQL Queries Required:**
```sql
-- Selection history for validator
SELECT sidechain_epoch, committee_seats, committee_size
FROM validator_epochs
WHERE sidechain_key = ?
ORDER BY sidechain_epoch;

-- Stake rank calculation
SELECT COUNT(*) + 1 as rank
FROM validator_epochs
WHERE sidechain_epoch = ?
  AND stake_lovelace > (SELECT stake_lovelace FROM validator_epochs WHERE sidechain_key = ? AND sidechain_epoch = ?)
  AND stake_lovelace IS NOT NULL;

-- Dynamic vs permissioned breakdown
SELECT is_permissioned, COUNT(DISTINCT sidechain_key), SUM(committee_seats)
FROM validator_epochs
WHERE sidechain_epoch = ?
GROUP BY is_permissioned;
```

**UI Location Options:**
- Option A: Add new tab/section to validator identity popup (preferred)
- Option B: Add to Performance view as expandable detail
- Option C: New "Selection History" popup accessible from validator row

**Effort:** Medium-Large (new queries + UI rendering)

**Value:** Helps operators understand that gaps in committee selection are normal, especially for lower-stake validators. Reduces confusion when GRANDPA shows "Not voting".

---

## v1.0 Roadmap Goals

From `docs/ROADMAP.md`, v1.0 should deliver:

### Documentation Polish
- [ ] Update ROADMAP.md current status to v0.9.1
- [ ] Review and update all command help text
- [ ] Ensure README reflects all current features
- [ ] Add troubleshooting section to README
- [ ] Review help screen for completeness

### Mainnet Readiness
- [ ] Test with mainnet timing parameters (5-day epochs, 10h sidechain epochs)
- [ ] Verify `ChainTiming` auto-detection works correctly
- [ ] Ensure graceful handling of network parameter changes
- [ ] Test install/upgrade path on fresh system

### Security Hardening
- [ ] Review `docs/SECURITY_AUDIT_2026-01-19.md` findings
- [ ] Add `strip = true` and `panic = "abort"` to Cargo.toml release profile
- [ ] Add `PRAGMA foreign_keys=ON` to database initialization

### Stability
- [ ] Fix all open bugs (Issues #4, #6, #7, #8, #9, #10)
- [ ] Add error handling for edge cases discovered in testing
- [ ] Ensure clean shutdown under all conditions
- [ ] Memory usage audit for long-running sync daemon

---

## Implementation Order

### Phase 1: Quick Fixes (30 mins)
1. Issue #7 - tDUST -> tADA label fix
2. Issue #9 - Validator popup justification
3. Issue #10 - Block popup justification
4. Security hardening - Cargo.toml + database pragma (from security audit)

### Phase 2: Bug Investigation (2-3 hours)
1. Issue #4 - Investigate `is_ours` flag persistence
2. Issue #6 & #8 - Investigate blocks/seats mismatch (likely same root cause)

### Phase 3: Bug Fixes (based on investigation)
- Implement fixes for Phase 2 findings

### Phase 4: New Feature - Committee Selection Statistics
1. Add database query functions for selection history and stake ranking
2. Design popup UI layout for statistics display
3. Implement rendering in validator identity popup
4. Test with various validator stake levels

### Phase 5: Documentation & Polish
1. Update ROADMAP.md status
2. Add troubleshooting section to README
3. Review all help text

### Phase 6: Testing & Release
1. Full manual test of all features
2. Test mainnet timing parameters
3. Fresh install test
4. Release v1.0

---

## Files Likely to Change

| File | Changes |
|------|---------|
| `src/tui/app.rs` | Issue #7 (tADA label), Issue #4 (is_ours persistence), Committee stats data fetching |
| `src/tui/ui.rs` | Issues #9, #10 (popup justification), Committee stats popup rendering |
| `src/db/blocks.rs` | Issues #6, #8 (blocks/seats queries) |
| `src/db/validators.rs` | New queries for selection history, stake ranking, committee breakdown |
| `src/db/mod.rs` | Security: Add `PRAGMA foreign_keys=ON` |
| `Cargo.toml` | Security: Add `strip` and `panic` to release profile |
| `docs/ROADMAP.md` | Update current status |
| `README.md` | Add troubleshooting section |

---

## Success Criteria

v1.0 is ready when:
- [ ] All 6 open GitHub issues are closed
- [ ] No known bugs
- [ ] Security audit recommendations implemented (see `docs/SECURITY_AUDIT_2026-01-19.md`)
- [ ] Committee selection statistics feature implemented and tested
- [ ] Documentation is current and complete
- [ ] Works correctly with mainnet timing parameters
- [ ] Clean install process verified on fresh system
- [ ] Stable operation for 24h+ without intervention

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Issue #4 may be complex to reproduce | Add logging to track `is_ours` changes |
| Issues #6/#8 may indicate deeper attribution bug | Time-box investigation, document findings even if not fully fixed |
| Mainnet parameters unknown | Design for configurability, test with projected values |

---

*Created: January 2026*
*Updated: 2026-01-20 (added committee selection statistics feature)*
*Target Release: Before Midnight mainnet launch*
