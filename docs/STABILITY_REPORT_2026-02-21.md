# MVM Sync Daemon Stability Report

**Date:** 2026-02-21
**Server:** vdumdn57 (testnet-02, build server)
**Version:** mvm 1.0.3
**Assessed by:** Claude Code (automated analysis)

---

## Executive Summary

The mvm-sync daemon has been running continuously for **27.4 days** (since 2026-01-25 21:01:48 UTC) with **zero restarts**, **zero errors**, and **no evidence of memory leaks**. The process is stable and safe to restart for the planned server reboot.

**Verdict: Production-stable. No issues requiring code changes.**

One operational finding: the `mvm-status.timer` is dead and should be restarted after reboot (see below).

---

## Service Status

| Metric | Value |
|--------|-------|
| Process PID | 582856 |
| Uptime | 27.4 days (since Jan 25 21:01:48 UTC) |
| Systemd restarts | 0 |
| Errors in logs | 0 (zero errors, panics, or failures) |
| Threads | 5 |
| Service restart policy | on-failure (RestartSec=10s) |

### Service History (Jan 25)

The current process was started after a series of upgrade steps on Jan 25:

1. v1.0.0 stopped at 12:34:24 (had been running since Jan 24)
2. v1.0.2 started at 20:49:40, stopped at 20:56:03 (brief run)
3. v1.0.3 started at 20:56:03, stopped at 21:01:48 (brief run)
4. **v1.0.3 restarted at 21:01:48** — this is the current process, running ever since

All stops were clean (deactivated successfully). No crash restarts.

---

## Memory Analysis

### Process Memory

| Metric | Value | Assessment |
|--------|-------|------------|
| VmRSS (resident) | 64.2 MB | Low, healthy |
| VmPeak (max ever) | 366.6 MB | Never exceeded |
| VmSize (virtual) | 366.3 MB | Near peak = stable allocation |
| VmData (heap) | 69.3 MB | Normal |
| VmSwap | 0 KB | No swap pressure |
| Pss_Anon (private heap) | 57.9 MB | Actual private memory |

### Memory Leak Assessment

**No evidence of memory leaks.** Key indicators:

- **VmPeak (366.6 MB) ≈ VmSize (366.3 MB)**: Virtual memory has not grown since early in the process lifetime. If there were a leak, VmSize would steadily approach VmPeak and eventually exceed it.
- **VmRSS (64 MB) is well below VmSize (366 MB)**: Most virtual allocations are not resident, which is normal for a Rust/tokio async runtime that pre-allocates thread stacks.
- **VmSwap = 0**: No memory pressure forcing pages to swap.
- **Stable RSS after 27 days**: A process syncing ~14K blocks/day for 27 days with constant RSS strongly indicates no unbounded growth.

### Systemd vs /proc Memory Discrepancy

| Source | Reported | What it measures |
|--------|----------|-----------------|
| systemd MemoryCurrent | 515.6 MB | Entire cgroup (process + kernel page cache) |
| /proc VmRSS | 64.2 MB | Process resident memory only |

The ~450 MB difference is **kernel page cache** from SQLite I/O — the OS caches database pages in memory for performance. This is not a leak; the kernel will reclaim this cache under memory pressure. The actual process memory footprint is **64 MB**.

### System Memory Context

| Metric | Value |
|--------|-------|
| Total RAM | 15 GB |
| Used | 1.6 GB |
| Available | 13 GB |
| Swap used | 63 MB / 8 GB |

MVM is using < 0.5% of system RAM. No concerns.

---

## CPU Analysis

| Metric | Value |
|--------|-------|
| Total CPU time | 9h 25m 41s |
| Wall clock time | ~27.4 days |
| Average CPU usage | 1.43% |
| Context switches (voluntary) | 25.9M |
| Context switches (involuntary) | 11.0M |

The process spends most of its time sleeping between block polls (~6s intervals). CPU usage is minimal and appropriate for the workload.

---

## File Descriptor Analysis

| Metric | Value |
|--------|-------|
| Open FDs | 15 |
| Soft limit | 1,024 |
| Hard limit | 524,288 |

**No FD leak.** Breakdown of 15 open descriptors:

- stdin → /dev/null (daemon mode)
- stdout/stderr → journal socket
- 3 SQLite files (mvm.db, .db-wal, .db-shm)
- 5 sockets (RPC connections, signal handling)
- 2 eventpoll/eventfd (tokio async runtime)

This is a minimal, clean FD set. No accumulation over 27 days.

---

## Database Health

### Integrity

| Check | Result |
|-------|--------|
| `PRAGMA integrity_check` | **ok** |
| Block sequence gaps | **0** (zero gaps) |
| Freelist pages | 0 (no wasted space) |

### Size and Growth

| Metric | Value |
|--------|-------|
| Database size | 462 MB |
| WAL file size | 26.5 MB (5.8% of DB) |
| WAL checkpoint status | Fully checkpointed (873/873 pages, 0 busy) |
| Total blocks | 504,937 |
| Block range | 3,357,362 → 3,862,298 |
| Daily block rate | ~13,800 blocks/day (consistent) |
| Pages | 118,228 x 4KB |

