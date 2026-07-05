# Turn Classification & Labeling — Design

**Date**: 2026-07-05
**Status**: Proposed (design-only; no code in this change)
**Author**: system-architect (hermes-loop pipeline)
**Implements**: ADR-067 P2 (turn classifier wired at `index_turn`), the graph-view
directive *"a node should always have some amount of classification with it, even a
user's text should go through classification, and this will be shown as edges etc."*
**Depends-On**: ADR-062 (ECC node/edge/crossref model), ADR-046 (forest), ADR-058
(session tier / `index_turn`), ADR-061 (voice VAD source), M2 (committed-turn loop),
M4 (spawn nodes). **HEAD verified**: `9b59b9fd`.

---

## 1. Problem & vision

Classification must be a **first-class, durable property of every committed turn node**
on the ECC forest — not a GUI afterthought. Today the plumbing is dormant:

- `session_forest::dual_write_turn` (`session_forest.rs:112`) accepts `emotion` / `goal`
  and writes `EmotionCause` / `GoalMotivation` crossrefs, but every call site passes
  `None, None` (`session_tier.rs:266`). No `intent` / `topic` is recorded at all.
- The `conversation.graph` RPC already emits a per-node `classification` field, read
  verbatim as an opaque JSON blob from node metadata
  (`daemon.rs:5837` → `meta.get("classification")`). It is `Null` today because nothing
  writes the key.
- The node metadata write in `dual_write_turn` does **not** store `text` either
  (`session_forest.rs:132`), so the RPC's `text` field is also empty. Adjacent gap;
  we piggyback the fix (§6).

So the target metadata keys, crossref types, dual-write params, and the RPC wire field
**already exist**. What is missing is the *extractor* and the *config gate*. This design
supplies both, in two tiers.

---

## 2. Decisions

### D1 — Two tiers: sync keyword (always-on when enabled) + optional async LLM enrichment

| Tier | When | Where | Latency added to turn | Produces |
|---|---|---|---|---|
| **Keyword (sync)** | inside `index_turn`, before `dual_write_turn` | `clawft-service-agent` | µs (pure CPU string ops) | full 4-axis vector, coarse |
| **LLM (async patch)** | after commit, off the turn path | daemon drain task | **zero on turn path** | refined intent/topic/emotion label |
| **Voice VAD** | already extracted (ADR-061 ECAPA) | voice-talk | n/a | authoritative emotion VAD |

**Recommendation: ship keyword-only first (Phase A); add the LLM async tier in Phase B.**

Argued honestly: the keyword tier alone satisfies the vision — *every* node gets a
non-null 4-axis classification blob, the graph view gets stable strings for hue/glyph,
and the floor gets an arousal scalar (§D3). The LLM tier is a **quality upgrade, not a
prerequisite**. The async patch carries real cost we should not pay on day one: it needs
a new mutable-metadata path on `CausalGraph` (races with `set_node_state`), a drain-task
execution home, idempotency/ordering rules, and it interacts with ADR-067 P1-graph
snapshots (a patch mints a new state-at-T that the scrubber will render as a
"classification refined" mutation — a *feature*, but one to design deliberately, not
stumble into). Phasing lets Phase A land behind the M2 loop immediately and de-risks
Phase B independently.

**Why the sync tier is safe on the anchor path.** `index_turn` is the *witness/index*
side, not the user's reply path — it is best-effort and already `await`s an embedder
inference in `index_chunk` (tens of ms, the dominant cost). A deterministic keyword pass
is microseconds of CPU; it is noise against the embed already there and never blocks the
reply the agent loop returns separately.

**Why the async tier does NOT ride the TalkModeLoop tick or the idle-reaper.** The tick
is 50 ms and floor-critical (ADR-062 D4) — an LLM round-trip there is unacceptable. The
idle-reaper is per-conversation and fires only on idle — too infrequent for "refine
shortly after the turn." Chosen home: a **bounded mpsc queue drained by one dedicated
daemon classifier task** (§D4). This decouples enrichment latency from the turn, gives
natural backpressure (drop-oldest under load), and serialises the cheap-model calls.

### D2 — Taxonomy v1 (four axes, small and honest)

Stored as one JSON blob under node metadata key `classification`:

```jsonc
"classification": {
  "intent":  "question",                 // enum, see below
  "topic":   "voice-tts",                // short open-vocab tag (≤3 words)
  "emotion": { "valence": 0.1, "arousal": 0.6, "dominance": 0.0, "label": "curious" },
  "goal":    null,                        // free text or null
  "tier":    "keyword",                  // provenance: keyword | llm | voice
  "v":       1                           // taxonomy version
}
```

