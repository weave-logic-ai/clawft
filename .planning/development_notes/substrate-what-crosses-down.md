# What crosses down — memory design for the engagement, in ruvector/ruflo terms

**Date**: 2026-09-02
**Status**: **RESEARCH. Nothing proposed here is built.**
**Supersedes** this file's first version (same day), which was written to a wrong premise —
*"WeftOS is being asked to be the ground."* **It is not.** WeftOS is a **concept garden to mine**,
not a dependency, not a runtime, and not the substrate. The engagement lead's constraint:
*"it is hard to sell to the client."*

⚠ **HARD BOUNDARY ON EVERYTHING BELOW: no recommendation may require WeftOS to run.** The
architecture that reaches a client-facing artifact stands on **ruvector and ruflo**, which are
already in the stack. WeftOS appears here **only in the left-hand column of §2** — as a place that
has already thought about a problem.

**Every claim is labelled TRUE TODAY (measured, with a path) or PROPOSED.** A third label,
**DECLARED-BUT-INERT**, is used heavily and deliberately: *a stub that returns a success-shaped
result is worse than an absence.*

---

## 0. The headline, because it inverts the expected recommendation

**I expected to find that ruflo lacks the primitives this design needs and to propose building
them. That is not what the measurement says.**

**Three of the four mechanisms this design needs are already written inside ruflo, and all three
have zero callers.**

| what the design needs | where it already exists in ruflo | state |
|---|---|---|
| **a support threshold** — a lesson must recur before it is trusted | `SONAOptimizer`: seed `0.5`, `+0.1×(1−c)` per success → **3 successes reaches 0.6355** against a consumption gate of `>= 0.6` (`sona-optimizer.js:216-232`, `:304`) | ⚠ **`getRoutingSuggestion` has ZERO callers.** The tallies accumulate; no decision ever reads them |
| **decay / abscission** | `SONAOptimizer.applyTemporalDecay()` — real exponential decay, deletes below `MIN_CONFIDENCE` (`sona-optimizer.js:395-415`) | ⚠ **zero callers anywhere in the dist tree** |
| **real distillation** (generalising a lesson out of instances) | `structured-distill.js` — 124 lines implementing the arXiv:2603.13017 four-field schema (`summary`/`detail`/`labels`/`paths`), a 45-verb action vocabulary, a path-anchor regex | ⚠ **zero importers.** `grep -l structured-distill` returns only the file itself |

**So the recommendation is not *build*. It is *wire*, and the ask is small enough to evidence.**

⚠ **But wiring an inert control is not free — you inherit whatever was wrong with it.** The support
count above is currently **unreadable by anything**, because `LocalReasoningBank.findSimilar`
overwrites the learned `confidence` with the query's cosine score on the way out
(`intelligence.js:485-490`). Its own downstream gate — `if (match.confidence < 0.5) continue`,
commented *"Only distill from high-confidence matches"* (`intelligence.js:246`) — is therefore
**testing query similarity, not learned confidence.** It cannot filter for well-supported patterns
because it never sees a support number. That is one line, and it is load-bearing.

**And what ruflo ships as "distillation" today is a verbatim copy.** `intelligence.js:896-911`:

```js
reasoningBank.store({
    type: step.type,
    content: step.content,                              // ← the "pattern" IS the trajectory step
    confidence: verdict === 'success' ? 0.8 : 0.4,      // ← inside if(verdict==='success'); 0.4 unreachable
```

**The pattern is the instance.** Nothing is generalised away, and every pattern is born at exactly
`0.8`. Over MCP the stored "pattern" is literally the string `"<action> → <result>"`
(`hooks-tools.js:2658-2662`). **The module that would do the real thing is sitting beside it,
unimported.**

---

## 1. Corrections carried forward from the first version (all measured 2026-09-02)

These survive the reframe because they are measurements, not architecture.

**1.1 The four/five stores are empty, and that is a different diagnosis than "landfill."**

| store (under `~/Clients/ctox/sansone`) | size | state |
|---|---|---|
| `.claude/memory.db` | 4,096 B | SQLite header, **zero pages**; `sqlite3` refuses it. Created 2026-06-11 14:12 |
| `.swarm/memory.db` | 4,096 B | byte-identical situation, **same minute** |
| `data/memory/memory.db` | 643,072 B | 31 tables; **7 rows**, all 28 other tables zero. Last written 2026-08-21 |
| `data/memory/agentdb-memory.db` | 876,544 B | 31 tables; **46 rows in `memory_entries`, every other table zero.** Written today 08:39 — **live** |
| `agentdb.rvf` | 162 B | **0 vectors** |

**The zeros are the finding.** In the store written to *daily*: `provenance_sources`,
`recall_certificates`, `episodes`, `skills`, `tiered_memory`, `consolidated_memories`,
`consolidation_runs`, `justification_paths`, `reasoning_patterns` — **all zero, always.**