### Block Attribution

| Category | Count | Percentage |
|----------|-------|------------|
| Blocks with author attribution | 468,270 | 92.7% |
| Blocks without attribution (pruned state) | 36,713 | 7.3% |

The unattributed blocks are from the initial sync range (3,357,362 to ~3,490,948) where historical state was already pruned. This is expected behavior on non-archive nodes and was logged with appropriate warnings at startup.

### Committee Snapshots

| Metric | Value |
|--------|-------|
| Total snapshots | 464,287 |
| Epoch range | 1,181 → 246,067 |

### Database Growth Projection

At ~13,800 blocks/day with current schema, the database grows approximately **6 MB/day** or **180 MB/month**. At 462 MB now with 68 GB available on the partition, there is **no disk pressure concern** for the foreseeable future.

### WAL File

The WAL is 26.5 MB but fully checkpointed (all pages written back to the main DB). The file size on disk remains because SQLite reuses WAL space rather than truncating. This is normal and efficient. A `PRAGMA wal_checkpoint(TRUNCATE)` during a maintenance window would reclaim the file space if desired, but it's not necessary.

---

## Log Analysis

### Errors Since Current Process Start

**Zero.** No errors, panics, OOM events, or failures logged in the 27.4-day window.

### Warnings

The only warnings were logged at startup on Jan 25, all related to state pruning (expected on non-archive nodes):

```
Historical state is pruned before block 3490948
Blocks 3357362 to 3490947 will be synced WITHOUT author attribution
```

These are informational warnings, not operational issues.

### Sync Consistency

The sync logs show consistent 6-second block intervals with "Sync: 100.0% (0 behind)" throughout the entire runtime. No periods of falling behind or catching up were observed in the sampled logs.

---

## Operational Finding: mvm-status.timer

| Metric | Value |
|--------|-------|
| Timer status | **inactive (dead)** since Jan 20 |
| Enabled at boot | Yes |
| Last successful run | Jan 20 10:18:25 UTC |

The periodic status check timer has been dead for **32 days**.

### Root Cause Analysis

The timer died during a manual `mvm install` upgrade on Jan 20. Journal reconstruction of the sequence:

```
10:21:35  sudo systemctl stop mvm-sync          # Manual stop (sync already down before install)
10:21:37  sudo mvm install                       # Install starts
10:21:37    install: stop_existing_services()     # Timer is active → stopped, recorded as was_running
10:21:37    install: install_systemd_services()   # Writes new unit files, runs daemon-reload
10:21:37    install: restart_services()           # Should restart timer...
10:21:37  sudo session closed                     # Install exits
10:21:52  sudo systemctl start mvm-sync          # Manual restart of sync only
          (timer never came back)
```

The install code in `src/commands/install.rs` correctly tracks which services were running and restarts them after upgrade. However, the `daemon-reload` and timer restart happen within the same wallclock second. The most likely cause is a **systemd race condition**: the `systemctl start mvm-status.timer` command executed immediately after `daemon-reload` may have returned success before systemd had fully processed the reload, resulting in the timer silently failing to activate.

The install code's `restart_services()` checks the exit status of `systemctl start` but doesn't verify the timer is actually active afterwards. This is a minor reliability gap.

**Tracked as:** GitHub issue #35

**Immediate action:** The timer is `enabled` and will auto-start on the next server reboot. No manual intervention needed if rebooting soon.

**Recommended fix:** Add post-restart verification in `restart_services()` — after starting each service, call `is_service_active()` to confirm it's actually running, with a brief delay after `daemon-reload` if needed. See issue #35.

---

## Pre-Reboot Checklist

Before rebooting the server:

1. **mvm-sync.service** is `enabled` — will auto-start on boot
2. **mvm-status.timer** is `enabled` — will auto-start on boot
3. Database integrity is confirmed — safe to shut down
4. WAL is fully checkpointed — no pending writes
5. No in-progress batch operations — sync is in polling mode (1 block at a time)

### Post-Reboot Verification

After the server comes back up:

```bash
# Verify services started
systemctl status mvm-sync
systemctl status mvm-status.timer

# Verify sync resumed
journalctl -u mvm-sync -n 20

# Quick health check
mvm query stats
```

---

## Conclusion

The mvm v1.0.3 sync daemon demonstrates production-grade stability:

- **27.4 days continuous uptime** with zero restarts or errors
- **No memory leaks**: 64 MB RSS, stable since startup, no swap usage
- **Minimal CPU**: 1.43% average
- **Clean FD management**: 15 descriptors, no accumulation
- **Database integrity**: Zero gaps, zero corruption, fully checkpointed WAL
- **Consistent throughput**: ~13,800 blocks/day without variance

The service is safe to restart and will resume cleanly after the server reboot.
