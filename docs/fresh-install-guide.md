# Fresh Midnight Validator Install Guide

This guide assumes you have:
- A clean Ubuntu server with Docker installed
- Your validator keys backed up (you'll restore them later)
- Completed SPO registration on Cardano Preview

---

## Step 1: Clean up previous install (if any)

```bash
# Stop and remove old containers
cd ~/midnight-node-docker 2>/dev/null && docker compose down --remove-orphans || true

# Remove old data (make sure you have backed up your keys safely OUTSIDE this directory first!)
cd ~ && rm -rf midnight-node-docker
```

**Checkpoint:** `docker ps` should show no midnight containers

---

## Step 2: Clone fresh midnight-node-docker

```bash
cd ~
git clone https://github.com/midnight-network/midnight-node-docker.git
cd midnight-node-docker
```

**Checkpoint:** You should be in `~/midnight-node-docker`

---

## Step 3: Install direnv (if not installed)

```bash
sudo apt update && sudo apt install -y direnv

# Add to shell (bash)
echo 'eval "$(direnv hook bash)"' >> ~/.bashrc
source ~/.bashrc
```

---

## Step 4: Configure .envrc

```bash
# Copy the example
cp .envrc.example .envrc

# Edit with your details
nano .envrc
```

**CRITICAL: Your .envrc must have these settings:**

```bash
# Replace YOUR_PUBLIC_IP with your server's public IP
export APPEND_ARGS="--public-addr /ip4/YOUR_PUBLIC_IP/tcp/30333 --validator --allow-private-ip --pool-limit 10 --trie-cache-size 0 --prometheus-external --unsafe-rpc-external --rpc-cors all --rpc-methods=unsafe"

# Postgres settings (use these exact values for docker setup)
export POSTGRES_HOST="postgres"
export POSTGRES_PORT="5432"
export POSTGRES_USER="postgres"
export POSTGRES_PASSWORD="postgres"
export POSTGRES_DB="midnight"

# Network
export CARDANO_NETWORK="preview"
```

Save and exit (Ctrl+X, Y, Enter)

```bash
# Allow direnv to load the file
direnv allow
```

**Checkpoint:** When you `cd` into the directory, you should see:
```
direnv: loading ~/midnight-node-docker/.envrc
direnv: export +APPEND_ARGS +POSTGRES_HOST ...
```

---

## Step 5: Start the stack

```bash
# Pull latest images
docker compose pull

# Start everything
docker compose up -d

# Check all containers are running
docker compose ps
```

**Checkpoint:** You should see containers for:
- midnight-node (or similar)
- postgres
- db-sync (or chain-indexer)
- possibly cardano-node

All should show "Up" or "running"

---

## Step 6: Verify node is accessible

Wait 30 seconds, then:

```bash
# Test RPC endpoint
curl -s http://localhost:9944 | head -c 100

# Test metrics endpoint
curl -s http://localhost:9615/metrics | head -3
```

**Checkpoint:**
- RPC should return something (even an error JSON is fine)
- Metrics should return prometheus-style text

**If you get "connection refused":** Check your APPEND_ARGS has `--unsafe-rpc-external` and `--prometheus-external`

---

## Step 7: Check sync progress

```bash
# View node logs
docker compose logs -f midnight-node 2>&1 | head -50
```

**Checkpoint:** You should see logs mentioning:
- Peer connections
- Block imports
- Sync progress

Press Ctrl+C to exit logs

---

## Step 8: Restore your keys

Your keys should be restored to the keystore directory:

```bash
# Create keystore directory if needed
mkdir -p ~/midnight-node-docker/data/chains/partner_chains_template/keystore

# Copy your backed up keys here
# Keys are files like: 6175726158abc123... (aura), 6772616e... (gran), 63726368... (crch)
cp /path/to/your/backed/up/keys/* ~/midnight-node-docker/data/chains/partner_chains_template/keystore/

# Restart node to pick up keys
docker compose restart midnight-node
```

**Checkpoint:** `ls ~/midnight-node-docker/data/chains/partner_chains_template/keystore/` should show your 3 key files

---

## Step 9: Install MVM for monitoring

```bash
# Download latest release
curl -LO https://github.com/adavault/midnight-validator-monitor/releases/latest/download/mvm-linux-x86_64

# Make executable and install
chmod +x mvm-linux-x86_64
sudo ./mvm-linux-x86_64 install
```

**Checkpoint:** `mvm --version` should show version 1.0.0 or later

---

## Step 10: Configure MVM

```bash
sudo nano /opt/midnight/mvm/config/config.toml
```

Add your keystore path:

```toml
[validator]
keystore_path = "/home/YOUR_USERNAME/midnight-node-docker/data/chains/partner_chains_template/keystore"
```

Replace `YOUR_USERNAME` with your actual username.

---

## Step 11: Start MVM sync daemon

```bash
sudo systemctl start mvm-sync
sudo systemctl enable mvm-sync
```

**Checkpoint:** `sudo systemctl status mvm-sync` should show "active (running)"

---

## Step 12: View your validator status

```bash
# One-time status check
mvm status --once

# Interactive TUI
mvm view
```

**What to expect:**
- During initial sync: 0 peers is NORMAL, sync % will be shown
- After sync: peers will appear, blocks will be attributed
- Your keys should show as "registered" if your SPO registration is complete

---

## Troubleshooting

### "Connection refused" on RPC/metrics
- Check APPEND_ARGS in .envrc has `--unsafe-rpc-external` and `--prometheus-external`
- Run `direnv allow` after editing .envrc
- Restart: `docker compose down && docker compose up -d`

### 0 peers for a long time
- This is normal during sync (can take 12-24 hours)
- Outbound connections happen first, inbound only after sync
- Check logs: `docker compose logs -f midnight-node`

### postgres connection errors
- Ensure POSTGRES_HOST="postgres" (not localhost)
- Check postgres is running: `docker compose ps`
- Check logs: `docker compose logs postgres`

### Keys not recognized
- Verify keys are in correct directory
- Key filenames must be exact (hex format)
- Restart node after adding keys

---

## Quick reference commands

```bash
# View node logs
docker compose logs -f midnight-node

# Restart node
docker compose restart midnight-node

# Check MVM sync status
mvm query stats

# View TUI
mvm view

# Check systemd services
sudo systemctl status mvm-sync
```

---

*Guide created for MVM v1.0.0 - January 2026*