⚠ **So the engagement does not have four landfills. It has an empty warehouse with a complete
filing system** — and that is worse, because a landfill announces itself and this does not. All
five stores report healthy.

**And the 162 bytes now has an explanation:** that is agenticow's constant branch size — a COW root
manifest. `~/weftos/agentdb.rvf` is the same 162 bytes. **Two projects, two empty COW roots.**

**1.2 `.brain/` is 2,588 records** (`wc -l corpus.jsonl`), layered 1101 plan / 582 sop-digest /
**301 sop-source — verbatim client SOP text** / 221 methodology / 140 extraction / 100 current-state
/ 93 engagement / 50 knowledge-graph. **No per-record confidentiality field.** Tiering is by layer
name and convention.

**1.3 `MEMORY.md` is over budget and silently truncating — TRUE TODAY, observed live this session:**
*"MEMORY.md is 25.7KB (limit: 24.4KB) — index entries are too long. **Only part of it was
loaded.**"* This matters in §4: it is the measured instance of an **evergreen** store accumulating
damage.

---

## 2. The parity mapping — WeftOS concept → ruvector/ruflo primitive → what the engagement needs

**⚠ The middle column decides everything.** A WeftOS concept with **no** ruvector/ruflo equivalent
is **inspiration only** and must not enter a recommendation — there is nothing the client already
has to build it on.

| # | WeftOS concept | ruvector / ruflo primitive doing the same job | verdict for the engagement |
|---|---|---|---|
| 1 | **ExoChain** — hash-linked tamper-evident ledger | **NONE.** ruvector's RVF witness chain is not reachable: `verify-witness` is in the ruvector README but **absent from the shipped CLI**; `rvfBranch()`/`rvfFreeze()` in `rvf-wrapper.js:100-106` call `store.branch()`/`store.freeze()`, which **do not exist** on the store | ⚠ **INSPIRATION ONLY.** And WeftOS's own version is weaker than advertised — its in-store chain hashes a *format string* (`"ingest:count={},epoch={}"`) with a self-declared non-cryptographic function, and `verify_integrity()` hardcodes `signature_verified: None`. **Use git instead — §5** |
| 2 | **ADR-057 read ACL** — path-glob, structural deny-by-default, typed non-existence-leaking denial | **ruflo namespaces.** Real SQL predicates, not advisory: `WHERE … AND namespace = ?` at `memory-initializer.js:2137, 2267, 2343`; HNSW post-filter at `:659`. **No cross-namespace leak** | ✅ **ADOPT — with two traps.** (a) `searchEntries` defaults to `'all'` for library callers (`:2062`); the MCP handler always passes one, so **the isolation is a property of the caller.** (b) ⚠ **HNSW namespace filtering is a POST-filter over a global top-2k** (`:653-659`) — a small namespace in a large store **silently under-returns, and a partial result never triggers the brute-force fallback** (`:2120-2128`). That is the failure that reads as "the store lost my data" |
| 3 | **`clawft-cow-memory`** — branch/fork/promote/lineage | **agenticow** — same idea, same author lineage. `branch`, `checkpoint`, `diff`, `lineage`, `promote`, `rollback`, `speculate` | ⚠ **NOT AVAILABLE TODAY — see §2.1. And not portable even when installed** |
| 4 | **`promote()` gated by human review** (WeftOS `with_turn_checkpoint`, review mode) | **agenticow `promote`** — the entire guard is an `instanceof` check plus "did you enable tracking" (`agenticow/src/index.js:476-489`) | ⚠ **NO GATE EXISTS.** Over MCP the ADR-171 clearance ladder collapses to two rungs: default → promotes on score alone; `requireClearance:true` → **can never clear**, because MCP inputs are JSON and the gate wants a callback. **A gated promotion is yours to build** |
| 5 | **Confidentiality tier on a record** | **NONE anywhere.** `grep -rn "confidential" weftos/crates/` → 0. ruflo has no classification field either | ⚠ **INSPIRATION ONLY — and §3 turns this into the design, not a gap** |
| 6 | **ADR-064 egress PII scrub** (WeftOS: proposed, 0 lines) | **aidefence / `transfer_detect-pii`** — exists and runs | ⚠ **UNUSABLE FOR THIS PURPOSE — measured, §2.2** |
| 7 | **`weft memory export/import`** (WeftOS: **import writes nothing**) | **`memory_export` / `memory_import`** — genuinely round-trip **values**, and **re-embed on import** (`memory-tools.js:1252`, `:1301`) | ✅ **ADOPT, knowing what it drops: tags, TTL, embeddings, timestamps.** `includeVectors` and `format:'csv'` are **accepted-and-ignored parameters** (`:1218`, `:1268`) |
| 8 | *(no WeftOS equivalent)* | **`memory_import_claude`** — resolves `~/.claude/projects/<hash>/memory/*.md` across five hash schemes, **splits on `## ` headings into one row per section**, **SHA-256 content dedup** across duplicate project dirs, and **enforces `excludeFilePatterns` globs** (`memory-tools.js:710-884`) | ✅ **THE STRONGEST THING IN THE STACK, and it is ruflo-only.** It is both the load path for the 100 session-memory files **and the confidentiality lever** — the exclusion is a real enforced predicate, unlike doc 07's list (§6). ⚠ Value is **truncated at 4096 chars**, silently |
| 9 | **RVF as a self-contained provenance-carrying container** | **`.rvf` carries vectors and ids only** | ⚠ **FALSE AS A CAPABILITY — §2.3** |
| 10 | **ECC decay / geometric shadowing** (WeftOS: `prune_old_edges` replaced by real decay, `causal.rs:2191`) | **`SONAOptimizer.applyTemporalDecay()`** — real, and **inert** | ✅ **WIRE IT — this is §4's abscission layer, already written** |
| 11 | *(no WeftOS equivalent)* | **`SONAOptimizer` success/failure tally** → 3 successes clears the 0.6 gate | ✅ **WIRE IT — this IS the two-attestation rule, §3** |
| 12 | *(no WeftOS equivalent)* | **`structured-distill.js`** — the real four-field distiller | ✅ **WIRE IT — this is reclamation, §4** |

