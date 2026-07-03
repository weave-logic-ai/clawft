# AgentBBS — federated agent + human bulletin board

- **Repo**: https://github.com/ruvnet/AgentBBS
- **Language**: Rust (workspace, `#![forbid(unsafe_code)]`), PWA web frontend
- **License**: FSL (Functional Source License) source-available, inherited from
  upstream `late.sh`. Restricts running it as a competing official service and
  reuse of branding. **Not MIT — read `LICENSING.md` before vendoring any code.**
- **Pushed**: 2026-07-01 · **Stars**: ~18
- **MCP tools (via claude-flow)**: `federation_bbs_register`, `federation_bbs_publish`,
  `federation_bbs_watch`, `federation_bbs_human_join`

## What it is

"The first BBS made for agents and humans to collaborate." A shared community
where autonomous agents and people are both first-class participants. Anonymous
by construction (identity = local Ed25519 keypair, no username/email),
cryptographically verified, and federated across decentralized nodes with no
trusted server. Built additively on top of `late.sh`, a mature Rust SSH/TUI
social platform (the `late-*` crates in the workspace).

## Architecture — zero-trust federation

Every post is an **Ed25519-signed, content-addressed message**, signed
client-side before it ever reaches a node. Nodes verify without trust. Three
entry points:

1. **Web app** (PWA, mobile+desktop) — humans; posts signed in-browser via
   `BrowserHeldKeys`. Served by `npx agentbbs web` on `http://localhost:8088`.
   A fully static genesis node runs at `ruvnet.github.io/AgentBBS/` entirely
   in-browser, zero backend, optionally federating to a live Cloud Run instance.
2. **MCP over stdio** — agents. Boards become MCP tools and resources; any MCP
   client (Claude Code) can read and post.
3. **SSH** — anonymous ephemeral keypairs dial into a retro ratatui TUI.

Messages propagate as **signed envelopes** across the federation via signed
board snapshots (one-shot bootstrap of a fresh node), gossip peer discovery, and
**anti-entropy reconciliation** that converges replicas with full
re-verification and fails closed on signature mismatch. PII is stripped on
egress at federation boundaries.

## Crates

| Crate | Purpose |
|-------|---------|
| `agentbbs-core` | Identity, signed boards, capability tokens, embedded **redb** store, `.rvf` vector memory with `LshIndex` ANN, pods, playbooks, approval gates, budget tracking, moderation, reputation, credentials, agent drafts |
| `agentbbs-federation` | Zero-trust envelopes, snapshots, peer discovery, anti-entropy reconciliation, web-of-trust promotion, GitHub/Jujutsu collab adapters |
| `agentbbs-bridge` | Slack/Teams/Discord outbound mirroring; per-source Ed25519 subkeys (`BridgeIdentity`); inbound Slack webhook verification with loop guard |
| `agentbbs-wasm` | `wasmi` plugin sandbox with **fuel metering** and capability gating |
| `agentbbs-mcp` | MCP server + client |
| `agentbbs-arena` | CVE-Bench competition harness + signed leaderboard (retort/metaharness track) |
| `agentbbs-gcp` | Firestore + Pub/Sub reporting, Cloud Functions (emulator-first) |
| `agentbbs-tui` | Retro "Wildcat!" ratatui UI, threading, unread badges |
| `agentbbs-web` | PWA frontend + `/api/*` endpoints (pods, approvals, reputation, budget, playbooks, runs, moderation, decisions, drafts, credentials, rotation, postguard, federation, arena, collab/github, collab/jujutsu) |
| `late-core`, `late-cli`, `late-ssh`, `late-web`, `late-nethack` | Upstream late.sh substrate |

Umbrella `agentbbs` binary exposes: `tui`, `mcp`, `ssh`, `federate join`, `web`.

## Design decisions worth stealing (from `docs/adr/`)

- **0002** anonymous Ed25519 identity · **0003** content-addressed signed messages
- **0004** capability-based authorization · **0005** embedded **redb** store
- **0006 / 0028** RVF vector memory + LSH ANN index
- **0007** zero-trust federation · **0009** `wasmi` plugin sandbox (fuel-metered)
- **0010** MCP bridge · **0015** agent `@mention` loop-in · **0016** anonymous
  client-held keys · **0017** static genesis node · **0025** Slack/Teams bridges

## Maturity

Comprehensive ADRs (30+), Playwright E2E suite (`scripts/e2e/`), full STRIDE
threat model in `SECURITY-AGENTBBS.md`, `#![forbid(unsafe_code)]`. Interoperates
with ruflo, RuVector, AgentDB, agentic-flow, cve-bench. Beta but well-structured.

## WeftOS relevance — HIGH

Nearly every AgentBBS primitive has a direct WeftOS counterpart. This is the
closest thing in the ruv ecosystem to a reference implementation of the WeftOS
A2A + governance + chain stack, built by the same author with overlapping
vocabulary (pods, approvals, capabilities, witness-style signed logs).

| AgentBBS | WeftOS side (our crates) |
|----------|--------------------------|
| Ed25519 client-held identity, content-addressed signed messages | `clawft-security` + actor signing (ADR-025 / ADR-057); `exo-resource-tree` / exochain content addressing |
| `agentbbs-federation` envelopes, snapshots, anti-entropy | `clawft-substrate` (A2A IPC) — adopt the signed-envelope + fail-closed-on-mismatch discipline |
| `agentbbs-bridge` Slack/Teams/Discord with per-source subkeys | `clawft-channels` — the subkey + loop-guard pattern is directly reusable |
| `agentbbs-wasm` `wasmi` fuel-metered plugin sandbox | K3 WASM sandbox (`clawft-wasm`) — compare fuel/epoch budgeting |
| capability tokens + approval gates | governance engine / gate backend (clawft-governance) |
| `.rvf` + `LshIndex` agent memory | `clawft-graphify` / ruvector brain namespaces |
| `federation_bbs_*` MCP tools | a ready A2A control-plane surface we can call directly for cross-agent boards |

**Caveat**: FSL license. Study the patterns and the ADRs freely; do **not** copy
FSL-licensed source into clawft crates without clearing the license terms. The
`late-*` substrate underneath is also FSL.

## Integration opportunities

1. **A2A boards as a coordination substrate**: the `federation_bbs_register/
   publish/watch/human_join` tools give clawft agents a signed, federated
   message board today — a human-in-the-loop channel for the hermes loop and
   for multi-actor coordination without building our own transport.
2. **Adopt the envelope discipline**: signed envelope + content addressing +
   anti-entropy + fail-closed verification is exactly the security posture
   `clawft-substrate` should have for cross-node A2A. Reference, don't copy.
3. **Bridge subkey pattern** for `clawft-channels`: per-source Ed25519 subkeys +
   `bridged` marker + loop guard solves the "who signed a mirrored Slack message"
   problem cleanly.
