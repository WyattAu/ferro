# Ferro Mesh Sync — LAN/WAN Peer Block Exchange

## Architecture

```
LAN:  peer A ←──HTTP blocks──→ peer B     (direct, ephemeral port 7878)
WAN:  peer A ──→ Ferro server ←── peer B  (relay via block CAS)
       └────────── mesh index ──────────┘
```

## How it works

1. After each sync cycle, the desktop client registers with the server's
   peer index (`POST /api/sync/peers`): device name, reachable LAN IPs,
   listener port, and block hashes it holds.
2. On the next cycle, before uploading blocks to the server, the client
   queries `GET /api/sync/peers/blocks?hash=<hash>` to find LAN peers that
   already hold a block — and fetches it directly instead of the WAN.
3. A lightweight HTTP listener on port 7878 serves `GET /blocks/{hash}`
   from the in-memory block cache populated during upload.

## WAN peers

For remote/home users, wrap the mesh in WireGuard via your existing
headscale instance:

```bash
# On the headscale server:
headscale nodes create --user wyatt --key <machine-key>

# On each desktop peer:
tailscale up --login-server https://headscale.wyattau.com

# The peer's tailnet IP (100.x.y.z) is added to mesh addresses automatically
# when the LAN-discovery fallback finds no non-loopback addresses.
```

## Security

- Peers authenticate with the same Bearer token as all other API calls.
- Block exchange is hash-verified (SHA-256, matching the CAS store).
- Peers only discover other users who share a space with them (Cedar-gated).
- The block listener binds to `0.0.0.0` but only serves content for hashes
  that were explicitly uploaded — no directory traversal, no auth bypass.

## Future (Phase 3)

- Native QUIC transport (quinn crate) instead of HTTP-over-TCP
- WireGuard automatic key exchange via headscale API
- Block-level delta compression (delta-kit) before transfer
