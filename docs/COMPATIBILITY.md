# Compatibility Policy

## Pre-v1.0 Compatibility

**Breaking changes are acceptable before v1.0.**

During the pre-release phase (v0.x), we prioritize getting the data structures and architecture right over maintaining backwards compatibility. This means:

- Database schema changes may require dropping and recreating the database
- Configuration file formats may change without migration paths
- API/RPC interfaces may change without deprecation periods
- Stored data formats may change

**Why?** We need flexibility to iterate on the data model based on real-world usage and feedback. Locking in a suboptimal schema early would be costly to maintain long-term.

**What this means for users:**
- Keep backups if you have important data
- Be prepared to re-sync the blockchain database after upgrades
- Check release notes before upgrading

## Post-v1.0 Compatibility

Once we release v1.0, we will follow semantic versioning strictly:

- **Major versions (2.0, 3.0)**: May include breaking changes with migration guides
- **Minor versions (1.1, 1.2)**: Backwards-compatible new features
- **Patch versions (1.0.1, 1.0.2)**: Backwards-compatible bug fixes

Database migrations will be provided for schema changes in minor and patch versions.

## Current Schema Version

As of v0.6.1, the database schema includes:

- `blocks` table with `sidechain_epoch` field (added in v0.6.1)
- `validators` table with `is_ours` preservation fix
- `committee_snapshots` table
- `sync_status` table

Upgrading from v0.6.0 to v0.6.1 requires recreating the database due to the schema change.

## Midnight Node Version Compatibility

### Supported Versions

| MVM Version | Midnight Node | Network Preset | Status |
|-------------|---------------|----------------|--------|
| v1.0.0 | 0.12.0, 0.12.1 | `testnet-02` | ✓ Supported |
| v1.0.0 | 0.18.0+ | `preview` | ✗ Not yet supported |

### Current Recommendation (January 2026)

**Use `midnightnetwork/midnight-node:0.12.1` with `CFG_PRESET=testnet-02`**

This is the configuration actively used by validators in the Midnight community. MVM v1.0.0 is tested and validated against this setup.

### v0.18.0 Analysis

Midnight node v0.18.0 was released in December 2025 as the first open-source release. However, it introduces breaking changes that prevent immediate adoption:

#### Breaking Changes in v0.18.0

1. **Network Preset Renamed**: `testnet-02` preset removed, replaced with `preview`
2. **Docker Registry Changed**: Images moved from `midnightnetwork/` to `midnightntwrk/`
3. **New Required Parameters**: `CARDANO_SECURITY_PARAMETER` and `CARDANO_ACTIVE_SLOTS_COEFF` (have defaults but required in config chain)
4. **DB-Sync Configuration**: New format required with `cardano_security_parameter` field
5. **Chain Data Incompatible**: v0.12.x chain data cannot be read by v0.18.0 ("non-canonical scale encoding")
6. **Bootnodes Not Published**: The `preview` chain spec has empty bootnodes array

#### Why v0.18.0 Doesn't Work Yet

The `preview` network preset in v0.18.0 requires bootnodes to connect to the network. These bootnodes are **not documented** in:
- The official `midnight-node-docker` repository (still has only `testnet-02` and `qanet` configs)
- The chain spec embedded in the v0.18.0 image (empty bootnodes array)
- Public Midnight documentation

Community members have reported the same issue in Discord (December 2025 - January 2026):
- "Does anyone know how to start syncing the 'preview' network with the latest node? I am looking for the BOOT_NODES" - multiple users asking
- "0.12.1 also worked, but version 0.18.0 notworking" - kit3184, Jan 4 2026
- "You cannot register a validator on 0.18.0. You need a newer version." - chuckk_420, Jan 7 2026

#### Migration Path (When Available)

Once Midnight publishes the preview network bootnodes, upgrading will require:

```bash
# 1. Update compose.yml
image: midnightntwrk/midnight-node:0.18.0  # Note: different registry
environment:
  - CFG_PRESET=preview                       # Changed from testnet-02
  - BOOTNODES=<preview-bootnodes>            # When published
  - CARDANO_SECURITY_PARAMETER=432
  - CARDANO_ACTIVE_SLOTS_COEFF=0.05
  - DB_SYNC_POSTGRES_CONNECTION_STRING=postgresql://...  # Note: postgresql:// not psql://

# 2. Wipe chain data (incompatible format)
docker volume rm midnight-node_midnight-data

# 3. Wipe MVM database (different network)
rm /opt/midnight/mvm/data/mvm.db

# 4. Restart and re-sync from genesis
docker compose up -d
```

#### Tracking v0.18.0 Support

MVM will add v0.18.0/preview support when:
1. Midnight publishes official preview network bootnodes
2. The `midnight-node-docker` repository is updated with preview configuration
3. Community validators confirm successful block production on v0.18.0

Track progress: Check `#block-producers` channel in Midnight Discord for announcements.

### Version Detection

MVM automatically detects the node version via `system_version` RPC:

```bash
mvm status --once  # Shows "Node version: 0.12.1-045f8372"
```

## Recreating the Database

To recreate the database after an upgrade:

```bash
# Stop the sync daemon
sudo systemctl stop mvm-sync

# Remove the old database
rm /opt/midnight/mvm/data/mvm.db

# Restart sync (will recreate database and re-sync)
sudo systemctl start mvm-sync

# Re-register your validator keys
mvm keys --keystore /path/to/keystore --db-path /opt/midnight/mvm/data/mvm.db verify
```