### 2.1 ⚠ Every `agenticow_*` tool in this session is DECLARED-BUT-INERT and returns `success: true`

`agenticow` **is not a dependency of the installed CLI** — not even optional. The tool definitions
ship; the library does not. Every handler opens with the same two lines
(`agenticow-tools.ts:56-58`, nine times):

```js
const api = await loadAgenticow();
if (!api) return degradedResult('agenticow-not-found');
```

and `degradedResult` (`agenticow-loader.ts:66-68`) returns **`{ success: true, degraded: true, reason: 'agenticow-not-found' }`**.

**A caller checking `success` gets a green light from a store that does not exist.** The header
comment states it as deliberate — *"so callers see one contract regardless of install state."* The
tell is the `degraded` key and nothing forces you to look at it. ⚠ **This is the project's own
signature defect class, sitting inside the tool list I was handed to design with.** One call
falsifies it: `agenticow_status` on any path.

**And even installed, three findings kill the "hand someone a branch" idea:**

- ⚠ **A branch is NOT a portable package.** `save()` writes `path.resolve(n.path)` — **absolute
  paths** (`index.js:605`) — and `load()` reopens them verbatim with no rebasing (`:630`). Move the
  directory *on the same machine* and the chain breaks. There is no relative mode.
- **The 162 bytes contain no data.** Reading a branch requires every ancestor `.rvf` up to and
  including the full base. **Constant size and portability are the same coin** — 162 bytes is cheap
  precisely because the data is elsewhere.
- **Text payloads live in the manifest JSON, not the store** (`index.js:85`, `:614`), forced from
  below because RVF carries no per-record metadata (§2.3). **The manifest is not an index over the
  data; it is part of the data.**
- **No GC, no TTL, no prune, no expiry** — `grep -niE "gc|ttl|expire|prune|evict|retention"` over
  `agenticow/src` returns **zero matches.** Every `checkpoint()` leaves a frozen node on disk
  permanently and mints a new random-suffixed working file. **Chain depth grows monotonically and
  exact-mode `query()` hits every node**, so read latency degrades with checkpoint count.
- **Its own headline numbers, refuted by ruflo's own committed benchmark**
  (`ruflo/docs/agenticow/findings.md`, darwin-arm64, harness and raw runs committed): **162 bytes —
  confirmed, exact and constant.** *"~0.5 ms"* — **not reproduced**, ~9–10 ms fixed, 20× off.
  *"83× faster"* — **not reproduced**; **below N ≈ 30,000 a full copy is faster.** Plus an
  undocumented **36-second first-query warmup** at N=50k. ✅ Credit where due: ruflo carries the
  corrected numbers in its own tool description.

### 2.2 ⚠ aidefence cannot be the stripping step. Verified by execution, not inferred.

This is the most consequential negative finding in the report, because **question 2 depends on it.**

Detection is **100% regex**, 938 lines, no model. **Six PII patterns, five types**
(`threat-detection-service.js:264-294`): `email`, `ssn`, `credit_card`, two `api_key` shapes,
`password`. **No phone. No address. No names. No custom-term registration** — `PII_PATTERNS` is a
module-level `const` with no constructor parameter, no config, no env var. **Adding a term is a
fork, not configuration.**

Executed against the installed package:

| input | `piiFound` |
|---|---|
| `The Sansone Group` | **false** |
| `Project Alkaline` | **false** |
| `SO-434` | **false** |
| `Nick Sansone approved the deal, 4200 Bose Industrial Park` | **false** |
| `fee split for Melissa is 12%` | **false** |
| `call me at 314-555-0134` | **false** |
| `john@x.com` / `123-45-6789` / `sk-ant-…` / `password: hunter22` | true |