- **`intent`** — closed enum: `Question`, `Request`, `Statement`, `Correction`,
  `Feedback`, `Social`, `Meta`. Keyword-derivable with high reliability
  (`?` → Question; imperative/verb-initial → Request; "no/actually/wait" → Correction;
  "thanks/lol/hi/bye" → Social; "you/it should" evaluative → Feedback). This is the
  glyph axis for the graph view. **Distinct from** the routing archetype
  (`Reasoning/CodeGen/…`) — that axis is about *tier complexity*, not conversational
  intent, and its classifier stays untouched (directive).
- **`topic`** — short open-vocab string (keyword tier: top non-stopword token via the
  graphify `tokenize` idiom; LLM tier: a 1–3 word tag). Drives the GUI hue/cluster by
  string hash. **Cluster stability caveat** (§7 risk): a per-turn top-token flickers;
  the tier carries a *topic-continuity* heuristic — inherit the prior turn's topic
  unless the token set shifts materially.
- **`emotion`** — `{valence, arousal, dominance}` scalars in `[-1, 1]` plus a coarse
  `label`. Arousal is **always present** (default `0.5` neutral) because the floor needs
  it (§D3). Sources by confidence: voice ECAPA VAD (best) > LLM label (Phase B) >
  keyword lexicon (Phase A floor). Keyword lexicon derives valence + arousal from a small
  sentiment/intensity wordlist plus surface cues (exclamation, all-caps, repetition);
  `dominance` defaults `0.0` (keyword can't infer it honestly).
- **`goal`** — free-text active goal/task thread, or `null`. **The keyword tier leaves
  this `null` for ordinary turns** — inferring a goal from a single user turn is
  unreliable, so we do not fabricate one. It is filled only when there is a real signal:
  a spawn node (goal = `SpawnSpec.goal`, §D5, a free win) or the LLM tier (Phase B).

What each tier can *reliably* produce: intent — high; topic — medium; emotion VAD —
low-to-medium (coarse); goal — low (usually `null`). The doc states this plainly so no
consumer over-trusts a keyword arousal.

### D2.1 — Taxonomy v2: dialogue acts + structure (additive, `v:2`)

Motivation (user, verbatim): *"I would also like it to classify the text as things
like question, clarification, comment, command, etc. Not just the emotional quality but
the directive, the data types, the underlying structure should be clear."*

v2 is **purely additive and versioned**. Every v1 key keeps its meaning; two new keys
(`act`, `structure`) are added, `v` bumps to `2`, and the `intent` key is **retained but
now projected from the refined act** so every existing consumer (the graph glyph, any
UID-keyed reader) keeps working unchanged. A reader given a `v:1` blob (no `act` /
`structure`) derives `act` from `intent` and treats `structure` as empty — the
`from_metadata_value` tolerance path.

```jsonc
"classification": {
  "intent":  "question",                       // v1 key, projected from act.refined
  "act":     { "class": "interrogative",
               "refined": "clarification-request" },
  "topic":   "voice-tts",
  "emotion": { "valence": 0.1, "arousal": 0.6, "dominance": 0.0, "label": "curious" },
  "structure": {
    "entities": [ { "kind": "path", "text": "src/voice/tts.rs", "confidence": 0.9 } ],
    "shape":    { "multi_part": false, "conditional": false, "refers_prior": false },
    "argument": null                            // {verb, object} — LLM tier only
  },
  "goal":  null,
  "tier":  "keyword",
  "v":     2
}
```

**Two-level dialogue act** — `act: { class, refined }`.

Coarse `class` (Searle-style illocutionary category), 5:

| class | meaning |
|---|---|
| `interrogative` | seeks information |
| `directive` | seeks an action / controls the exchange |
| `assertive` | commits the speaker to a state of the world |
| `expressive` | conveys attitude / social stance |
| `commissive` | commits the speaker to future action (LLM-only in v2) |

Refined `refined`, 10 — covers the user's examples + the conversational reality M2/M4
gave us. Each maps deterministically up to a `class` and down to a v1 `intent`:

| refined | class | v1 `intent` | keyword cue (honest) |
|---|---|---|---|
| `question` | interrogative | question | `?` without a clarification cue |
| `clarification-request` | interrogative | question | "what do you mean", "which one", "you mean …?", "to clarify …?" |
| `clarification-provide` | assertive | statement | "i mean", "i meant", "in other words", "to be clear" |
| `command` | directive | request | verb-initial imperative |
| `comment` | assertive | statement | declarative default |
| `correction` | assertive | correction | leading no / actually / wait, "that's wrong" |
| `feedback` | expressive | feedback | "you should", praise lexicon |
| `acknowledgment` | expressive | social | "ok", "got it", "makes sense", "sounds good" |
| `social` | expressive | social | greeting / thanks / farewell |
| `meta` | directive | meta | "start over", "nevermind", "new topic" |

Back-compat: the old 7 `Intent` variants each map **into** a refined act
(`question→question`, `request→command`, `statement→comment`, plus identity for
correction/feedback/social/meta), which is how a `v:1` blob is upgraded on read.

**Structure / data-type layer** — `structure: { entities, shape, argument }`.

`entities: [{ kind, text, confidence }]` — typed spans extracted by the keyword tier via
regex/heuristic, each with a per-span confidence:

| kind | keyword source | conf |
|---|---|---|
| `url` | `https?://…` | 0.95 |
| `path` | contains `/` or a code file-ext (`.rs`,`.toml`,…) | 0.90 |
| `quote` | `"…"` | 0.90 |
| `speaker` | matches a supplied **enrolled-speaker** vocab | 0.90 |
| `date` | ISO `YYYY-MM-DD`, `M/D`, month-name + day | 0.85 |
| `time` | `HH:MM[:SS]`, `N am/pm` | 0.85 |
| `duration` | `N (ms\|s\|min\|h\|d\|w…)` | 0.80 |
| `code` | backtick span, `a::b`, `foo()`, `snake_case` | 0.70 |
| `number` | bare `\d+(.\d+)?` in an otherwise-unclaimed span | 0.70 |

Speaker spans are emitted **only** when an enrolled-speaker vocabulary is supplied; absent
one the keyword tier emits none — it will not guess names (the v1 honesty stance).

`shape: { multi_part, conditional, refers_prior }` — coarse booleans: sequencing /
conjunction ("and then"), conditional ("if …"), reference-to-prior ("that one", "like
before"). `argument: { verb, object } | null` — the command's predicate + object;
**keyword tier leaves this `null`** (reliable predicate-argument extraction needs the
LLM, same stance as `goal`), the enrichment tier fills it.

**What the keyword tier can/can't do honestly.** Acts — high on the surface-cue moves
(question / command / social / acknowledgment), medium on clarification (cue-based, misses
paraphrase), the coarse `class` is exact once `refined` is chosen. Structure entities —
high for the regex kinds (url / path / number / date / time), medium for code-ish tokens,
**none** for speakers without a vocab. Argument structure — **none** (LLM only). The
enrichment (LLM) tier v2 refines the act, adds `argument`, and may promote `class` to
`commissive` where the keyword tier never does.

**GUI (ADR-067 D6, note only — no RPC change; the blob is served verbatim).** The glyph
axis moves from `intent` to `act.refined` (a finer glyph set) with `act.class` as the
coarse hue-family; entity spans render as inline chips (path / url / number / date / code),
and the shape flags as small markers (multi-part ⋯, conditional branch, refers-prior ↩).

### D3 — Storage shape & crossref emission

- **Node metadata**: the single `classification` blob above (matches the RPC's existing
  `meta.get("classification")` read — **no RPC change needed** for the sync tier).
- **Crossrefs** (unchanged types, reused):
  - `emotion.label` → `EmotionCause` crossref, `turn_uid → emotion-anchor`. Anchor is a
    synthetic `UniversalNodeId(conv_id, 0, label, b"emotion")` — exactly today's dormant
    path (`session_forest.rs:180`). The RPC already stubs these non-turn endpoints as
    "identity/classification anchors" (`daemon.rs:5902`) so the edge never dangles.
  - `goal` → `GoalMotivation` crossref, same pattern with `b"goal"`.
  - `intent` / `topic` get **no crossref** — they are scalar node *attributes*, not
    relationships. They live only in the blob. (Emotion/goal earn crossrefs because the
    forest models them as causes/motivations with clusterable anchor nodes: reverse-walk
    the "frustrated" emotion anchor to find every frustrated turn.)
- **Threading**: extend `dual_write_turn` with a `classification: Option<&serde_json::Value>`
  param written into the metadata map alongside `state`/`uid`; the derived `emotion.label`
  and `goal` feed the already-present `emotion`/`goal` params. `index_turn` runs the
  classifier, then passes all three.

### D4 — Execution homes

- **Sync keyword tier**: a `TurnClassifier` trait + `KeywordTurnClassifier` impl in a new
  `clawft-service-agent/src/turn_classifier.rs`. `SessionTier` holds an
  `Option<Arc<dyn TurnClassifier>>`; `index_turn` calls it before `dual_write_turn`. For
  voice, the emotion axis is overridden by the ECAPA VAD the voice path already has
  (voice constructs the blob with `tier: "voice"`).
- **Async LLM tier (Phase B)**: a bounded `mpsc<EnrichJob{conv_id, node_id, chain_seq,
  text}>` in the daemon; one drain task pulls jobs, calls a **separate** four-axis
  classification prompt against the daemon `LlmClient` (via the existing `Classifier`
  backend trait — *not* by mutating `ClassifierOutput`, which routing owns), and patches
  the node blob (`tier → "llm"`). Enqueue happens right after the sync write in
  `index_turn` when `mode == full`. Drop-oldest on a full queue (best-effort, honest).

### D5 — Spawn / subagent nodes are classified too (free win)

Per the vision, spawn nodes are nodes. In `subagent.rs`:

- **`spawn_goal` node** (`subagent.rs:238`): set `classification` from `SpawnSpec.goal` —
  `goal` axis = the goal string itself (literally `GoalMotivation` material), `topic` =
  top token of the goal, `intent` = `Request`, `emotion` = neutral, `tier: "keyword"`.
  Also emit a `GoalMotivation` crossref `spawn_goal → goal-anchor`.
- **`spawn_result` node** (`subagent.rs:454`): classify from the result summary; `intent`
  = `Statement`, `topic` inherited from the goal. Cheap and makes the spawn tree render
  with the same hue as its parent goal in the graph view.

### D6 — Config gate under `[kernel.agent]`

Follow the `talk_loop` / `subagents` precedent — a nested block on `AgentAnchorConfig`:

```toml
[kernel.agent.classification]
mode          = "keyword"     # off | keyword | full   (default: off)
model_override = "haiku-3.5"  # only consulted when mode = full
queue_bound    = 256          # async enrich queue depth; full ⇒ drop-oldest
```

`mode` (enum `ClassificationMode { Off, Keyword, Full }`, default `Off`) keeps non-ECC /
cost-sensitive deployments paying nothing (ADR-067: "gate it so it does not regress turn
latency"). **Operator guidance**: enable `keyword` whenever `talk_loop`/`anchor_causal`
is on — the sync tier is cheap enough that keyword is the natural companion to the ECC
loop, and the graph view is inert without it. We default `Off` rather than auto-on to
match the conservative `talk_loop=false` precedent and avoid touching every deployment,
but the daemon logs a hint when the loop is on and classification is `off`.

---

## 3. Data flow (sync tier)

```
agent turn ──▶ SessionTier::index_turn(conv, seq, kind, role, text)
                 │  (mode != off)
                 ├─▶ TurnClassifier::classify(role, text)  ──▶ ClassificationVector
                 │        intent, topic, emotion{v,a,d,label}, goal
                 ├─▶ dual_write_turn(..., classification=blob,
                 │                    emotion=label, goal=goal)
                 │        · causal node.metadata["classification"] = blob
                 │        · causal node.metadata["text"] = text        (§6 piggyback)
                 │        · EmotionCause / GoalMotivation crossrefs (if present)
                 └─▶ (mode == full) enqueue EnrichJob → daemon drain task ─┐
                                                                            │ Phase B
   conversation.graph RPC reads node.metadata["classification"] verbatim ◀─┘
                 └─▶ GUI: topic→hue, emotion→VAD badge, intent→glyph (ADR-067 D6)
```

---

## 4. What is reused vs new

**Reused (no destructive change):**
- `KeywordClassifier` + `PATTERNS` (`clawft-core/src/pipeline/classifier.rs:96`) — the
  substring-pattern idiom and keyword lists seed the topic/intent heuristics. Stays on
  the routing path untouched; the turn classifier borrows the *pattern*, not the struct
  (different crate, different output type).
- `tokenize` + stop-word list (`clawft-graphify/src/conversation.rs:248`) — the topic
  token extractor. Promote to a shared helper or copy the ~20-line idiom.
- `Classifier` backend trait (`llm_classifier.rs:106`) — reused by the Phase-B enrich
  task for the cheap-model round-trip. `LlmClassifierRouter` / `ClassifierOutput` stay
  **exactly as-is** (routing contract, directive).
- `CrossRefType::{EmotionCause, GoalMotivation}` — no enum change (`#[non_exhaustive]`).
- `conversation.graph` RPC `classification` field — no wire change for the sync tier.

**New:**
- `turn_classifier.rs` (trait + keyword impl + taxonomy types + lexicon).
- `classification: Option<&Value>` param on `dual_write_turn`; `text` into metadata.
- `ClassificationConfig` / `ClassificationMode` on `AgentAnchorConfig`.
- Spawn-node classification in `subagent.rs`.
- (Phase B) enrich queue + drain task + `CausalGraph::merge_node_metadata`.

---

## 5. Floor-arousal readiness (ADR-062 D4)

`compute_urgency` consumes emotion **arousal**. The blob guarantees
`classification.emotion.arousal ∈ [-1,1]` on every node (default `0.5`). Provide a small
pure extractor `arousal_of(node) -> Option<f32>` so voice Phase 1 can wire floor scoring
without reshaping the blob. Keyword arousal is flagged low-confidence via `tier`; the
floor may weight `tier == "voice"` higher than `tier == "keyword"`. This is *readiness*,
not a wire-up — the floor change is a separate voice-phase item that depends on this.

---

## 6. Piggyback fix: `text` in node metadata

`dual_write_turn` omits `text` from the metadata JSON, so the RPC's `text` field
(`daemon.rs:5836`) is empty and node-inspect (ADR-067 D4) shows nothing. Since this
change already extends that exact metadata write, add `"text": text` in the same edit.
Small, in-scope, unblocks node-inspect. (If a privacy concern exists for text-at-rest,
gate it behind the same `classification` mode or a `store_text` flag — note for review.)

---

## 7. Riskiest calls (the three to watch)

1. **Async LLM patch = mutable causal-node metadata (Phase B).** No
   `set_node_metadata`/merge exists on `CausalGraph`; adding one risks racing with
   `set_node_state`'s `metadata.state` writes, and every patch mints a new state-at-T that
   the ADR-067 P1-graph snapshot-diff renders as a mutation. *Mitigation*: a single
   `merge_node_metadata(id, patch)` that merges keys under the node's existing lock (never
   clobbers `state`/`uid`); treat "classification refined" as an intended scrubber event;
   **defer the whole tier to Phase B** so Phase A ships without touching graph mutability.
2. **Keyword emotion/goal honesty vs the floor.** A lexicon VAD is crude; feeding a
   guessed arousal into floor urgency could mis-prioritise the conversation. *Mitigation*:
   conservative arousal default (`0.5`), `tier` provenance so the floor down-weights
   keyword vs voice, and `goal` left `null` rather than guessed.
3. **Topic-cluster flicker + default config reach.** Per-turn top-token topics flicker,
   destabilising the GUI hue clusters; and defaulting `mode` on would add work to every
   deployment's turn path. *Mitigation*: topic-continuity carry heuristic (inherit unless
   material shift); LLM tier stabilises topic in Phase B; `mode` defaults `Off`, sync tier
   only, gated.

---

## 8. Testing strategy

- **Unit (taxonomy)**: `KeywordTurnClassifier` deterministic outputs — `?`→Question,
  imperative→Request, "actually/no"→Correction, "thanks/hi"→Social; exclamation/caps→high
  arousal; lexicon valence bounds; topic = expected top token; goal stays `null`.
- **`index_turn` integration**: after `index_turn` with `mode=keyword`, the causal node's
  `metadata.classification` is non-null with all four axes; `EmotionCause`/`GoalMotivation`
  crossrefs present iff label/goal non-empty; `metadata.text` populated.
- **RPC surfacing**: extend `conversation_graph_scopes_and_shapes` (`daemon.rs:6411`) to
  assert nodes carry a populated `classification` and non-empty `text`.
- **Spawn-node classification**: `spawn_goal` node carries `classification.goal ==
  SpawnSpec.goal` and a `GoalMotivation` crossref.
- **Floor-arousal readiness**: `arousal_of(node)` returns a value in range for a
  classified node and `None`/default for a legacy node.
- **Phase B**: enrich patch flips `classification.tier` keyword→llm; RPC reflects the
  refined blob on next read; re-patch is idempotent; `merge_node_metadata` preserves
  `state`/`uid`.

---

## 9. Deferred (not v1)

- Embedding-based topic cluster ids (reuse the SessionView turn embedding + a clusterer) —
  v1 topic is a string; GUI hashes it.
- Emotion `dominance` from text (keyword can't; LLM/voice only).
- Retro-classification of pre-existing nodes (v1 classifies forward from enablement).
- Multi-turn goal-thread inference (v1 goal is per-turn signal or spawn goal only).
