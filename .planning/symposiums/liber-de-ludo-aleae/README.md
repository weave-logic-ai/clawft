# Symposium: Liber de Ludo Aleae — Scoring Under Uncertainty

**Working title**: "Count the Circuit Before You Score"
**Subtitle**: "Cardano's Book on Games of Chance as a scoring, fairness, and expected-value doctrine for WeftOS"
**Status**: Running. First Grok-hosted symposium (Claude format, Grok executor).
**Date opened**: 2026-08-13
**Host**: Grok Build + Ruflo team bus
**Owner**: Mathew Beane
**MC**: Gerolamo Cardano (persona agent)
**Reports up to**: this directory. Local ADR namespace `LDA-ADR-NNN` — does **not** touch the WeftOS central ADR sequence until a verdict graduates.

---

## Why now

WeftOS already scores constantly: `EffectVector` (ADR-034), `QualityScorer`, `NodeScoring` (trust / performance / difficulty / reward / reliability / velocity), MetaHarness readiness, routing complexity, flywheel promote gates. Most of that scoring is **static, unweighted, and silent about the sample space**. Cardano's *Liber de Ludo Aleae* (written ~1564, published 1663) is the first systematic attempt to make chance *accountable*: enumerate the circuit, lay wagers in proportion, refuse unequal conditions, detect fraud when outcomes refuse the circuit, and treat luck as something you do not claim as skill.

The user brief: use Cardano as a basis for scoring, decision-making under uncertainty, fairness in deals, avoiding house edge, showing disciplined outcome instead of luck, detecting bias / uncertainty / advantage, expected-value thinking for any economic decision, diversification and delegation, risk management, pricing and arbitrage indexing, checks against gambling, and knowledge as portfolio construction.

This symposium **does not ship code**. It produces a framework, a mapping onto existing WeftOS surfaces, workshop artifacts, and a 10-page keynote. Graduation to a real ADR / Plane item happens only if the mapping is load-bearing.

## Format (two rooms, then one table)

This is the first Grok-hosted symposium. Cadence is Claude-house (README + AGENDA + experts + panels + workshop HTML + keynote) with a **layered** run:

1. **Phase I — separate rooms.** Room **Nova** works only the book and the new doctrine. Room **Vetus** works only what WeftOS already scores (EffectVector, FitnessScorer, NodeScoring, cost circuit-breaker, MetaHarness receipts, SOUL minimax, DeFi stake, ECC coherence). They do not visit each other.
2. **Phase II — Coniunctio.** Both rooms sit at one table and combine. This is where the score contract, examples, and LDA-ADRs are born.
3. **Keynote.** Interactive HTML, McKinsey/EY density with DESIGN.md chrome. Not bound to ten slides. Dedicated slide + dedicated doc for the *already-in-tree* Cardano rhymes.

WeftOS agents (`governance-counsel`, `ecc-analyst`, `doc-weaver`, `defi-networker`) sit in Vetus and Coniunctio. Ruflo brain / AgentDB / MetaHarness score are instruments, not oracles.

## Thesis

> A score that does not name its **circuit** is a wager dressed as measurement. WeftOS should refuse to treat luck as skill, refuse undisclosed house edge, and price every economic or governance decision as an expected value over an enumerated (or honestly incomplete) sample space.

## What this symposium decides

1. What Cardano actually said (not the later Pascal-Fermat myth).
2. Which of his ideas are still the right primitive (circuit, equality, odds, power rule, frequency, fraud catalog, remaining-work division) and which we must correct (reasoning on the mean, leftover luck-as-force, incorrect problem-of-points arithmetic).
3. How those primitives land on WeftOS scoring surfaces without violating ADR-090 R1–R5 or making MetaHarness a runtime dependency.
4. What a Cardano-shaped score *looks like* as a contract: circuit, odds, edge, ruin, calibration.

## Deliverables

| # | Deliverable | Path | Status |
|---|-------------|------|--------|
| 1 | Source digest (chapters, quotes, corrections) | `sources/chapter-digest.md` | drafting |
| 2 | Workshop A — Circuitus research | `panels/P1-circuitus.md` | pending |
| 3 | Workshop B — Tabula research | `panels/P2-tabula.md` | pending |
| 4 | Team slideware (HTML) | `workshops/circuitus/index.html`, `workshops/tabula/index.html` | pending |
| 5 | Optional EV / house-edge demo | `demos/circuit-ev.html` | pending |
| 6 | Research synthesis | `deliverables/01-research-synthesis.md` | pending |
| 7 | Cardano framework (named primitives) | `deliverables/02-cardano-framework.md` | pending |
| 8 | WeftOS mapping + LDA-ADR drafts | `deliverables/03-weftos-mapping.md` | pending |
| 9 | MC opening / weave / close | `panels/P0-mc-cardano.md` | pending |
| 10 | **Keynote 10-page HTML deck** | `decks/keynote/index.html` | pending |

