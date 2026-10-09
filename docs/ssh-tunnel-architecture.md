# SSH Tunnel Architecture (SqlKit)

> 测试环境搭建见 [ssh-tunnel-testing.md](ssh-tunnel-testing.md)。

SqlKit's SSH tunnel is aligned with DocKit's implementation (see
`geek-fun/dockit` `docs/ssh-tunnel-architecture.md`). The backend is the
same `russh`-based engine; this document maps it onto SqlKit's connection
model and records SqlKit-specific capabilities.

## Capability Matrix (aligned with dockit)

| Capability | Status | Notes |
|---|---|---|
| Password / private key / SSH agent auth | ✅ | agent: Unix socket + Windows Pageant |
| Multi-hop chains (ordered hops) | ✅ | `TunnelManager.start_chain` — hop N connects to hop N+1, last hop forwards to the database |
| Keepalive + idle health ping | ✅ | 30s SSH keepalive, 10s idle ping timeout |
| Reconnect with backoff | ✅ | 5s → 60s exponential, max 10 attempts |
| Deterministic tunnel keys | ✅ | `tunnel_key()` in `commands/helpers.rs` — same hops + same target reuse one tunnel across connections |
| `~/.ssh/config` import | ✅ | `ssh/ssh_config.rs` parser + `list_ssh_config_hosts` command + UI import |
| SSH profiles (stored, reusable) | ✅ | `.store.dat` key `sshProfiles` + CRUD commands + management UI |
| System proxy for the first hop | ✅ | `detect_system_proxy` + `use_system_proxy` on the hop config (HTTP CONNECT) |
| SOCKS5 / HTTP CONNECT local servers | ✅ | `socks5.rs` / `http_proxy.rs` — `expose_lan = false` selects SOCKS5/CONNECT mode over PortForward |
| Host-key TOFU verification | ✅ (SqlKit-first) | `ssh/known_hosts.rs` — first connection pins the fingerprint, mismatches reject; `unpin` allows recovery. DocKit pending |
| Ultimate entitlement gate | ✅ | `ensure_local_ultimate_global("SSH tunnel")` before any tunnel starts |

## Connection Flow

```
ConnectionConfig.transport_layers (ordered, frontend-expanded)
        │
        ▼  commands/helpers.rs :: connection_host_port
   file-based DB? ── yes ──► direct (host, port)
        │ no
        ▼
   ensure_local_ultimate_global("SSH tunnel")   ← 403 gate
        │
        ▼
   tunnel_key(layers, host, port)               ← deterministic reuse key
        │
        ▼  ssh/transport.rs :: start_transport_layers
   resolve_ssh_tunnel_config per layer          ← ~/.ssh/config alias resolution
        │
        ├─ 1 layer ──► TunnelManager.start_tunnel
        └─ n layers ─► TunnelManager.start_chain (hop N → hop N+1)
```

## Field Model

`ConnectionConfig.transport_layers` carries fully-expanded hop configs
(frontend expands `sshTunnel.profileIds` through the profile store). Each
`SshTunnelConfig` supports `use_system_proxy` (HTTP CONNECT to the bastion
through the OS proxy) and `expose_lan` (bind the local listener beyond
loopback / select SOCKS5 mode).

Legacy saved connections with the old flat `sshTunnel` shape keep working:
the frontend expands them exactly as before.

## Known Limitations

1. **Host key verification requires `verify_host_key: true`** — the default
   remains lenient (accept-any) for approachability. With the flag on, TOFU
   protects against passive-to-active tampering after first use.
2. **TLS over tunnel + SNI** — when the database enforces TLS with a real
   certificate, the tunnel rewrites the connection target to
   `127.0.0.1:{local_port}`; rustls then uses `127.0.0.1` for SNI/cert
   validation, which fails against domain-issued certificates (same class as
   dockit issue #472). Affected: `ssl = require`-style setups. Workaround
   today: `ssl = disable` inside trusted tunnels, or OS-level tunneling.
3. **Passwords at rest** — connection credentials (including the SSH hop
   password) are stored with the connection config. OS keychain integration
   is planned (dockit shares this limitation).
4. **No proxy chaining beyond the first hop** — the system proxy applies to
   the first hop only (later hops connect from the previous bastion).
