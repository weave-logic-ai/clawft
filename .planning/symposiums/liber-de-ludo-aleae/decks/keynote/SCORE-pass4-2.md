# SCORE-pass4-2 — Liber de Ludo Aleae (all four decks)

**Pass:** 4  
**Scorer:** 2 of 4 (independent cold read).  
**Did not read:** `SCORECARD.md`, `SCORE-pass2-*.md`, `SCORE-pass3-*.md`, or any other `SCORE*` file.  
**Did not edit** HTML, outlines, or assets.

**Read:** `references/rubric.md`; keynote `index.html` + `OUTLINE.md` + three SVGs + both heroes; `workshops/{nova,vetus,coniunctio}/index.html`; `AGENDA.md`; `README.md`; `panels/P1-nova.md` / `P2-vetus.md` / `P3-coniunctio.md` (heads + landings); `deliverables/02-cardano-framework.md`, `03-weftos-mapping.md`, `04-existing-spaces.md` (inventory); `adrs/LDA-ADR-001`…`005`; `demos/circuit-ev.html` + `demos/examples.md`.

Page scores are against **that slide’s own goals** (keynote: `data-primary-goal` / `data-secondary-goals`; workshops: title + lede + on-slide so-what, because Nova/Coniunctio never declare `data-*-goal`). Whole-deck is **not** a page average.

**Pass bar:** page ≥ 95 and whole-deck ≥ 95.

| Deck | Pages < 95 | Whole-deck | Ship? |
|------|------------|------------|-------|
| Keynote | none | **96** | **yes** (this scorer) |
| Nova | none | **92** | **no** — whole-deck 90–94 |
| Vetus | **V7 = 94** | **95** | **no** — page fail |
| Coniunctio | none | **95** | **yes** (this scorer, on the line) |

---

## Keynote — `decks/keynote/index.html` (K01–K16)

Thesis on the glass: *Count the circuit before you score.* Audience: WeftOS / clawft engineers, governance, MetaHarness operators.

| id | score | fail? |
|----|------:|:------|
| K01 | 97 | |
| K02 | 95 | |
| K03 | 97 | |
| K04 | 96 | |
| K05 | 96 | |
| K06 | 97 | |
| K07 | 96 | |
| K08 | 95 | |
| K09 | 98 | |
| K10 | 97 | |
| K11 | 97 | |
| K12 | 95 | |
| K13 | 95 | |
| K14 | 96 | |
| K15 | 97 | |
| K16 | 98 | |
| **Whole-deck** | **96** | |

### Page notes

**K01 · 97.** Primary (“thesis in one glance”) is the H1 itself; the lede restates it with a first-use gloss on *circuit*; the so-what names WeftOS scores as hidden dice. Secondaries land: Cardano chairs in the lede, “First Grok symposium” in the eyebrow, 1564 / 1663 / 2026 as the three-beat. Hero is dark enough that `#E0DEE8` holds; no clutter.

**K02 · 95.** Three room-cards state Nova → Vetus → Coniunctio in five seconds; rhyme / cousin / false friend and *genesis* are glossed on first use; Try / See / Why matches the click-highlight. Residual risk, not a fail: `.lede` / `.so` sit on a 45% overlay over the tan half of `hero-two-rooms.jpg`, so dim text is the weakest contrast in the deck. Cards themselves are solid panels and carry the method.

**K03 · 97.** Live die enumerates a six-face *circuitus*, prints `r : s`, `p = r/6`, and fair payout `s/r`; refuse-on-empty is correct. SVG flow (circuitus → favorable → odds → fair wager / EV = 0) is readable and matches ch. 14. Try / See / Why is next to the control.

**K04 · 96.** Slider is the promised np-as-P toggle: at `n = 3`, ROTM = 0.50 and circuit = 0.421; copy updates when the circuit crosses ½. Three-dice SVG (25 / 27 / 27 / 25 in 216) is the right second contaminant and is captioned as partitions ≠ ways. Slightly two ideas, but the primary (do not inherit ROTM) is never ambiguous.

**K05 · 96.** Dedicated wall: thirteen chips, class in the label, expand-in-place, honest “13 of 31” plus the inventory path. Auto-opens V1 so the glass is never empty. Classifications match `04-existing-spaces.md` (V6 false friend, V17 marked-deck rhyme).

