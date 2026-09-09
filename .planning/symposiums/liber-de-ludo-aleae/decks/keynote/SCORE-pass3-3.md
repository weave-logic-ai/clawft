# SCORE — Pass 3, Scorer 3

- **Scorer:** 3 of 3 (independent). Did not read `SCORE-pass2-*`, `SCORECARD.md`, or other Pass 3 files.
- **Date:** 2026-08-13
- **Rubric:** `~/.grok/skills/deck/references/rubric.md` (page ≥95; whole-deck ≥95, not a page average).
- **Fidelity locks:** Pascal race-to-6 stopped 5–3 is **7:1**. House edge at 4:1 on a fair six is **16.7% of stake** if defined as **−EV**.
- **Method:** HTML + inline JS + SVGs + hero assets + LDA-ADR-003 / P3 landings. No live screenshot pass.

**Ship (this scorer):** **NO** — any page <95 or any whole-deck <95 blocks.

---

## Keynote · `decks/keynote/index.html` (K01–K16)

Goals taken from each section’s `data-primary-goal` / `data-secondary-goals`.

### K01 — Count the circuit before you score — **97**

Primary (thesis in one glance) lands in the h1 + lede; `circuit` and *Liber de Ludo Aleae* are first-use glossed; so-what is on-slide (WeftOS scores that hide a sample space are bets). Secondary (Cardano chairs; first Grok symposium) is in the lede and eyebrow. Hero grid is texture, not clutter; 1564 / 1663 / 2026 cards scan. −3 only for a slightly poetic first sentence before the so-what names gates / quality / harness.

### K02 — Two rooms, then one table — **96**

Primary (two-room method) is the title and the I / I / II cards. Nova, Vetus, Coniunctio, circuitus, aequitas, rhyme / cousin / false friend, and genesis are glossed on first use. Try / See / Why is present. Click-to-highlight is thin (border only, no new charge) but the support line states the point. So-what (fairness rule applied to inquiry) is on-slide.

### K03 — The general rule — **97**

Primary (ch. 14 as a live circuit) is the face picker; secondary (odds `r:s` = fair price) prints `r : s`, `p=r/6`, and fair payout. Diagram `circuitus.svg` is readable (CIRCUITUS → FAVORABLE → ODDS → FAIR WAGER / EV=0). Try / See / Why sits on the control. Default face 6 ⇒ 1:5 / 5:1 is the fair six. So-what (EV = 0 is justice) is on-slide.

### K04 — What you will not inherit — **91** FAIL

Primary (refuse ROTM) is met: slider at `n=3` shows ROTM **0.50** vs circuit **0.421**; four throws to cross ½ is correct (`1−(5/6)⁴≈0.518`). Try / See / Why and the so-what (length-based quality scores) are on-slide. Fail deductions: right card is a second slide (Prince, multiply-odds, triangular split, cheating-as-method, *and* the 25-vs-27 chart) — dense hierarchy **−5**; “triangular split of an interrupted match” is first-use with no meaning until K11 — **−4**. `three-dice.svg` itself is accurate (25 / 27 / 27 / 25 on 216).

### K05 — What was already in the walls — **95**

Primary (existing rhymes) and secondary (dedicated wall; expand V-chips) hold: thirteen chips, class on the chip, one-line gap in `#vdetail`. Try names V1 / V6 / V7 / V17. So-what (name rhymes and false friends; do not invent a green field) is on-slide. Rhyme / cousin / false friend defined before or as you click. Barely a 95: default pane is a prompt, not a worked rhyme, and thirteen chips will wrap; the support line covers the empty state.

### K06 — Circuit is not a circuit-breaker — **98**

Primary (split the homonym) is the title; secondary (circuitus vs WEFT-322) is in the lede and `homonym.svg` (What can happen? vs When do we leave?). So-what (perfect stop-loss, crooked table) is on-slide. Breaker is glossed. Static SVG is self-labeled; no widget, so no Try/See/Why debt.

### K07 — Five unlabeled dice — **96**

Primary (MH as one cast) and secondary (75 / 100 / 65 / 90 / 53) match the bars and the footer. `n = 1`, no interval, cost tag **$0.024 is a price not a sixth face**, prior 2026-07-31 snapshot called out. Memory bar is actually red (`v < 60`). Try / See / Why is present. Title “unlabeled” means unlabeled *favorable set* — lede saves it in one breath.

