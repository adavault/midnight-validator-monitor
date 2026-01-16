#!/bin/bash
# Installation script for Midnight Validator Monitor
# Installs to /opt/midnight/mvm with systemd services

set -e

# Determine the real user (if running via sudo)
if [ -n "$SUDO_USER" ]; then
    REAL_USER="$SUDO_USER"
    REAL_HOME=$(getent passwd "$SUDO_USER" | cut -d: -f6)
else
    REAL_USER="$USER"
    REAL_HOME="$HOME"
fi

# Configuration
BINARY_NAME="mvm"
INSTALL_BASE="/opt/midnight/mvm"
BIN_DIR="$INSTALL_BASE/bin"
DATA_DIR="$INSTALL_BASE/data"
CONFIG_DIR="$INSTALL_BASE/config"
SYSTEMD_DIR="/etc/systemd/system"

# Check if running with sudo/root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        echo "ERROR: This script must be run with sudo"
        echo "Usage: sudo ./scripts/install.sh"
        exit 1
    fi
}

# Create directory structure
create_directories() {
    echo "==> Creating directories"

    mkdir -p "$BIN_DIR"
    mkdir -p "$DATA_DIR"
    mkdir -p "$CONFIG_DIR"

    # Set ownership to real user
    chown -R "$REAL_USER:$REAL_USER" "$INSTALL_BASE"
    chmod 755 "$INSTALL_BASE"
    chmod 755 "$BIN_DIR"
    chmod 755 "$DATA_DIR"
    chmod 755 "$CONFIG_DIR"

    echo "    Directories created at $INSTALL_BASE"
}

# Install binary
install_binary() {
    echo "==> Installing binary"

    # Check if binary exists
    if [ -f "./target/release/$BINARY_NAME" ]; then
        cp "./target/release/$BINARY_NAME" "$BIN_DIR/$BINARY_NAME"
    elif [ -f "./$BINARY_NAME" ]; then
        cp "./$BINARY_NAME" "$BIN_DIR/$BINARY_NAME"
    else
        echo "ERROR: Binary not found. Please build with 'cargo build --release' first."
        exit 1
    fi

    chmod 755 "$BIN_DIR/$BINARY_NAME"
    chown "$REAL_USER:$REAL_USER" "$BIN_DIR/$BINARY_NAME"

    # Create symlink to /usr/local/bin
    ln -sf "$BIN_DIR/$BINARY_NAME" "/usr/local/bin/$BINARY_NAME"

    echo "    Binary installed to $BIN_DIR/$BINARY_NAME"
    echo "    Symlink created at /usr/local/bin/$BINARY_NAME"
}

# Create default configuration
create_config() {
    echo "==> Creating configuration"

    local config_file="$CONFIG_DIR/config.toml"

    if [ -f "$config_file" ]; then
        echo "    Config already exists, skipping"
    else
        cat > "$config_file" << EOF
[rpc]
url = "http://localhost:9944"
metrics_url = "http://localhost:9615/metrics"

[database]
path = "$DATA_DIR/mvm.db"

[sync]
batch_size = 100
poll_interval_secs = 6

[daemon]
pid_file = "$DATA_DIR/mvm-sync.pid"
EOF

        chown "$REAL_USER:$REAL_USER" "$config_file"
        echo "    Config created at $config_file"
    fi
}

# Install systemd services
install_systemd_services() {
    echo "==> Installing systemd services"

    # Create mvm-sync.service
    cat > "$SYSTEMD_DIR/mvm-sync.service" << EOF
[Unit]
Description=Midnight Validator Monitor - Block Sync Daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$REAL_USER
WorkingDirectory=$INSTALL_BASE
Environment="MVM_DB_PATH=$DATA_DIR/mvm.db"
ExecStart=$BIN_DIR/mvm sync --daemon --pid-file $DATA_DIR/mvm-sync.pid
Restart=on-failure
RestartSec=10s
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

    # Create mvm-status.service
    cat > "$SYSTEMD_DIR/mvm-status.service" << EOF
[Unit]
Description=Midnight Validator Monitor - Status Check
After=network-online.target

[Service]
Type=oneshot
User=$REAL_USER
WorkingDirectory=$INSTALL_BASE
Environment="MVM_DB_PATH=$DATA_DIR/mvm.db"
ExecStart=$BIN_DIR/mvm status --once
StandardOutput=journal
StandardError=journal
EOF

    # Create mvm-status.timer
    cat > "$SYSTEMD_DIR/mvm-status.timer" << EOF
[Unit]
Description=Midnight Validator Monitor - Periodic Status Check

[Timer]
OnBootSec=1min
OnUnitActiveSec=5min
Persistent=true

[Install]
WantedBy=timers.target
EOF

    chmod 644 "$SYSTEMD_DIR/mvm-sync.service"
    chmod 644 "$SYSTEMD_DIR/mvm-status.service"
    chmod 644 "$SYSTEMD_DIR/mvm-status.timer"

    # Reload systemd
    systemctl daemon-reload

    echo "    Systemd services installed"
}

# Stop existing services
stop_existing_services() {
    if systemctl is-active --quiet mvm-sync 2>/dev/null; then
        echo "==> Stopping existing mvm-sync service"
        systemctl stop mvm-sync
    fi

    if systemctl is-active --quiet mvm-status.timer 2>/dev/null; then
        echo "==> Stopping existing mvm-status timer"
        systemctl stop mvm-status.timer
    fi
}

# Show completion message
show_completion() {
    echo ""
    echo "========================================"
    echo "Installation Complete!"
    echo "========================================"
    echo ""
    echo "Installation location: $INSTALL_BASE"
    echo "  Binary:    $BIN_DIR/$BINARY_NAME"
    echo "  Data:      $DATA_DIR"
    echo "  Config:    $CONFIG_DIR/config.toml"
    echo "  Database:  $DATA_DIR/mvm.db"
    echo ""
    echo "Running as user: $REAL_USER"
    echo ""
    echo "Next steps:"
    echo ""
    echo "  1. Start the sync daemon:"
    echo "       sudo systemctl start mvm-sync"
    echo ""
    echo "  2. Enable auto-start on boot:"
    echo "       sudo systemctl enable mvm-sync"
    echo ""
    echo "  3. (Optional) Enable periodic health checks:"
    echo "       sudo systemctl enable --now mvm-status.timer"
    echo ""
    echo "  4. View logs:"
    echo "       sudo journalctl -u mvm-sync -f"
    echo ""
    echo "  5. Check status:"
    echo "       sudo systemctl status mvm-sync"
    echo ""
    echo "  6. Interactive TUI:"
    echo "       mvm view"
    echo ""
    echo "  7. Query data:"
    echo "       mvm query stats"
    echo "       mvm query validators"
    echo ""
    echo "All commands will use $DATA_DIR/mvm.db by default"
    echo ""
}

# Main installation
main() {
    echo ""
    echo "Midnight Validator Monitor - Installation"
    echo "=========================================="
    echo ""
    echo "Installing to: $INSTALL_BASE"
    echo "User: $REAL_USER"
    echo ""

    check_root
    stop_existing_services
    create_directories
    install_binary
    create_config
    install_systemd_services
    show_completion
}

main "$@"
