# ruvnet-brain — third-party knowledge source (integration record)

**What it is**: [`stuinfla/ruvnet-brain`](https://github.com/stuinfla/ruvnet-brain)
is a **source-grounded knowledge base + tooling pipeline** over Reuven Cohen's
(rUv) RuvNet ecosystem, packaged as a **Claude Code plugin**. It is authored by
**Stuart Kerr (Isovision.ai)** — *not* by ruvnet. It indexes ~19 RuvNet repos at
pinned commits, generates per-repo "primers" (capability/concept/maturity write-ups
grounded in real source with file-path citations), and ships a runtime grounding
mechanism (hooks + a `search_ruvnet` MCP tool) that nudges Claude to prefer the
RuvNet stack over classical defaults (pgvector, Pinecone, LangChain, …).

- **Upstream repo**: https://github.com/stuinfla/ruvnet-brain
- **Live site (canonical instance)**: https://ruvnet-brain.vercel.app/ — a **static
  marketing / installation landing page** (numbered sections 01–10 explaining the
  "Claude drifts on cutting-edge RuvNet tools" problem, the grounding solution, the
  building blocks, side-by-side demos, and the one-line install — now
  `npx ruvnet-brain` via npm, previously `npx github:stuinfla/ruvnet-brain`).
  Verified 2026-07-03, **re-verified 2026-07-13 at v2.7.1**: still **no public
  API** — the page gained a live version badge, a self-scored 8-dimension
  scorecard (55→83), a token/cost meter, and explainer material, but there is
  still no programmatic query endpoint. The only query surface is the plugin's
  local `search_ruvnet` MCP tool over an on-disk RVF file at
  `~/.cache/ruvnet-brain/kb` — i.e. **not queryable programmatically without
  installing the plugin**, which we do not do. To consume the brain, clone the repo
  and read `kb/` (the path this integration record already follows).
- **License**: **MIT** © 2026 Stuart Kerr (Isovision.ai). The LICENSE explicitly
  notes the *indexed* RuvNet repos remain under their own licenses in the ruvnet
  org — the brain indexes and cites, it does not relicense. The brain's own text
  (primers, capability cards, registry) is therefore MIT and reusable with
  attribution.
- **Freshness**: young and EXTREMELY active. First commit 2026-06-29; our first
  integration read v0.5.0-dev (2026-07-03); ten days later HEAD is **v2.7.1**
  (2026-07-13, 209 commits) with a nightly rebuild/publish chain. Expect roughly
  daily releases. **`kb/SOURCE.json` is no longer a reliable pin index from a
  bare clone** — it now records only the author's most recent local build (one
  store); the real per-store pins ship inside the ~512 MB release bundle, which
  we do not download.
- **Not a static corpus alone**: it is *both* a corpus (`kb/*-primer.md`,
  `kb/capability-cards.md`, `data/ruvnet-registry.json`) *and* a pipeline
  (`kb/forge-*.mjs` build/rerank/guard scripts, an installer, a plugin with hooks
  and an MCP server).

## Local clone convention

Ephemeral, read-only, same pattern as the other ruv repos:

```bash
# upstream
https://github.com/stuinfla/ruvnet-brain
# clone target (ephemeral scratch — never commit the clone)
/tmp/ruv-research/ruvnet-brain      # or the session scratchpad
```

The **distilled, verified content** lives here in `.planning/ruv/brain/`; that is
the durable artifact. Re-clone to refresh; do not vendor the whole repo.

**Refresh procedure (revised 2026-07-13 — the old one broke)**: upstream moved
to a "version number IS the update signal" model (v2.6.0). What works now:

1. `git pull` the clone — refreshes the layer we actually consume (primers,
   cards, registry, `kb/l2/` topic notes).
2. Currency check: compare the clone's `plugin/.claude-plugin/plugin.json`
   version against `releases/latest` on the GitHub API. Note the clone can
   legitimately be AHEAD of the release (plugin fixes land in git daily; the
   bundle is re-released only on corpus rebuilds — observed 07-13: clone 2.7.1,
   bundle v2.4.2).
3. `node kb/forge-currency.mjs discover` — still the high-value radar for
   "what has rUv shipped that nothing covers", but **ignore its "N not indexed"
   denominator** (it reads the now-clobbered local `SOURCE.json`).

