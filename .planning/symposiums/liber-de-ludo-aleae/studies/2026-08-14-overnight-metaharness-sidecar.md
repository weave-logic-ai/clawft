# Overnight metaharness + Cardano sidecar — first live eval

**Study of:** attaching the Liber de Ludo Aleae score contract to a
recurring MetaHarness loop, unattended, then reading what the circuit
actually did.  
**Date:** 2026-08-13 → 2026-08-14 (stopped after turn 014).  
**Status:** first live sidecar on a real eval. Does **not** graduate
LDA-001. Does **not** promote a generator.  
**Claim type of this study:** *weak_scientia* (n=104 scored casts;
method needs n≥30 *and* a stable circuit — we have the first, not the
second).  
**Parents:** `deliverables/02-cardano-framework.md`; P1-nova; LDA-ADR-001
(score names a circuit); LDA-ADR-003 (house edge); mapping §“first three
moves” item 2 — *sidecar the contract on the next metaharness throw*.

This note is about the **method**, not the domain. The overnight loop
happened to draft build-guides against a held-out community survey. The
same contract would attach to any MetaHarness throw: a router decision,
a promote gate, a judge, a remaining-work split. Domain nouns are
omitted except where they name a circuit face.

---

## 0. What we are claiming, and what we are not

A score that does not name its circuit is a wager dressed as
measurement. Overnight we ran that sentence as an instrument.

**Improved (method):**

| Face | Before | After |
|------|--------|-------|
| Circuit named? | Silent 14-row “known set”, then a cached 47 | Live intersection, recomputed every turn; census on every receipt |
| p vs np | Mean score 62.1 published as if it were P | p is a probability; np is a **count**; ROTM never published as P |
| Claim type | One green draft would have been a promote | n=1 *fortuna* — show, do not promote. 13 collected drafts stayed *weak_scientia* |
| Equal conditions | One contaminated pass led with the answers | Blind generator; GT sealed until collect; writers ≠ HTTP API |
| Sample space | 47 scorable, frozen | 68 scorable once hidden labels and missing files were counted |
| Detector | “the model is getting better / worse” | lean = observed − np; turn 8 fired −3.33 and was *read*, not averaged away |

**Did not improve (and we will not pretend it did):**

- Overnight hit-rate **42 / 104 = 0.4038**.
- Seed prior (run-002 substance match) **17 / 42 = 0.4048**.
- Those are the same probability. The generator did not earn a promote.
- Latest unique-slug mean match is **61.4 / 100**. That is *not* a lift
  from the old 33.9 mean: the 33.9 was a **wording** rubric. The 61.4
  is a **substance** rubric. Different dice. Do not subtract them.

The method’s lift is that we can now *say that* without laundering
luck as skill.

---

## 1. What “using a MetaHarness” meant on this table

MetaHarness, in the WeftOS / Ruflo sense, is not a model and not a
score. It is a **loop that produces receipts**: route a unit of work,
evaluate it under named conditions, write what happened, refuse to
promote on a single green face.

This run used that loop as the *host* of the Cardano sidecar.

```
every 45 minutes, one turn, then stop
        │
        ▼
  1. harvest 1–2 new labels into the held-out survey
        │   (grows the labeled face of the circuit)
        ▼
  2. recompute the live pool  =  existing artifacts ∩ current labels
        │   (circuitus — name the sample space this throw)
        ▼
  3. draft 8 uniformly from THAT pool, never the whole 560, never a missing file
        │   (aequitas — same info, same tools, no rating peek)
        ▼
  4. seal ground truth inside compare packets; host does not read them
        ▼
  5. eight Grok Build scorers, one packet each, write JSON only
        │   (independent casts; no api.x.ai; no shared peek)
        ▼
  6. collect → Cardano sidecar {circuit, p, np, observed, lean, claim_type}
        │   (R07 frequency; R15 refuse ROTM; R17 refuse luck-as-skill)
        ▼
  7. append growth.jsonl. Do not promote. Do not commit master.
```

That is the metaharness. The “router” here routed **work shape**, not
model vendor:

| Work | Who | Isolation |
|------|-----|-----------|
| Harvest + draft + collect | host (this session) | shared tree, no GT peek |
| Compare (score) | `spawn_subagent` × 8 | one packet, write JSON |
| Grow (write a missing labeled artifact) | `spawn_subagent` × 1–2 | blind generate packet |
| HTTP `api.x.ai` / `tools/grok.py` | **retired** (403, and it broke aequitas) | — |

