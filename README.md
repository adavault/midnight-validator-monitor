# Midnight Validator Monitor

A Rust CLI tool for monitoring Midnight blockchain validator nodes.

## Features

- **Node Health**: Sync status, peer count, block height
- **Block Production**: Track blocks produced via Prometheus metrics
- **Key Status**: Auto-detect keys from Substrate keystore
- **Registration Check**: Verify validator registration status (permissioned, valid, invalid)

## Installation

Requires Rust 1.70+

```bash
cargo build --release
```

## Usage

```bash
# Basic monitoring (runs every 60s)
cargo run -- --keystore /path/to/keystore

# Single check
cargo run -- --once --keystore /path/to/keystore

# Custom endpoints
cargo run -- \
  --rpc-url http://localhost:9944 \
  --metrics-url http://localhost:9615/metrics \
  --keystore /path/to/keystore \
  --interval 30
```

### CLI Options

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--rpc-url` | `-r` | Node RPC endpoint | `http://localhost:9944` |
| `--metrics-url` | `-M` | Prometheus metrics endpoint | `http://localhost:9615/metrics` |
| `--keystore` | `-K` | Path to Substrate keystore directory | - |
| `--keys-file` | `-k` | Path to keys JSON file (alternative to keystore) | - |
| `--interval` | `-i` | Monitoring interval in seconds | `60` |
| `--verbose` | `-v` | Enable debug logging | `false` |
| `--once` | - | Run once and exit | `false` |

### Keys File Format

If using `--keys-file` instead of `--keystore`:

```json
{
  "sidechain_pub_key": "0x...",
  "aura_pub_key": "0x...",
  "grandpa_pub_key": "0x..."
}
```

## Output Example

```
INFO Starting Midnight Validator Monitor
INFO RPC endpoint: http://localhost:9944
INFO Metrics endpoint: http://localhost:9615/metrics
INFO Loaded validator keys from keystore
INFO   Sidechain: 0x037764d2d83c269030fef6df5aeb4419c48762ada2cf20b0e4e6ede596809f4700
INFO   Aura: 0xe05be3c28c72864efc49f4f12cb04f3bd6f20fdbc297501aa71f8590273b3e1e
INFO   Grandpa: 0xf5a39df9227f630754f78bbae43bd66a693612eeffa9ceec5681f6c05f48d0e8
INFO Node version: 0.12.0-29935d2f
INFO ─────────────────────────────────────────
INFO Health: ✓ | Syncing: ✓ | Peers: 12
INFO Block: 3349667 | Finalized: 3349665 | Sync: 100.00%
INFO Blocks produced: 1
INFO Sidechain: epoch 245624 slot 294749055 | Mainchain: epoch 1178 slot 101838307
INFO Keys: sidechain ? | aura ? | grandpa ?
INFO Registration: ✓ Registered (valid)
```

### Registration Status Types

| Status | Meaning |
|--------|---------|
| `✓ Permissioned candidate` | In the static permissioned validators list |
| `✓ Registered (valid)` | Dynamically registered with valid stake |
| `⚠ Registered but INVALID` | Registered but stake/signature validation failed |
| `✗ Not registered` | Not found in any registration list |
| `?` | Unable to check (RPC restricted) |

### Key Status

The `author_hasKey` RPC method is restricted on external connections, so key loading status shows `?`. Registration status is checked via `sidechain_getAriadneParameters`.

## Architecture

```
src/
├── main.rs      # CLI entry point and argument parsing
├── rpc.rs       # JSON-RPC 2.0 client
├── types.rs     # Response data structures
├── metrics.rs   # Prometheus metrics parser
├── monitor.rs   # Polling logic and display
└── keys.rs      # Keystore loading and registration checks
```

## RPC Methods Used

- `system_health` - Node health and peer count
- `system_version` - Node version
- `system_syncState` - Sync progress
- `chain_getHeader` - Current block header
- `chain_getFinalizedHead` - Finalized block hash
- `sidechain_getStatus` - Epoch/slot info
- `sidechain_getAriadneParameters` - Validator registration data

## Prometheus Metrics

- `substrate_proposer_block_constructed_count` - Blocks produced by this validator
- `substrate_block_height{status="best"}` - Current block height
- `substrate_block_height{status="finalized"}` - Finalized block height

## License

MIT
