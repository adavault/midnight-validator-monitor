#!/bin/bash
# Uninstallation script for Midnight Validator Monitor

set -e

# Configuration
BINARY_NAME="mvm"
INSTALL_BASE="/opt/midnight/mvm"
SYSTEMD_DIR="/etc/systemd/system"
SYMLINK="/usr/local/bin/$BINARY_NAME"

# Check if running with sudo/root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        echo "ERROR: This script must be run with sudo"
        echo "Usage: sudo ./scripts/uninstall.sh"
        exit 1
    fi
}

# Stop and disable services
stop_services() {
    echo "==> Stopping and disabling services"

    if systemctl is-active --quiet mvm-sync 2>/dev/null; then
        systemctl stop mvm-sync
        echo "    Stopped mvm-sync"
    fi

    if systemctl is-enabled --quiet mvm-sync 2>/dev/null; then
        systemctl disable mvm-sync
        echo "    Disabled mvm-sync"
    fi

    if systemctl is-active --quiet mvm-status.timer 2>/dev/null; then
        systemctl stop mvm-status.timer
        echo "    Stopped mvm-status.timer"
    fi

    if systemctl is-enabled --quiet mvm-status.timer 2>/dev/null; then
        systemctl disable mvm-status.timer
        echo "    Disabled mvm-status.timer"
    fi
}

# Remove systemd services
remove_systemd_services() {
    echo "==> Removing systemd services"

    rm -f "$SYSTEMD_DIR/mvm-sync.service"
    rm -f "$SYSTEMD_DIR/mvm-status.service"
    rm -f "$SYSTEMD_DIR/mvm-status.timer"

    systemctl daemon-reload
    echo "    Systemd services removed"
}

# Remove symlink
remove_symlink() {
    if [ -L "$SYMLINK" ]; then
        echo "==> Removing symlink"
        rm -f "$SYMLINK"
        echo "    Removed $SYMLINK"
    fi
}

# Prompt to remove data
prompt_remove_data() {
    echo ""
    echo "=========================================="
    echo ""
    read -p "Remove data directory $INSTALL_BASE? This will delete your database. (y/N) " -n 1 -r
    echo ""

    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "==> Removing data"
        rm -rf "$INSTALL_BASE"
        echo "    Removed $INSTALL_BASE"
    else
        echo "==> Data preserved at:"
        echo "    $INSTALL_BASE"
        echo ""
        echo "    To manually remove later:"
        echo "      sudo rm -rf $INSTALL_BASE"
    fi
}

# Show completion message
show_completion() {
    echo ""
    echo "========================================"
    echo "Uninstallation Complete!"
    echo "========================================"
    echo ""
    echo "Removed:"
    echo "  - Systemd services"
    echo "  - Binary symlink"
    if [[ ! -d "$INSTALL_BASE" ]]; then
        echo "  - Data directory"
    fi
    echo ""
}

# Main uninstallation
main() {
    echo ""
    echo "Midnight Validator Monitor - Uninstallation"
    echo "============================================"
    echo ""

    check_root
    stop_services
    remove_systemd_services
    remove_symlink
    prompt_remove_data
    show_completion
}

main "$@"
