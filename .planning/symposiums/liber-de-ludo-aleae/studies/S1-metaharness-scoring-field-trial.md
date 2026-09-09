# S1 — Field trial: Cardano scoring inside a metaharness

**Status**: Study (symposium-local) · **Date**: 2026-08-14
**Subject**: a local model-evaluation lab, ~60 GPU-hours of prior history, one overnight run
**Parents**: LDA-ADR-001…005; `deliverables/02-cardano-framework.md`; the LDA keynote's 17 applications
**Claim of this document**: the framework is not decorative. Applied to a working measurement
system it changed what that system could know, and the change is quantified below.

---

## 0. Summary

A lab had been running benchmarks for a fortnight and believed it was measuring models. It
was measuring its own configuration. The Cardano primitives — enumerate the circuit, state
independence, refuse luck-as-skill, price the wager, check ruin — were implemented as
executable scoring code and placed inside a two-tier metaharness. One overnight run then
did something the lab had never done in sixty hours: **it measured its own noise floor.**

| | before the method | after |
|---|---|---|
| intelligence per hour (best prior run) | **0.08** | **0.81** |
| intelligence per hour (worst prior run) | **0.00** | — |
| hours returning no defensible claim | **58 of 59** | 0 of 8.5 |
| P(a run measured what it claimed) | ~0, undetected 11 days | 1.0, verified per cell |
| attributability of a result to a cause | 0.25 (four fields moved) | 1.0 (one field) |
| run-to-run variance | **never measured** | ±2.6 and ±25.0 points, by arm |

The single most valuable output was a **falsified prediction**. That is the correct
outcome to prize, and the scoring system is built so that it prices as a win.

---

## 1. What a metaharness is, and why the harness is the thing under test

A *harness* runs the subject and produces a score. A *metaharness* treats the harness
itself as the object of study — "freeze the model, evolve the harness."

That inversion is forced by evidence, not preference. In this lab, one configuration
change on a fixed model and fixed problem moved a score from 48% to 81%; the *same* change
on a different problem moved it from 64% to 18%. Configuration moved results further, and
in both directions, than any difference between models ever had. A lab that treats its
harness as a constant is publishing its own settings and calling them a leaderboard.

The trial used a **two-tier arrangement**:

```
UPPER WHEEL   one hypothesis, one single-field change, expensive validation
                        │  drives k
LOWER WHEELS  k independent replicates — cheap, fast, in the informative band
```

The upper wheel asks one question. The lower wheels are the casts. Crucially the wheels
must be *independent* for the multiplier to be real: wheels sharing a seed, a workspace,
or a problem re-create the same correlation one level up and buy nothing.

An external readiness scorer was also run against the repo for an outside view. Its
dimensions (harnessFit, compileConfidence, taskCoverage, toolSafety, memoryUsefulness,
plus an estimated cost-per-run) are reported here as the tool emitted them; the two lowest
were memoryUsefulness and harnessFit, which matched the internal diagnosis — findings were
being written to prose rather than to anything retrievable.

---

## 2. The scoring, primitive by primitive

Each item below is executable code, not a principle. The Cardano name is kept; the
framework forbids proliferating synonyms.

### 2.1 *Circuitus* — every score names its sample space

Per LDA-ADR-001, a published score carries an enumerated circuit or `incomplete:<why>`.
**Silence is a hidden die.** In practice this immediately reclassified the lab's existing
claims. A score of "100%" from a six-item pack reports a Wilson interval of **[0.61, 1.00]**
— which is not a capability claim, and the code says so in words.

### 2.2 *pⁿ* — independence is **stated**, never assumed

The trial's sharpest technical finding. A run produced 566 test outcomes, and the lab had
been reporting them as 566 casts. They are **clustered**: one workspace, one model
instance, and a broken entry point fails every test in its checkpoint together.

Measured intra-cluster correlation: **ICC ≈ 0.94–0.97**, design effect ≈ 67.

> **566 test outcomes carried the information of about 8 independent casts.**

Treating them as 566 is exactly the error the framework names as **ROTM** — *np* mistaken
for *p*, "a pricing error, not a purse". Any circuit that omits its cluster structure is
reported as `independence: UNSTATED` rather than silently assumed independent.

### 2.3 The claim ladder — *scientia* vs *fortuna*

The keynote's thresholds were adopted verbatim rather than paraphrased:

| effective casts | claim |
|---|---|
| 1 | *fortuna* — no coronation |
| ≥ 5 | amber — a basis exists; direction only |
| ≥ 30 | green — may promote **with receipts** `{circuit, n, claim_type, interval, aequitas}` |

Applied retrospectively, a previously published ranking ("88% beats 75%") resolved to
7-of-8 against 6-of-8 — **one item** — and the comparator returned *indistinguishable;
this is fortuna, not a ranking*, together with the number of casts that would settle it: **75 per arm.**
"Run more" became a schedule.

### 2.4 The fan multiplier — and its honest discount

k independent wheels multiply effective casts by k. But shared factors are penalised
explicitly, and when the trial's four wheels shared model, config and seed — varying only
*problem* — the multiplier came out at **1.12, not 4**. Four wheels of wall-clock, one
wheel of information. The report also surfaced a between-wheel spread of **0.59**, meaning
the *problem* dominated the treatment: variance was being read as effect.

### 2.5 Discrimination is symmetric — the correction that mattered most

The first implementation treated saturation as a ceiling problem only. That was half the
picture. **A test everyone fails discriminates exactly as poorly as one everyone passes.**
Using `4p(1−p)`:

| test | rate | discrimination | |
|---|---|---|---|
| A | 0.396 | **0.957** | prime |
| B | 0.137 | 0.473 | weak |
| C | 0.116 | 0.411 | weak, and 5.8× the cost per cast |
| D | 0.053 | **0.201** | floor-saturated — separates nothing |

This produced the trial's most practical result. Four *different* problems bought 2.04
discrimination in 4.2 h (0.49/h). **Four replicates of the prime test bought 3.83 in 1.8 h
(2.13/h) — 4.3× the information for less than half the time.** Re-running a known-good
test with variation beat reaching for novel hard ones, because three of the four novel
wheels were saturated.

### 2.6 The fraud catalog, turned inward

Cardano's catalog is not a metaphor when you are simultaneously player, house, and die.
Each entry has an exact analogue in a benchmark, and the lab had committed most of them:

| Cardano | analogue | occurred? |
|---|---|---|
| loaded / false dice | the model served is not the config claimed | **yes** — 11 days undetected |
| shortened dice | a token budget too small to contain the outcome | **yes** — below the published floor |
| tilted board, bad light | unequal conditions; the result cannot be observed | **yes** — differing temperatures; no provenance |
| dice-box tricks | the harness itself alters the outcome | **yes** — a parser silently discarding valid output |
| keeping only winners | discarding casts one dislikes | **at risk** — 6 cells deleted |

The audit runs against the **artifact**, never the launch command, because a claimed and an
actual configuration diverged for eleven days precisely because nothing read the result.
On the historical run it returns **REFUSE — advantage outside the declared circuit.**

The last row deserves care. LDA-ADR-003 names "evals that keep only winners" as a house
table. Six cells were deleted here — all for *invalidity*, none for being unwelcome — but
that distinction exists only if it is recorded, so the audit demands a reason per deletion
and counts unjustified ones as blocking.

### 2.7 Ruin and stop-gates — LDA-ADR-004 with hours as the bankroll

Time was the scarce resource; intelligence the prize. That makes every run a wager: **stake
hours, get paid in claim strength.** Gates fire *before* (ruin fraction, whether the planned
casts can support the intended claim, whether a cheaper cast answers the same question),
*during* (bail conditions, so the stop is not only at the end), and *after* (the fraud audit).

Priced against a proposed 5-hour re-run with 8 hours remaining, the gate returned **REFUSE**
on two independent grounds: ruin 0.625, and a 2-hour alternative answering the same question.
Paying five hours for that answer was accepting a worse price — an undeclared house edge the
lab had been running against itself.

### 2.8 The four factors underneath

The price is an outcome; these are the levers, and they **multiply**:

```
intel/hour = P(valid) × attributability × information_per_cast × casts_per_hour
```

Any factor at zero makes the run worthless regardless of the others — which is why a single
ratio could not distinguish two very different failures. Naming the binding constraint is
what makes it actionable: *"the run was bad"* is not; *"attributability was 0.25 because you
moved four fields"* is.

---

## 3. How it worked overnight

**Design.** One hypothesis. Two arms differing in **exactly one field**, machine-checked
against a genome file. Four replicates per arm on the prime instrument. A fresh server per
replicate so no warm cache correlated them. Every cell gated before it ran; every result
recording the configuration that produced it.

**Replication came free.** Sampling was already stochastic — no seed control existed or was
needed. Identical re-runs *were* genuine replicates. The lab had simply never collected them.