No dollar figure is claimed. Routing receipts for token cost were not
the circuit under test. The circuit under test is **P(substance hit)**
on a sealed pair.

Coniunctio example E6 asked: without a circuit, is a MetaHarness face
of 75 favorable? Against what? *n*? Interval? This overnight run is
the first time that question had a written answer on a live loop.

---

## 2. How scoring was implemented (the LDA contract on the throw)

### 2.1 Two circuits, never one number

Cardano’s first move is to name the sample space (*circuitus*) or write
`incomplete:<why>`. Silence is a hidden die (R01, R11).

The loop published **two** circuits, side by side:

1. **Match circuit** — what a single eval cast can land on.

   | Face | Interval | Meaning |
   |------|----------|---------|
   | bust | [0, 40) | missed the identity / win condition |
   | lean | [40, 70) | some jobs, not the engine |
   | hit | [70, 85) | independently derived the substance |
   | solid | [85, 101) | substance + feel; still not a parrot |

   Favorable (*r*) is **match ≥ 70**. That is the face whose
   probability is *p*. Wording, nicknames, and tier letters score
   zero. The rubric forbids grounding-fixes that say “copy the
   survey.” That is R08 (undeclared take-rate / parroting is house
   edge) applied to the *judge*.

2. **Pool circuit** — who is even eligible to be drafted.

   ```
   scorable(t) = artifacts_on_disk(t)  ∩  labels_in_survey(t)
   ```

   Recomputed after harvest, in-process. Never cached as “the 47.”
   Census `{guides, labeled, scorable, labeled_no_guide,
   guides_unlabeled}` rides every growth row so a moving sample
   space cannot hide inside a rising mean.

A draft of a missing artifact is not a cast. It is an incomplete:
nothing to score. The house rule “draft 8 from the live pool, never
the 560” is R01 in operational form.

### 2.2 p and np stay apart (R07, R15)

| Symbol | Unit | Meaning |
|--------|------|---------|
| *p* | probability in [0, 1] | per-cast P(hit ≥ 70), seeded then updated as a hit-**rate** |
| *n* | count | scored casts this draft (unscored slots do not count) |
| *np* | **count** | planned number of hits this draft |
| *1−(1−p)^n* | probability | legal P(at least one hit), independence assumed and declared |
| *min(1, n·p)* | **not a probability** | ROTM trap; stored as `rotm_forbidden`, never published as P |
| *observed_count* | count | hits actually scored ≥ 70 |
| *lean* | count | observed − np |

The seed *p* is run-002’s **hit-rate 17/42 = 0.4048**, not the 62.1
mean score. A mean is not a probability. Using it as *p* would have
been reasoning on the mean — the error Nova already flagged in
Cardano himself.

After the first collected draft, *p* for the *next* throw is the
running hit-rate on scored casts only (draft-sealed rows have *n*=0
and do not dilute).

### 2.3 Claim types (R04, R17)

| *n* scored | claim_type | What you may say |
|------------|------------|------------------|
| 1 | *fortuna* | show the face; do not promote |
| 5 | *weak_scientia* | a lean can be *noticed*; not a method |
| 30 | *method* | calibrate; still not a treasury |

Thirteen collected drafts of *n*=8 are each *weak_scientia*. The
pooled *n*=104 would be *method* **if** the circuit were stable.
It was not: the pool grew, the hyphen parser changed who counted as
labeled, and identity packets arrived mid-run. So the **study** stays
*weak_scientia* even though the raw count cleared 30. That is R14
(independence receipt) plus R01 (name the circuit you actually threw
on). Mixing early-frozen-47 casts with late-live-68 casts and calling
the blend a method is a hidden die.

### 2.4 Equal conditions (R02, R08)

| Condition | How it was equalized | Tilt if we had skipped it |
|-----------|----------------------|---------------------------|
| Information | Generator never reads the survey or community ratings | Parrot score (the 07:57 contamination; discarded) |
| Tools | Same synergy / gear / floor-ceiling / corpus packet | Inventor stats |
| Eval | One sealed packet, one scorer, substance rubric | Wording judge, or peeking neighbors |
| Stake | No promote, no master, no treasury on a green draft | Luck-as-skill shipped |
| Writer | Host *is* Grok; subagents write; HTTP API retired | 403s, and a second, unequal judge |

