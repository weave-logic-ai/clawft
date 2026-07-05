# Turn Classification & Labeling — Implementation Plan

**Pairs with**: `classification-design.md` (same directory). **HEAD**: `9b59b9fd`.
**Tracker**: create Plane items in cycle `0.7.x` (P2 is an ADR-067 must-ship
prerequisite) per the `plane-workflow` skill; link each phase task back to this plan
and ADR-067 P2.

Two phases. **Phase A (keyword sync tier) is the shippable core** — it satisfies the
"every node has classification" vision end-to-end and is independent of the GUI. **Phase B
(LLM async enrichment)** is a quality upgrade layered behind the `full` config mode.

---

## Phase A — Keyword sync tier (ships first)

Goal: every committed turn node carries a non-null 4-axis `classification` blob, produced
synchronously at `index_turn`, gated by `[kernel.agent.classification]`, surfaced by the
existing `conversation.graph` RPC with no wire change.

### A1 — Taxonomy types + keyword classifier  *(clawft-service-agent)*
- **New** `crates/clawft-service-agent/src/turn_classifier.rs`:
  - `ClassificationVector { intent: Intent, topic: String, emotion: Vad, goal:
    Option<String>, tier: Tier, v: u8 }`, `Intent` enum (7 variants), `Vad {valence,
    arousal, dominance, label}`, `Tier {Keyword, Llm, Voice}`. `Serialize` for the blob;
    a `to_metadata_value()` producing the exact `classification` JSON in design §D2.
  - `trait TurnClassifier { fn classify(&self, role: &str, text: &str) ->
    ClassificationVector; }`
  - `KeywordTurnClassifier`: intent from surface cues (`?`, imperative, correction/social
    lexicons); topic via a `tokenize` helper (copy the ~20-line graphify idiom incl. stop
    words); emotion VAD from a small sentiment/intensity lexicon + exclamation/caps cues
    (arousal defaults `0.5`, dominance `0.0`); goal always `None`; topic-continuity carry
    (take a `prev_topic: Option<&str>`).
  - `pub fn arousal_of(node_meta: &Value) -> Option<f32>` (floor readiness, design §5).
- **Estimate**: ~250 lines incl. lexicons; keep the file < 500. **Tests**: A1 unit set
  (design §8 taxonomy).
- **Model routing**: Tier-2/3 (new module, deterministic logic + lexicon curation).

### A2 — Thread classifier through `dual_write_turn` + `index_turn`  *(clawft-service-agent)*
- `session_forest.rs`: add `classification: Option<&serde_json::Value>` param to
  `dual_write_turn`; write it into the metadata map alongside `state`/`uid`; **also add
  `"text": text`** (design §6). Keep `emotion`/`goal` params — they now receive the
  derived label/goal instead of `None`.
- `session_tier.rs::index_turn` (line 234): hold `Option<Arc<dyn TurnClassifier>>` on
  `SessionTier`; when set, `classify(role, text)` → build blob → pass blob + `emotion.label`
  + `goal` into `dual_write_turn`. Thread `prev_topic` from the `ConvForest` (add a
  `last_topic` cell) for continuity.
- **Depends on**: A1. **Estimate**: ~1 day. **Tests**: A2 `index_turn` integration
  (design §8) — extend `session_tier_tests.rs` / `text_ecc_commit.rs`.

### A3 — Config gate  *(clawft-types + clawft-weave)*
- `config/kernel.rs`: add `ClassificationMode { Off, Keyword, Full }` (default `Off`) and
  `ClassificationConfig { mode, model_override: Option<String>, queue_bound: usize }` as a
  `#[serde(default)]` block on `AgentAnchorConfig` (mirror `SubagentsConfig` exactly:
  `default_*` fns, camelCase aliases).
- `daemon.rs` agent-service boot: construct `KeywordTurnClassifier` and attach to
  `SessionTier` when `mode != Off`; log the "loop-on but classification=off" hint.
- **Depends on**: A1, A2. **Estimate**: ~0.5 day. **Tests**: config round-trip
  (partial/absent block → defaults); daemon wires classifier iff `mode != Off`.

### A4 — Spawn-node classification  *(clawft-service-agent)*
- `subagent.rs:238` (`spawn_goal`) + `:454` (`spawn_result`): build a
  `ClassificationVector` from `SpawnSpec.goal` / result summary and write the blob into the
  node metadata at `add_node` time; emit `GoalMotivation` crossref `spawn_goal →
  goal-anchor` (reuse `link_cross_conv`).
