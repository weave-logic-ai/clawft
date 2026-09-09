# S2 — Two circuits: Darwin variant score vs live fitness floors

**A calibration study of MetaHarness scoring and one overnight continue**  
**Symposium:** Liber de Ludo Aleae  
**Date:** 2026-08-14  
**Filename note:** this is the Darwin/floors report (not S1, not the sidecar note).  
**Circuit declared:** two named sample spaces (Darwin variant score; live fitness floors), plus an overnight labor loop that is *not* a third score.  
**Claim type:** mixed — *scientia* on the floors, *fortuna*-guarded on Darwin promote.  
**Does not:** graduate an LDA-ADR; does not smash EffectVector genesis; does not claim the Darwin winner improved.

Thesis of the book, applied: a score that does not name its circuit is a wager dressed as measurement. This study names the circuits we actually threw, the weights on the faces, the promote rule (the house cannot pocket a worse table), and what an overnight run did to each.

---

## 1. What we implemented

### 1.1 MetaHarness, not a vibe

MetaHarness (ADR-096/097 rhyme; Vetus V7) is used as a **frozen-model, evolving-harness** flywheel:

| Layer | What is frozen | What may change |
|---|---|---|
| Model | The executor (Grok in-session; not an xAI HTTP client for the wheels) | Nothing about weights “because the model felt it” |
| Harness | Kernel + host adapter (`@metaharness/kernel`, Darwin `evolve`) | Mutation surfaces only |
| Promote | Never silent | `finalScore > parent + 0.05` **and** safety holds **and** no test regression |

Darwin mutates one surface at a time: `planner`, `toolPolicy`, `reviewer`, `contextBuilder`, `scorePolicy`, `memoryPolicy`, `retryPolicy`. Each child is a copy-on-write variant with a receipt in `.metaharness/runs/<id>.json`. The lineage is the *n* casts (LDA-001: receipts = counted throws).

This is the opposite of `NoopScorer` (blank die) and of `BasicScorer` (length as luck). See expert `weftos-scoring-architect.md` and LDA-ADR-001.

### 1.2 The Darwin score — faces and weights

The **score policy** (mutation surface `scorePolicy`) is a weight vector over six *positive* faces. Weights are non-negative and sum to 1:

| Face | Weight | What “favorable” means |
|---|---|---|
| `taskSuccess` | 0.35 | The bounded task exited as specified |
| `testPassRate` | 0.20 | Fitness tests against the live circuit (below) |
| `traceQuality` | 0.15 | Trace is complete enough to recount |
| `costEfficiency` | 0.10 | Spend vs a declared budget |
| `latencyEfficiency` | 0.10 | Duration vs a declared bound |
| `safetyScore` | 0.10 | No secret, no destructive action, no hallucinated path, no tool-loop, no cost overrun |

Five **penalty faces** sit outside the weight vector and can zero a promotion: `secretExposure`, `destructiveAction`, `hallucinatedFile`, `toolLoop`, `costOverrun`.

`baseScore` is the weighted sum. `finalScore` applies the penalties. Promote language from a live receipt:

> promoted: finalScore 0.9850 > parent 0.0000 + delta 0.05 (safety 1.00, no test regression)

Parent `0` is only legal for the first baseline. Every later child must beat *its* parent by **0.05**. That delta is the house rule we disclose (LDA-ADR-003: undeclared edge is a house table). We do not pretend 0.05 is a derived EV; it is a published bar.

### 1.3 The live fitness circuit (not Darwin’s mock)

Darwin’s default mock traces can pass with `verify: PASS` and still tell you nothing about the work. So the harness grew a **second, named circuit**: live fitness tests (vitest). Three enumerated faces, fixtures pre-embedded (no model load):

| Face | Favorable | How it is counted |
|---|---|---|
| Index integrity | embedding rows = passage count; sampled vectors unit-norm | Exact equality; ‖v‖ − 1 < 0.01 |
| Probe recall | fingerprint probe retrieves its own title inside a 2% window (min 200) | hits / applicable probes |
| Certification rate | witnessed `CERTIFIED.ok` among manifest-scoped texts | passed / decided |