The 07:57 contamination is the exhibit for R08. Leading the generator
with the community rating is an undeclared take-rate: the eval then
measures copying, not derivation. It was caught, killed, and *not*
averaged into the overnight *p*.

### 2.5 The seventeen rules, as they actually fired

| id | name | What the overnight did |
|----|------|------------------------|
| R01 | circuitus | Pool census every turn; match buckets named |
| R02 | aequitas | Blind draft; sealed GT; one-packet scorers |
| R03 | odds_rs | Favorable = hit ≥ 70; *r*:*s* = hits : misses this draft |
| R04 | scientia_fortuna | Sidecar splits model identity from residual |
| R05 | systematic_lean | lean column; turn 8 is the worked example |
| R06 | power_rule | *p^n* never published; agents are not i.i.d. |
| R07 | frequency_np | np stored as its own key forever |
| R08 | fraud_catalog | Contaminated pass discarded; no rating peek |
| R09 | remaining_work | Unscored slots are *n*=0, not a sunk 0.0 *p* |
| R10 | small_stakes | Stake = one draft of 8; ruin = a false promote |
| R11 | name_or_incomplete | Unscored draft publishes `incomplete:unscored` |
| R12 | convert_to_odds | Hit defined before any number is claimed |
| R13 | size_to_ruin | Ruin named: shipping a generator change |
| R14 | independence_receipt | Repeats of the same slug are *not* i.i.d.; noted |
| R15 | refuse_rotm | `rotm_forbidden` field; legal at-least-one is `1-(1-p)^n` |
| R16 | refuse_multiply_odds | Odds never multiplied |
| R17 | refuse_luck_as_skill | Zero promotes in 14 turns |

---

## 3. How the overnight actually ran

- **Host:** Grok Build, recurring scheduled task every 45 minutes,
  one turn per fire, then stop. Cancelled after turn 014.
- **Turns:** 001 draft-only; 002–014 collected (*n*=8 each).
- **Casts:** 13 × 8 = **104** scored. 42 favorable.
- **Writers/scorers:** `spawn_subagent` only.
- **Growth ledger:** `growth.jsonl` — *p* and *np* are separate keys
  on every row, including the sealed (unscored) draft row. That row
  exists so a crash between draft and collect cannot be back-filled
  with a peeked *p*.

Two mid-run corrections changed the circuit itself. They are method
events, not domain events:

1. **Hyphen labels.** Harvest wrote `### ENC-PAL-ROG`. The parser only
   accepted `ENC/PAL/ROG`. Sixteen already-harvested slugs were
   sitting in the survey and not in the circuit. Fixing the parser
   moved labeled 53 → 69 in one recompute. That is R01: the sample
   space was silently short. A rising *p* on the short space would
   have been a house table (R08).
2. **Grow-guides.** A label without a file cannot be drafted. Each
   turn now writes 1–2 blind generate packets for labeled-missing
   artifacts so the *next* throw’s pool can include them. Pool
   47 → 68 is mostly this plus the hyphen fix, not a better model.

---

## 4. Results

### 4.1 The headline number

> Overnight *p̂* = 42 / 104 = **0.4038**.  
> Seed *p* = 17 / 42 = **0.4048**.  
> Lean vs seed, whole night: 42 − 0.4048 × 104 = **−0.1 hits.**  
> That is a fair table. It is not an improvement of the generator.

If someone quotes “we hit 42” without *n* and without *np*, they are
publishing ROTM. Planned count at the seed is ~42.1. We landed 42.

### 4.2 Per-turn ledger (collect rows only)

*p* in this table is the **prior** used to plan *np* for that draft
(running hit-rate, except turn 2 which is the 17/42 seed). *obs* is
the count of faces ≥ 70. *lean* = obs − np.