### K08 — Fairness is not yet aequitas — **96**

Primary (fairness dim ≠ aequitas) is the title + lede; both terms glossed. Secondary (ADR-034 L2; counsel drift) is the two cards, including √5 ≈ 2.236, 0.7 vs 0.8, and cpu/memory/… vs risk/fairness/…. So-what (do not rename the face; add an equality check) is on-slide. −4 because L2 magnitude and the marked briefing are two different inequalities sharing one beat; still one principle (our table is already uneven).

### K09 — House edge on opportunities — **85** FAIL

Live JS is the −EV definition: `ev = (1/6)*payout + (5/6)*(-1)`, `edge = −ev` when negative. At payout **4**, EV = **−0.167**, edge = **16.7% of stake**. Support text states that. **That number is correct if house edge is −EV.** Fail: the same `<abbr>` defines house edge as `1 − (offered payout ÷ fair payout)`, which at 4:1 vs 5:1 is **20%**, and that is also the formula in `LDA-ADR-003`. Glossary-adjacent wording (“how much worse the offer is than fair”) can be read either way. Two formulas, one displayed number — wrong-science **−15**. Try / See / Why and the hide-list (router / eval / vendor / judge) are otherwise good.

### K10 — Luck is not a method — **95**

Primary (`n` before a method claim) is the slider; claim_type moves fortuna → mixed → scientia. Secondary (flywheel already forbids silent promote) is in the lede. Promote and scientia/fortuna are glossed. Try / See / Why present. 5 and 30 are demo cutoffs, not claimed as Cardano’s law; notes correctly refuse Bernoulli inflation.

### K11 — Pay what remains — **97**

Fidelity lock **held:** Pacioli sunk **5:3** forbidden; still needed **1 vs 3**; Pascal remaining paths **7:1** (B wins only as BBB = 1/8); explicitly not 4:1 and not Cardano’s triangular 6:1. So-what (interrupted swarm leftover budget) is on-slide, not only in notes. Three-card hierarchy is the 5-second read.

### K12 — Lay the wager — **95**

Primary (open the explorer) is the `demos/circuit-ev.html` CTA with Try / See / Why (leave six, drag to 4, negative EV). Secondary (knowledge as a book of wagers) is the so-what. Teaching-toy disclaimer is present. Barely 95: the count does not happen on this slide, and E1–E10 overlap K14.

### K13 — Sidecar, not genesis — **97**

Primary (sidecar contract) is four fields (circuit / odds-edge / stake-ruin / calibration). Sidecar and genesis glossed. So-what (ship the contract this quarter; no sixth face) is on-slide. Secondary (LDA-001 fields) plus the 001–005 local-namespace line. Hole-as-honesty is stated.

### K14 — Make it grokable — **96**

Primary (tabbed worked cases) works; default E1 is filled. E1: 4:1 vs 5:1, EV = −1/6, edge **16.7%** (−EV, consistent with K09’s *number*). E5: **7:1**, and “0.5:0.125 is not the match.” Try names E1 / E2 / E7. So-what (replay on a six-die or keep it out of the kernel) is on-slide.

### K15 — What we will not do — **96**

Primary (refusals) and secondary (no genesis smash; no auto-promote) are the Locked column, including ROTM-as-prior, breaker rename, fairness-without-check, invented $ from $0.024, auto-promote. Sidecar column restates the ship path. So-what (self-graduating exploration is a silent promote) is on-slide. Six-plus-six is long but the split is the hierarchy.

### K16 — Three moves — **98**

Primary (three next moves) and secondary (drift fix; one sidecar eval; no genesis) are the three cards. Plane and “Deliverable 03” are glossed in the lede. So-what (do these before a central ADR) is on-slide. Thesis returns as the last line.

### Keynote whole-deck — **91** FAIL (not the page average)

| Dimension | /weight | Score | Note |
|-----------|--------:|------:|------|
| Narrative arc | 25 | 24 | Thesis on K01, method K02, doctrine K03–04, walls K05–08, instruments K09–11, contract K12–15, ask K16. Cold reader has the thesis by slide 1. |
| One idea per slide | 20 | 17 | K04 is two slides; K12/K14 echo examples; K08 splits L2 vs briefing. |
| Visual system | 15 | 14 | Gold/ink/cards/glossary consistent; SVGs labeled; CSS radar is crude but colored honestly. |
| Fidelity to source | 20 | 16 | Pascal **7:1** good. ROTM 0.421 good. MH faces match the live throw. **K09 defines edge two ways** (ADR-003 20% formula vs displayed −EV 16.7%). |
| Decision value | 20 | 19 | Three moves, sidecar-not-genesis, no silent promote. Viewer knows what to do. |