**It catches credential-shaped and format-shaped strings. It catches nothing that identifies a
client.**

⚠ **And it is intermittently wrong about the things it does catch.** `PII_PATTERNS` carry the `/g`
flag and `detectPII` calls `.test()` on the shared module-level objects, which **advances and
persists `lastIndex`**. Executed three times on one email-bearing string: **`true, false, true`.**
Any `scan`/`analyze`/`is_safe` call anywhere in the process perturbs the state the next
`has_pii` reads, and the MCP server caches one defender for the process lifetime. **This is a
confirmed intermittent false negative in the only PII tool ruflo exposes.**

⚠ **And `safe` ignores PII entirely** — computed from prompt-injection threats only
(`threat-detection-service.js:333-340`), so the library's own `isSafe()` returns `true` for a
document full of SSNs. The **single** enforcement site in the entire codebase
(`wasm-agent-tools.js:762-781`) calls `defence.scan()` — **a method that does not exist** — throws,
is swallowed by the surrounding `catch`, and **the import proceeds.** The one gate fails open.

**Design consequence, and it is the good news hiding in a bad measurement: because no usable
scrubber exists and none can be configured without a fork, the answer to question 2 cannot be a
scrubber. It has to be structural.** §3 was already going there; this measurement removes the
alternative rather than merely disfavouring it.

### 2.3 ⚠ An `.rvf` cannot carry per-record metadata. It is silently dropped.

Three layers, each narrower:

- **Type level:** `metadata?: Record<string, RvfFilterValue>` where the value is
  `number | string | boolean` (`@ruvector/rvf/dist/types.d.ts:167`, `:48`). **Flat scalars. No
  nesting, no arrays.**
- **Write level:** `NodeBackend.ingestBatch` never passes it. The comment names the parameter it
  then omits — `// NAPI signature: ingestBatch(vectors, ids, metadata?)` at `backend.js:118`,
  followed by `this.handle.ingestBatch(flat, ids)` at `:136`. **No third argument.**
  `entries[i].metadata` is read nowhere.
- **Read level:** `RvfSearchResult` is `{id, distance}` — **no metadata field**, and
  **`RvfDatabase` has no `get(id)` method.** You cannot retrieve a stored record, only its id and
  distance.

The format reserves the slot (`META_SEG 0x03`); the SDK never fills it. ⚠ **Treat "RVF carries
metadata" as a format claim, not a capability.** Ids themselves need a sidecar
(`<store>.rvf.idmap.json`), and `saveMappings` **swallows every error** with a bare `catch {}`
(`backend.js:394-396`) — **a read-only or full filesystem loses your identifiers with no error and
no warning.**

**This reconciles with what the engagement already does:** `.brain/dependency-graph.rvf` sits beside
`.idmap.json` and `dependency-graph.json`; `sansone-brain.rvdb` sits beside `corpus.jsonl`. **Every
working store here already keeps its payload in a sibling.** That is not a workaround; it is the
only shape available.

---

## 3. Questions 1 & 2 — and they are one mechanism

**Q1: what generalises. Q2: how stripping becomes auditable.** The lead's abscission message says
these are the same operation from two ends, and the mechanism below is why.

### 3.1 The test — three predicates, cheapest first (PROPOSED)

1. **Entity-free rewrite** *(mechanical, refuses most)*. Replace every named entity — client,
   person, repo, path, ticket id, table, column, product — with a typed placeholder. If the rewrite
   stops being a complete claim, it was a fact about a system.
   *"a `state:` tag on this board is reliably stale"* → *"a ⟨tag⟩ on ⟨board⟩ is reliably stale"* —
   now **false as a general claim.** Refused. The refusal **names the entity it could not remove.**
2. **Falsifiable check** *(cheap, refuses platitudes)*. The lesson must name an assertion that could
   come back either way. *"Be careful"* names none; *"a guard is two independent decisions — scope
   and predicate — and both need measuring"* names two. **Can you write the assertion?**
3. **Two disjoint attestations** *(the design)*. Fired twice, in two contexts not sharing the
   stripped entity. **One instance is an incident; two disjoint instances is a defect class.**

**✅ Predicate 3 is already implemented in ruflo and inert.** `SONAOptimizer` seeds at `0.5`, adds
`0.1×(1−c)` per success → 0.550, 0.595, **0.6355** — and the consumption gate is `>= 0.6`
(`sona-optimizer.js:304`). **Three successes before a pattern may influence a decision.** That is
the only promotion threshold anywhere in the stack, and **`getRoutingSuggestion` has zero callers.**

⚠ **Two things must be fixed to wire it, and they are small but not cosmetic.** (a) `findSimilar`
must stop overwriting `confidence` with the cosine score (`intelligence.js:485-490`), or the support
count remains unreadable. (b) `usageCount` counts **reads**, not validations — it is incremented
*inside the search function* — so it is not a support count and must not be used as one.

