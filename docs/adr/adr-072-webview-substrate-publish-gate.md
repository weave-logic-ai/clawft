# ADR-072: Webview vs daemon substrate write boundary

- **Status**: Accepted (2026-07-30)
- **Closes**: WEFT-496
- **Related**: WEFT-250 (daemon allowlist refresh), WEFT-253 (`agent.chat_stream`),
  ADR-057 (substrate read ACLs), chat-agent-v1 plan decision #1,
  `extensions/vscode-weft-panel/src/{extension,allowlist}.ts`

## Context

The VSCode / Cursor panel proxies webview `postMessage` RPC to the local
daemon UDS through an allowlist (`STATIC_ALLOWED_METHODS`, runtime-merged
with `daemon.list_methods` via WEFT-250). A comment claimed
"`substrate.publish` stays blocked; the webview is a viewer, not a writer."

That wording is wrong in two ways:

1. **The panel already mutates.** Allowed methods include `agent.chat`,
   `terminal.*`, `control.set_enabled`, `cron.add`, `service.start`,
   `kernel.kill-process`, etc. The webview is not read-only.
2. **`agent.chat` causes daemon-side substrate writes.** Conversation
   turns, stream frames, soul-journal entries, and routing logs land under
   `substrate/_derived/…` via `publish_gated(_with_grants)`. Post-D3 the
   concierge tool registry is the full `clawft_tools::register_all` surface
   (not just `read_file` / `list_directory`), but those tools mutate
   workspace FS / memory file / subagent registry under the governance
   gate — they do **not** call `substrate.publish` as a tool.

WEFT-250's refresh *unions* daemon-advertised methods into the runtime
allowlist. Without a hard denylist, a daemon that lists `substrate.publish`
would re-open the raw write verb the static seed deliberately omitted.

## Decision

**Mediated mutators yes; raw substrate pen no.**

| Layer | May write substrate? | Mechanism |
|-------|----------------------|-----------|
| Webview → proxy | **No** raw `substrate.publish` | `WEBVIEW_DENIED_METHODS` always wins over allowlist / WEFT-250 union |
| Webview → high-level RPC | Side effects OK | `agent.chat`, `terminal.*`, `control.*`, … stay allowed |
| Daemon sinks / services | **Yes**, grant-gated | `publish_gated` / `publish_gated_with_grants` under `_derived/` topics |
| Agent tools in `agent.chat` | Workspace / registry, not raw substrate | `clawft_tools::register_all` + `GovernanceGate` + workspace sandbox |

### Invariant (replaces "viewer only")

> The webview may invoke intentional high-level mutators. It must never
> hold a raw `substrate.publish` pen. Daemon-mediated substrate writes
> triggered by those mutators are grant-gated under `_derived/` and are
> not a bypass of this rule.

### Implementation

- `extensions/vscode-weft-panel/src/allowlist.ts` — pure helpers
  (`isMethodAllowed`, `mergeAllowlist`) + `WEBVIEW_DENIED_METHODS`
  (`substrate.publish` today; expandable).
- `extension.ts` `handleRpc` / `refreshAllowlist` use those helpers.
- Denylist error string: `method denied for webview: …` (distinct from
  `method not allowed: …`).

## Rationale

1. **Unprivileged WASM must not attribute node-scoped writes.**
   `substrate.publish` requires node id + signature and owns the mesh
   write boundary (WEFT-433 prefix gate). Letting the panel forge writes
   collapses that boundary to "whatever the local webview posts."
2. **High-level mutators are the product.** Chat, terminal, and control
   plane are why the panel exists; tightening them to pure reads would
   delete the product. Daemon-side gates (governance, session ownership,
   derived-write grants) are the correct enforcement layer for their
   side effects.
3. **WEFT-250 union needs a floor.** Introspection refresh is valuable
   against allowlist drift; the denylist is the non-negotiable floor that
   refresh must not erase.
4. **Do not shrink the agent tool surface for this ticket.** D3 expanded
   tools beyond the spike pair; that is intentional (chat-agent-v1 §2
   decision 3). Substrate integrity is protected by *not* exposing
   `substrate.publish` as a tool and by grant-gated sinks — not by
   reverting tools to read-only.

## Consequences

- Positive: invariant matches reality; regression covered by pure unit
  tests; audit open question closed with WEFT-496 / ADR-072.
- Negative: denylist is hand-maintained; new raw-write RPCs must be
  added there if they must stay webview-inaccessible.
- Follow-up: daemon-side per-caller capability tags (audit open Q on
  webview vs daemon allowlist semantics) remain desirable so UDS clients
  other than the panel share the same floor.

## Alternatives considered

| Option | Why rejected |
|--------|----------------|
| Keep "viewer only" and remove write RPCs from allowlist | Breaks chat, terminal, control, scheduler, services |
| Tighten agent.chat tools back to `read_file`/`list_directory` | Wrong layer; product needs write tools; substrate not at risk via tools |
| Rely only on static omission of `substrate.publish` | WEFT-250 union re-opens it if the daemon advertises the method |
| Move enforcement solely into daemon capability table | Still needed long-term, but does not replace the proxy floor for untrusted webview code today |
