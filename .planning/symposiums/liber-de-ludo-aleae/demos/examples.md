# Grokable examples — Cardano on a WeftOS table

Small, countable circuits. Use these in Coniunctio and in the keynote explorer. Every example names **circuit, favorable, odds, stake, edge, ruin**.

---

## E1 — Fair die vs house die

**Circuit (fair):** 6 faces, equally likely. Bet 1 on a six. Payout 5:1 (plus stake back = 6).

| | Fair table | House table (payout 4:1) |
|--|------------|---------------------------|
| p | 1/6 | 1/6 |
| Stake | 1 | 1 |
| Win pays | +5 | +4 |
| EV | `(1/6)(5) + (5/6)(-1) = 0` | `(1/6)(4) + (5/6)(-1) = −1/6` |
| House edge (−EV / stake) | 0 | **16.7%** |

Cardano: lay wagers in proportion to the circuit. The 4:1 table is unequal conditions even if the die is fair.

**WeftOS rhyme:** a router that bills "tier-3 prices" for a tier-1 task is a 4:1 table.

---

## E2 — Reasoning on the mean (the error)

Claim: "P(six) = 1/6, so in 3 throws we have 1/2."

| Method | P(at least one six in 3) |
|--------|--------------------------|
| ROTM (forbidden) | 3 × 1/6 = 0.50 |
| Circuit | 1 − (5/6)³ = 91/216 ≈ **0.421** |
| Four throws | 1 − (5/6)⁴ ≈ **0.518** |

**WeftOS rhyme:** FitnessScorer treating "longer + no refusal phrase" as P(quality) is ROTM. `NoopScorer = 1.0` is a die with one face painted six.

---

## E3 — Three dice (Cardano ch. 13, Galileo later)

Circuit = 6³ = 216.

| Sum | Ways | P |
|-----|------|---|
| 9 | 25 | 25/216 |
| 10 | 27 | 27/216 |
| 11 | 27 | 27/216 |
| 12 | 25 | 25/216 |

Partitions are not the circuit. "Six combinations each" is the marked deck.

**WeftOS rhyme:** counting *kinds* of failures instead of *weighted ways* (unique vs duplicate partitions).

---

## E4 — Cost circuit-breaker is not a circuitus

Conversation bankroll B = 200_000 tokens. Stake per step s. Cap is a **ruin bound**, not a sample space.

| | Circuitus | Circuit-breaker |
|--|-----------|-----------------|
| Question | What can happen? | When do we leave the table? |
| WeftOS | (missing) | WEFT-322 `cost_budget`, `circuit_breaker_no_op_limit=3` |

You can have a perfect breaker and still be playing a 4:1 house game.

---

## E5 — Problem of points (delegation)

Match to 6. Stopped at 5–3. Pacioli split 5:3 (sunk). Cardano: split by **remaining** (1 vs 3) — right question, triangular-number arithmetic wrong. Pascal: 7:1 remaining-paths.

**WeftOS rhyme:** an interrupted swarm. Do not credit tokens already burned. Credit P(this agent finishes the remaining work).

| Agent | Remaining tasks | P(finish each) | Continuation |
|-------|-----------------|----------------|--------------|
| A | 1 | 0.5 | 0.5 |
| B | 3 | 0.5 | 0.125 |

Fair split of the leftover budget ≈ 0.5 : 0.125 = **4:1**, not 5:3.

---

## E6 — MetaHarness as five unlabeled dice

This session's throw:

| Face | Value |
|------|------:|
| harnessFit | 75 |
| compileConfidence | 100 |
| taskCoverage | 65 |
| toolSafety | 90 |
| memoryUsefulness | 53 |

Without a circuit: is 75 a favorable cast? Against what? *n*? Interval?

Coniunctio proposal: publish `{circuit: "MH-score-v1 faces", n: 1, claim_type: scientia+fortuna, interval: unknown}`.

---

## E7 — Marked deck in our own briefing

`agents/weftos/governance-counsel.md` EffectVector = `{cpu, memory, network, storage, trust_delta}`.

ADR-034 / `effects.rs` = `{risk, fairness, privacy, novelty, security}`.

Two dice, one name. Unequal conditions for any agent that reads the counsel file.

---

## E8 — Knowledge as a book of wagers

Each memory / skill / model is a row:

| Asset | Implied p | Stake | Ruin if wrong |
|-------|-----------|-------|---------------|
| Pattern in AgentDB | last *n* successes / *n* | attention tokens | bad promote |
| FitnessScorer phrase list | English-only (declared) | GEPA weight | false fitness |
| One green demo | unknown | reputation | shipping luck |

Diversify uncorrelated circuits (hosts, judges, evals). One model + one judge = one die.

---

## E9 — Promote only after *n* casts

Flywheel doctrine already: evaluate → receipt → confirm. Cardano: a single win is not a method (*np* only for large *n*).

| Casts | What you may claim |
|------:|--------------------|
| 1 | *fortuna* — show, don't promote |
| 5 | weak scientia — publish interval |
| 30+ | method — still name the circuit |

---

## E10 — Equality of conditions bake-off

If model A sees the gold answers and model B does not, the table is tilted. Fair bake-off = same tools, same context, same hidden cards.

**Check:** `equal_info && equal_tools && equal_eval && equal_stake`. Any false ⇒ refuse or disclose the edge.
