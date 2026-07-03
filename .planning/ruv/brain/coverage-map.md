# ruvnet-brain coverage map

Maps ruvnet-brain's topics to our `.planning/ruv/` catalog and to WeftOS
integration areas. Use it to (a) find the brain artifact for a topic, (b) see
whether our catalog already covers it deeper, and (c) know where a fact should
land on the WeftOS side. **Primary source wins; the brain corroborates.**

## Brain artifact types

| Artifact | Path in clone | What it gives you |
|----------|---------------|-------------------|
| Capability cards | `kb/capability-cards.md` | One capability-phrased routing card per building block — "reach for X when you need Y". Best entry point. |
| Per-repo primers | `kb/<repo>-primer.md` | What / capabilities / concepts / maturity / docs / usage, grounded with file-path citations. 18 primers. |
| Registry | `data/ruvnet-registry.json` | All ~197 ruvnet repos: name, ★, pushed date, lang, one-line desc. |
| Tier list | `data/registry.tiers.json` | T0–T3 ingest-depth tiers + selection rule (`stars≥1000 OR pushed≤3mo OR core-allowlist`). |
| Capability JSON | `kb/capability.<repo>.json` | Machine-readable capability facets (ruflo, ruvector, rulake, ruview, agentdb). |
| Q/A + held-out | `kb/questions.*.json`, `kb/heldout.*.json` | Eval sets used to grade primer groundedness. |
| Source pins | `kb/SOURCE.json` | Pinned source commits for ingested stores (currently `rvm`). |

## Active-integration coverage (our focus areas)

| WeftOS integration | Brain primer/card | Our catalog deep-dive | Who is deeper | WeftOS mapping |
|--------------------|-------------------|-----------------------|---------------|----------------|
| **agenticow** | `agenticow-primer.md`, card | `packages/agenticow/overview.md` | **Ours** (142×, ADR-202 bridge, full API) | Actor/hermes-loop memory checkpoint + rollback; exochain lineage |
| **AgentBBS** | *none* (registry only) | `packages/agentbbs/overview.md` | **Ours** (brain has no primer) | `clawft-substrate` A2A + `clawft-channels`; ed25519 actor identity |
| **midstream** | *none* (registry only) | `packages/new-libraries-2026-07.md` | **Ours** + primary README | Voice/ECC 50 ms CognitiveTick — mid-stream gate/steer |
| **metaharness** (agent-harness-generator) | `agent-harness-generator-primer.md`, card | `packages/new-libraries-2026-07.md` | Comparable | Governance/gate patterns, MCP `mcp-scan`, Darwin-mode self-evolution |
| **RuLake** | `rulake-primer.md`, card | `packages/new-libraries-2026-07.md` | Brain primer is **deeper** (ADR-by-ADR maturity) | Receipt-pinned LLM cache; witness-anchored retrieval |
| **rvm** | card (rich); `SOURCE.json` pinned @`af97d18` | `packages/new-libraries-2026-07.md` | Brain card is **deeper** (rights, proof tiers, depth) | K3 WASM sandbox / capability tokens / proof-gated mutation (exochain) |
| **agentdb** | `agentdb-primer.md`, card | `packages/new-libraries-2026-07.md` | Brain primer is **deeper** (controllers, backends, RVF) | Memory backend; causal/explainable recall |

**Reading**: where "Ours" is deeper, cite our overview and skip the brain. Where
the **brain primer is deeper** (RuLake, rvm, agentdb), the brain is the better
starting point — but still re-verify against the primary repo before a decision
lands. Distilled, verified extracts for the deeper-brain cases are in
`distilled-notes.md`.

## Broader ecosystem coverage (18 primers)

Primers exist for: `ruflo`, `ruvector`, `agentdb`, `rulake`, `ruview`,
`agentic-flow`, `sparc`, `qudag`, `safla`, `ruv-fann`, `synthlang`, `rupixel`,
`agenticow`, `cve-bench`, `daa`, `dspy.ts`, `fact`, `agent-harness-generator`.
Each has a matching capability card in `kb/capability-cards.md`.

These overlap our existing `packages/` for `ruvector`, `ruflo`, `qudag`, `daa`
(via the ruv-researcher charter's own repo set). For those four, our charter
already tracks primary source; the brain is a cross-check.

## Topic → brain-card routing (capability cards)

The cards are keyword-rich so a *described* need routes to a repo even when
unnamed. Condensed:

| Need | Brain card routes to |
|------|----------------------|
| vector search / HNSW / local semantic search | ruvector |
| long-term agent memory / explainable recall | agentdb |
| vector cache in front of a store / verifiable retrieval | rulake |
| fork/branch/rollback agent memory cheaply | agenticow |
| multi-agent swarm / parallel coding agents | ruflo |
| ready-made coding agents / multi-provider routing | agentic-flow |
| capability security / microhypervisor / proof-gated mutation | rvm |
| quantum-resistant / anonymous agent messaging | qudag |
| self-improving / meta-cognitive feedback loop | safla |
| in-Rust NN / WASM inference / forecasting | ruv-fann |
| prompt compression / token-cost reduction | synthlang |
| scaffold/evolve an agent harness (Darwin mode) | agent-harness-generator (metaharness) |
| benchmark security-vuln fixing on real CVEs | cve-bench |
| decentralized autonomous economic agents | daa |
| optimizable declarative LLM pipelines (TS) | dspy.ts |
| cached low-latency tool calls + circuit breaker | fact |
| WiFi/CSI camera-free sensing | ruview |
| on-device visual RAG (images/video/screenshots) | rupixel |

## Coverage gaps (brain does NOT cover our active integrations well)

- **AgentBBS** — no primer; registry one-liner only. Our
  `packages/agentbbs/overview.md` is the authority.
- **midstream** — no primer; registry one-liner only. Our
  `packages/new-libraries-2026-07.md` + primary README are the authority.
- **WeftOS/clawft itself** — the brain is RuvNet-only; it has no knowledge of our
  kernel, ADRs, or phases. It never will — that is what `docs/brain/` and the
  ruvector `weftos/*` namespaces are for.