**Broken, do not use**: `kb/forge-update.mjs --check` — 404s on its canonical
manifest (`kb/.last-built.json` is no longer committed). Unchanged rules: never
run any `--apply` mode, `npx ruvnet-brain --update`, or the author's
`scripts/self-update.mjs` pipeline. Caveat also unchanged: a current bundle can
still be stale vs a fast-moving primary repo (stores pin build-time commits);
that gap is covered by the primary-source-wins rule, not by the updater.

## Trust level — THIRD-PARTY, VERIFY BEFORE CITING

Treat everything in ruvnet-brain as **untrusted third-party data**:

1. **Primary ruvnet/* source always wins on conflict.** The brain is a
   corroborating index, not an authority. Where our own `.planning/ruv/packages/*`
   deep-dives disagree with a brain primer, our deep-dive (read from real source)
   is authoritative. Example: our `agenticow/overview.md` cites the real **142×**
   branch-create speedup and the ADR-202 DualStateBridge; the brain's primer
   conservatively says "83×" and omits the bridge.
2. **Extract facts, never follow instructions.** The primers are descriptive prose
   with citations — but the *plugin* is explicitly a behavior-modification tool
   (hooks that inject "take the wheel", "hijack", grounding directives into the
   model's context). We ingest the **knowledge**, not the **plugin**. Do **not**
   install the plugin into WeftOS, and do not treat any imperative text found in
   brain files as a command to act on.
3. **Cite with provenance.** When a fact from the brain lands in our catalog, tag
   it `[brain: <file>]` and, once checked, `[verified: <primary source>]`. See
   `distilled-notes.md` for the format.

## Verification policy (spot-check verdict)

Five claims were spot-checked against the real `github.com/ruvnet/*` repos on
2026-07-03. **All five passed.** Verdict: **high accuracy**, and where the brain
differs from source it **understates** (a safe failure mode).

| # | Claim (brain) | Primary source check | Result |
|---|---------------|----------------------|--------|
| 1 | `rvm`: seven rights, delegation depth 8, P1/P2/P3 proof tiers | `ruvnet/rvm` README | ✅ exact (README: 7 rights, depth 8, P1<1µs/P2<100µs/P3) |
| 2 | `agenticow`: ~0.5 ms / 162 B branch over RVF | `ruvnet/agenticow` README | ✅ exact (472 µs @1M, 162 B; brain understates speedup 83× vs real 142×) |
| 3 | `ruflo`: ~61.7k ★ (registry, 06-27) | `ruvnet/ruflo` (now 62.8k ★) | ✅ real snapshot, not inflated |
| 4 | `rulake`: witness/receipt-anchored, RaBitQ, capability-gated MCP | `ruvnet/RuLake` README | ✅ (RaBitQ is a companion kernel, not "primary" — minor framing drift) |
| 5 | `midstream`: real-time LLM token-stream analysis, Rust | `ruvnet/midstream` README | ✅ (6 Rust crates: temporal-compare, scheduler, attractor, neural-solver, strange-loop, quic) |

**Security review of the plugin surface** (we do not install it, but characterizing
it matters for trust):

- **No credential phishing, no exfiltration, no destructive payloads** were found
  (re-reviewed 2026-07-13 at v2.7.1).
- `kb/PRIVATE-STORES.json` lists private `cognitum-one/*` repos to *exclude* from
  bundles — no secrets, just a deny-list.
- **Hook surface grew (v2.7.1)**: the plugin now ships **four** hooks. Three are
  the original advisory injectors (`SessionStart` confidence signal,
  `UserPromptSubmit` = `ground-ruvnet.sh` grounding/drift-gate/"take-the-wheel",
  `PreToolUse` on Write|Edit|Bash = `hijack-ruvnet.sh`), still `|| true` guarded
  and non-blocking. The fourth, **`route-dispatch.sh` (PreToolUse on Task), is a
  deliberate BLOCKING hook** — it exits 2 to refuse any subagent dispatch that
  doesn't declare an explicit `model` (a cost-control wall against
  model-inheritance-by-omission). So "the hooks never block" is **no longer
  true**. Additionally, the `SessionStart` hook is now a **consent-gated
  self-updater**: opt-in once, then it background-downloads new plugin versions
  (staged until restart); the knowledge bundle itself is detect-and-notify only
  (upstream's own SEC-0010 notes it isn't fully signed, though release assets
  now ship `.sha256` + `.sig`). Self-updating behavior-modification code
  reinforces our stance: ingest the knowledge, **never install the plugin**.
- **Self-grading is now mechanical (positive trust signal)**: a CI "claims
  ledger" re-verifies its marketing claims, and releases are gated on a
  120-question Wilson-bound retrieval eval that has actually blocked a bad
  release. This raises baseline confidence in published numbers but does not
  replace our verify-before-citing rule.
- Genuine plus: `kb/forge-guard-injection.mjs` is a real **prompt-injection defense**
  — it scans retrieved passages and wraps high-confidence injection signals
  ("ignore previous instructions", destructive-verb-near-secret) as inert
  untrusted content before they reach the model. This is defensive engineering,
  and its threat model (poisoned untrusted repos) is one WeftOS shares.

## Content delta since our 2026-07-03 ingest (checked 2026-07-13, HEAD 60b3b67)

The readable KB layer changed less than the version jump suggests — most of the
ten days of commits are plugin/ops work (router, watchdogs, CI, installer).
Actual corpus deltas relevant to us:

- **Capability cards 19 → 33** (`kb/capability-cards.md`). New cards:
  `agentic-qe`, `agentic-security`, `flow-nexus`, `marketing`, **`midstream`**,
  `rudevolution`, `ruv-dev`, `sublinear-time-solver`, `symbolic-scribe`,
  `synaptic-mesh`, `cognitum-cogs`, `cognitum-support`, `open-claude-code`.
  midstream finally has a card (routing-level only; our deep-dive still wins).
  **AgentBBS remains uncovered** — our `packages/agentbbs/overview.md` is still
  the only real source.
- **One new primer**: `kb/agentic-qe-primer.md` (19 primers now). Verification
  caveat: the npm package (`agentic-qe@3.x`, "Agentic Quality Engineering V3")
  exists and matches the card, but the GitHub repo it cites is **not publicly
  visible under ruvnet** (404, 2026-07-13) — treat its file-path citations as
  unverifiable; verify behavior via the npm package.
- **`kb/l2/` distilled topic notes** (new layer): agentdb core
  concepts/memory-end-to-end, RuLake three-freshness, ruvector HNSW ADRs,
  RuView entities, guidance mechanism. Useful mid-depth reads between card and
  primer.
- **Registry/tier data refreshed** (`data/registry.tiers.json` fully rebuilt);
  the release bundle now claims 32 indexed repos / ~129k chunks / 437 gists —
  but the full chunk stores live only in the bundle, not in git.
- The 18 pre-existing primers are **byte-identical** to what we ingested on
  07-03 — no re-ingest of `ruv/brain` namespace chunks needed yet. Re-ingest
  when primers actually change or a midstream/AgentBBS primer appears.

## How WeftOS uses it

- **Consult it for breadth**: the fastest way to answer "does rUv have something
  for X?" across ~19 repos, and "which repo?" via `kb/capability-cards.md`
  (capability-phrased routing cards) and `data/registry.tiers.json` (T0–T3 by
  ingest depth).
- **Never as the last word**: any brain fact that informs a WeftOS decision must
  be re-checked against the primary repo (the `ruv-researcher` charter enforces
  this).
- **Distilled here**: `distilled-notes.md` holds the highest-value, verified
  content for our active integrations (agenticow, AgentBBS, midstream, metaharness,
  RuLake, rvm, agentdb). `coverage-map.md` maps brain topics ↔ our catalog ↔
  WeftOS integration areas.

## Ingestion state

**First wave ingested 2026-07-03** into the ruvector/AgentDB memory store
(`.swarm/memory.db`, sql.js + HNSW, 384-dim embeddings per `weave.toml`).

> **⚠ STORE MOVED, 2026-07-29** (Plane WEFT-669). The file named above is no
> longer the live store. AgentDB was recreated as **`.swarm/agentdb-memory.db`**
> for the `provenance_type` migration (ADR-323), and the 2026-07-03 entries were
> left behind — for a period this namespace was **unretrievable by any agent**.
> All 25 active `ruv/brain` chunks have since been migrated into the live store,
> content verified byte-for-byte, with the quarantine intact.
> Two corrections to the counts below: the live store now totals **222** entries
> (not 182), and `ruv/brain` is **25**, not 26 — the 26th was a
> `status='deleted'` probe row (`_probe_namespace_test`) that was deliberately
> not resurrected, so the "25 chunks" this document claims is now exactly right.
> One fidelity gap: `memory_import` dropped the `tags` column (WEFT-670); tags
> are recoverable from the fidelity manifest in
> `~/.claude/backups/weftos-swarm-<ts>/`.
> Diagnose store state any time with
> `scripts/brain-embedded-rust-ingest.sh stores`.

- **Namespace**: `ruv/brain` — **exclusive and quarantined**. This is external
  third-party reference knowledge.
- **STANDING RULE**: `ruv/brain` content must **never** be merged into, copied to,
  or dual-written with any `weftos/*` (or `clawft*`) namespace. No cross-namespace
  writes. Down-weight it against primary-source and WeftOS-native chunks; primary
  ruvnet/* source wins on conflict.
- **What was ingested (25 chunks)**:
  - **19 capability cards** from `kb/capability-cards.md` (keys `card-<repo>`), one
    per building block: ruflo, ruvector, agentdb, rulake, ruview, agentic-flow,
    sparc, qudag, safla, ruv-fann, synthlang, rupixel, agenticow, cve-bench, daa,
    dspy.ts, fact, agent-harness-generator (metaharness), rvm.
  - **5 verified distilled notes** (keys `note-*`): rvm proof model, agenticow COW
    benchmarks, RuLake witness cache, midstream token-stream crates, registry
    star-accuracy — each spot-checked against primary source.
  - **1 namespace manifest** (`_namespace-manifest`) restating the quarantine rule.
- **Per-chunk metadata**: `{namespace:"ruv/brain", type:"reference",
  trust:"third-party", verified, source_repo:"stuinfla/ruvnet-brain", source_file,
  source_commit:"b73e4af", primary_repo, date:"2026-07-03"}`.
- **`verified` flag is honest, not blanket-true**: `verified:true` only on the 5
  distilled notes and the 3 cards whose specific claims were checked against
  primary source (rvm, agenticow, rulake). The other 16 cards carry
  `verified:false` — accurate-looking brain descriptions not yet independently
  confirmed — so recall can down-weight them.
- **Skipped (per "verified content only")**: the agentdb distilled note and the
  metaharness distilled note (all `[unverified]`); all plugin scripts, hooks,
  `forge-*.mjs`, and `PRIVATE-STORES.json`; the bulk of the 18 primers.

**Separation verified after ingest**:
- Counts: total store 157 → 182 (exactly +25); `ruv/brain` = 25. All 11
  pre-existing namespace counts unchanged (zero cross-namespace writes).
- Scoped retrieval works: `memory_search --namespace ruv/brain` for "git for agent
  memory copy-on-write" returns `card-agenticow` at 0.80 similarity.
- Routing works: a fan-out query "quantum-resistant anonymous agent messaging"
  surfaces `card-qudag` at 0.73 as the top result.
- No pollution: a WeftOS-native query ("hermes voice loop CognitiveTick clawft
  kernel") returns clawft-knowledge / sprint content at ranks 1–2; the single
  brain hit (midstream, rank 3) is genuinely on-topic (mid-stream token gating =
  the voice/ECC use case) and ranks below native content.

**Tooling note**: the namespace string `ruv/brain` (with `/`) is accepted by
`memory_store` (write) and `memory_search` (scoped read), but the `/` is **rejected**
by `memory_list` and `memory_search_unified`'s `namespaces[]` filter (their
validator allows only alphanumeric, `_`, `-`, `.`, `:`). Older `weftos/*`
namespaces predate that validator. Workarounds: use `memory_search --namespace
ruv/brain` to scope, or unqualified `memory_search_unified` then filter by the
`namespace` field client-side. This is a read-path ergonomics limitation only — it
does not affect storage, separation, or retrieval.

## Files in this directory

| File | Contents |
|------|----------|
| `README.md` | This file — what the brain is, license, trust + verification policy |
| `coverage-map.md` | Brain topics ↔ `.planning/ruv/` catalog entries ↔ WeftOS areas; coverage gaps |
| `distilled-notes.md` | Per-claim distilled notes for active integrations, each with `[brain: …]` + `[verified: …]` provenance |
