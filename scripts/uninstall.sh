#!/bin/bash
# Uninstallation script for Midnight Validator Monitor

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

stop_services() {
    log_info "Stopping and disabling services"

    if systemctl is-active --quiet mvm-sync; then
        systemctl stop mvm-sync
    fi

    if systemctl is-enabled --quiet mvm-sync 2>/dev/null; then
        systemctl disable mvm-sync
    fi

    if systemctl is-active --quiet mvm-status.timer; then
        systemctl stop mvm-status.timer
    fi

    if systemctl is-enabled --quiet mvm-status.timer 2>/dev/null; then
        systemctl disable mvm-status.timer
    fi
}

remove_systemd_services() {
    log_info "Removing systemd service files"

    rm -f "$SYSTEMD_DIR/mvm-sync.service"
    rm -f "$SYSTEMD_DIR/mvm-status.service"
    rm -f "$SYSTEMD_DIR/mvm-status.timer"

    systemctl daemon-reload
}

remove_binary() {
    log_info "Removing binary"
    rm -f "$INSTALL_DIR/$BINARY_NAME"
}

remove_user() {
    if id "$USER" &>/dev/null; then
        log_info "Removing system user $USER"
        userdel "$USER" 2>/dev/null || true
    fi
}

prompt_remove_data() {
    echo ""
    read -p "Do you want to remove data directories? This will delete your database and logs. (y/N) " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        log_warn "Removing data directories"
        rm -rf "$DATA_DIR"
        rm -rf "$RUN_DIR"
        rm -rf "$LOG_DIR"
        rm -rf "$CONFIG_DIR"
    else
        log_info "Data directories preserved:"
        echo "  - $DATA_DIR"
        echo "  - $LOG_DIR"
        echo "  - $CONFIG_DIR"
    fi
}

# Main uninstallation
main() {
    log_info "Starting Midnight Validator Monitor uninstallation"

    check_root

    stop_services
    remove_systemd_services
    remove_binary
    remove_user
    prompt_remove_data

    echo ""
    log_info "Uninstallation complete!"
    echo ""
}

main "$@"
