# Release Plan: v0.3.0

## Overview

v0.3.0 focuses on operational improvements for production deployment, adding daemon support for continuous monitoring and an interactive TUI for real-time visualization. This release transforms MVM from a CLI tool into a production-ready monitoring solution.

## Status: v0.2.0-alpha → v0.3.0

### ✅ Completed in v0.2.0-alpha

- [x] Block author attribution with validator set management
- [x] Validator tracking and database population (185+ validators)
- [x] Query commands (validators, validator, performance)
- [x] Enhanced keys command with database integration
- [x] Performance rankings and filtering by "ours"
- [x] Mainchain/sidechain epoch tracking
- [x] Batch validator set caching

### 🎯 Goals for v0.3.0

#### 1. Systemd Daemon Support (HIGH PRIORITY)

**Goal**: Enable MVM to run as a system service for continuous monitoring.

**Features**:
- Systemd service file for `mvm sync` daemon
- Systemd service file for periodic `mvm status` health checks
- Systemd timer for scheduled checks and alerts
- Graceful shutdown handling (SIGTERM, SIGINT)
- PID file management
- Log rotation integration
- Auto-restart on failure
- Installation script for systemd setup

**Implementation**:
1. **Service Files**:
   - `systemd/mvm-sync.service` - Continuous block sync daemon
   - `systemd/mvm-status.service` - One-shot health check
   - `systemd/mvm-status.timer` - Periodic health check schedule
   - `systemd/mvm-view.service` - Optional TUI service

2. **Signal Handling**:
   - Implement graceful shutdown in sync command
   - Handle SIGTERM for clean database closure
   - Handle SIGHUP for config reload (future)
   - Flush database transactions on shutdown

3. **Daemon Options**:
   - Add `--daemon` flag to sync command
   - Add `--pid-file` option for PID management
   - Add `--log-file` option for file logging
   - Add `--syslog` option for systemd journal integration

**Files to create**:
- `systemd/mvm-sync.service`
- `systemd/mvm-status.service`
- `systemd/mvm-status.timer`
- `systemd/mvm-view.service`
- `scripts/install.sh`
- `scripts/uninstall.sh`
- `DEPLOYMENT.md`

**Files to modify**:
- `src/commands/sync.rs` - Add signal handling and daemon mode
- `src/main.rs` - Add global signal handlers
- `Cargo.toml` - Add signal-hook dependency

#### 2. Text User Interface (TUI) View Mode (HIGH PRIORITY)

**Goal**: Real-time monitoring dashboard with interactive terminal UI.

**Features**:
- Live block sync status with progress bars
- Real-time validator performance display
- Our validator statistics highlighted
- Block production rate graphs
- Epoch and slot information
- Network health indicators
- Key status indicators
- Keyboard navigation and interaction
- Auto-refresh capabilities
- Themeable interface

**Implementation**:
1. **Core TUI Framework**:
   - Use `ratatui` (formerly tui-rs) for terminal UI
   - Use `crossterm` for cross-platform terminal handling
   - Implement async event handling

2. **View Components**:
   - **Dashboard View**: Overview of system status
   - **Blocks View**: Recent blocks with authors
   - **Validators View**: Scrollable validator list
   - **Performance View**: Rankings and charts
   - **Logs View**: Real-time log output
   - **Keys View**: Session key status

3. **Dashboard Layout**:
   ```
   ┌─────────────────────────────────────────────────────────────┐
   │ Midnight Validator Monitor v0.3.0          [Q] Quit [?] Help│
   ├─────────────────────────────────────────────────────────────┤
   │ Network Status:                  Epoch: 1179   Slot: 294758 │
   │  Chain Tip:    #3359132          Sync Status: ● Synced      │
   │  Finalized:    #3359130          Last Update: 2s ago        │
   ├─────────────────────────────────────────────────────────────┤
   │ Our Validators (1):              Performance Rank: #1/185   │
   │  0x037764d2...809f4700           Blocks: 9  Share: 0.51%    │
   │  Status: ✓ Registered            Recent: 3m ago (blk #3358) │
   ├─────────────────────────────────────────────────────────────┤
   │ Recent Blocks:                                              │
   │  #3359132  slot 294758944  epoch 1179  author: 0x0250...  │
   │  #3359131  slot 294758943  epoch 1179  author: 0x03ab...  │
   │  #3359130  slot 294758942  epoch 1179  author: 0x024f...  │
   ├─────────────────────────────────────────────────────────────┤
   │ Block Production (last 100 blocks):                         │
   │  █▄▄█▄█▄▄▄█▄▄▄▄█▄█▄▄█▄▄▄▄▄█▄▄█▄▄█▄█▄▄                      │
   ├─────────────────────────────────────────────────────────────┤
   │ [1] Dashboard [2] Blocks [3] Validators [4] Performance    │
   └─────────────────────────────────────────────────────────────┘
   ```

