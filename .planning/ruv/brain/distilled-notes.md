# Distilled notes from ruvnet-brain (verified)

Highest-value content for WeftOS active integrations, distilled from ruvnet-brain
with per-claim provenance. **Provenance format**: `[brain: <file>]` = where the
claim came from; `[verified: <primary>]` = checked against primary source on
2026-07-03; `[unverified]` = from the brain, not yet independently confirmed —
treat as a lead, not a fact.

Focus is on the integrations where the brain is **deeper than our current
catalog** (rvm, RuLake, agentdb) plus the spot-checked corroborations. For
agenticow / AgentBBS / midstream, our own `packages/*` deep-dives remain the
authority and are not duplicated here.

---

## rvm (ruvix) — proof-gated microhypervisor → K3 sandbox / exochain capabilities

The brain's rvm capability card is the richest short description of rvm anywhere in
our catalog. **All key claims verified against `ruvnet/rvm` README.**

- rvm is a **capability-secure microhypervisor** bringing seL4/CHERI-style
  formally-verified-OS security to an AI vector-and-agent substrate: every
  privileged state mutation needs an **unforgeable capability token AND a
  verifiable proof** recorded in an **append-only witness log**.
  `[brain: kb/capability-cards.md]` `[verified: ruvnet/rvm README]`
- **Seven rights**: `READ, WRITE, GRANT, REVOKE, EXECUTE, PROVE, GRANT_ONCE`.
  `[brain: capability-cards.md]` `[verified: rvm README — exact]`
