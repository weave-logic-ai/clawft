# ADR-071: WASM panel auth — per-panel token / capability model for the webview proxy

- **Status**: Accepted (2026-07-30)
- **Closes**: WEFT-495
- **Related**: WEFT-479 (daemon per-method capability gating), ADR-066
  (capability tokens / human-join), `extensions/vscode-weft-panel`,
  `crates/clawft-weave/src/capability.rs`, audit open question in
  `.planning/reviews/0.7.0-release-gate/15-mcp-integration.md`

## Context

The VSCode / Cursor webview panel reaches the kernel daemon through the
extension host's UDS proxy (`extensions/vscode-weft-panel/src/{extension,rpc}.ts`).
Today that path has:

1. A **static + runtime method allowlist** on the proxy (what methods the
   webview may name).
2. **Daemon-side capability classes** (WEFT-479) on the JSON-RPC `auth`
   field (`Read` / `Chat` / `Write` / `Admin`).
3. **No per-panel identity.** Every open panel shares the same local-user
   UDS trust. There is no token, capability set, or panel id on the proxy
   layer.

For single-user developer workstations this is acceptable: the filesystem
permissions on `kernel.sock` are the trust boundary. For multi-user /
multi-tenant kernels (shared host, shared daemon, multiple human operators
or agents opening panels) it is not — a panel must not inherit the full
caller surface of whoever owns the socket.

> Note on ADR-042: the WEFT-495 audit row referenced "ADR-042 modes" for
> multi-user. ADR-042 defines the *cognitive* modes (Act / Analyze /
> Generate), not tenancy. Multi-user here is a **deployment / tenancy
> mode** of the daemon + panel stack, orthogonal to ADR-042.

## Decision

### 1. Token-based per-panel identity (preferred over per-panel UDS)

Each webview panel session is issued a **panel session** by the extension
host when multi-user mode is active:

```
PanelSession
  ├─ panelId:   UUID                 (stable for the panel lifetime)
  ├─ token:     opaque secret        (random; never sent to webview)
  ├─ scopes:    CapScope[]           (subset of read|chat|write|admin)
  ├─ issuedAt:  ms epoch
  └─ expiresAt: ms epoch | null
```

The webview **never** holds the token. The extension host is the only
process that attaches `auth` on the wire (UDS JSON-RPC, same field as
WEFT-479). Compromised webview JS cannot escalate beyond the panel's
scoped token because it cannot mint or replace `auth`.

Wire format reuses the WEFT-479 literal scope tokens so the existing
daemon gate applies without a new AuthService path:

```
auth: "read,chat"          # default panel viewer
auth: "read,chat,write"    # elevated operator panel
auth: "admin"              # full (operator CLI / DaemonClient default on UDS)
```

Future work (not required by WEFT-495) may register `panel:<panelId>`
tokens with `AuthService` for revocation and ExoChain audit of issuance.

### 2. Capability scoping mirrors daemon classes

| CapScope | Matches daemon `Capability` | Typical panel methods |
|----------|-----------------------------|------------------------|
| `read`   | `Read`  | `kernel.status`, `substrate.read`, `mcp.list`, … |
| `chat`   | `Chat`  | `agent.chat`, `agent.chat_stream`, `llm.prompt` |
| `write`  | `Write` | `terminal.*`, `control.set_enabled`, `cron.add`, … |
| `admin`  | `Admin` | `kernel.kill-process`, `kernel.restart-service`, … |

Default multi-user panel scopes: **`read` + `chat`** (viewer / concierge).
Write and Admin require an explicitly elevated session (operator UX or
config), not the default webview open path.

Proxy enforcement order (multi-user only):

1. Method on runtime allowlist? else deny (`method not allowed`).
2. Panel session present and unexpired? else deny (`panel identity required`).
3. Session scopes cover the method's required CapScope? else deny
   (`permission denied: panel scopes … lack …`) — **denied-by-identity**.
4. Attach `auth` from session scopes and forward to UDS.
5. Daemon re-checks via WEFT-479 (defense in depth).

### 3. Gated behind multi-user mode

| Mode | How enabled | Proxy behaviour |
|------|-------------|-----------------|
| **Single-user (default)** | unset / false | Unchanged 0.7 posture: allowlist only; no `auth` forced on wire; daemon treats absent auth as anonymous `{Read, Chat}`. |
| **Multi-user** | `WEFTOS_MULTI_USER=1` / `true`, or VSCode setting `weft.multiUser: true` | Issue `PanelSession` per panel; enforce scopes; attach `auth` on every proxied RPC. |

Single-user remains the default for local dev workstations. Multi-user
does not change UDS socket ownership; it adds a second factor *inside*
the already-connected extension host so multiple panels cannot silently
share a write-capable identity.

### 4. Alternatives considered

| Option | Why not chosen |
|--------|----------------|
| Per-panel UDS path (`kernel.panel.<id>.sock`) | Heavy: daemon must mint + chmod sockets, clean up on dispose; still needs a scope model on each socket. Token reuses existing gate. |
| Pass token into webview | Webview is untrusted JS; token would be stealable via XSS / extension bugs. Host-held token is the binding. |
| Always-on (even single-user) | Breaks 0.7 zero-config DX; ticket explicitly defers multi-user. Gate keeps single-user path unchanged. |

## Consequences

### Positive

- Multi-tenant / multi-operator deployments can open panels without each
  panel inheriting full Write/Admin of the socket owner.
- Reuses WEFT-479 wire `auth` and capability classes — one mental model.
- Denied-by-identity is testable without a live daemon (pure proxy unit).

### Negative / residual

- AuthService-backed panel tokens (issuance, revoke, chain audit) remain
  a follow-up; multi-user currently uses literal scope tokens.
- Kernel config flag `kernel.multi_user` is not yet plumbed; mode is
  env / VSCode-setting only so the proxy can decide without a daemon
  round-trip on open.
- Elevated Write panels still share the same UDS; OS-level isolation of
  multi-user kernels is out of scope.

### Neutral

- Extension allowlist stays advisory relative to the daemon; multi-user
  adds a *second* advisory-to-enforce step keyed on panel identity.
- ADR-066 capability tokens remain the long-term mesh/object-scoped
  primitive; this ADR is the panel-proxy specialization of the same
  four-class model.

## Implementation map

| Component | Location |
|-----------|----------|
| Mode detection + session + authorize | `extensions/vscode-weft-panel/src/panelAuth.ts` |
| Proxy attach + deny responses | `extensions/vscode-weft-panel/src/extension.ts` |
| Wire `auth` on UDS JSON-RPC | `extensions/vscode-weft-panel/src/rpc.ts` |
| Daemon re-check | `crates/clawft-weave/src/{capability,daemon}.rs` (WEFT-479, unchanged) |
| Unit tests (deny-by-identity) | `extensions/vscode-weft-panel/test/suite/panelAuth.test.ts` |
