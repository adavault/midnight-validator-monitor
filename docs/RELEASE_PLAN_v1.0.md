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

### Phase 4: Documentation & Polish
1. Update ROADMAP.md status
2. Add troubleshooting section to README
3. Review all help text

### Phase 5: Testing & Release
1. Full manual test of all features
2. Test mainnet timing parameters
3. Fresh install test
4. Release v1.0

---

## Files Likely to Change

| File | Changes |
|------|---------|
| `src/tui/app.rs` | Issue #7 (tADA label), Issue #4 (is_ours persistence) |
| `src/tui/ui.rs` | Issues #9, #10 (popup justification) |
| `src/db/blocks.rs` | Issues #6, #8 (blocks/seats queries) |
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
*Target Release: Before Midnight mainnet launch*