- **seL4-style derivation tree**: mint / derive with **monotonic attenuation**
  (a derived cap has ≤ its parent's rights) / epoch-based revoke; **max delegation
  depth 8**. `[brain: capability-cards.md]` `[verified: rvm README — "depth bounded at 8", DC-5]`
- **Three-tier proof system**:
  - **P1** — hash capability check, **<1 µs** on the syscall hot path (README:
    ~17 ns measured), **ships**.
  - **P2** — witness-chain policy validation, **<100 µs**, constant-time.
  - **P3** — deep / zero-knowledge derivation-chain proof, **<10 ms**, accepted
    but partly deferred.
  `[brain: capability-cards.md]` `[verified: rvm README — P1<1µs/P2<100µs/P3]`
- Source pinned in the brain at commit `af97d18`. `[brain: kb/SOURCE.json]`

**WeftOS mapping**: this is the closest external prior art to the WeftOS exochain
capability model — proof-gated mutation + witness log + capability derivation maps
onto `crates/exo-resource-tree/` and the K3 sandbox capability story. The
seven-rights + monotonic-attenuation + depth-bound design is a concrete reference
for the WeftOS capability-token scheme (compare against ADR-025/057 actor identity
and the gate backend). **Study, adapt — do not vendor** (it is a full runtime, not
a library drop-in).

---

## RuLake — witness-anchored vector cache → receipt-pinned LLM cache

The brain's rulake primer is **ADR-by-ADR deeper** than our
`new-libraries-2026-07.md` survey line. Verified against `ruvnet/RuLake` README.

- ruLake is a **witness-anchored vector federation / cache** layer that sits in
  front of a vector store, giving deterministic retrieval + provenance.
  `[brain: kb/rulake-primer.md]` `[verified: RuLake README]`
- **Cryptographic receipt on every answer** — each result is fingerprinted;
  tampered or stale data is rejected; an answer can be **verified locally, no
  server**. `[brain: rulake-primer.md]` `[verified: RuLake README — exact]`
- Search uses **RaBitQ** (random binary quantization, 1-bit compression kernel,
  L2² scoring). Nuance: the primer calls RaBitQ the "primary search mechanism";
  the README frames it as a **companion kernel** (`ruvector-rabitq`) in the
  RuVector ecosystem — minor framing drift, not a factual error.
  `[brain: rulake-primer.md]` `[verified: RuLake README — companion kernel]`
- **MCP server with capability-gated tools** — two-layer enforcement (route
  filtering + per-handler `effective_caps` checks), read/publish/admin tiers,
  stdio + HTTP transports (OAuth, mTLS, replay protection), audit logging to JSONL.
  `[brain: rulake-primer.md]` `[verified: RuLake README — 18 tools, capability tiers, ADR-009]`
- Substrates: rvDNA (genomics) and ruQu (quantum) plug in as ruLake backends;
  IPFS + GCS storage adapters; Iceberg/Delta/BigQuery adapters roadmapped.
  `[brain: rulake-primer.md]` `[unverified — from primer's ADR citations]`

**WeftOS mapping**: the receipt-pinned "refuse to guess when the data changed"
semantics is the pattern for a WeftOS LLM/answer cache with witness verification —
pairs naturally with our exochain witness chain. The capability-gated MCP server
(default-deny, tiered, two-layer) is a reference for the WeftOS gate backend's
MCP surface.

---

## agentdb — cognitive memory backend

Brain agentdb primer is deeper than our survey line. Not independently re-verified
end-to-end (agentdb lives inside the `agentic-flow` monorepo); treat performance
numbers as `[unverified]` leads.

- AgentDB = **hybrid vector + graph cognitive DB** for agent memory: vector
  embeddings + n-ary hyperedges + **explainable recall** (feature attributions:
  "why did I recall that?"). `[brain: kb/agentdb-primer.md]` `[unverified]`
- **Multi-backend with automatic fallback**: RuVector (claimed 150× faster) or
  HNSWLib, unified `VectorBackend` interface; SQLite (WASM/better-sqlite3) for
  relational side; default **384-dim** embeddings (MiniLM) — matches WeftOS
  `weave.toml [embedding] dimensions = 384`. `[brain: agentdb-primer.md]` `[unverified]`
- **Controller architecture**: `ReflexionMemory`, `SkillLibrary`,
  `CausalMemoryGraph`, `EmbeddingService`, with declared safety levels
  (`pure`, `opens-resource`, …). `[brain: agentdb-primer.md]` `[unverified]`
- **RVF backend** integration (20+ methods: `embedKernel`, `extractKernel`,
  `verifyWitness`) — same RVF format WeftOS uses. `[brain: agentdb-primer.md]` `[unverified]`
- Ships an MCP server (stdio) exposing vector ops. `[brain: agentdb-primer.md]` `[unverified]`

**WeftOS mapping**: the 384-dim MiniLM default and RVF backend make agentdb
storage-compatible with the WeftOS ruvector brain; the CausalMemoryGraph +
explainable-recall pattern parallels our own ECC CausalGraph. Candidate reference
for the memory backend, **verify perf claims before relying on them**.

---

## Spot-check corroborations (our catalog already covers these deeper)

Recorded for the audit trail; our `packages/*` remain authoritative.

- **agenticow** — brain confirms ~0.5 ms / 162 B COW branch over RVF, exact
  read-through (child wins, tombstone-masked), rollback ~0.57 ms p50, 1,000-branch
  acceptance at recall@10 = 100%. Brain **understates** the headline speedup as
  "83×"; the real README (and our overview) say **142×** @1M vectors.
  `[brain: kb/agenticow-primer.md]` `[verified: ruvnet/agenticow README — 472µs@1M, 142×]`
- **midstream** — brain has only a registry one-liner. Primary README: 6 Rust
  crates (`temporal-compare` DTW/LCS/edit-distance, `scheduler` ns-priority-queue,
  `attractor` Lyapunov/phase-space, `neural-solver` LTL+neural, `strange-loop`
  meta-learning, `quic` 0-RTT). `[brain: data/ruvnet-registry.json]` `[verified: ruvnet/midstream README]`
- **ruflo star count** — registry snapshot 61,698 ★ (2026-06-27) matches the real
  ~62.8k ★ today; **not inflated**. `[brain: data/ruvnet-registry.json]` `[verified: ruvnet/ruflo]`

---

## metaharness (agent-harness-generator)

- "metaharness" is confirmed as the alias for **agent-harness-generator**: a
  factory toolchain that scaffolds host-agnostic agent harnesses (Claude Code,
  Codex, pi.dev, other MCP hosts), **scores** repos for harness-fit and harnesses
  for readiness/safety, A/B tests variants, and self-evolves them under fixed
  safety rails (**Darwin Mode**). `[brain: kb/capability-cards.md,
  agent-harness-generator-primer.md]` `[unverified — alias & scope from brain; consistent with our new-libraries survey]`

**WeftOS mapping**: harness-fit scoring + default-deny MCP policy + `mcp-scan`
tool auditing are governance/gate reference patterns; see
`packages/new-libraries-2026-07.md`.