Floors live in the harness fixtures. A ratchet may **raise** a floor to `measured − 0.03`, never lower it. That is a one-way odds adjustment after the circuit is counted — not reasoning on the mean.

### 1.4 Overnight labor is not a third score

Two interpretive wheels fire as **watchable subagents** every 30 minutes (research; UI). A third loop is mechanical: curate → ratchet floors → Darwin evolve (1 generation × 3 children) → `metaharness score`.

| Wheel | Stake | Ruin check (LDA-004) | What it may write |
|---|---|---|---|
| Research | one evidenced effect + one keystone bead + one spoke pass | no certified DB writes; no invented day-counts | staging JSON, design notes |
| UI | remesure + five-expert critique | **no HTML**; log must stay under 500 lines | revision log only |
| Darwin / curate | one evolve generation | no silent promote; floors never drop | variants, floors, score snapshot |

An earlier API wheel (`grok-client` → api.x.ai) hit a monthly spend limit. That is ruin on the credit bankroll. Interpretive Grok was moved onto in-session subagents so each cast is a named thread, not a hidden HTTP die. The HTTP client remains for one-shot geocode / Moderator only.

Equal conditions (LDA-ADR-002): the UI wheel **measures** a surface it is forbidden to rewrite. The research wheel may not upgrade grain to hide a join. Both refuse free-recall as a marked card.

---

## 2. How the scoring is supposed to work

```
declare circuit
    → throw N variants / N overnight cycles
    → count faces (weights published)
    → compare to parent / floor
    → promote only on delta + safety
    → ratchet floors only upward
    → publish receipts
```

Three refusals, taken from the book and the LDA-ADRs:

1. **Do not treat a lucky child as skill.** Generation-2 mutations that score 0.545–0.765 stay in the archive. They are not “the new way we work.”
2. **Do not flatten clocks.** A converted-CE year is a *recording*, not the native frame.
3. **Do not invent the missing face.** Day-counts and skip-lengths that are not in-repo stay `role=gap`.

The 5-die **readiness** throw (`harnessFit`, `compileConfidence`, `taskCoverage`, `toolSafety`, `memoryUsefulness` + `estCostPerRunUsd`) is a **snapshot**, not EV. Vetus already said so. This study reports it as a price tag on the table, not as overnight improvement.

---

## 3. Overnight results

Window: 2026-08-13 evening through 2026-08-14 midday. Darwin keeper stayed up. Interpretive wheels ran as durable 30-minute subagents after the API path was refused.

### 3.1 Circuit A — Darwin variant score

*n* = 10 variants (1 baseline + 3 gen-1 + 6 gen-2).

| id | gen | surface | finalScore | promoted? |
|---|---|---|---|---|
| baseline | 0 | planner | **0.985** | yes (first table) |
| g1_v0 | 1 | planner | 0.985 | no (delta 0) |
| g1_v1 | 1 | toolPolicy | 0.985 | no |
| g1_v2 | 1 | reviewer | 0.985 | no |
| g2_v0 | 2 | reviewer | 0.765 | no |
| g2_v1 | 2 | planner | 0.765 | no |
| g2_v2 | 2 | contextBuilder | **0.545** | no |
| g2_v3 | 2 | toolPolicy | 0.765 | no |
| g2_v4 | 2 | scorePolicy | 0.765 | no |
| g2_v5 | 2 | contextBuilder | 0.875 | local yes; **did not beat baseline** |

| Statistic | Value |
|---|---|
| Mean finalScore | 0.842 |
| Max / min | 0.985 / 0.545 |
| Children worse than baseline | 6 / 9 |
| Children that cleared +0.05 vs baseline | **0** |
| Safety on every receipt | 1.00 |
| Published winner | **baseline** |
| Delta over baseline | **+0.000** |

