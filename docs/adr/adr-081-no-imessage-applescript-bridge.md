# ADR-081: No first-party iMessage AppleScript channel (0.8.x / indefinite)

- **Status**: Accepted (2026-07-31)
- **Closes**: WEFT-175
- **Related**: Element 06 E4 (Signal only in tree),
  `.planning/reviews/0.7.0-release-gate/05-channels.md` (orphaned `imessage`),
  `.planning/sparc/phase4/06-channel-enhancements/00-orchestrator.md`,
  `docs/guides/channels.md`, ADR-065 (channel bridge identity / loop guard)

## Context

SPARC Element 06 E4 was described as a paired **Signal / iMessage** consumer
channel. Signal landed as a feature-gated adapter under
`crates/clawft-channels/src/signal/` (subprocess / `signal-cli` path). An
`imessage` tree was never created:

- No `crates/clawft-channels/src/imessage/`
- No `imessage` Cargo feature
- No factory registration
- Channels guide lists eleven platforms; iMessage is not among them

The 0.7.0 release-gate audit flagged this as **orphaned scope** (Task List
#19 → Plane **WEFT-175**): either implement a macOS-only AppleScript /
`Messages.app` bridge, or formally drop it with recorded rationale.

Ticket notes already preferred drop for 0.8.x: long-horizon, macOS-only,
low priority, blocked by macOS test capacity.

## Decision

**Do not implement a first-party iMessage AppleScript (or Shortcuts /
Messages.app) channel in 0.8.x.** Treat iMessage as **out of product
scope** for first-party `clawft-channels` until a future cycle reopens it
with an explicit design (not as a silent E4 leftover).

Concretely:

| Surface | Policy |
|---------|--------|
| `clawft-channels` | No `imessage` module, feature flag, or factory |
| Element 06 E4 | **Signal only** — iMessage is not part of E4 completion |
| Docs / trackers | Orphan callouts removed; this ADR is the rationale |
| Future work | Reopen only via a new Plane item (0.9.x+) with design + macOS CI story |

This is a **formal drop of first-party scope**, not a deferral that keeps
WEFT-175 open. Reintroduction requires new acceptance criteria, not
reopening the orphan ticket by default.

## Rationale

1. **Platform asymmetry.** An AppleScript / `Messages.app` bridge is
   macOS GUI-session–bound. WeftOS prioritizes headless, multi-OS, and
   daemon-first channels (Telegram, Slack, Discord, web gateway). A
   login-session automation path does not match the shipping channel
   contract (`Channel` / `ChannelAdapter` + `PluginHost` lifecycle).

2. **Support and CI cost.** Reliable tests need a signed-in Messages
   session, Automation/Accessibility permissions, and non-flaky UI
   automation. The project does not have that capacity on the 0.8.x
   critical path; Signal already covers a similar “local bridge”
   product niche with a subprocess model that is CI-testable via mock
   TCP.

3. **Security surface.** Driving `Messages.app` via AppleScript expands
   the trust boundary (Automation entitlements, contact/PII access,
   injection into script arguments). ADR-065-style bridge identity and
   loop-guard work applies better to documented network/subprocess
   bridges than to opaque UI scripting.

4. **Honest product surface.** Shipping stubs for seven other channels
   already requires careful “not production” labeling. Adding another
   macOS-only stub (or half-bridge) increases foot-guns without
   unblocking 0.8 ship goals. Prefer **no module** over a lying stub.

5. **E4 is already Signal-complete for tracking purposes.** The Element
   06 tracker records E4 as Signal runtime; iMessage never had files.
   Recording the drop closes the audit orphan without rewriting history
   of draft SPARC docs.

## Alternatives considered

| Option | Why rejected (for 0.8.x) |
|--------|-------------------------|
| **Implement AppleScript bridge** | macOS-only, weak CI, high ops cost; not on 0.8 publish path |
| **Land trait stub only** (`imessage` feature + synthetic `send`) | Same foot-gun class as other stubs, zero user value, keeps orphan alive as “half done” |
| **Defer to 0.9.x without closing** | Leaves WEFT-175 / Task #19 open with no design; preferred path was formal drop |
| **Third-party / skill-only later** | Acceptable *future* path; not a first-party channel commitment today |

## Implications

- **Code**: no change required — there is no `imessage` tree to delete.
- **Trackers**: WEFT-175 closed as decision-done (drop); audit Task #19
  marked resolved by ADR-081; Element 06 orchestrator wording updated so
  E4 is Signal-only.
- **Guides**: `docs/guides/channels.md` states iMessage is intentionally
  not a first-party channel (see this ADR).
- **Revisit criteria** (if ever): documented transport (not only UI
  script), identity/loop-guard plan (ADR-065 class), and a macOS
  automation test strategy before any feature flag lands.

## Consequences

### Positive

- Closes a multi-release audit orphan with an explicit decision.
- Avoids macOS-only debt and Automation permission matrix for 0.8.x.
- Keeps channel matrix honest: listed platforms match crates on disk.

### Negative

- Apple-ecosystem users cannot talk to the agent via iMessage without a
  third-party bridge or a future first-party design.
- Draft SPARC docs that still say “Signal / iMessage” are historical;
  live trackers and guides supersede them.

### Neutral

- Signal remains the supported local/subprocess consumer messaging path
  under E4.
- Community plugins may still implement external bridges outside
  `clawft-channels` without contradicting this ADR.