4. **Keyboard Controls**:
   - `q` / `Esc` - Quit
   - `1-5` - Switch views
   - `↑/↓` - Scroll lists
   - `r` - Force refresh
   - `f` - Toggle filter (our validators)
   - `/` - Search
   - `?` - Help screen

**Dependencies to add**:
```toml
ratatui = "0.26"
crossterm = "0.27"
```

**Files to create**:
- `src/commands/view.rs` - Main TUI command
- `src/tui/mod.rs` - TUI module root
- `src/tui/app.rs` - Application state management
- `src/tui/ui.rs` - UI rendering
- `src/tui/event.rs` - Event handling
- `src/tui/widgets/` - Custom widgets
  - `dashboard.rs`
  - `blocks.rs`
  - `validators.rs`
  - `performance.rs`

**Files to modify**:
- `src/commands/mod.rs` - Add view command
- `src/main.rs` - Register view command
- `Cargo.toml` - Add TUI dependencies

#### 3. Configuration File Support (MEDIUM PRIORITY)

**Goal**: Persistent configuration management for easier deployment.

**Features**:
- TOML configuration file (`mvm.toml`, `~/.config/mvm/config.toml`)
- Environment variable support
- Priority: CLI flags > Environment > Config file > Defaults
- `mvm config` command to show/validate/init config
- Per-environment configs (dev, staging, prod)

**Configuration Schema**:
```toml
[rpc]
url = "http://localhost:9944"
timeout_ms = 30000

[database]
path = "./mvm.db"
# Optional: connection pool settings
max_connections = 5

[validator]
keystore_path = "/path/to/keystore"
label = "MyValidator"  # Friendly name for display

[sync]
batch_size = 100
poll_interval_secs = 6
finalized_only = false
start_block = 0  # 0 = auto-detect

[view]
refresh_interval_ms = 2000
theme = "dark"  # dark, light, midnight

[daemon]
pid_file = "/var/run/mvm.pid"
log_file = "/var/log/mvm.log"
enable_syslog = true

[alerts]
# Future: webhook URLs for alerts
# webhook_url = "https://..."
# alert_on_down = true
```

**Files to create**:
- `src/config.rs` - Configuration loading and validation
- `examples/mvm.toml` - Example configuration

**Files to modify**:
- All command files to read from config
- `src/commands/mod.rs` - Add config command

#### 4. Enhanced Logging and Observability (MEDIUM PRIORITY)

**Goal**: Better logging for production debugging and monitoring.

**Features**:
- Structured logging with JSON output option
- Log levels: ERROR, WARN, INFO, DEBUG, TRACE
- Log file rotation support
- Systemd journal integration
- Performance metrics logging
- Request/response logging for RPC calls (debug mode)
- Database query logging (trace mode)

**Implementation**:
- Add `--log-format` flag (text, json)
- Add `--log-file` flag for file output
- Add request ID tracking for RPC calls
- Add timing metrics for operations

**Dependencies to add**:
```toml
tracing-appender = "0.2"  # For file rotation
tracing-journald = "0.3"  # For systemd integration
```

**Files to modify**:
- `src/main.rs` - Enhanced tracing setup
- All command files - Add structured logging

#### 5. Installation and Packaging (MEDIUM PRIORITY)

**Goal**: Easy installation and distribution.

**Features**:
- Installation script (`install.sh`)
- Uninstallation script (`uninstall.sh`)
- Systemd service installation
- Binary distribution in GitHub releases
- Optional: Debian package (.deb)
- Optional: RPM package

**Installation Script Features**:
- Detect OS and init system
- Create system user `mvm`
- Install binary to `/usr/local/bin/mvm`
- Install systemd services to `/etc/systemd/system/`
- Create directories: `/var/lib/mvm`, `/var/log/mvm`
- Set appropriate permissions
- Initialize default config at `/etc/mvm/config.toml`

**Files to create**:
- `scripts/install.sh`
- `scripts/uninstall.sh`
- `scripts/build-release.sh`
- `DEPLOYMENT.md`

#### 6. Performance Optimizations (LOW PRIORITY)

**Goal**: Improve sync speed and reduce resource usage.

**Features**:
- Connection pooling for HTTP requests
- Parallel block fetching (within batch)
- Database write batching
- Reduced validator set queries (cache longer)
- Optional: In-memory cache for recent blocks

**Files to modify**:
- `src/rpc/client.rs` - Connection pooling
- `src/commands/sync.rs` - Parallel fetching
- `src/db/mod.rs` - Batch writes