### 3.2 Why predicate 3 IS the stripping step

**What crosses is not a redacted lesson. It is the intersection of two attestations from disjoint
contexts — and the intersection cannot contain either context's particulars, because it had to be
true of both.** You do not remove the client name; you compute what survives, and the client name
cannot.

*"We removed the client names"* is unauditable for a structural reason: **it is a claim about an
absence, and a missed name looks identical to one that was never there.** Every reviewer who
inspects the output and finds nothing has confirmed the claim without testing it.

**The auditable version is a derivation, and it is reproducible:** a reviewer re-runs the
intersection over the two submitted attestations and gets the same record, or does not.
*Reproducible* is what auditable means.

Three supports, all PROPOSED:

- **Commitment, not pointer.** Store `H(context_id ‖ salt)` per attesting context. The store proves
  **two disjoint contexts attested this** without naming either; the originating engagement can
  later prove *"that one was us"* by revealing its salt, voluntarily. Standard construction, no
  novel cryptography.
- **No free-text field.** ⚠ **If there is anywhere to put a client name, one eventually will be
  there.** The promoted record accepts a closed vocabulary and refuses anything outside it *at the
  door*. **Honest cost: this will refuse real lessons, including good ones, because they cannot be
  said in it.** That is the intended trade, and after §2.2 it is not a trade against a working
  scrubber — it is a trade against nothing.
- **A process boundary, not a flag.** ✅ **This one has a real primitive: ruflo namespaces are
  enforced SQL predicates** (§2, row 2). Promotion is a *write into a different namespace*, not a
  flag on a row — with the HNSW post-filter caveat understood.

---

## 4. Question 4, reframed as the lead framed it: abscission

**"What does the store refuse" needs a gatekeeper who knows the future. "What does the store let
decay" does not.** The second is the better question and I am answering that one.

### 4.1 ⚠ Abscission is a built mechanism, and digital decay is worse than biological decay

A tree does not lose leaves by failing to hold them — **it builds an abscission layer and cuts them
off on a trigger.** A memory system that merely accumulates is not deciduous; it has no abscission
layer at all. **That is all five stores in §1.1 today.**

⚠ **But the metaphor breaks in exactly one place and it is the dangerous place: nutrients genuinely
return to the tree; a deleted row does not.** Digital "decay" that is really scheduled deletion is
irreversible in a way leaf fall is not. **So if reclamation silently fails and abscission proceeds,
you have built a system that deletes its own lessons on a schedule and reports success.** That is
this estate's signature defect class — the control that fails in the direction nobody checks.

**The answer is not more care. It is to make the drop structurally impossible without the
reclamation.** ⚠ **Do not verify reclamation as a step in a script — make it a precondition with a
referential constraint.** A record may transition to `dropped` only if a reclaimed record exists
that **cites it by id**. No reclaimed record, no drop — the transition refuses, and the refusal is
the alarm. A scheduled job that finds nothing to reclaim then does nothing, loudly, instead of
succeeding quietly.

**ruflo's `memory_cleanup` already has this shape's dangerous half and not its safe half.** It is
real TTL reaping (`memory-tools.js:1130-1185`) — but it **defaults to `dryRun: true`**, **nothing
sets a TTL by default** so the candidate set is normally empty, and its `stale` and `lowQuality`
counts are **hardcoded zeros**, deferred to *"the agentdb consolidation curator"*, which is
`createConsolidationStub()` — a function returning `{promoted: 0, pruned: 0}` unconditionally.
⚠ **The moment those zeros become real numbers without the reclamation gate above, this is the
schedule that deletes lessons and reports success.**

*(And `memory_compress` is not a compressor at all: it lists entries, sums their sizes, and returns
`{before, after}` as **the same object reference** with `ratio: 1, method: 'none'` and a `note`
admitting it — `memory-tools.js:1186-1207`.)*

### 4.2 A decomposition model, not a retention policy

**Retention asks "keep or delete." Decomposition asks "what does this become as it ages."** Litter
does not decompose uniformly — sugars go fast, **lignin persists for years** — and that maps
directly.

**A record is not kept or deleted. It loses fractions on a schedule.** Three fractions, each with
its own half-life:

| fraction | examples | behaviour |
|---|---|---|
| **labile** | a sha, a row count, a date, a file path, `SO-434`, a person's name | **decays fast, and should.** ⚠ This is also, exactly, the confidentiality-bearing fraction — which is why decay and stripping are one mechanism, not two |
| **structural** | the check that was run, the assertion that bit, the shape of the repro | intermediate — survives long enough to be re-attested, then compresses into the next fraction |
| **lignin** | *"derived metadata beats primary text in a careful reader's head, and that is a defect class"* | **persists as structure long after the incident that produced it is unrecoverable** |

