# Liber de Ludo Aleae Symposium — Agenda

**Status**: running (first Grok host)
**Date opened**: 2026-08-13
**Reference**: `README.md` for kickoff brief and expert roster.
**MC**: Gerolamo Cardano
**Q/A**: none (focused brief)

## Sequence (two rooms, then one table, then keynote)

| # | Phase | Session | Room | Leads | Output | Status |
|---|-------|---------|------|-------|--------|--------|
| P0 | Open | Cardano takes the chair | Plenary | `girolamo-cardano` | `panels/P0-mc-cardano.md` | **DONE** |
| P1 | **I — separate** | Room Nova — new doctrine from the book | New | `probability-historian` + `decision-theorist` | `panels/P1-nova.md` + `workshops/nova/index.html` | **DONE** |
| P2 | **I — separate** | Room Vetus — what WeftOS already scores | Old | WeftOS `governance-counsel` + `ecc-analyst` + scoring architect | `panels/P2-vetus.md` + `deliverables/04-existing-spaces.md` + `workshops/vetus/index.html` | **DONE** |
| P3 | **II — together** | Room Coniunctio — combine new + old | Joint | all of the above + `doc-weaver` + `knowledge-portfolio` | `panels/P3-coniunctio.md` + `demos/` + LDA-001–005 | **DONE** |
| P4 | Close | Keynote (interactive HTML, 16 slides) | Plenary | lead + MC spine | `decks/keynote/` | **DONE** |
| P5 | Close | Synthesis + mapping + local ADRs | Plenary | lead | `deliverables/01–03` | **DONE** |

No Q/A. Close after keynote.

**Phase I rule:** Nova and Vetus do **not** solve each other's problems. Nova does not retrofit WeftOS. Vetus does not invent Cardano. They meet only in P3.

## Workshop goals (binding)

### Team Circuitus — Decode the Book

1. Reconstruct the 32-chapter argument: morality → equality of conditions → circuit → odds → power rule → frequency → cards/dice/tables → fraud.
2. Name primitives in Cardano's language and in ours.
3. Mark errors (reasoning on the mean; leftover luck-as-force; wrong problem-of-points ratio) so we do not import them.
4. Produce a workshop HTML deck (not the keynote) that a cold reader can walk in 5 minutes.

### Team Tabula — Score the Table

1. Inventory every WeftOS scoring surface and say, for each: has a circuit? has odds? has edge? has ruin? has calibration?
2. Map user topics: uncertainty, fairness in deals, house edge, disciplined outcome vs luck, bias/advantage detection, EV for economic decisions, diversification/delegation, risk, pricing/arbitrage index, anti-gambling checks, knowledge as portfolio.
3. Propose a **score contract** (fields, not code) that could sit beside EffectVector without breaking genesis.
4. Produce workshop HTML + a tiny interactive demo if it clarifies EV / house edge.

### MC (Cardano)

1. Open in first person. You wrote the book. You also cheated, went broke, and published Tartaglia. Be that man.
2. Visit both workshops. Steal what is true; refuse flattery.
3. Hand the lead a 10-slide keynote spine (titles + one sentence each + speaker notes).

## Working budget (this session)

This is a **single-session** symposium (unlike sonobuoy's multi-turn). Grok lead scaffolds, fans out teams, synthesizes, builds the keynote.

1. Scaffold + source digest + expert prompts.
2. Ruflo team create + spawn both workshops + MC in one turn.
3. Collect artifacts.
4. Synthesis + keynote (ascii-svg diagrams, 10 HTML slides, SCORECARD ≥90/page, whole-deck ≥95).

## Local ADR namespace

`LDA-ADR-NNN` under `adrs/` if a panel pins a decision. Does not continue WeftOS 096+. Candidates:

- **LDA-ADR-001** — Score contract: every published score names a circuit.
- **LDA-ADR-002** — Equality of conditions as the fairness primitive (not a 0–1 vibe).
- **LDA-ADR-003** — House-edge / advantage index on opportunities and routing.
- **LDA-ADR-004** — Ruin / anti-gambling check before treasury-scale bets.
- **LDA-ADR-005** — Remaining-work EV for interrupted / delegated tasks (problem of points).

## Gap-analysis methodology

Mirror `scripts/weaver/analyze-gaps.py` applied to scoring:

1. **Orphan claims**: scoring language in docs with no circuit.
2. **Untested capability**: QualityScorer / EffectVector claims without calibration against outcomes.
3. **Pending decisions**: K2 C9 N-dimensional EffectVector; uncertainty quantification flagged in k2 `04-industry-landscape.md`.
4. **Cross-corpus inconsistency**: 5D vs 6D vs MH scores with no conversion.

Each gap → `gap-analysis/G-NN-<topic>.md` if a team finds one.

## Coherence pass

After workshops land, one linear pass: primitive names identical in P0, P1, P2, deliverables 01–03, and keynote. Headline thesis quoted identically. Cross-references resolve.

## A note on Plane

Exploratory. No Plane item until a mapping graduates. If deliverable 03 returns "go", recommend cycle (`0.8.x` or later) and which LDA-ADRs become real ADRs.