## Testing Requirements

### Unit Tests
- [ ] Signal handling for graceful shutdown
- [ ] Configuration file parsing and validation
- [ ] TUI component rendering
- [ ] Event handling in TUI

### Integration Tests
- [ ] Daemon startup and shutdown
- [ ] Config file priority (CLI > Env > File)
- [ ] TUI interaction scenarios
- [ ] Log output formats

### Manual Testing
- [ ] Install via install.sh on fresh system
- [ ] Run as systemd service for 24 hours
- [ ] TUI responsiveness with large datasets
- [ ] Configuration changes without restart
- [ ] Log rotation behavior

## Documentation Updates

- [x] Create DEPLOYMENT.md with systemd setup guide
- [ ] Update README with:
  - Installation instructions
  - Systemd service setup
  - TUI keyboard shortcuts
  - Configuration file documentation
- [ ] Create examples/ directory:
  - `examples/mvm.toml` - Example config
  - `examples/systemd/` - Service file examples
  - `examples/docker/` - Docker deployment (future)
- [ ] Update CLAUDE.md with new commands and features
- [ ] Add architecture documentation for TUI

## Dependencies to Add

```toml
# TUI
ratatui = "0.26"
crossterm = "0.27"

# Signal handling
signal-hook = "0.3"
signal-hook-tokio = { version = "0.3", features = ["futures-v0_3"] }

# Logging
tracing-appender = "0.2"
tracing-journald = "0.3"

# Configuration
config = "0.14"  # or toml = "0.8"
directories = "5.0"  # For XDG config paths
```

## Breaking Changes

None planned - v0.3.0 should be fully backward compatible with v0.2.0-alpha.

Configuration file is optional and only used if present.

## Migration Guide

### From v0.2.0-alpha to v0.3.0

No database schema changes required. Simply:
1. Stop existing `mvm` processes
2. Install new binary
3. Optionally create configuration file
4. Optionally install systemd services
5. Restart

## Timeline Estimate

- **Systemd Daemon Support**: 2-3 days
- **TUI View Mode**: 3-4 days
- **Configuration Support**: 1-2 days
- **Logging Enhancements**: 1 day
- **Installation Scripts**: 1-2 days
- **Performance Optimizations**: 1-2 days
- **Testing & Documentation**: 2-3 days

**Total**: ~2-3 weeks of development

## Success Criteria

v0.3.0 is ready for release when:

1. [ ] Sync can run as systemd service with auto-restart
2. [ ] TUI provides real-time monitoring with all views functional
3. [ ] Configuration file works with all commands
4. [ ] Installation script successfully deploys on Ubuntu/Debian
5. [ ] Daemon runs continuously for 7+ days without issues
6. [ ] TUI is responsive with 10,000+ blocks in database
7. [ ] All tests pass
8. [ ] Documentation is complete with deployment guide

## Future: v0.4.0

Deferred to v0.4.0:
- Alert webhooks and notifications
- WebSocket support for real-time RPC updates
- Historical performance metrics and trends
- Session key rotation detection
- Multi-validator support (multiple keystores)
- Export functionality (CSV, JSON, Prometheus metrics)
- API server for external integrations
- Docker container image
- Kubernetes deployment manifests

## Implementation Phases

### Phase 1: Daemon Foundation (Week 1)
- Signal handling
- Systemd service files
- Basic daemon mode
- Installation script

### Phase 2: TUI Development (Week 2)
- TUI framework setup
- Dashboard view
- Blocks and validators views
- Performance view
- Keyboard navigation

### Phase 3: Polish & Production Ready (Week 3)
- Configuration file support
- Enhanced logging
- Performance optimizations
- Documentation
- Testing and refinement

## Notes

### Systemd Considerations
- Use `Type=simple` for mvm-sync.service (foreground process)
- Use `Type=oneshot` for mvm-status.service
- Enable `Restart=on-failure` with `RestartSec=10s`
- Set `User=mvm` and `Group=mvm` for security
- Use `StandardOutput=journal` for systemd integration

### TUI Considerations
- TUI requires terminal with at least 80x24 characters
- Colors may not work on all terminals (provide fallback)
- SSH sessions should work via proper terminal handling
- Consider `screen` and `tmux` compatibility
- Provide `--no-color` flag for CI/CD environments

### Configuration Considerations
- Support both `/etc/mvm/config.toml` (system-wide) and `~/.config/mvm/config.toml` (user)
- Environment variables: `MVM_RPC_URL`, `MVM_DB_PATH`, etc.
- Validate configuration on load with helpful error messages
- Provide `mvm config validate` subcommand