**What remains after full decomposition IS the generalisation.** That is the lead's point made
mechanical: **you do not decide whether to keep the leaf; you decide what to pull out of it before
letting it go** — and the fraction that decays fastest is the fraction that must not travel anyway.

✅ **The reclaimer already exists and is unimported:** `structured-distill.js`'s four-field schema
(`summary` / `detail` / `labels` / `paths`) is a near-exact fit — **`paths` and `detail` are the
labile fractions, `summary` and `labels` are what persists.** Wiring it is the single highest-value
change in this report.

### 4.3 Deciduous or evergreen — the strategy, and what it costs

Both are real strategies with a real trade: deciduous sheds accumulated parasites and damage at
seasonal cost; evergreen is cheaper per season and **accumulates damage.**

⚠ **The engagement's `MEMORY.md` is evergreen and has accumulated damage — measured live in this
session: 25.7KB against a 24.4KB limit, and "Only part of it was loaded."** A memory index that
silently truncates is the exact failure mode the strategy predicts.

**PROPOSED: deciduous INDEX, evergreen RECORDS.**

- **The index is dropped and regrown every cycle** from what the records reclaimed. It is cheap to
  regrow because it is **derived** — and the engagement already has a hard rule that anything
  reporting a count or a list is *derived, never transcribed.* An index that is rebuilt cannot
  silently exceed its budget, because the rebuild is where the budget is enforced.
- **The records persist** and decompose by fraction (§4.2) rather than being dropped.

**Cost, stated plainly:** the regrow step is a real job that must run and can fail; a deciduous index
is briefly *absent* rather than *stale*, which is a different failure to design for. **The
compensating property is that absence is loud and staleness is silent**, and this project has been
bitten by the silent one repeatedly.

### 4.4 Where the litter lands, given WeftOS is not the substrate

**The compost has to fall somewhere that already exists, that the client already has, and that
outlives the engagement. That is git.**

The repo already provides everything the discarded substrate was going to be asked for:
**durability past the engagement, portability with no new component, authorship and timestamps on
every change, a tamper-evident history, and review as the promotion gate.** ✅ **It also satisfies
the client constraint by construction — there is nothing to sell.**

So: **reclaimed records land as committed files in the repo; the live store is a cache over them.**
That inverts today's arrangement, where the store is the original and nothing durable exists. It
also means the promotion gate (§5.7) is a pull request — a mechanism the engagement already runs,
already trusts, and already requires a human for.

⚠ **What git does not give you is recall.** It is a substrate, not an index. The store stays, and
`memory_import_claude` (§2, row 8) is the load path from files back into it.

### 4.5 What the store still refuses

Decay replaces most of the gate, but not all of it. Each refusal is **typed, carries a reason, and
returns to the submitter.** ⚠ **A silent drop is forbidden, and an empty result must never stand in
for a refusal** — empty-because-no-match and empty-because-refused are different values.

`insufficient_attestation` (one instance — §3.1.3) · `attestations_share_context` ·
`unstripped_entity` · `no_checkable_claim` · `unwitnessed` · `machine_may_not_promote` (machines
submit, humans promote — ✅ precedent already **built** in the engagement's `refuseDefinitionActor()`,
and now naturally expressed as "a human opens the PR").

**And one that must NOT be a refusal:** a re-assertion of an existing class **attaches an attestation
and raises the count** — that is the learning mechanism, and it is what makes the store improve by
getting *denser* rather than longer. ⚠ Measured: **the only dedup in ruflo is content-hash matching
in `LocalReasoningBank.store()`, and the MCP path bypasses it** by minting a fresh
`pattern-${Date.now()}-${random}` key per call (`hooks-tools.js:2806`) — so identical text stored
twice becomes two rows.

**And what it must never refuse: a lesson that contradicts one it holds.** Contradiction means the
class was drawn at the wrong boundary. Hold both, link them, mark it **contested.** The
characteristic failure of every knowledge base is converging on its first answer.

---

## 5. The three propagation models, costed against ruvector/ruflo

### Model A — SEED / NUT: a portable package establishing an independent tree

**The design insight is the endosperm, not the genome.** A seed carries the whole genome **plus a
starter food supply** — enough to establish before it can photosynthesise. **A memory package handed
to a new engagement needs the lessons AND enough worked context to be usable before that engagement
has generated any of its own. A bare index of abstractions is a seed with no endosperm: genetically
complete, and it starves.** ✅ **This is exactly the shape of the task packages on the Liber side —
say so there.**

**Cost against the stack:**
- ❌ **agenticow branches cannot be the package** — absolute paths, delta-not-package, manifest-is-data
  (§2.1). And unavailable today.
- ❌ **A bare `.rvf` cannot be the package** — no metadata, no `get(id)`, ids in a sidecar whose write
  errors are swallowed (§2.3).