**24+17+14+16+19 = 91.** Do not ship. Fix K09’s definition (pick −EV *or* `1−offered/fair`, not both) and unpack K04 before another pass.

---

## Room Nova · `workshops/nova/index.html` (N01–N08)

No `data-*-goal` attributes. Goals inferred from titles + P1 charge (book only; no crate types).

### N01 — Doctrine from the book — **96**

Primary (this room is book-only) is the first sentence. So-what (if it is not in the glossary, it is not a primitive) is on-slide and teaches G. Sparse on purpose; not a wall, not a missing diagram.

### N02 — Equal conditions first — **97**

Primary (aequitas before probability) is the lede; aequitas is glossed. So-what (a bake-off that hands one model the answers is a marked deck) is a decision, not a vibe. Cardano’s list (opponent, bystanders, money, situation, box, die) is the check.

### N03 — Count the whole circuit — **92** FAIL

Primary (circuit = all faces; wager `r:s`) is the title + lede. Tiny calculator defaults to **27:25** (three-dice 10 vs 9) — correct circuit. Try / See / Why is present. Fail: output is classed `.so` and prints **even-money EV** next to “Fair odds r:s” without saying even-money means bet 1 to win 1. At 27:25 that EV is ≈ +0.038, which a reader who just learned “fair ⇒ EV = 0” (glossary) will read as an unfair circuit. Unexplained term + contradictory so-what **−8**.

### N04 — Refuse ROTM — **96**

`3×1/6 = ½` vs `1−(5/6)³ ≈ 0.421`; four throws to cross a half. False-claim vs circuit cards. So-what (never treat `n×p` as `P`) is on-slide. No slider (unlike K04) but the one idea is clean.

### N05 — Lean is not luck — **97**

Primary (keep the lean detector; fire the Prince) — Prince glossed as luck-as-a-person. So-what (a score that always “just equals what it should be” is as suspicious as a lean) is sharp and book-legal.

### N06 — Pay remaining work — **96**

Fidelity lock **held:** Cardano asked the right question and paid triangular **6:1**; Pascal remaining paths **7:1**, not 6:1 and not 4:1; BBB = 1/8. So-what (do not credit tokens already burned) is slightly WeftOS-flavored for a book room but names no crate type. Arithmetic is the load-bearing claim and it is right.

### N07 — Small stakes, named ruin — **97**

Ruin glossed. Rich purse vs thin purse on the same stake is not an even table. So-what (stop-loss is not a sample space) is the homonym taught without visiting WEFT-322 — correct for Nova.

### N08 — What Coniunctio may use — **96**

May-leave / may-not lists match P1 primitives and refusals (Prince, ROTM, multiply-odds, triangular 6:1, cheating-as-method, crate types). So-what (combining is next door) + `P1-nova.md`. `np` in the leave column is frequency, not ROTM; ROTM is in the refuse column.

### Nova whole-deck — **91** FAIL

| Dimension | /weight | Score | Note |
|-----------|--------:|------:|------|
| Narrative arc | 25 | 24 | Room rule → aequitas → circuit → ROTM → lean → points → ruin → handoff. Thesis of the room is clear by N01–N03. |
| One idea per slide | 20 | 19 | Clean kit. N03’s calculator is the only muddy beat. |
| Visual system | 15 | 12 | Same gold/left-bar language as the keynote, but no diagrams, no cards on most boards, no texture. Workshop-plain, not keynote-grade. |
| Fidelity | 20 | 18 | 7:1 good. 27:25 good. ROTM good. Even-money EV on N03 fights the glossary’s EV = 0. |
| Decision value | 20 | 18 | Handoff is usable. Book-only rule holds except a light swarm/token so-what on N06. |

**24+19+12+18+18 = 91.** Do not ship. Fix N03’s even-money line (show fair EV = 0 at odds `r:s`, and label even-money separately).

---

## Room Vetus · `workshops/vetus/index.html` (V01–V08)

Goals inferred from eyebrows / titles + P2 charge (inventory only; class, do not merge).