**K06 · 97.** Homonym is one sentence and one diagram: *What can happen?* vs *When do we leave?* WEFT-322 is glossed in the lede. So-what (“perfect stop-loss, crooked table”) is on the glass, not in notes. Static SVG, so missing Try / See / Why is not a rubric hit.

**K07 · 96.** Faces are exactly this session’s throw (75 / 100 / 65 / 90 / 53); `n = 1` and “no interval” are in the lede; cost `$0.024` is fenced as a price tag, not a sixth face; prior 2026-07-31 snapshot is labeled a pair, not a calibration. Color rule is in Try / See / Why. Bars-as-“radar” are readable; the page is tight in `vh` but not a wall.

**K08 · 95.** Lede splits the fairness *dim* from *aequitas*; so-what is the landing rule (check beside, do not rename). Both secondaries are present: L2 formula + √5 + 0.7 vs 0.8, and the counsel 5D ≠ ADR-034 5D. No weight-drag (outline-only); HTML goals do not require it. The two cards are examples of unequal conditions, not a picture of the two primitives — enough to pass, not a 98.

**K09 · 98.** House-edge identity matches LDA-003: at 4 : 1, EV = −0.167 and edge = 16.7% (not the 20% payout-shortfall). Fair 5 : 1 reads EV = 0. Hide-list (router / eval / vendor / judge) is the product mapping. Try / See / Why tells the reader to drag to 4.

**K10 · 97.** `n` slider changes claim type at 5 and 30 and *says* those cutovers are this house’s rule, not Bernoulli. Flywheel receipts and no-silent-promote are on the glass. Try / See / Why is complete.

**K11 · 97.** 5–3 of 6 is the worked circuit: Pacioli 5 : 3 (forbidden), still-needed 1 vs 3, Pascal 7 : 1 because BBB is 1/8. Explicitly refuses both Cardano’s 6 : 1 and the 0.5 : 0.125 = 4 : 1 independent-task mix-up that `demos/examples.md` E5 still prints as a WeftOS split. Swarm so-what is on-slide. LDA-005 is in the eyebrow.

**K12 · 95.** Relative link `../../demos/circuit-ev.html` resolves; the demo itself has Try / See / Why and the same 16.7% identity. Secondary (“knowledge as a book of wagers”) is the so-what plus a pointer at E1–E10 — not a portfolio widget. Primary is “open the demo,” and it does.

**K13 · 95.** Four field cards are the sidecar (circuit, odds/edge, stake/ruin, calibration) plus the LDA-001…005 strip; genesis-untouched is the so-what. Cards are `tabindex` and hover like expanders but reveal nothing — a false affordance, deducted as clutter, not a goal miss.

**K14 · 96.** Tabs E1 / E2 / E4 / E5 / E6 / E7 / E10 are countable landings, not metaphors; E5 repeats the 7 : 1 vs 4 : 1 distinction. Subset of E1–E10 is fine against the HTML goal (“make it grokable”). Try / See / Why names the three to click.

**K15 · 97.** Locked vs sidecar is a two-column refusal list that matches `deliverables/03-weftos-mapping.md` (no sixth face, no rename, no ROTM-as-prior, no invented dollars from $0.024, no auto-promote). So-what names silent self-graduation as already-forbidden promote.

**K16 · 98.** Three moves are the same three as Coniunctio C5 and deliverable 03 (align counsel, sidecar one MH throw, leave genesis / Plane). Thesis refrain closes the arc. “Deliverable 03” and “Plane” are glossed on the slide.

### Keynote whole-deck · 96

