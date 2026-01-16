#!/bin/bash
# Installation script for Midnight Validator Monitor
# Supports: Ubuntu, Debian, and other systemd-based Linux distributions

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
BINARY_NAME="mvm"
INSTALL_DIR="/usr/local/bin"
DATA_DIR="/var/lib/mvm"
RUN_DIR="/var/run/mvm"
LOG_DIR="/var/log/mvm"
CONFIG_DIR="/etc/mvm"
SYSTEMD_DIR="/etc/systemd/system"
USER="mvm"
GROUP="mvm"

# Functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_root() {
    if [ "$EUID" -ne 0 ]; then
        log_error "This script must be run as root (use sudo)"
        exit 1
    fi
}

check_systemd() {
    if ! command -v systemctl &> /dev/null; then
        log_error "systemd not found. This script requires systemd."
        exit 1
    fi
}

create_user() {
    if id "$USER" &>/dev/null; then
        log_info "User $USER already exists"
    else
        log_info "Creating system user $USER"
        useradd --system --no-create-home --shell /bin/false "$USER"
    fi
}

create_directories() {
    log_info "Creating directories"

    mkdir -p "$DATA_DIR"
    mkdir -p "$RUN_DIR"
    mkdir -p "$LOG_DIR"
    mkdir -p "$CONFIG_DIR"

    chown -R "$USER:$GROUP" "$DATA_DIR"
    chown -R "$USER:$GROUP" "$RUN_DIR"
    chown -R "$USER:$GROUP" "$LOG_DIR"
    chown -R "$USER:$GROUP" "$CONFIG_DIR"

    chmod 750 "$DATA_DIR"
    chmod 755 "$RUN_DIR"
    chmod 755 "$LOG_DIR"
    chmod 755 "$CONFIG_DIR"
}

install_binary() {
    log_info "Installing binary to $INSTALL_DIR"

    # Check if binary exists in current directory or target/release
    if [ -f "./$BINARY_NAME" ]; then
        cp "./$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
    elif [ -f "./target/release/$BINARY_NAME" ]; then
        cp "./target/release/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
    else
        log_error "Binary not found. Please build with 'cargo build --release' first."
        exit 1
    fi

    chmod 755 "$INSTALL_DIR/$BINARY_NAME"
    log_info "Binary installed successfully"
}

install_systemd_services() {
    log_info "Installing systemd service files"

    if [ ! -d "./systemd" ]; then
        log_error "systemd directory not found. Please run from project root."
        exit 1
    fi

    # Copy service files
    cp ./systemd/mvm-sync.service "$SYSTEMD_DIR/"
    cp ./systemd/mvm-status.service "$SYSTEMD_DIR/"
    cp ./systemd/mvm-status.timer "$SYSTEMD_DIR/"

    chmod 644 "$SYSTEMD_DIR/mvm-sync.service"
    chmod 644 "$SYSTEMD_DIR/mvm-status.service"
    chmod 644 "$SYSTEMD_DIR/mvm-status.timer"

    # Reload systemd
    systemctl daemon-reload

    log_info "Systemd services installed"
}

show_postinstall_info() {
    echo ""
    log_info "Installation complete!"
    echo ""
    echo "Next steps:"
    echo "  1. Configure your validator keystore in $DATA_DIR/keystore"
    echo "  2. Start the sync daemon:"
    echo "       sudo systemctl start mvm-sync"
    echo "  3. Enable auto-start on boot:"
    echo "       sudo systemctl enable mvm-sync"
    echo "  4. (Optional) Enable periodic status checks:"
    echo "       sudo systemctl enable --now mvm-status.timer"
    echo ""
    echo "Useful commands:"
    echo "  - View sync logs:    sudo journalctl -u mvm-sync -f"
    echo "  - View status:       sudo systemctl status mvm-sync"
    echo "  - Stop daemon:       sudo systemctl stop mvm-sync"
    echo "  - Restart daemon:    sudo systemctl restart mvm-sync"
    echo "  - Check validator:   $BINARY_NAME keys --keystore $DATA_DIR/keystore verify"
    echo ""
    echo "Directories:"
    echo "  - Binary:       $INSTALL_DIR/$BINARY_NAME"
    echo "  - Data:         $DATA_DIR"
    echo "  - Logs:         $LOG_DIR (or use journalctl)"
    echo "  - Config:       $CONFIG_DIR"
    echo "  - PID files:    $RUN_DIR"
    echo ""
}

# Main installation
main() {
    log_info "Starting Midnight Validator Monitor installation"

    check_root
    check_systemd

    # Stop services if running
    if systemctl is-active --quiet mvm-sync; then
        log_warn "Stopping existing mvm-sync service"
        systemctl stop mvm-sync
    fi

    if systemctl is-active --quiet mvm-status.timer; then
        log_warn "Stopping existing mvm-status timer"
        systemctl stop mvm-status.timer
    fi

    create_user
    create_directories
    install_binary
    install_systemd_services

    show_postinstall_info
}

main "$@"