### V01 — What WeftOS already scores — **90** FAIL

Primary (this workshop is the inventory) and so-what (combining is next door) are stated. MH faces **75 / 100 / 65 / 90 / 53** + **.024** as a price tag, `n` unnamed — matches K07. Support says **“Read the red memory face”** but every `.die .face` is accent gold; nothing is red. Try / See / Why that names a color the chart does not use — **−10**. The sixth tile still *looks* like a face despite the caption.

### V02 — Four voices, one rule — **96**

Four seats + Phase I wall (Nova does not retrofit; Vetus does not invent Cardano). So-what (Coniunctio cannot claim nobody looked at the walls) is on-slide. Class = rhyme / cousin / false friend is restated. Honesty line (no file cites the book) is the right epistemic claim.

### V03 — Thirty-one surfaces, three classes — **96**

Primary (classed inventory) is the live V1–V31 table; expand-a-row is the interaction. Try names V1 / V6 / V17. So-what (classification is not a merge; a rhyme still lacks a circuit) is on-slide. Scrollable table is the right object for this board, not a wall of prose. This is the set Coniunctio is allowed to land on.

### V04 — Circuit vs circuit-breaker — **98**

Best board in the room. In-tree breaker (`BudgetUsage.circuit_open`, no-op limit, `TerminationReason`, routing.md Level-2 **stub**) ≠ in-book circuitus. So-what (perfect stop-loss, crooked table) matches K06. Stake vs fairness pills prevent the next collapse.

### V05 — Casts you can recount — still no circuit — **95**

Three closest casts (MH 5-die, EffectVector L2 with 0.7 vs 0.8, FitnessScorer 0.4/0.2/0.2/0.2 − 0.2 plus two Noops 1.0 / 0.5). So-what (none name a sample space; that is the landing zone) is on-slide. Dense cards, but three labeled casts. Barely 95 for crate-level density (RVF, WEFT-54) without a first-use gloss on RVF.

### V06 — Governance-counsel EffectVector drift — **96**

Marked deck: briefing `cpu · memory · network · storage · trust_delta` ≠ live `risk · fairness · privacy · novelty · security`. Sibling 0.7 vs 0.8. Lede carries the so-what (Vetus does not “fix” it; Coniunctio may use it). No `.so` rule-line; the lede is enough. Visual `≠` matches V04.

### V07 — Words already spent twice — **91** FAIL

Two clusters on one board: four *coherence* formulas, and the twins (GateDecision ×2 + GovernanceDecision, two Trajectory types, two breakers, EML uncertainty). So-what (always say which formula; do not inventory a stub as live) is on-slide and true. **−5** one-idea strain; **−4** first-use **K2 §5** with no gloss (not in the on-page glossary either). Stub 7-factor callout is otherwise the right honesty.

### V08 — What Vetus hands Coniunctio — **97**

Named gaps only; sidecar first; C9 deferred; explicit non-delivery (no contract, no LDA text, no Cardano arithmetic). So-what is the last rule of Phase I. Handoff is actionable.

### Vetus whole-deck — **90** FAIL

| Dimension | /weight | Score | Note |
|-----------|--------:|------:|------|
| Narrative arc | 25 | 23 | Live cast → who sat → table → homonym → three scores → marked deck → other homonyms → handoff. V01’s MH hero slightly steals the inventory thesis. |
| One idea per slide | 20 | 17 | V07 is two inventories. V01 mixes “here is the room” with a six-tile MH throw. |
| Visual system | 15 | 12 | Chrome, pills, `≠` cards are consistent. V01 Try claims red and paints gold. |
| Fidelity | 20 | 19 | Faces, 0.7/0.8, stub-not-shipped, breaker ≠ circuitus all match the tree story. No Pascal/edge arithmetic on this deck. |
| Decision value | 20 | 19 | Coniunctio may land only on V1–V31. Do not smash genesis. |

**23+17+12+19+19 = 90.** Do not ship. Color the memory face (or drop “red”) and split V07.

---

## Room Coniunctio · `workshops/coniunctio/index.html` (C01–C05)

Goals inferred from titles + P3 four tests.

### C01 — A primitive lands only on a named gap — **97**

Four tests (named gap, no genesis break, not Cardano’s error, both parents cited) are the lede. Sequence Nova → Vetus → this room is stated. So-what (orphans honorable; forgeries not) is the ethic of the room.

