#!/bin/bash
# Remote build script for MVM
# Builds on vdumdn90 and optionally deploys to local machine

set -e

BUILD_HOST="rezi@vdumdn90"
BUILD_DIR="~/midnight-validator-monitor"
BINARY_NAME="mvm"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() { echo -e "${GREEN}[BUILD]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --pull       Pull latest changes before building"
    echo "  --deploy     Deploy binary to /usr/local/bin after build"
    echo "  --clean      Clean build (cargo clean first)"
    echo "  --help       Show this help"
    echo ""
    echo "Examples:"
    echo "  $0                    # Build only"
    echo "  $0 --pull --deploy    # Pull, build, and deploy"
}

DO_PULL=false
DO_DEPLOY=false
DO_CLEAN=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --pull)
            DO_PULL=true
            shift
            ;;
        --deploy)
            DO_DEPLOY=true
            shift
            ;;
        --clean)
            DO_CLEAN=true
            shift
            ;;
        --help)
            usage
            exit 0
            ;;
        *)
            error "Unknown option: $1"
            ;;
    esac
done

# Check SSH connectivity
log "Checking SSH connectivity to $BUILD_HOST..."
if ! ssh -o ConnectTimeout=5 $BUILD_HOST "echo 'connected'" >/dev/null 2>&1; then
    error "Cannot connect to $BUILD_HOST"
fi

# Pull latest if requested
if [ "$DO_PULL" = true ]; then
    log "Pulling latest changes..."
    ssh $BUILD_HOST "cd $BUILD_DIR && git pull"
fi

# Clean if requested
if [ "$DO_CLEAN" = true ]; then
    log "Cleaning build directory..."
    ssh $BUILD_HOST "cd $BUILD_DIR && cargo clean"
fi

# Build
log "Building release binary on $BUILD_HOST..."
BUILD_START=$(date +%s)
ssh $BUILD_HOST "source ~/.cargo/env && cd $BUILD_DIR && cargo build --release" 2>&1
BUILD_END=$(date +%s)
BUILD_TIME=$((BUILD_END - BUILD_START))
log "Build completed in ${BUILD_TIME}s"

# Get version
VERSION=$(ssh $BUILD_HOST "$BUILD_DIR/target/release/$BINARY_NAME --version" | awk '{print $2}')
log "Built version: $VERSION"

# Deploy if requested
if [ "$DO_DEPLOY" = true ]; then
    log "Deploying to local machine..."

    # Check if mvm-sync is running and stop it
    if systemctl is-active --quiet mvm-sync 2>/dev/null; then
        warn "Stopping mvm-sync service..."
        sudo systemctl stop mvm-sync
        RESTART_SYNC=true
    else
        RESTART_SYNC=false
    fi

    # Copy binary
    TEMP_BIN=$(mktemp)
    scp $BUILD_HOST:$BUILD_DIR/target/release/$BINARY_NAME $TEMP_BIN
    sudo mv $TEMP_BIN /usr/local/bin/$BINARY_NAME
    sudo chmod +x /usr/local/bin/$BINARY_NAME

    # Verify
    INSTALLED_VERSION=$(/usr/local/bin/$BINARY_NAME --version | awk '{print $2}')
    log "Installed version: $INSTALLED_VERSION"

    # Restart service if it was running
    if [ "$RESTART_SYNC" = true ]; then
        log "Restarting mvm-sync service..."
        sudo systemctl start mvm-sync
    fi

    log "Deployment complete!"
fi

log "Done!"