**Result — the noise floor, measured for the first time:**

| arm | four runs | mean | sd | spread |
|---|---|---|---|---|
| with the treatment | 77.3 · 72.7 · 78.2 · 79.6 | 77.0% | **±2.6** | 6.9 pts |
| without | 15.3 · 76.9 · 63.0 · 75.0 | 57.5% | **±25.0** | **61.6 pts** |

Three findings, in order of importance:

**(a) Every prior delta was inside the noise.** Differences of +33 and −46 points had been
reported as decisive. The relevant arm's run-to-run spread is 61.6 points. Those were single
draws presented as measurements.

**(b) The prediction was falsified, and the direction reversed.** The treatment was expected
to *harm* a long tool loop, following one external source. It *stabilises* it — matching a
different source. Both external claims can stand; they concern different models and different
loops. The value was in learning which applies here.

**(c) The failure mode is bimodal, not a shifted mean.** Three runs without the treatment sit
in a normal band; one collapses to 15.3%. The treatment does not raise a good day — it
prevents a bad one. This also exposed a limitation in the scoring itself: `discrimination()`
assumes a stable *p* per test, and a bimodal test can look "prime" on average while being a
coin flip between regimes. Recorded as a known defect rather than papered over.

**The formal verdict remained *indistinguishable*.** Even at four replicates per arm, the
comparator refused to rank. That is the system working: it declined to hand out a coronation
its casts could not pay for.

---

## 4. Overall improvement

| measure | before | after | note |
|---|---|---|---|
| intel/hour, best prior run | 0.08 | **0.81** | ~10× |
| intel/hour, worst prior run | 0.00 | — | invalid; nothing downstream mattered |
| lifetime rate over 59 h of history | **0.29** | — | 58 of 59 hours returned nothing |
| time to detect a bad configuration | **11 days** | **~30 s** | pre-flight gate |
| provenance fields in an artifact | 0 of 10 | 10 of 10 + probed serving | |
| noise floor | unmeasured | **measured** | the precondition for every other claim |

Two honest deductions from that table. The overnight run's binding constraint became
**casts_per_hour**, not validity or attributability — the cheap failures were fixed and the
expensive one remains. And the run priced at only a *fair* price, not an excellent one:
8.5 hours for one reusable measurement plus one direction-only claim.

The largest single return in the entire history was not a benchmark at all. It was **half an
hour spent building the pre-flight gate**, which converted an eleven-day detection latency
into thirty seconds — permanently, for every future run. Sixty hours of casting produced less
durable knowledge than one afternoon of making the table honest.

---

## 5. Transferable findings

1. **Measure the noise floor before believing any delta.** It is usually cheap — stochastic
   sampling makes identical re-runs into replicates for free — and until it exists no
   difference is interpretable. This lab ran 60 hours without it.
2. **The factors multiply.** Fix the smallest first. A perfectly executed experiment on an
   invalid configuration returns exactly zero.
3. **State independence or lose most of your n.** Clustered outcomes are not casts. An ICC
   of 0.95 turned 566 observations into 8.
4. **Discrimination is symmetric.** Floor saturation is as fatal as ceiling saturation, and
   is easier to miss because failure feels like difficulty rather than blindness.
5. **Prefer known-good instruments with small variations** over novel hard ones. Measured at
   4.3× the information per hour, because hard-and-saturated teaches nothing.
6. **Vary one field.** Four simultaneous changes cost 75% of a run's attributability and made
   a 46-point movement unassignable.
7. **Audit the artifact, not the intent.** Every configuration defect here was invisible in
   the launch command and visible in the response bytes.
8. **A falsified prediction is a positive return** and should be priced as one. So is a
   negative result that closes a line of enquiry — in this lab three such analyses cost zero
   hours and prevented several days of work.
9. **Refuse the wager the circuit cannot pay.** The stop-gate's most valuable output was
   *REFUSE*, twice, on runs that felt productive.

---

## 6. Open

- **`casts_per_hour` is now binding.** With validity and attributability fixed, throughput is
  the constraint, and clustering caps it. No seed control exists; the full fan multiplier is
  therefore unavailable.
- **Bimodality breaks the discrimination model.** `4p(1−p)` assumes a stable *p*.
- **n ≥ 30 remains out of reach** for expensive instruments. Nothing here has yet earned a
  green claim; everything is amber, and the system correctly refuses to say otherwise.