### C02 — What landed — **97**

Table 001–005 matches P3: circuitus, aequitas check, odds/edge, ruin, remaining work — each on a named Vetus gap. So-what (sidecar fields, not a sixth genesis face) is on-slide. circuitus / aequitas hover-glossed.

### C03 — What did not land — **96**

Matches P3’s honorable orphans: WEFT-322 as circuitus, fairness as aequitas without the check, ROTM as prior, invented $ from $0.024, auto-promote. So-what (pretty forgeries fail the four tests). One paragraph, not a wall.

### C04 — The sidecar contract — **94** FAIL

Primary (publish the fields or print the hole) is there; so-what (shipping the hole is the honesty) is on-slide. Try / See / Why points at `circuit-ev.html` as LDA-003. Fail: nine fields in one run-on sentence (circuit, favorable, odds, stake, edge, ruin, calibration, claim_type, equal_conditions) — no hierarchy, no example JSON — dense **−6**. A cold reader gets “print the hole” in 5s but cannot recap the contract.

### C05 — Three moves — **97**

Same three moves as K16 (align counsel; sidecar the next MH throw; leave genesis and Plane alone until 2 is done). Plane glossed. So-what (before anyone files a central ADR) + minutes link. Decision value is the point of the room.

### Coniunctio whole-deck — **94** FAIL

| Dimension | /weight | Score | Note |
|-----------|--------:|------:|------|
| Narrative arc | 25 | 24 | Rule → landed → refused → contract → ask. Shortest, cleanest arc in the set. |
| One idea per slide | 20 | 19 | C04’s field dump is the only overload. |
| Visual system | 15 | 12 | Same thin Nova shell. Tables work. No contract diagram, no cards, no color states. |
| Fidelity | 20 | 19 | Landings and refusals match P3. No on-slide 4:1 or 5–3 arithmetic (none required here). |
| Decision value | 20 | 20 | Three moves before a central ADR. Viewer knows what to do. |

**24+19+12+19+20 = 94.** Do not ship. Card the C04 fields (or show the sidecar JSON) and the room still wants one visual beat so it does not feel like minutes-on-slides.

---

## Fidelity register (this scorer)

| Claim | Required | Where | Verdict |
|-------|----------|-------|---------|
| Pascal 5–3 of 6 | **7:1** (B only as BBB = 1/8) | K11; K14 E5; N06 | **Pass.** Pacioli 5:3 and Cardano 6:1 correctly refused. “Not 4:1” stated. |
| House edge at 4:1 fair-six | **16.7% of stake** if defined as **−EV** | K09 JS + support; K14 E1 | **Number passes** (`−1/6`). **Definition fails** on K09: `title` and LDA-ADR-003 use `1−(offered/fair)` = **20%**. |
| ROTM 3×1/6 | 0.50 claimed vs ≈0.421 circuit | K04; N04; K14 E2 | **Pass.** Four throws to cross ½. |
| MH this session | 75 / 100 / 65 / 90 / 53; $0.024 not a face | K07; V01; V05 | **Pass** on the numbers. V01 fails the “red” caption. |

---

## Roll-up (Scorer 3)

| Deck | Pages <95 | Whole-deck | Ship? |
|------|-----------|------------:|:-----:|
| Keynote | K04 **91**, K09 **85** | **91** | No |
| Nova | N03 **92** | **91** | No |
| Vetus | V01 **90**, V07 **91** | **90** | No |
| Coniunctio | C04 **94** | **94** | No |

**Must-fix before Pass 4 (this scorer’s list, not an edit):**

1. **K09 / LDA-ADR-003** — One house-edge definition. If the live number stays 16.7%, the tooltip and ADR formula cannot stay `1−(offered/fair)`. If the ADR stays 20%, the slider must not print 16.7%.
2. **K04** — ROTM demo on one board; refuse-catalog + 25/27 chart on another (or a one-line catalog with the triangular split postponed to K11).
3. **N03** — Do not put even-money EV in the `.so` slot beside “fair odds.”
4. **V01** — Paint memory red or stop calling it red; do not let `.024` read as a sixth face.
5. **V07** — Split coherence vs type-twins; gloss K2.
6. **C04** — Field cards or the sidecar JSON, not a nine-noun sentence.

No page was scored 95 without a named residual. Whole-deck scores are not page averages.