| Dimension | /guide | Score | Note |
|-----------|-------:|------:|------|
| Narrative arc | 25 | 24 | Thesis is slide 1, not 2–3. Method → doctrine → contaminants → walls → instruments → contract → refusals → ask. K12–K14 is a short appendix bulge, still in order. |
| One idea per slide | 20 | 18 | K04 (ROTM + three-dice), K08 (L2 bar + marked briefing), K12 (demo + portfolio sentence) are the only doubles. |
| Visual system | 15 | 14 | `docs/DESIGN.md` chrome (`#08080A` / `#C4A25C` / `#6EC896` / `#DC5F5F`), two heroes, three labeled SVGs, glossary drawer, progress dots. K02 overlay and K13 false affordance keep it off 15. |
| Fidelity to source | 20 | 20 | MH faces, 25/27/216, 7 : 1, 16.7%, V-classes, LDA field set, ADR-090/096 refusals — checked against P2/P3, LDA-001–005, and `04-existing-spaces.md`. No dollar-savings invention. |
| Decision value | 20 | 20 | Viewer leaves with three concrete moves and a locked/sidecar split. |

Does **not** fail any page. Ship for this scorer.

---

## Nova — `workshops/nova/index.html` (N1–N8)

Book-only room. No `data-primary-goal` metadata (process gap; scored from title / so-what). Glossary present. Phase I rule held: zero crate types.

| id | score | fail? |
|----|------:|:------|
| N1 | 96 | |
| N2 | 97 | |
| N3 | 95 | |
| N4 | 96 | |
| N5 | 97 | |
| N6 | 97 | |
| N7 | 96 | |
| N8 | 96 | |
| **Whole-deck** | **92** | **yes — below 95** |

### Page notes

**N1 · 96.** Room law in one breath: extract a kit, leave the arithmetic crimes, do not visit WeftOS. So-what (glossary is the primitive gate) is the right door for a 5-minute walk. Sparse, not confused.

**N2 · 97.** *Aequitas* is glossed as the principle; probability is the check, not the trick. Bake-off / marked-deck so-what is a decision sentence a cold engineer can use the same day.

**N3 · 95.** Calculator is a real circuit: `p = r/(r+s)`, fair odds `r:s`, even-money EV distinguished from those odds, `p²` as the repeat rule (do not square the odds). Default 27 : 25 is named as three-dice 10 vs 9. Missing a `.so` product line — Why carries the teaching, so this is a −5, not a −10. Empty circuit refuses.

**N4 · 96.** ROTM 3 × 1/6 = ½ vs 1 − (5/6)³ ≈ 0.421, four throws to cross a half. Two cards, one idea. So-what forbids treating `n×p` as `P`.

**N5 · 97.** Lean-detector kept, Prince fired, in Cardano’s own “always just equal” clause (P1 §2.2). So-what makes a too-perfect score as suspicious as a biased one.

**N6 · 97.** Right question / wrong triangles / Pascal 7 : 1, and it names the two wrong numbers (6 : 1, 4 : 1). So-what is leftover budget, not sunk tokens.

**N7 · 96.** Equal stake ≠ equal ruin; stop-loss ≠ sample space. That is the LDA-004 / WEFT-322 distinction without ever saying a crate name. So-what is on the glass.

**N8 · 96.** May-leave / may-not is the Coniunctio contract: primitives out, Prince / ROTM / multiply-odds / 6 : 1 / cheating-as-method / crate types stay. `pⁿ`, `np`, and the fraud catalog are exported even though they never got their own boards — honest kit, slightly front-loaded onto the last card.

### Nova whole-deck · 92

| Dimension | /guide | Score | Note |
|-----------|-------:|------:|------|
| Narrative arc | 25 | 23 | Kit arc is clean (equality → circuit → ROTM → lean → points → ruin → handoff). It is **not** the 32-chapter reconstruction AGENDA Team Circuitus still lists; power, frequency, fraud, and mixed games exist only as N8 tokens. Thesis sentence is never quoted. |
| One idea per slide | 20 | 20 | Best one-idea discipline of the four decks. |
| Visual system | 15 | 12 | Same gold / ink tokens and glossary drawer as the keynote, but no grain, no diagram, no progress dots, no hero. Reads as a typed primer, not the same object class as Vetus / keynote. |
| Fidelity to source | 20 | 19 | Primitives and refusals match `02-cardano-framework.md` and P1. 27 : 25 and 7 : 1 are right. Missing beats are omissions, not errors. |
| Decision value | 20 | 18 | Coniunctio can take the kit. A cold reader can walk it in five minutes. It does not yet *feel* like a finished room next to Vetus. |