| turn | prior *p* | np (count) | obs | lean | buckets (bust / lean / hit / solid) |
|-----:|----------:|-----------:|----:|-----:|--------------------------------------|
| 002 | 0.405 | 3.24 | 2 | −1.24 | 2 / 4 / 1 / 1 |
| 003 | 0.250 | 2.00 | 4 | +2.00 | 1 / 3 / 3 / 1 |
| 004 | 0.375 | 3.00 | 2 | −1.00 | 2 / 4 / 2 / 0 |
| 005 | 0.333 | 2.67 | 5 | +2.33 | 0 / 3 / 5 / 0 |
| 006 | 0.406 | 3.25 | 3 | −0.25 | 0 / 5 / 2 / 1 |
| 007 | 0.400 | 3.20 | 4 | +0.80 | 1 / 3 / 2 / 2 |
| **008** | **0.417** | **3.33** | **0** | **−3.33** | **2 / 6 / 0 / 0** |
| 009 | 0.357 | 2.86 | 2 | −0.86 | 1 / 5 / 2 / 0 |
| 010 | 0.344 | 2.75 | 4 | +1.25 | 0 / 4 / 3 / 1 |
| 011 | 0.361 | 2.89 | 2 | −0.89 | 2 / 4 / 2 / 0 |
| 012 | 0.350 | 2.80 | 5 | +2.20 | 1 / 2 / 4 / 1 |
| 013 | 0.375 | 3.00 | 5 | +2.00 | 3 / 0 / 4 / 1 |
| 014 | 0.396 | 3.17 | 4 | +0.83 | 2 / 2 / 2 / 2 |

Bucket totals over 104 casts: bust 17 · lean-band 45 · hit 32 · solid 10.
Favorable (hit+solid) = 42. The mass sits in the lean-band: the
generator often finds *jobs* and misses the *engine*. That is a
systematic lean (R05), not noise around a rising *p*.

### 4.3 Windows — the only place a lean looks like lift

The night is not stationary. Split after the fact (declared here, not
used as a promote):

| Window | turns | hits / n | *p̂* | vs seed 0.405 |
|--------|-------|----------:|-----:|---------------|
| Early (identity packets just landing) | 002–007 | 20 / 48 | 0.417 | +0.6 hits |
| Mid (includes the zero draft) | 008–011 | 8 / 32 | 0.250 | −5.0 hits |
| Late (live pool + grow-guides) | 012–014 | 14 / 24 | 0.583 | +4.3 hits |
| Whole night | 002–014 | 42 / 104 | 0.404 | −0.1 hits |

Turn 8 is the mid-window. Planned ~3.3 hits, observed 0. Under a
fair binomial with *p*=0.42, *n*=8, P(0 hits) ≈ 0.013. That is a
lean with a reason, not a cold night: scorers kept writing the same
gap — the prediction called high-band, already-labeled kits
“unrated / novel / mid-pack.” The *scientia* (mechanics identity)
was being overwritten by a missing “this is already on the board”
term. We logged it. We did **not** patch the generator from *n*=8.

The late window’s 14 / 24 = 0.58 is the number a careless promote
would quote. *n*=24 is still *weak_scientia*, the circuit moved
underneath it (hyphen fix on turn 13, pool 50 → 66), and several
slugs were re-throws of earlier casts (R14: agents remember). It is
a lean worth a *next* designed experiment. It is not a method.

### 4.4 Mean score is the wrong column

| Instrument | Mean | What it measures |
|------------|-----:|------------------|
| Round-1 wording rubric (blind, *n*=10) | 33.9 | Did the prose echo the survey? |
| Latest unique-slug substance scores (*n*=47 files) | 61.4 | Did independent reasoning land the same strategy? |
| Run-002 substance (the *p* seed) | 62.1 mean / **0.405 hit-rate** | Same instrument as overnight |

The jump 33.9 → 61.4 is almost entirely **changing the die**. A
wording judge and a substance judge are unequal conditions (R02).
We keep 33.9 as a historical warning, not as a baseline to beat.

If you need a single improvement number for the *method*, use this:

> We stopped publishing a mean as if it were a probability, and we
> stopped promoting on a color. The calibrated hit-rate did not move
> (0.405 → 0.404). The named circuit grew (47 → 68). One draft that
> would have been called “the model collapsed” is now a lean of
> −3.33 with a written reason.

That is the overnight improvement.

### 4.5 Sample-space growth (circuitus, not quality)

| Moment | artifacts | labeled | scorable |
|--------|----------:|--------:|---------:|
| First overnight census (turn 003) | 420 | 51 | **47** |
| Cached-looking plateau (turns 003–008) | 420 | 51 | 47 |
| First real label growth (turn 009) | 420 | 53 | 48 |
| After grow-guides (end 012) | 422 | 53 | 50 |
| After hyphen parser (live 013) | 424 | 69 | 64 |
| Wheel stopped (end 014) | 426 | 71 | **68** |

