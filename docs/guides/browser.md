# Browser Node Guide

Browser nodes connect to a WeftOS mesh over WebSocket and run in an
untrusted client environment (user-inspectable JS, session-scoped
keys, no raw TCP). They are intentionally second-class by default
(symposium decision **S7**): restricted IPC, no spawn, no network,
and governance `browser_policy` rules that cannot be bypassed by
lowering effect magnitude alone.

For WASM packaging and the client feature-flag split, see also
[`docs/browser/`](../browser/) (architecture, building, deployment).

---

## Default capabilities

Browser-platform agents receive
[`AgentCapabilities::browser_default()`](../../crates/clawft-kernel/src/capability.rs):

| Capability | Browser default |
|------------|-----------------|
| `ipc_scope` | `IpcScope::Restricted([])` |
| `can_spawn` | `false` |
| `can_network` | `false` |
| `can_ipc` | `true` (needed for kernel/mesh control) |
| `can_exec_tools` | `true` (still gated by tool permissions / sandbox) |
| Memory / CPU / messages | Tighter than native defaults (64 MiB / 60 s / 500 msgs) |

`IpcScope::Restricted` means:

1. **Direct PID messaging** — only allow-listed PIDs (empty list ⇒ none).
2. **Topics** — only [public topics](#public-topics) (WEFT-108).

Elevation above these defaults requires a governance-gated capability
elevation request (`AgentCapabilities::request_elevation` /
`request_elevation_gated`).

---

## Public topics

A topic is **public** when it is exactly `public` or starts with the
`public.` prefix:

| Topic | Public? |
|-------|---------|
| `public` | yes |
| `public.events` | yes |
| `public.health.status` | yes |
| `admin.secrets` | no |
| `mesh.internal` | no |
| `chain.append` | no |

Enforcement is dual-layer:

1. **Capability layer** — `AgentCapabilities::can_topic` / `CapabilityChecker::check_ipc_topic`
   for any agent with `IpcScope::Restricted`.
2. **Governance layer** — `browser_policy` rules (see below) when the
   request context includes `platform=browser`.

---

## `browser_policy` rule type

Governance rules carry a `rule_type` discriminator:

| `GovernanceRuleType` | Evaluation |
|----------------------|------------|
| `General` (default) | Effect-magnitude threshold path |
| `BrowserPolicy` | Platform-aware checks independent of magnitude |

Default browser_policy rules (anchored at governance genesis as
**BP-001** … **BP-003**):

| ID | Constraint |
|----|------------|
| `BP-001` | Browser nodes may only publish/subscribe to public topics |
| `BP-002` | Browser nodes cannot spawn children without elevation |
| `BP-003` | Browser nodes cannot enable network without elevation |

Helpers:

```rust
use clawft_kernel::governance::{
    browser_policy_default_rules, GovernanceEngine, GovernanceRequest,
    GovernanceRule, GovernanceRuleType,
};

// Install the three default BP rules
let mut engine = GovernanceEngine::new(0.7, false);
for rule in browser_policy_default_rules() {
    engine.add_rule(rule);
}

// Or construct a custom browser_policy rule
engine.add_rule(GovernanceRule::browser_policy(
    "BP-CUSTOM",
    "Custom browser restriction",
));

// Evaluate a browser topic subscribe
let decision = engine.evaluate(
    &GovernanceRequest::new("browser-agent", "ipc.topic.subscribe")
        .with_context_entry("platform", "browser")
        .with_context_entry("topic", "admin.secrets"),
);
// => Deny — non-public topic
```

Request context keys used by browser_policy:

| Key | Purpose |
|-----|---------|
| `platform` | Must be `"browser"` for rules to apply |
| `topic` | Topic name for subscribe/publish actions |
| `can_spawn` / `can_network` | Elevation request flags |

Actions matched for topic policy include `ipc.topic.*`,
`ipc.subscribe` / `ipc.publish`, and `topic.subscribe` /
`topic.publish`.

---

## Operator notes

- Cluster operators may grant broader capabilities to browser nodes by
  superseding genesis rules (`governance.root.supersede`) and approving
  capability elevation — the defaults are security-first, not absolute.
- Browser nodes still use the same kernel gate stack (`GovernanceGate`,
  `CapabilityGate`); browser_policy is an additional evaluation path,
  not a separate process.
- Chain audit: genesis payloads include `rule_type` for each rule so
  BP rules are distinguishable from general constitutional rules.

---

## Related docs

- Security model: [`docs/weftos/k5-symposium/03-security-and-identity.md`](../weftos/k5-symposium/03-security-and-identity.md) §6
- Kernel governance API: [`docs/weftos/kernel-governance.md`](../weftos/kernel-governance.md)
- WASM browser packaging: [`docs/browser/architecture.md`](../browser/architecture.md)
- Operator kernel guide: [`docs/guides/kernel.md`](kernel.md)