- **Depends on**: A1. **Estimate**: ~0.5 day. **Tests**: A4 spawn-node test (design §8) —
  extend `subagent_spawn_commit.rs`.

### A5 — RPC surfacing test + gate  *(clawft-weave)*
- No RPC code change (blob is read verbatim). Extend
  `conversation_graph_scopes_and_shapes` (`daemon.rs:6411`) to assert populated
  `classification` + non-empty `text`.
- Run `scripts/build.sh gate` before commit (tsc/lint/build/clippy/tests per CLAUDE.md).
- **Depends on**: A2–A4. **Estimate**: ~0.5 day.

**Phase A total: ~3 days.** Exit criterion: with `[kernel.agent] talk_loop=true` and
`classification.mode="keyword"`, a user turn commits a node whose `conversation.graph`
projection carries a full 4-axis `classification` and its `text`, plus
`EmotionCause`/`GoalMotivation` crossrefs where applicable.

---

## Phase B — LLM async enrichment (behind `mode="full"`)

Goal: after a turn commits, a cheap-model round-trip refines intent/topic/emotion label
and patches the node blob (`tier: "llm"`) off the turn path. The GUI shows the refinement
on its next poll.

### B1 — Mutable metadata merge  *(clawft-kernel)*
- `CausalGraph::merge_node_metadata(id, patch: Map)` merging keys under the node's existing
  lock; **must not** clobber `state`/`uid`/`chain_seq`/`text`. Unit test the race
  invariant against a concurrent `set_node_state`.
- **Estimate**: ~0.5 day. **Highest-risk item** (design §7 risk 1) — review carefully.

### B2 — Four-axis LLM prompt + parser  *(clawft-service-agent or clawft-core)*
- A **separate** `EnrichmentClassifier` reusing the `Classifier` backend trait
  (`llm_classifier.rs:106`) with a new system prompt emitting `{intent, topic, emotion,
  goal}` JSON; robust parse (fence-strip, fallback-to-none), mirroring the existing
  router's hardening. **Do not touch** `ClassifierOutput` / `LlmClassifierRouter` (routing
  contract).
- **Estimate**: ~1 day. **Tests**: parse hardening (valid/fenced/malformed/partial).

### B3 — Bounded enrich queue + drain task  *(clawft-weave)*
- `mpsc<EnrichJob>` (bound = `config.queue_bound`); `index_turn` enqueues after the sync
  write when `mode == Full` (drop-oldest on full, log). One daemon task drains, calls B2,
  patches via B1. Serialises cheap-model calls.
- **Depends on**: B1, B2, A2, A3. **Estimate**: ~1 day. **Tests**: patch flips
  `tier` keyword→llm; RPC reflects on next read; idempotent re-patch; queue-full drops
  oldest without blocking `index_turn`.

**Phase B total: ~2.5 days.** Exit criterion: with `mode="full"`, a turn first shows a
keyword blob, then (within the enrich latency) an `llm`-tier blob on the next
`conversation.graph` poll, with no measurable change to turn-commit latency.

---

## Swarm shape

Hierarchical, 3 implementers + reviewer (specialized strategy, per CLAUDE.md). Phase A is
the critical path; Phase B starts once A2/A3 land.

| Agent | Owns | Phase |
|---|---|---|
| `coder` #1 (classifier) | A1 taxonomy+keyword impl; A4 spawn nodes; B2 LLM prompt | A→B |
| `coder` #2 (wiring) | A2 dual_write/index_turn thread; A3 config+daemon; A5 RPC test | A |
| `backend-dev` (kernel/daemon) | B1 metadata merge; B3 enrich queue+drain | B |
| `reviewer` | correctness + 500-line rule + gate green across both phases | A, B |

Comms: pipeline — `#1 → #2` (classifier before wiring), `#2 → reviewer` for Phase A;
`#1(B2) + backend-dev(B1) → backend-dev(B3) → reviewer` for Phase B. Spawn A-phase agents
together; hold B-phase until Phase A gate is green.

## Build / verification (every commit)
`scripts/build.sh gate` (11-check phase gate) per CLAUDE.md — never raw cargo. A full test
sweep is already running in the background this session; do not launch heavy builds until
it clears.

## Sequencing summary
1. **A1** (types+keyword) → **A2** (wiring) + **A4** (spawn) in parallel → **A3** (config)
   → **A5** (RPC test + gate). *Ship Phase A.*
2. **B1** (merge) + **B2** (prompt) in parallel → **B3** (queue+drain) → gate. *Ship B.*
