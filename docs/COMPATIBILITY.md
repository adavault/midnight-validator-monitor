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
