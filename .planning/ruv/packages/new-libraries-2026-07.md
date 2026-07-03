# New ruv libraries survey — 2026-07 reparse

Covers the significant libraries added/updated since the 2026-04-14 parse, other
than the two deep-dives (`agentbbs/`, `agenticow/`).

## metaharness — agent-harness factory

- Repo: https://github.com/ruvnet/metaharness · TS · MIT · ~360★ · v0.1.x beta
- MCP tools (claude-flow): `metaharness_bench`, `_score`, `_evolve`, `_genome`,
  `_similarity`, `_mcp_scan`, `_oia_audit`, `_redblue`, `_threat_model`,
  `_security_bench`, `_drift_from_history`, `_audit_list/_trend`.

A factory that scaffolds branded, npm-publishable agent harnesses (own CLI, MCP
server, memory namespace, governance policy) from a GitHub repo. "Every serious
repo deserves its own agent." Three tiers: author → `agent-harness-generator` →
your harness → `@metaharness/kernel` (Rust→WASM/NAPI) → host adapters.

Key packages: `@metaharness/kernel` (7 subsystems), `/router` (cost-perf model
routing), `/darwin` (gradient-free self-evolving config), `/weight-eft` (LoRA
cost reduction), `/jujutsu` (op-log — the code side of agenticow's DualStateBridge).
9 host adapters (claude-code, codex, pi-dev, **hermes**, openclaw, rvm, copilot,
opencode, github-actions), 18 vertical templates.

**WeftOS relevance — MED.** ruflo is described as the meta-harness this generator
"factors apart." Two things matter for us: (1) the **default-deny MCP policy** +
`harness mcp-scan` ("npm audit for agent tools", flags shell/network grants and
unguarded secrets) — a pattern for clawft's governance/gate MCP surface; (2) a
`hermes` host adapter exists, which is worth checking against our hermes loop.

## midstream — real-time LLM token-stream analysis

- Repo: https://github.com/ruvnet/midstream · Rust · MIT OR Apache-2.0 · ~128★
- 6 crates on crates.io v0.2.1; WASM-first; npm `@midstream/wasm`, `midstream` CLI.

"Treat an LLM token stream as a first-class signal — pattern-match it, score it,
intervene on it — while the tokens are still arriving." Crates:
`midstreamer-temporal-compare` (DTW / LCS / edit distance, ~38 µs),
`midstreamer-scheduler` (ns-scale priority queue, 85–120 ns),
`midstreamer-attractor` (Lyapunov / phase-space), `midstreamer-neural-solver`
(Linear Temporal Logic verification), `midstreamer-strange-loop` (self-referential
mid-stream policy adjustment), `midstreamer-quic` (QUIC multi-stream, 0-RTT).
41 ADRs, fuzz/proptest baselines.

**WeftOS relevance — HIGH for voice/ECC.** The voice-ECC design (ADR-058–061,
`.planning/voice-ecc-synthesis.md`, memory `weftos-voice-ecc-design-current`)
runs a 50 ms CognitiveTick and needs to gate/steer before an utterance completes.
midstream is purpose-built for exactly that: microsecond-overhead inflight gating,
drift detection, and LTL-verified steering of a stream mid-flight. Candidate to
back the ECC decision loop rather than hand-rolling stream analysis. MIT/Apache,
WASM-first — vendorable. Relevant crates on our side: `clawft-voice-talk`,
`clawft-voice-tts`, `clawft-voice-aec`, `clawft-voice-onnx`.

## RuLake — cache-coherent memory fabric

- Repo: https://github.com/ruvnet/RuLake · Rust · MIT/Apache-2.0 · ~11★
- `rulake` crate; `pip install rulake`; `npm install rulake` / `rulake-wasm`;
  Claude plugin `rulake-stack@rulake-marketplace`.

Self-learning memory layer that caches results from existing sources (S3,
BigQuery, Snowflake, Parquet, files) with no dedicated vector DB. "Memory that
gets faster the more it's used" — ~1 ms lookups over 100k items, 32× less RAM
than raw embeddings, $0/query. Cryptographically **pins answers to receipts** and
**refuses to guess when underlying data changed** (returns "I don't know" rather
than stale). Part of the RuVector ecosystem with `ruvector-rabitq` (1-bit
compression) and RVF.

**WeftOS relevance — MED.** The receipt-pinned, fail-honest cache-coherence model
maps onto WeftOS witness/receipt semantics (exochain) — an LLM/answer cache that
is provably consistent with source data. Relevant to `clawft-llm` /
`clawft-service-llm` and the coherence work (`eml-core`).

## agentdb (now standalone) — self-improving vector memory

- Repo: https://github.com/ruvnet/agentdb · TS · ~72★ · pushed 2026-06-22

Previously vendored inside agentic-flow; now its own repo. "Vector memory that
gets smarter every time your agent uses it." Backs the many `agentdb_*` MCP tools
in claude-flow (hierarchical store/recall, causal edges, graph pathfinder,
semantic route, consolidate, controllers). **MED** — it is the memory substrate
under a large fraction of the claude-flow tool surface; agenticow branches sit
conceptually above a store like this.

## rvm — the agentic VM

- Repo: https://github.com/ruvnet/rvm · Rust · ~114★ · "VM built for the agentic age"
- A `@metaharness/rvm` host adapter exists. **MED** — potential reference for the
  K3 WASM sandbox / execution model. Not yet deep-analyzed.

## Sensor / domain repos (LOW for kernel, adjacent to sonobuoy work)

RVF/ruvector-substrate demos and RF/acoustic sensing projects that are not
kernel-relevant but overlap the WeftOS sensor/actor and sonobuoy tracks:

- **RuView** (Rust) — WiFi signals → real-time spatial intelligence.
- **rvcsi** (Rust) — edge RF-sensing runtime, normalizes WiFi CSI from
  Nexmon/ESP32. Directly adjacent to the ESP32 sonobuoy firmware track.
- **worldgraph** (Rust) — privacy-aware environmental digital twin for ambient/RF.
- **rufield** (Rust) — open spec for camera-free multimodal field sensing.
- **skygraph** (Rust) — realtime all-sky radar (ADS-B + SGP4) in browser.
- **SonicChamber** (Rust) — acoustic digital-human workbench (ultrasound CT).
- **rupixel** (Rust) — pixel-native visual RAG on the ruvector ANN substrate.
- **rvFACE** (Rust+WASM, Burn) — face recognition SDK.
- **rvdna / ruqu / PhotonLayer** — genomics / quantum / optical-AI, all "pure
  Rust + WASM on RVF" demos showing the RVF substrate generalizing to new domains.
- **helix** (Rust) — local-first anti-hallucination personal health record; the
  anti-hallucination + local-first pattern echoes our coherence/prime-radiant work.

## Meta / harness verticals (LOW)

- **hackerone** — HackerOne defender-triage meta-harness (a metaharness vertical).
- **CVE-bench** — reproduce-and-fix security benchmark (AgentBBS arena consumes it).
- **retort** — platform-evolution/distillation engine (AgentBBS arena track).
- **rudevolution** — semantic decompiler. **open-claude-code** — Claude Code CLI
  decompile/rebuild. **Repo-Explainer** — repo → visual explainer page.