**90–94 rule:** do not polish chrome on a thin arc — add one board (power / `pⁿ`, or frequency / `np`, or the fraud catalog as detector-only) **or** bring the visual system up to Vetus (cards on every board, one SVG, progress). Pages can stay.

---

## Vetus — `workshops/vetus/index.html` (V1–V8)

Inventory room. Strongest workshop artifact. Phase I rule held: no combining contract, no Cardano arithmetic.

| id | score | fail? |
|----|------:|:------|
| V1 | 96 | |
| V2 | 96 | |
| V3 | 95 | |
| V4 | 97 | |
| V5 | 95 | |
| V6 | 98 | |
| V7 | **94** | **FAIL** |
| V8 | 97 | |
| **Whole-deck** | **95** | |

### Page notes

**V1 · 96.** Title is the room. Six dice are this session’s MH cast, memory in crit, `$0.024` fenced as a price tag, prior throw cited. Try / See / Why points at the red face. Intro + live cast is one exhibit (the dice already on the table), not two theses.

**V2 · 96.** Four seats, one Phase I rule, honesty line (structural influence, no bibliographic cite). So-what: Coniunctio cannot claim the walls were unread. Role names are glossed in-line.

**V3 · 95.** Thirty-one expandable rows, class pills, symbols, Try / See / Why on V1 / V6 / V17. The 5-second read is the title + “classification is not a merge.” Scroll-in-52vh is the right pattern for an inventory; it is dense by job, not by neglect. Tabula’s five questions (circuit / odds / edge / ruin / calibration) are in the row bodies, not in columns — acceptable against *this* slide’s goal.

**V4 · 97.** Breaker vs *circuitus* is a labeled ≠ with shipped symbols (`BudgetUsage.circuit_open`, `circuit_breaker_no_op_limit`, `TerminationReason::CircuitBreaker`) and the routing Level-2 **stub** called out. So-what is the same punch as K06. Fairness-is-not-*aequitas* is a one-line fence, not a merge.

**V5 · 95.** Three closest casts (MH, EffectVector L2, FitnessScorer), each still missing a sample space — that *is* the landing pad. GEPA / RVF / WEFT-54 are only half-glossed (Noop 1.0 vs 0.5 and “not a safety control” save it). Stated audience is in-house; still a cluster of first-use ticket-speak. Primary goal survives.

**V6 · 98.** Marked deck is a picture: `cpu · memory · network · storage · trust_delta` vs `risk · fairness · privacy · novelty · security`. So-what is “equal our own table before we lecture.” 0.7 vs 0.8 sits as a sibling inequality, not a hijack.

**V7 · 94. FAIL.** Primary idea (coherence is four formulas; do not collapse twins) is on the glass and classed. The unexplained-term test fails on the last twin: “EML `uncertainty` does *not* close K2 §5.” Neither **EML** nor **K2 §5** is glossed on this slide or in the room glossary, and a cold reader in the stated audience cannot say what K2 §5 *is* without opening another file. That is an −8-class hit; I take −6 because it is a note-level bullet, not the H2. Fix: one parenthetical — “K2 industry-landscape §5: no uncertainty quantification on EffectVector” — and `EML` → “in-tree world-model head.”

**V8 · 97.** Handoff list is complete (V1–V31 path, homonym + stub, four-way coherence, three live scores, counsel drift, no genesis smash, C9 deferred). Explicit non-delivery (no contract, no LDA text, no Cardano arithmetic) is the Phase I so-what.

### Vetus whole-deck · 95

| Dimension | /guide | Score | Note |
|-----------|-------:|------:|------|
| Narrative arc | 25 | 24 | Inventory → who sat → table → homonym → closest casts → marked deck → collisions → handoff. Thesis of the room is slide 1. |
| One idea per slide | 20 | 18 | V1 is intro + cast; V7 is two collision families. |
| Visual system | 15 | 14 | Chrome header/footer, class pills, dice, ≠ columns, sticky dots. Same DESIGN tokens. No hero, still a professional console. |
| Fidelity to source | 20 | 20 | V1–V31 bodies and classes match `04-existing-spaces.md` (including V6 false friend, V23/V24/V27 coherences, V31 unshipped). MH numbers match P2. |
| Decision value | 20 | 19 | Coniunctio may land only on named gaps. Correctly refuses to write the contract in this room. |