Δ scorable = **+21**. Almost none of that is “the model wrote better
guides.” It is the circuit being told the truth: harvest headers
count; a label without a file is not a cast; a file without a label
is not a cast.

A MetaHarness that reports “quality up 45%” because its eligible set
quietly grew is the house. Census on the receipt is the counter.

---

## 5. What the method bought us that a mean never would

1. **A promote gate with teeth.** Fourteen turns, 104 casts, zero
   promotes. R17 is not a slogan if the scheduler is allowed to
   land a generator change at 04:00 because one draft went green.
   Ruin was named in advance (a false promote). We did not take it.

2. **A fraud we could *see*.** The 07:57 lead-with-the-rating pass
   would have raised match numbers and destroyed the experiment.
   Because favorable was defined as *independent* substance, the
   contamination was a category error, not a tempting lift.

3. **A lean that stayed a lean.** Turn 8’s zero would have been
   smoothed into “overnight average 40%.” Isolated, it points at a
   single missing *scientia* term (already-on-the-board vs novel).
   That is the beginning of a designed next throw, not a vibe.

4. **A moving circuit that could not hide.** The 47-plateau was a
   parser, not a universe. Once named, it moved. Future *p̂* must
   either freeze a cohort or declare the mix.

5. **p and np as separate columns, forever.** Anyone quoting this
   study can lie, but they have to lie *against a ledger* that
   already printed the planned count.

6. **Equal writers.** Retiring the HTTP API removed a second, failing,
   unequal judge. Scorers and the host are the same kind of agent
   under the same sealed packet. That is aequitas, not a vendor
   preference.

---

## 6. What we still may not say

- We may not say the generator improved overnight. *p̂* is the seed.
- We may not say the late 58% is the new *p*. *n*=24, circuit moved,
  repeats are not independent.
- We may not convert 42 hits into “the probability of success is 42”
  or “np = 42%.”
- We may not average wording-33.9 with substance-61.
- We may not graduate LDA-001. This is one sidecar on one loop.
  Mapping item 2 is **done once**. Item 3 (Plane / kernel) stays
  closed.
- We may not invent dollar savings. No routing-receipt circuit was
  on the table.

---

## 7. How to reuse this without the domain

The contract is four objects. They attach to any MetaHarness throw.

```
sidecar = {
  circuit:        <named faces, or incomplete:why>,
  favorable:      <which face is r>,
  p:              <P(favorable) per cast>,          // probability
  n:              <scored casts>,                   // count
  np:             <p × n>,                          // COUNT, never P
  p_at_least_one: <1 − (1 − p)^n>,                  // only with R14
  rotm_forbidden: <min(1, n·p)>,                    // do not publish as P
  observed_count: <k>,
  lean:           <k − np>,
  claim_type:     fortuna | weak_scientia | method,
  equal:          <info, tools, eval, stake — or the tilt>,
  ruin:           <what a false promote costs>,
  census:         <if the eligible set can move, print it>
}
```

**Metaharness job:** produce the receipt, update *p* as a hit-rate
(not a mean), refuse promote unless `claim_type == method` **and**
the circuit is the one you named at the start of the stake.

**Operator job:** when lean is large, write the reason before you
touch the model. Turn 8’s reason was sitting in the scorers’ gap
lists the same night. The method is the pause, not the patch.

---

## 8. Next designed throw (not a promote)

If this sidecar is reused, the next *experiment* (a new circuit,
declared in advance) is:

> Freeze a cohort of *N*≥30 labeled artifacts that already have
> files. Do not harvest into that cohort mid-run. Draft without
> replacement. Score substance ≥ 70. Prior *p* = 0.4048. Promote
> the generator only if lean is systematically positive over that
> frozen circuit *and* the gap lists name a mechanics term you
> can change without reading the survey.

Until that throw exists, the overnight result is:

**the table is fair, the circuit is finally named, and luck was
not cashed as skill.**

---

## Sources

- Symposium: `~/weftos/.planning/symposiums/liber-de-ludo-aleae/`
  (`02-cardano-framework.md`, P1-nova, LDA-ADR-001/003, demos/E6).
- Ledger: overnight `growth.jsonl` collect rows, turns 002–014.
- Seed: run-002 substance match 17/42.
- Rubric change: wording baseline 33.9 discarded as a comparable
  (08:0x methodology correction).
- Scheduler `019ffe211c69` cancelled after turn 014.