- ✅ **`memory_export` → `memory_import` IS a seed format, and it is the only one.** It round-trips
  values and **re-embeds on import** — a real write, not a validator. ⚠ It drops **tags, TTL,
  embeddings and timestamps**, and `includeVectors` / `format:'csv'` are accepted-and-ignored.
- ✅ **Better still: the package is a committed file** (§4.4), loaded by `memory_import_claude` with
  its enforced `excludeFilePatterns`.

**Verdict: SUPPORTED, weakly, and only by `memory_export`/`import` plus git — never by agenticow.**

### Model B — CLONAL COLONY: many stems, one organism, shared roots

⚠ **The lead's example needs correcting and the correction is the design point.** The canonical
clonal colony is **quaking aspen — *Populus tremuloides*, Pando** — 47,000 genetically identical
stems over 106 acres from one seed up to 80,000 years ago, propagating by **root suckering** driven
by cytokinin. **Pines generally reproduce sexually, by seed** — so a stand of white pines is Model A
repeated, not Model B. **The concept is right; the two mechanisms are genuinely different and the
difference IS the choice.**

✅ **And Pando supplies the detail that makes the model worth having: individual stems rarely live
past 100–150 years, while the organism persists for tens of thousands.** That is precisely the
relation being designed — **engagements are stems and they die; the root system is what must
outlive them.**

**It also reconciles the lead's two statements.** *"We don't need a forest of trees, but I am not
against liber being able to handle that"* — **a clonal colony is not a forest. It is one organism
with many stems.** Multiple engagements sharing a root system would be **one knowledge organism**,
not many, which is a different and cheaper claim than "copy Liber into each repo."

**Cost against the stack:**
- ✅ **Per-stem scoping is real** — ruflo namespaces are enforced SQL predicates (§2, row 2).
- ⚠ **But the SHARED ROOT is broken by construction.** The store resolves as `resolve('.swarm')` —
  **CWD-relative** (`memory-initializer.js`). Under this engagement's one-worktree-per-lane rule,
  **N lanes means N divergent `.swarm/memory.db` files.** That is not a colony; it is N seedlings
  that look like a colony from the tool surface.
- ⚠ **And recall over a small namespace in a large shared store silently under-returns** (§2, row 2).

**Verdict: the ISOLATION primitive is real; the SHARED part is not. Making this model work is a
store-location decision (one absolute path, not CWD-relative) before it is anything else.**

### Model C — MYCORRHIZAL NETWORK: separate trees, mediated exchange

Separate engagements that **cannot** share client data, connected by a layer that moves generalised
nutrient between them without merging them. **The fungus is the stripping-and-transfer layer, and it
is a third party to both trees** — the auditable intermediary question 2 asks for.

⚠ **Do not cite it as natural law, and the discipline here is the point.** Karst, Jones & Hoeksema,
*Nature Ecology & Evolution* 7:501-511 (2023), reviewed **more than 1,500 papers** and found the
proportion of **unsupported claims doubled over 25 years**. Specifically: that common mycorrhizal
networks are widespread in forests, and that resource transfer through them improves seedling
performance, are **insufficiently supported**; and that mature trees preferentially send resources
and defence signals to their offspring has **no peer-reviewed published evidence at all.**

**So the metaphor's own evidence base is a case study in the failure this project keeps finding: a
claim that spread by citation rather than by evidence, becoming more confident as it travelled
further from its source.** Use the shape; do not borrow the authority.

**Cost against the stack:**
- ❌ **There is no stripping primitive.** aidefence catches nothing that identifies a client, is
  intermittently wrong about what it does catch, cannot be extended without a fork, and its one
  enforcement site fails open (§2.2). **This is measured, not estimated.**
- ⚠ `transfer_detect-pii` is a **different and better** implementation — 8 categories including
  phone and paths, stateless `.match()` with no `lastIndex` bug — **but it has no SSN, no credit
  card, no password, and no names either. Neither list is a superset of the other and nothing merges
  them.**

**Verdict: INSPIRATION ONLY as a tool story — and the strongest structural idea in the report. Since
no mediating scrubber exists or can exist without a fork, the mediation must be §3.2's intersection:
the "fungus" is a derivation both parties can re-run, not a filter either party has to trust.**

---

## 6. The April WeftOS material: what survives, since it is the concept garden

Headers say 2026-04-04; git says the two surveys first appear 04-14/15 — **after the release that
implements them.** Their "current state" tables were retrospective when committed.

**Still holds:** HNSW parameters exactly (`ef_search=100, ef_construction=200, 384d`),
`instant-distance` over `hnsw-rs`, the dual `HashEmbedder`/`ApiEmbedder` split, temperature
quantization (`Hot/Warm/Cold`), workspace `0700` and symlink-escape refusal, `DateTime<Utc>` +
RFC-3339, and **all four of the phase-2 survey's "what NOT to adopt" predictions.** Its own top P0 —
*"cross-file resolution only handles Python"* — is **still open verbatim, five months on.**