**Overall improvement on this circuit: none.** That is a successful night. The gate did the job Cardano asked of a fair table: we did not lay a new wager because a child looked different. g2_v5 was allowed as a lineage node, then lost the leaderboard to baseline. The leaderboard still prints winner on baseline. Mock traces (`verify: PASS` in tens of milliseconds) are why Circuit B exists.

### 3.2 Circuit B — live fitness floors

First committed floors (2026-08-13) vs last ratchet this window (2026-08-14T11:40Z):

| Face | Floor then | Floor now | Measured then | Measured now |
|---|---|---|---|---|
| Probe recall | 0.932 | **0.970** | 0.962 | **1.000** |
| Certification rate | 0.650 | **0.935** | 0.395 | **0.965** |

| Change | Amount |
|---|---|
| Probe floor | +0.038 (+4.1%) |
| Probe measured | +0.038 to a **ceiling** (every applicable probe hits) |
| Cert floor | +0.285 (+43.8%) |
| Cert measured | +0.570 |

The 0.03 slack in the ratchet (`floor := max(old, measured − 0.03)`) is disclosed: we do not lock a floor to a single lucky certify pass. Certification was the broken die on 08-13 (measured 0.395 under a 0.650 floor — the suite could not have been honest until the ledger caught up). By 08-14 the same circuit reads 0.965 measured / 0.935 floor.

**This is the overall improvement of the method.** Not a Darwin headline number. A named circuit that got healthier, with a one-way floor so we cannot forget.

### 3.3 Readiness snapshot (not EV)

`npx metaharness score` on the overnight host, first vs last lines of the wheel log:

| Face | Early overnight | Late overnight |
|---|---|---|
| harnessFit | 50 | 49 |
| toolSafety | 100 | 100 |
| memoryUsefulness | 36 | 36 |
| estCostPerRunUsd | 0.048 | 0.048 |

Fit did not rise. Safety stayed perfect. Memory usefulness stayed low. Cost doubled vs the WeftOS snapshot in Vetus (`$0.024` → `$0.048`) because this host is a different table. We do **not** average these with Circuit A or B.

Compare Vetus (WeftOS, n=1, 2026-08-13): fit 75 / compile 100 / coverage 65 / safety 90 / memory 53 @ $0.024. Different repo, different circuit. Publishing them as one “MH score” would be a marked card.

### 3.4 Overnight labor (inventory under an evidence bar)

Not a MetaHarness face. Reported so the study does not hide what the subagents actually did while Darwin refused to promote.

Research wheel (≈ 20 half-hour casts after the API path was killed):

| Stock | Start of method (pass-1 starters) | After overnight |
|---|---|---|
| Evidenced cross-spoke effects | 5 | **33** |
| Keystone strings | 0 | **4** (flood, contested place-identity, collapse-cluster, calendar-frames) |
| Keystone beads | 0 | **40** |
| Gods candidates | 12 | **27** (incl. a dispute-gap, not a settled being) |
| Creatures candidates | 7 | **14** (class + four named) |
| Analysis spokes at pass ≥ 4 | two | **six** |

Every effect still carries a verbatim `file:line`. Gaps that would have been free-recall were left as gaps. That is calibration against the evidence bar, not a score inflation.

UI wheel: remesure rotation across public surfaces; five-expert rows as `needs-human-review`; **zero HTML**. Live critique log was split whenever it approached 500 lines. The method’s improvement here is *discipline*, not pixels: we did not spend the night rewriting the hall under a measure-only contract.

Corpus scorecard (curate, not Darwin): passages 19,737 → 53,373; certified texts ~70 → 83. Verse-alignment **fell** 35% → 13% because the denominator grew. Publishing only the passage count would be a house edge. The percentage drop is the honest face.

---

## 4. Overall improvement, stated as a method

If the question is “did MetaHarness make the Darwin winner better overnight?” the answer is **no** (+0.000 vs baseline; 6 of 9 children worse). If that disappoints, the book is working.