Whole-deck would ship; **V7 must be revised** before the page gate is green.

---

## Coniunctio — `workshops/coniunctio/index.html` (C1–C5)

Combining table. No `data-primary-goal` metadata. Four-test rule matches P0/P3.

| id | score | fail? |
|----|------:|:------|
| C1 | 96 | |
| C2 | 96 | |
| C3 | 96 | |
| C4 | 95 | |
| C5 | 97 | |
| **Whole-deck** | **95** | |

### Page notes

**C1 · 96.** One sentence, four tests (named gap, no genesis break, not Cardano’s error, both parents cited). So-what — orphan claims honorable, forgeries not — is the room’s moral. Sparse on purpose; 5-second read holds.

**C2 · 96.** Five landings = LDA-001…005, each on a Vetus gap already named (silence / fairness vibe / no edge index / breaker-as-stop-loss / delegation-as-classifier). So-what fences them as sidecar fields, not a sixth genesis face. Table matches P3 §3 and deliverable 03.

**C3 · 96.** Not-landed list is the same five forgeries as K15 / 03 (WEFT-322 as *circuitus*, fairness as *aequitas* without the check, ROTM as prior, $0.024 savings, auto-promote). One paragraph, not a wall.

**C4 · 95.** Field list matches the P3 JSON (circuit, favorable, odds, stake, edge, ruin, calibration, claim_type, equal_conditions) including “print the hole.” Demo link resolves and is captioned Try / See / Why. Small so-what wobble: after listing the *whole* contract it says “that is LDA-003 as a die” — true of the explorer, slightly narrower than the slide. −3, not a goal miss.

**C5 · 97.** Same three moves as K16 / 03, plus a minutes link. So-what is “before anyone files a central ADR.” Plane is in the glossary.

### Coniunctio whole-deck · 95

| Dimension | /guide | Score | Note |
|-----------|-------:|------:|------|
| Narrative arc | 25 | 24 | Tightest of the four: rule → land → refuse → contract → ask. |
| One idea per slide | 20 | 20 | No overloaded board. |
| Visual system | 15 | 11 | Same tokens and glossary, but tables-only, no cards on C1/C3, no sidecar diagram, no texture. Professional as minutes; thinner than Vetus. Narrative weight holds the 95. |
| Fidelity to source | 20 | 20 | Landings, refusals, sidecar fields, and the three moves are bit-identical in spirit to P3 + LDA-001–005 + 03. |
| Decision value | 20 | 20 | Viewer knows the three moves and what must not graduate. |

On the line. A sidecar field-strip (the K13 four-card) on C4 would take visual off the knife-edge; it is not required for *this* scorer to pass the deck.

---

## Cross-deck coherence (not a fifth score)

- Thesis string is identical in README, P1, P3, K01, K16.
- MH throw 75 / 100 / 65 / 90 / 53 @ $0.024 is identical in K07, V1, V5, examples E6.
- Three moves are identical in K16, C5, and deliverable 03.
- 7 : 1 (match) vs 4 : 1 (independent remaining-work) is handled carefully on K11 / K14 and N6; `demos/examples.md` E5 still prints 4 : 1 as the WeftOS split without the keynote’s warning — that is a **source** wart, not a deck fail.
- Nova and Coniunctio still lack per-slide goal metadata the skill requires. That did not move any page under 95; it did make workshop scoring inferential.

## What must move before a Pass-4 merge can ship all four

1. **Nova whole-deck (92).** Add one missing kit beat (power, frequency, or fraud-as-detector) **or** raise chrome to Vetus. Do not average this with 96.
2. **Vetus V7 (94).** Gloss `EML` and `K2 §5` on the slide (or in the room glossary *and* pointed from the bullet).
3. Optional, not gating for this scorer: K02 overlay contrast; K13 dead `tabindex`; C4 “LDA-003 as a die” so-what; Nova/Coniunctio `data-primary-goal` attributes.

Keynote (96) and Coniunctio (95) ship for this scorer. Merge = **minimum** across Pass-4 scorers, not the mean.
