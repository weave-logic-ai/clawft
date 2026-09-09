# P3 — Room Coniunctio

**Phase**: II — together. Nova’s primitives meet Vetus’s walls.
**Date**: 2026-08-13
**Rule of combination** (P0): gap + no genesis break + not Cardano’s error + cite both parents.
**Thesis** (identical):

> A score that does not name its **circuit** is a wager dressed as measurement. WeftOS should refuse to treat luck as skill, refuse undisclosed house edge, and price every economic or governance decision as an expected value over an enumerated (or honestly incomplete) sample space.

Nova spoke from the book (`sources/chapter-digest.md`, P0 glossary). Vetus spoke from the tree (`panels/P2-vetus.md`, `deliverables/04-existing-spaces.md`). This room only **lands** or **refuses**.

WeftOS seats: `governance-counsel`, `ecc-analyst`, `defi-networker`, `doc-weaver`, plus `knowledge-portfolio`.

---

## 1. Nova, first pass (no crate types)

Primitives that survive the refusals:

| Primitive | Meaning | Error we do not import |
|-----------|---------|------------------------|
| *circuitus* | Enumerated sample space, or `incomplete: why` | — |
| *aequitas* | Equal conditions (info, tools, stakes, light) | Fairness-as-mood |
| odds *r:s* | Fair price; EV = 0 is justice | Multiply odds |
| *scientia* / *fortuna* | Model vs residual | Prince / luck-as-skill |
| systematic lean | Calibration / bias / fraud | — |
| *pⁿ* | Independence, stated | ROTM; *np* as *P* |
| *np* over large *n* | Frequency / receipts | One green demo |
| fraud catalog | Hidden take-rate detector | Cheating-as-method |
| remaining work | Continuation value | Sunk score; triangular split |
| small stakes | Ruin bound | Treasury on one cast |

---

## 2. Vetus, second pass (no new doctrine)

Every live surface: **no circuit, no odds, no edge index, no ruin probability, no calibration of a declared *p***. Closest cousins: MH receipts, `NodeScoring::blend` EMA, `CoherencePrediction.uncertainty`, cost/no-op breakers (stop-loss), SOUL EV commandment, DeFi bond.

Homonyms to keep split: **circuitus ≠ circuit-breaker**; **coherence** (four formulas); **fairness** (vibe ≠ *aequitas*).

Marked deck already in-house: `governance-counsel.md` 5D ≠ ADR-034 5D; magnitude bars 0.7 vs 0.8.

---

## 3. Landings (row by row)

| Nova primitive | Vetus gap | Landing | Cite both | LDA |
|----------------|-----------|---------|-----------|-----|
| *circuitus* | K2 §5 UQ gap; every score silent on sample space | **Sidecar field `circuit`** on any published score | Gould ch.14; K2 `04-industry-landscape.md` §5 | **001** |
| *aequitas* | `EffectVector.fairness` is 0–1 vibe; counsel drift | **Equality-of-conditions check** beside fairness dim; do not rename the dim | Bellhouse/Gould equal-conditions; ADR-034; V17 | **002** |
| odds / EV = 0 | Router / deals / promote have no fair price | **`odds` + `edge` sidecar**; declare take-rate | ch.14; routing.md; MH receipts | **003** |
| systematic lean | Fitness/MH/Node scores have no *n* vs *p* | **`calibration` over n**; lean ⇒ investigate | ch.11; FitnessScorer WEFT-54 honesty; MH score | (001 field) |
| *np* / receipts | Flywheel already: no silent promote | **Keep**. Name it frequency, not luck | last chapter; ADR-096 | — already landed |
| remaining work | Auto-delegation is a classifier, not a split | **Continuation EV** for interrupted swarms | problem of points; WEFT-201 | **005** |
| small stakes / ruin | Cost breaker exists, is stop-loss | **`ruin` field** + keep breaker as breaker | *Practica* rich/poor; WEFT-322 | **004** |
| fraud catalog | No house-edge index | **Advantage index** on opportunities | fraud chapters; V8 stub 7-factor | **003** |
| knowledge portfolio | AgentDB patterns store recipes | **Store *n* + calibration**, not only the recipe | last chapter; memoryUsefulness 53 | (practice) |

**Not landed (honorable orphans):**

- Genesis-smash of a 6th EffectVector face (C9 still deferred; sidecar first).
- Calling WEFT-322 a *circuitus*.
- Calling `fairness` *aequitas* without the check.
- ROTM heuristics as “priors.”
- Dollar savings invented from `estCostPerRunUsd 0.024`.

---

## 4. Score contract (fields, not Rust)

Sidecar JSON that can sit beside EffectVector / QualityScore / MH score without genesis edit:

```json
{
  "circuit": "enumerated | incomplete:<why>",
  "favorable": "what a win is",
  "odds": {"r": 1, "s": 5, "p": 0.1667},
  "stake": {"kind": "tokens|time|trust|treasury", "amount": 1},
  "edge": 0.0,
  "ruin": {"p_bust": null, "bankroll": null},
  "calibration": {"n": 1, "observed": null, "interval": "unknown"},
  "claim_type": "scientia | fortuna | mixed",
  "equal_conditions": {"info": true, "tools": true, "eval": true, "stake": true}
}
```

If a field cannot be filled, **publish the hole**. That publication is the honesty.

---

## 5. Worked landings (easy to grok)

See `demos/examples.md` E1–E10 and `demos/circuit-ev.html`.

| Example | Lands on |
|---------|----------|
| E1 fair vs 4:1 table | LDA-003 edge on routing/vendor |
| E2 ROTM 3×1/6 | FitnessScorer / NoopScorer |
| E3 three-dice 25/27 | counting kinds vs ways |
| E4 breaker ≠ circuitus | WEFT-322 |
| E5 5–3 remaining | LDA-005 delegation |
| E6 MH five dice n=1 | this session’s score |
| E7 marked briefing | governance-counsel drift |
| E8 knowledge book | AgentDB patterns |
| E9 n before promote | ADR-096 already |
| E10 bake-off equality | LDA-002 |

---

## 6. Local ADRs opened

Drafts in `adrs/`. Exploratory. No Plane item until deliverable 03 says go.

---

## 7. What we hand the keynote

- Thesis unchanged.
- Two rooms were the method.
- Dedicated wall slide → `deliverables/04-existing-spaces.md`.
- Dedicated interactive slide → `demos/circuit-ev.html`.
- Contract + five LDA numbers.
- Three next moves: (1) fix counsel drift (equal conditions inside the house), (2) sidecar the contract on one MH eval, (3) do not touch genesis.
