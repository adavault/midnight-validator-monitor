# Security Audit - January 19, 2026

**Audited Version:** v0.9.1
**Overall Grade:** B+ (Good)

---

## Executive Summary

MVM is a well-written Rust application with solid security fundamentals. No critical vulnerabilities were found. The codebase follows best practices for SQL injection prevention, safe command execution, and error handling.

---

## Build Configuration

### Current (Cargo.toml)
```toml
[profile.release]
opt-level = 3
lto = true
```

### Recommended Additions
```toml
[profile.release]
opt-level = 3
lto = true
strip = true       # Remove debug symbols from binary
panic = "abort"    # Smaller binary, reduced panic attack surface
```

**Rationale:**
- `strip = true` - Removes debug symbols, making reverse engineering harder and reducing binary size
- `panic = "abort"` - Prevents unwinding attacks, reduces binary size by ~10%

---

## Security Strengths

| Area | Status | Details |
|------|--------|---------|
| SQL Injection | Secure | All queries use parameterized statements via `params!` macro |
| Command Injection | Secure | Uses `Command::new()` with array args, not shell strings |
| Unsafe Code | Minimal | Only one instance: `libc::geteuid()` in install.rs:125 (safe) |
| Error Handling | Good | Proper `anyhow::Result` with context throughout |
| File Permissions | Good | Binary 0o755, service files 0o644, proper ownership |
| Systemd Integration | Good | Services run as unprivileged user, use journal logging |
| PID File Handling | Good | Automatic cleanup via Drop trait |

---

## Findings

### High Priority - None

### Medium Priority

| Issue | Location | Recommendation |
|-------|----------|----------------|
| Default RPC is HTTP | `src/config.rs:168` | Document security implications; consider warning on non-localhost HTTP |
| No TLS cert options | `src/rpc/client.rs` | Consider adding custom cert support for self-signed certs |

### Low Priority

| Issue | Location | Recommendation |
|-------|----------|----------------|
| No `PRAGMA foreign_keys=ON` | `src/db/mod.rs:34` | Add for referential integrity |
| Keystore permissions not validated | `src/midnight/keystore.rs` | Consider warning if keystore is world-readable |
| Config files readable by any user | N/A | Document that config with keystore paths should be 0o600 |
| Env vars visible in process list | N/A | Document; recommend systemd service for sensitive configs |

---

## Detailed Analysis

### SQL Queries - SECURE

All database queries use parameterized statements. Example from `src/db/blocks.rs:77-81`:

```rust
"SELECT ... WHERE block_number = ?1"
// with params![block_number as i64]
```

Dynamic IN clauses properly generate placeholders:
```rust
let placeholders: Vec<String> = (0..author_keys.len())
    .map(|i| format!("?{}", i + 3))  // Safe placeholder generation
    .collect();
```

### Command Execution - SECURE

All system commands use safe array-based arguments. Example from `src/commands/install.rs`:

```rust
Command::new("systemctl").args(["stop", "mvm-sync"]).status();
```

No shell invocation via `/bin/sh -c`.

### Network Security

- `reqwest` 0.11 uses system root certificates for TLS validation
- Retry logic has sensible error detection (no retry on auth failures)
- Configurable timeouts prevent hanging

### Sensitive Data Handling

- Keys loaded from filesystem only, never hardcoded
- Key display truncated for safety (`keystore.rs:115-123`)
- No credentials in log output at INFO level

---

## Recommendations Summary

### Quick Wins (v1.0)

1. **Cargo.toml** - Add `strip = true` and `panic = "abort"` to release profile
2. **Database** - Add `PRAGMA foreign_keys=ON` in `src/db/mod.rs:34`

### Future Considerations

3. Add warning when using HTTP RPC on non-localhost
4. Document keystore permission requirements (should be 0o700)
5. Consider adding `--insecure` flag required for HTTP on remote hosts

---

## Files Reviewed

- `/home/midnight/midnight-validator-monitor/Cargo.toml`
- `/home/midnight/midnight-validator-monitor/src/db/mod.rs`
- `/home/midnight/midnight-validator-monitor/src/db/blocks.rs`
- `/home/midnight/midnight-validator-monitor/src/db/validators.rs`
- `/home/midnight/midnight-validator-monitor/src/db/schema.rs`
- `/home/midnight/midnight-validator-monitor/src/rpc/client.rs`
- `/home/midnight/midnight-validator-monitor/src/config.rs`
- `/home/midnight/midnight-validator-monitor/src/commands/install.rs`
- `/home/midnight/midnight-validator-monitor/src/daemon.rs`
- `/home/midnight/midnight-validator-monitor/src/midnight/keystore.rs`

---

*Audit performed by Claude Code*