If the question is “did naming circuits and refusing silent promote improve the *practice of scoring*?” the answer is **yes**, on four counts that transfer off this table:

1. **Two circuits, two verdicts.** Darwin stayed put. Live floors moved a lot (especially certification). Mixing them into one “we improved 0.985” number would be ROTM’s cousin.
2. **Promote is a wager with a published bar.** +0.05 and safety=1.00. No child cleared it against baseline. The archive is full of losing tickets. That is *n*, not failure.
3. **Floors only rise.** Probe 0.932→0.970; cert 0.650→0.935. Measured probe hit 1.0. The ratchet’s 0.03 slack is the remaining-work of a single night (LDA-ADR-005): do not spend the whole surplus locking a floor to one cast.
4. **Ruin was checked.** The paid API wheel busted the monthly purse. Interpretive work moved to named subagent threads. Cost of a Darwin score run stayed $0.048 with safety 100. We did not “make it cheaper” by hiding spend in a vendor that had already refused us.

What did **not** improve, and must be said:

- harnessFit 50→49. Memory usefulness 36. The readiness die is still a snapshot without an interval (Vetus gap).
- Darwin still scores mock traces unless Circuit B is wired. A 70 ms `verify: PASS` is not a counted circuitus.
- Verse-alignment percentage fell as the hall grew. Growth without a matching alignment circuit is inventory, not skill.
- UI unpaid asks stacked (seed captions, foot-links, brass-on-titles, `:focus-visible`). Measure-only is correct; it is also not delivery.

---

## 5. Score contract for this study (sidecar)

Per LDA-ADR-001 / architect contract. Sidecar, not a sixth EffectVector face.

```
circuit        Darwin variants (n=10); fitness floors (probe, cert);
               readiness 5-die (snapshot); labor inventory (not a score)
favorable      Darwin: finalScore ≥ parent + 0.05 and safety=1
               Floors: measured ≥ floor; floor := max(old, measured−0.03)
odds           Darwin bar 0.05 is a published house rule, not derived EV
stake          evolve generations; overnight tokens; xAI monthly credits (busted)
edge           refused: treating g2 mean 0.747 as “the new baseline”
ruin           API wheel hit spend limit; interpretive work left that table
calibration    floors: probe 0.962→1.000, cert 0.395→0.965 (n = ratchet casts)
claim_type     mixed
```

---

## 6. What to copy next time

The method is the portable thing:

1. Freeze the model. Mutate the harness. Score children on **published weights**.
2. Give Darwin a **live** circuit (tests that can fail on real artifacts), not only mock traces.
3. Ratchet floors up, never down. Disclose the slack.
4. Run overnight labor as **named casts** (subagents you can watch), with a ruin check on vendor spend.
5. Report **each circuit separately**. A night that refuses to promote a 0.545 child *and* lifts certification from 0.395 to 0.965 is a good night. A night that averages them into “+12% scoring” is a cheat.

Cardano: count the circuit before you score. This overnight, we counted two, improved one, and left the other honest.

---

## Sources (receipts)

| Claim | Where |
|---|---|
| Score weights | host harness `score_policy.ts` (six faces sum to 1) |
| Promote delta + safety | `.metaharness/reports/winner.json` |
| Lineage *n*=10 | `.metaharness/lineage.json` |
| First floors | git initial `floors.json` (probe 0.932 / cert 0.65) |
| Last floors | harness fixtures `floors.json` (0.97 / 0.935; measured 1.0 / 0.965) |
| Fitness circuit | harness vitest fitness suite |
| Ratchet rule | `scripts/ratchet-floors.mjs` (`measured − 0.03`, never down) |
| Readiness late | Darwin wheel log (`fit=49 safety=100 memory=36 est=$0.048/run`) |
| LDA contract | `adrs/LDA-ADR-001` … `005`; `experts/weftos-scoring-architect.md` |
| Vetus MH snapshot | `deliverables/04-existing-spaces.md` (different host, n=1) |