No Q/A session — the brief is focused.

**Studies** (live sidecar evals, not kernel): `studies/`. First attachment
is the 2026-08-14 overnight metaharness run.

## Expert roster

### MC

| Agent | Role |
|-------|------|
| `girolamo-cardano` | Host. Speaks as Cardano: physician, algebraist, gambler, author of the book. Opens, weaves both workshops, delivers keynote speaker notes. Honest about his own errors. |

### Team Circuitus — "Decode the Book"

**Workshop goal**: extract a usable, named framework from the text and its historiography (Ore, Bellhouse, Gould, Gorroochurn). Separate *scientia* from *fortuna*. Correct ROTM and leftover superstition.

| Agent | Role |
|-------|------|
| `probability-historian` | Chapter-by-chapter reading; Gould/Ore/Bellhouse/Gorroochurn. Circuit, odds, power rule, three-dice, problem of points, luck chapter. |
| `decision-theorist` | Map circuit → modern decision theory: EV, Knightian uncertainty, Kelly, Savage, calibration, ruin. What Cardano almost had. |

### Team Tabula — "Score the Table"

**Workshop goal**: land the framework on WeftOS scoring, deals, routing, MetaHarness promote, delegation, and knowledge-as-portfolio. Produce a score contract, not a vibe.

| Agent | Role |
|-------|------|
| `weftos-scoring-architect` | EffectVector, QualityScorer, NodeScoring, routing, MH score. Gaps vs circuit/EV/uncertainty. |
| `fairness-and-deals` | Equality of conditions, house edge, agreement fairness, fraud/bias detection, anti-gambling / ruin checks. |
| `knowledge-portfolio` | Diversification, delegation, remaining-work (problem of points), knowledge as a book of wagers. |

### Existing project agents used as-is

- `clawft-governance-specialist` — 5D effect algebra, three-branch gate
- `ruflo-architect` — if a later ADR graduates
- `general-purpose` / `explore` — corpus reads

## Constraints

- Exploratory. Local `LDA-ADR` only.
- Removable / optional: no new runtime dep; MetaHarness remains optional (ADR-096 / ADR-150).
- Do not evolve policies that violate ADR-090 R1–R5.
- Honest about Cardano's errors. We inherit the *questions*, not the arithmetic mistakes.
- First Grok symposium: follow Claude house format (README + AGENDA + experts + panels + workshops + keynote), execute via `spawn_subagent` + Ruflo team bus.

## Related WeftOS surfaces

- ADR-034 EffectVector (risk, fairness, privacy, novelty, security)
- `crates/clawft-core/src/agent/effects.rs`
- `crates/clawft-core/src/scoring.rs` (`QualityScorer`, `NoopScorer`, `BasicScorer`)
- `crates/exo-resource-tree/src/scoring.rs` (`NodeScoring` 6D)
- ADR-096 MetaHarness foundation
- `docs/weftos/k2-symposium/04-industry-landscape.md` §5 (explicit gap: no uncertainty quantification)
- K2 C9 / D20 — N-dimensional EffectVector still deferred

## Sources (primary)

- Girolamo Cardano, *Liber de Ludo Aleae*, in *Opera Omnia* vol. I (Lyon, 1663), pp. 262–276. Latin. [Internet Archive](https://archive.org/details/imgmar3940MiscellaneaOpal)
- Sydney Henry Gould trans., *The Book on Games of Chance* (reprinted from Øystein Ore, *Cardano: The Gambling Scholar*, Princeton 1953; Dover 1961/2015)
- Øystein Ore, *Cardano: The Gambling Scholar* (1953)
- David Bellhouse, "Decoding Cardano's *Liber de Ludo Aleae*", *Historia Mathematica* 32 (2005)
- Prakash Gorroochurn, "Some Laws and Problems of Classical Probability and How Cardano Anticipated Them", *Chance* 25.4 (2012)
- Local digest: `sources/chapter-digest.md`