**Overtaken:** nearly every P0/P1 shipped as KG-001…KG-018, often **under different names**
(`causal_trace`→KG-003, `retrieve_pipeline`→`trace_data_flow`), so grepping the docs' identifiers
reports absence wrongly. `prune_old_edges` is gone, replaced by real decay. `rvf_io.rs` was deleted,
making doc 07's entire code block unrunnable. Doc 07 specifies ONNX MiniLM and ships SHA-256 hash
embeddings. Doc 09's assessment thesis has **zero grep matches for every metric it defines**, and its
cheapest signal was archived.

⚠ **Three silent overtakings, and they are why the garden is worth walking rather than trusting:**
`clawft-core::embeddings::witness` is 592 lines, fully tested, **zero callers** — doc 07's *"every
piece of knowledge is cryptographically linked"* describes a complete, correct, **unwired** control.
`weft memory import` **writes nothing** and prints *"WITNESS chain validation: passed"* without
running. H1's own named load-bearing test, `link_shared_namespace_rejects_traversal`, is **gone from
the tree**, and the branch it guarded now has none.

### ⚠ The single most transferable lesson in the April corpus, and it is about enforcement

**Doc 07 lists seven things that must never enter the knowledge base** — `.planning/`, `.weftos/`,
sprint notes, ADRs, git history, **credentials**, test files. **The exclusions hold. There is no
exclusion predicate.** The builder takes two arguments and filters on `extension == "mdx"`; what
actually keeps `.planning/` out is **one hard-coded string in a shell script.** Change that argument
and every exclusion evaporates silently.

**That is the same failure as §2.2's fail-open gate and §2.1's `success: true`, and it is the
failure this whole design must not repeat.** ✅ **It is also why `memory_import_claude`'s
`excludeFilePatterns` matters so much: it is the one exclusion in either stack that is an
actually-enforced predicate rather than a list somebody remembered to honour.**

---

## 7. The contract with the Liber lane — reconcile before either side builds

1. **A stable opaque context id per engagement**, with a salt kept by the tree. The shared store
   only ever sees `H(context_id ‖ salt)`.
2. **Liber submits an OBSERVATION, never a memory file** — entity-free rewrite, falsifiable check,
   context commitment. The memory file stays in the tree.
3. **The tree holds single-instance candidates and must recognise a later observation as the second
   attestation of one written weeks earlier.** ⚠ **This is the hardest engineering problem in the
   design and it is on Liber's side.** The shared store can only check disjointness and count.
4. **Downward is submission; upward is read.** No exceptions — the guarantee is the *absence* of a
   path, and one exception is the same as none.
5. **Refusals return to Liber and are retained.** A refused candidate is the tree's own record that
   this was tried — and how it learns which of its lessons are local.
6. **Seed packages carry endosperm** (§5-A): a task package is lessons **plus enough worked context
   to be usable before the new engagement has any of its own.**
7. **⚠ A store-location decision, and it is Liber's to make: `.swarm/memory.db` is CWD-relative.**
   Under one-worktree-per-lane that is N divergent stores. **Model B is unavailable until this is an
   absolute path**, and this is cheaper to decide now than to migrate later.
8. **`memory_import_claude`'s `excludeFilePatterns` is the confidentiality lever** — it is enforced.
   Liber should own that list explicitly rather than inherit a default.

---

## 8. What I could NOT establish

- **The 17-row client memory table** (production). No production reach; carried as the lead's
  measurement, dated 2026-09-02, **not as mine.**
- **Per-record confidentiality of `.brain/`.** I have the layer distribution (§1.2); I did **not**
  establish that any layer is uniformly safe. `sop-source` is 301 records of verbatim client SOP
  text.
- **Whether `agenticow_*` really returns `degraded: true` in this session.** Derived from dependency
  resolution plus shipped `dist` code, **not from an observed response** — every handler creates an
  `.rvf` when the path is missing, so probing is a write. ⚠ **Falsifiable in one call**
  (`agenticow_status`) and worth making before anyone plans around those tools.
- **Whether Engine A (`@ruvector/core`, redb) returns metadata on search.** Its binary carries a
  metadata field its `.d.ts` does not declare. Settling it needs a write. **Do not build on it
  either way.**
- **Whether ruvector's published performance numbers hold** — "52,341 ops/sec", "150× faster",
  "859 tests passing" are unsourced README values and the tarball ships no test or bench directory.
  **Unverified, not refuted.** (Contrast agenticow, whose numbers ruflo actually re-measured and
  partly refuted — §2.1.)
- ⚠ **Whether the intersection rule produces USEFUL records.** It needs two real engagements and
  there is one. **§3.2 carries the weight of the whole design as an untested hypothesis** — written
  down now precisely so the second engagement can refute it cheaply rather than expensively.
- **Whether a closed vocabulary (§3.2) can express enough to be worth having.** It will refuse good
  lessons and nobody knows the ratio.
