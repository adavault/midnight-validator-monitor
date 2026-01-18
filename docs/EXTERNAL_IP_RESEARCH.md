# External IP Display Research

This document captures research findings on external IP address detection in Substrate nodes.

## The Problem

The TUI displays external IP from `system_unstable_networkState.externalAddresses`, which returns ALL addresses the node knows about. In testing, this returned 24+ addresses including:

- **Configured external address (correct)**: `203.0.113.10:59357` (from `--public-addr` flag)
- **Peer-reported addresses (noise)**: `198.51.100.50:30333`, many `10.14.x.x` overlay addresses
- The `157.x.x.x` address is a Hetzner server in Helsinki (AS24940) - likely Midnight relay infrastructure

## Why This Happens

libp2p's identify protocol allows peers to tell your node "this is how I see you". All these addresses accumulate in `externalAddresses`:

1. **Configured addresses**: From `--public-addr` command line flag
2. **NAT-discovered addresses**: Via UPnP/NAT-PMP (if enabled)
3. **Peer-reported addresses**: Other nodes telling this node how they see it

The peer-reported addresses are often incorrect because they may see you through:
- Relay servers
- Load balancers
- Overlay networks

## Available RPC Endpoints

| Endpoint | Returns |
|----------|---------|
| `system_localListenAddresses` | Only local addresses (127.0.0.1, docker IP) |
| `system_unstable_networkState.listenedAddresses` | Same as above |
| `system_unstable_networkState.externalAddresses` | ALL addresses (configured + peer-reported) |
| `system_localPeerId` | Just the peer ID |

**Key Finding**: No RPC endpoint distinguishes between configured vs discovered addresses.

## Docker Node Logs Show Correct Address

The node logs the correctly configured address at startup:

```
Discovered new external address for our node: /ip4/203.0.113.10/tcp/59357/p2p/12D3KooW...
```

The node knows the right address internally but doesn't expose which addresses were explicitly configured via any RPC endpoint.

## Solution

Use the standard P2P port (30333) consistently:

1. **Docker port mapping**: Use standard port
   ```yaml
   ports:
     - "30333:30333"
   ```

2. **Node configuration**: Set `--public-addr` with port 30333
   ```
   --public-addr /ip4/<YOUR_PUBLIC_IP>/tcp/30333
   ```

3. **Firewall/NAT**: Forward external port 30333 to the node

This way, both the configured address and peer-reported addresses use port 30333, resulting in consistent display.

## Filtering Relay Addresses

Even with correct port configuration, peers may report relay/infrastructure IPs (e.g., Hetzner nodes). To filter these out, configure your expected IP in `mvm.toml`:

```toml
[view]
expected_ip = "203.0.113.10"
```

Or via environment variable:
```bash
MVM_EXPECTED_IP="203.0.113.10" mvm view
```

Only addresses matching this IP will be displayed, filtering out relay addresses.

## Future Investigation Areas

1. **Substrate Enhancement**: Investigate if Substrate/libp2p could expose a separate list of explicitly configured addresses vs discovered ones

2. **Log Parsing**: Consider parsing node startup logs to find the configured `--public-addr` value (would require log access and is fragile)

3. **libp2p Address Scoring**: libp2p scores addresses by confidence - investigate if this scoring could help distinguish configured from discovered

4. **Multiaddr Priority**: The libp2p identify protocol may have priority metadata that could help

## References

- libp2p identify protocol: https://github.com/libp2p/specs/blob/master/identify/README.md
- Substrate network configuration: https://docs.substrate.io/reference/command-line-tools/node-template/#network-options
