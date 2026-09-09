# SCORE-pass4-1 — independent scorer 1 of 4

**Date:** 2026-08-13  
**Scorer:** Pass 4 scorer 1  
**Scope:** all four Liber de Ludo Aleae decks. Read-only. Did not edit HTML. Did not read any `SCORE-pass4-*.md`. Opened `SCORE-pass3-1.md` only long enough to copy the score-file *shape*; numbers below are from the current HTML, not that file’s table.

**Method:** Full HTML/CSS/JS of `decks/keynote/index.html`, `workshops/nova/index.html`, `workshops/vetus/index.html`, `workshops/coniunctio/index.html`; the three keynote SVGs; `demos/circuit-ev.html`; `demos/examples.md` E1/E5; keynote `OUTLINE.md`; `SCORECARD.md` expert-consult line; `adrs/LDA-ADR-003-house-edge-index.md` and `LDA-ADR-005-remaining-work-ev.md`. Visual judgment is from source (hierarchy, contrast, overflow rules, widget math), not live screenshots.

**Gates:** page pass ≥ 95 against *that slide’s* goals. Whole-deck is a separate narrative score, **not** a page average. Merge later = minimum across the four pass-4 scorers.

---

## Fidelity locks (this pass)

| Lock | Verdict | Evidence |
|------|---------|----------|
| Pascal 5–3 of 6 = **7:1** | **Hold** on every glass that states a split | K11 third card: `7 : 1`, BBB = 1/8, explicit “Not 4:1, not Cardano’s 6:1.” K14 E5: “Match odds are 7:1 for A. (0.5:0.125 is not the match.)” Nova N06 + Nova glossary `7:1` match. Arithmetic is correct (race-to-6, A needs 1, B needs 3; 8 equally likely 3-cast paths). `examples.md` E5 still prints a **4:1** leftover-budget table at p=0.5; the *decks* refuse that as the match. |
| House edge = **−EV / stake = 16.7%** at 4:1 on a fair six | **Hold** on the named artifacts | LDA-ADR-003: `edge = −EV / stake`, 16.7% not 20% payout-shortfall. Keynote glossary: same sentence. K09 hover: “−EV / stake. At 4:1 this is 16.7%, not 20%.” K09 widget: `ev = (1/6)·payout + (5/6)·(−1)` → at 4, EV = −0.167, `ed = −ev` with implicit stake 1. K14 E1: “EV = −1/6. Edge 16.7%.” `circuit-ev.html`: `edge = −ev / stake`; Try/See “house edge ≈ 16.7%.” |
| Hover, glossary, widget, E1, LDA-ADR-003, `circuit-ev.html` agree | **Hold**, with one incomplete cousin | No remaining 20% hover. Coniunctio glossary defines house edge as “expected loss per unit stake when the payout is worse than fair” — same quantity, **no 16.7% check number** and no `−EV/stake` symbol. Nova and Vetus glossaries omit the term (correct for those rooms). K12 / `circuit-ev` share the ambiguous Try line “leave 6/six selected” (meant: the six-pip face, default `Set([6])`). |

---

## 1. Keynote — `decks/keynote/index.html`

Audience/goals from `OUTLINE.md` + each slide’s `data-primary-goal`. 16 slides.

| id | title | score | ≥95 |
|----|-------|------:|:---:|
| K01 | Count the circuit before you score | 97 | yes |
| K02 | Two rooms, then one table | 96 | yes |
| K03 | The general rule | 97 | yes |
| K04 | What you will not inherit | 95 | yes |
| K05 | What was already in the walls | 96 | yes |
| K06 | Circuit is not a circuit-breaker | 98 | yes |
| K07 | Five unlabeled dice | 96 | yes |
| K08 | Fairness is not yet aequitas | 96 | yes |
| K09 | House edge on opportunities | 97 | yes |
| K10 | Luck is not a method | 97 | yes |
| K11 | Pay what remains | 97 | yes |
| K12 | Lay the wager | 94 | **no** |
| K13 | Sidecar, not genesis | 96 | yes |
| K14 | Make it grokable | 97 | yes |
| K15 | What we will not do | 97 | yes |
| K16 | Three moves | 97 | yes |

**Keynote whole-deck: 96.** Thesis is the title of K01; method is K02; doctrine → walls → live instruments → points → explorer → contract → ask. One-idea discipline holds except a light double-load on K04. Visual system is consistent (gold rail, panel cards, three SVGs, hero only on K01–K02). Fidelity locks hold on hover / G / widget / E1. Decision value is three concrete moves. K12 is a weak handoff beat, not an arc break — the page still fails the 95 gate.

### Page notes

**K01 — 97.** Title *is* the thesis; a cold reader has it in five seconds. Lede names Cardano as chair and glosses *circuit*; eyebrow carries “First Grok symposium”; `.so` lands the WeftOS claim (published numbers without a sample space are bets). The 1564 / 1663 / 2026 cards only highlight — they do not reveal more — but they still carry the secondary timeline.

**K02 — 96.** Nova / Vetus / Coniunctio are hovered and sequenced; `.so` states the fairness-of-inquiry rule. T/S/W is present. Click only toggles a gold border on text that is already visible, so “See” is slightly overclaimed, not false.

**K03 — 97.** Interactive die plus `circuitus.svg` *is* chapter 14. Odds line reports `r:s`, `p=r/6`, and fair payout `s/r`; T/S/W and EV = 0 so-what are on-slide. Diagram is a readable four-box flow.

**K04 — 95.** Primary refusal is unmistakable: slider at *n*=3 shows ROTM 0.50 vs circuit 0.421, and the copy names four throws to cross ½. T/S/W and so-what are correct. Right card also packs 25-vs-27 plus a parenthetical “Prince” with no on-slide gloss (glossary has it; first-use test is thin). Hierarchy still leads with ROTM, so this stays at the gate rather than under it.

**K05 — 96.** Dedicated wall: 13 chips, expand-in-place class + gap, pointer to `04-existing-spaces.md`. T/S/W names V1 / V6 / V7 / V17 and those chips exist. Expansions assume ADR-034 / L2 / GEPA; stated audience is WeftOS engineers, so that is acceptable. “Marked deck” on V17 is still first-use until K08.

**K06 — 98.** Homonym split is the whole slide. Lede + `homonym.svg` + so-what (“perfect stop-loss, crooked table”) are one idea. WEFT-322 is in the hover. Notes’ “4:1 house game” is the *payout* example, not the Pascal split.

**K07 — 96.** Five bars are 75 / 100 / 65 / 90 / 53 with the stated color rule (memory crit). Lede explains *n*=1 and missing interval; so-what forbids promote-on-one-cast. Prior throw and $0.024-as-price-not-face keep the secondary honest. Outline “hover faces” was not built; HTML goals do not require it.

**K08 — 96.** Fairness-dim vs *aequitas* is the title and the so-what (“do not rename; add a check”). Left card glosses L2 and the 0.7 / 0.8 dual bar; right card is the marked briefing. Two supporting facts, one decision. Outline’s weight-drag was not built; HTML goals do not require it.

**K09 — 97.** Primary pricing works and matches the lock: drag-to-4 → EV = −0.167, house edge = 16.7% of stake. Hover, G drawer, Try/See, E1, LDA-ADR-003, and `circuit-ev.html` now say the same formula. So-what (router / winner-only eval / vendor / judge) is on-slide. Stake is implicit 1 (no slider); that is equivalent, not a second definition.

**K10 — 97.** *n*-slider moves fortuna → mixed → scientia at 5 and 30; T/S/W says so. Promote is hovered; flywheel receipts are named as signed eval records. So-what is the release rule. Cutovers are honestly labeled a house rule, not Bernoulli.

**K11 — 97.** Pascal 7:1 is on the glass, with BBB = 1/8 and an explicit refusal of 4:1 and Cardano 6:1. Sunk 5:3 vs still-needed 1-vs-3 is the right question. Swarm so-what is on-slide. “Triangular numbers” is hovered as the wrong 6:1 arithmetic.

**K12 — 94. Fail.** Primary goal (open the explorer) is a link, and the book-of-wagers so-what is on-slide. T/S/W exists but says “leave six selected, drag payout to 4” / “Negative EV” — it never names 16.7%, and “six selected” can be read as all six faces (which inverts the sign). `circuit-ev.html` uses the same wording against default `Set([6])`. Deduct ambiguous demo support (−6). No in-deck portfolio widget; HTML goals do not ask for one.

**K13 — 96.** Four sidecar fields are the contract; genesis is hovered as “do not smash a sixth face.” LDA-001–005 are listed as a local namespace. So-what is ship-this-quarter, not kernel. “Edge = departure from fair” is looser than −EV/stake but does not reintroduce 20%.

**K14 — 97.** Tabs + T/S/W work. E1 is 16.7% EV-edge. E5 is 7:1 and correctly warns that 0.5:0.125 is not the match. Seven of ten examples are enough for “grokable.”

**K15 — 97.** Locked vs sidecar is scannable in five seconds. So-what names silent self-graduation as already-forbidden auto-promote. Refusals match the doctrine (no genesis smash, no breaker rename, no ROTM prior, no invented $ savings).

**K16 — 97.** Three moves = counsel drift, one sidecar eval, hands off genesis/Plane. Plane and “deliverable 03” are parenthetically explained. Thesis reprises. Cards are tabindex-only, not extra pressables; the ask still sticks.

---

## 2. Nova — `workshops/nova/index.html`

Book-only room. Goals inferred from each board’s title + lede (no OUTLINE). 8 slides.

| id | title | score | ≥95 |
|----|-------|------:|:---:|
| N01 | Doctrine from the book | 96 | yes |
| N02 | Equal conditions first | 96 | yes |
| N03 | Count the whole circuit | 93 | **no** |
| N04 | Refuse ROTM | 97 | yes |
| N05 | Lean is not luck | 97 | yes |
| N06 | Pay remaining work | 95 | yes |
| N07 | Small stakes, named ruin | 97 | yes |
| N08 | What Coniunctio may use | 92 | **no** |

**Nova whole-deck: 93.** Arc is the right kit in the right order (aequitas → circuit → ROTM → lean → points → ruin → handoff) and correctly refuses WeftOS crate types. Visual system is the gold-rail sibling of the keynote but thinner — no diagram, no texture. Decision value is the may/may-not list. It does not ship: the load-bearing circuit board has no `.so`, and the handoff dumps `pⁿ` / `np` as unexplained kit names. Omitting a 16.7% house-edge index is correct for this room.

### Page notes

**N01 — 96.** Room charge in one glance: book only, arithmetic crimes stay on the table. `.so` (glossary = primitive gate) is the decision rule. Thin, not empty.

**N02 — 96.** *Aequitas* is hovered; probability is the check, not the carnival. Bake-off so-what is the product landing. “Marked deck” is used as the punchline and is contextually defined, not glossed.

**N03 — 93. Fail.** The r:s calculator is the right chapter-14 toy (default 27:25 = three-dice 10 vs 9); T/S/W explains even-money EV and `p²`. No `.so` decision sentence — every other Nova board has one. The lede *is* the rule, but the punch-list so-what is missing (−7). Widget math (`ev = 2p−1`) is honestly labeled even-money, not a second house-edge formula.

**N04 — 97.** 3 × 1/6 = ½ vs 1 − (5/6)³ ≈ 0.421, four throws to cross a half. So-what is the np-as-P ban. One idea.

**N05 — 97.** Detector stays; Prince is hovered and fired. So-what (a score that always “equals what it should be”) is the calibration warning.

**N06 — 95.** Pascal 7:1, not 6:1, not 4:1, with BBB = 1/8. Swarm so-what is on-slide. “Triangular numbers” is only glossed by its wrong 6:1 result; a cold reader cannot define the term, but can refuse the number.

**N07 — 97.** Rich purse ≠ even table; ruin is hovered. So-what splits stop-loss from sample space — the homonym seed Coniunctio needs.

**N08 — 92. Fail.** Two-column may / may-not is the right handoff. `.so` points at Coniunctio and `P1-nova.md`. The leave-list dumps `pⁿ` and `np` with no on-slide gloss and no glossary entries (−8). Prince / ROTM / triangular 6:1 in the refuse column are recoverable from earlier boards.

---

## 3. Vetus — `workshops/vetus/index.html`

Inventory room. Goals inferred from eyebrow + title. 8 boards.

| id | title | score | ≥95 |
|----|-------|------:|:---:|
| VT01 | What WeftOS already scores | 95 | yes |
| VT02 | Four voices, one rule | 96 | yes |
| VT03 | Thirty-one surfaces, three classes | 95 | yes |
| VT04 | Circuit vs circuit-breaker | 97 | yes |
| VT05 | Casts you can recount — still no circuit | 91 | **no** |
| VT06 | Governance-counsel EffectVector drift | 91 | **no** |
| VT07 | Words already spent twice | 91 | **no** |
| VT08 | What Vetus hands Coniunctio | 96 | yes |

**Vetus whole-deck: 92.** Arc is inventory-correct: live cast → who sat → table → homonym → three closest scores → marked deck → word-collapses → handoff. Internally consistent chrome (header/footer, 13px body) but a different visual family from keynote/Nova. Fidelity is honest: no invented Cardano, MH faces match 75/100/65/90/53, $0.024 is a price tag. Decision value is “Coniunctio may land only on these named gaps.” It does not ship: three middle boards fail first-use / density, and the workshop still reads more like a scrollable brief than a slide sequence.

### Page notes

**VT01 — 95.** Workshop charge + live MH cast + `.so` (inventory only). Memory 53 is actually red (`style="color:var(--crit)"`). T/S/W (“read the red memory face”) matches the glass. Six equal tiles include `estCostPerRunUsd .024`; the caption says it is not a sixth die. Layout is a little too even, but the name on the tile carries the distinction. First-use: MetaHarness and *rhyme* are hovered.

**VT02 — 96.** Four seats + Phase I rule on one board. `.so` is why the seats exist. No demo, no T/S/W needed. *Coherence* / spectral lean live in the G drawer.

**VT03 — 95.** Thirty-one expandable rows is the dedicated inventory. Classification ≠ merge is the so-what. T/S/W names V1 / V6 / V17 and those rows exist. Dense by job, not by accident; 52vh scroll keeps it on a board.

**VT04 — 97.** Homonym split with in-tree symbols vs in-book circuitus. So-what: perfect stop-loss, crooked table. WEFT-322 and the unshipped Level-2 stub are both named. One idea.

**VT05 — 91. Fail.** Three recountable casts, still no circuit — the so-what is right. Cards are packed (L2 formula, dual 0.7/0.8 bar, Fitness weights, two Noops). **RVF** is an unexplained layer acronym on first use (−8). Cold reader gets the gap from the title; the glass is a brief, not a slide.

**VT06 — 91. Fail.** Side-by-side 5D (cpu/memory/… vs risk/fairness/…) is the marked deck, scannable in five seconds. `.so` tells us to fix our own briefing first — and uses ***aequitas*** , which is **not** in the Vetus glossary and has no hover on this board (−8). 0.7 vs 0.8 is correctly a sibling inequality, not the house-edge lock.

**VT07 — 91. Fail.** Four *coherence* formulas + twins (gates, trajectories, breakers) is the collapse warning. So-what (“always say which; do not inventory a stub”) is the decision. **K2 §5** is first-use with no gloss and no glossary entry (−8). Stub-vs-live on the 7-factor router is honest.

**VT08 — 96.** Named handoff (V1–V31, homonym, twins, three live scores, counsel drift, no genesis smash). `.so` forbids landing off the list. Explicit non-delivery (no contract, no LDA text, no Cardano arithmetic) is the room’s integrity.

---

## 4. Coniunctio — `workshops/coniunctio/index.html`

Combining room. Goals inferred from title + lede. 5 slides.

| id | title | score | ≥95 |
|----|-------|------:|:---:|
| C01 | A primitive lands only on a named gap | 97 | yes |
| C02 | What landed | 96 | yes |
| C03 | What did not land | 97 | yes |
| C04 | The sidecar contract | 95 | yes |
| C05 | Three moves | 97 | yes |

**Coniunctio whole-deck: 95.** Tightest narrative of the four: rule → landings → refusals → contract → ask. One idea per board. Visual system matches Nova (sparse, gold rail, tables). Decision value is the same three moves as K16, with both parents cited. Fidelity: remaining-work lands without restating 7:1 (LDA-005 allows “Pascal remaining-paths when enumerable”); house-edge glossary agrees in words, omits the 16.7% check number. Pages clear 95; whole-deck sits on the gate.

### Page notes

**C01 — 97.** Four tests (named gap, no genesis break, not Cardano’s error, both parents) in one lede. `.so` (orphans honorable, forgeries not) is the ethic. *Landing* is in the G drawer.

**C02 — 96.** Five primitives → five Vetus gaps → LDA 001–005. `.so` blocks a sixth genesis face. *Circuitus* / *aequitas* are hovered. “Odds / edge” is not hovered; G’s house-edge line is the incomplete cousin of the lock (no 16.7%).

**C03 — 97.** Refusals match K15 / P3 (breaker ≠ circuitus, fairness ≠ aequitas without the check, ROTM is not a prior, no $ invented from 0.024, no auto-promote). So-what: pretty forgeries fail the four tests. ROTM and WEFT-322 are recoverable (hover / G).

**C04 — 95.** Field list is the sidecar contract. T/S/W opens `circuit-ev.html` and says to watch edge when payout ≠ fair — true, and the demo’s own Try nails 16.7% at 4:1. This board does not repeat the check number; G’s definition is the word form of −EV/stake. At the gate, not above it.

**C05 — 97.** Same three moves as K16 (align briefing, sidecar the next MH throw, leave genesis/Plane). `.so` forbids a central ADR before those land. Plane is in the G drawer. Link to `P3-coniunctio.md`.

---

## Ship gate (this scorer)

| deck | pages ≥95? | whole-deck | ship? |
|------|:----------:|----------:|:-----:|
| keynote | no (K12 = 94) | 96 | **no** |
| nova | no (N03 = 93, N08 = 92) | 93 | **no** |
| vetus | no (VT05 = VT06 = VT07 = 91) | 92 | **no** |
| coniunctio | yes | 95 | **yes*** |

\*Coniunctio ships *on this scorer only* if the other three pass-4 scorers also put every page and the whole-deck at ≥ 95. Symposium set does **not** ship.

### Must-fix (this scorer)

1. **K12** — Rewrite Try/See to the unambiguous lock: “leave only face 6 on, drag payout to 4, See EV = −0.167 / house edge = 16.7%.”
2. **N03** — Add a `.so` (fair price is justice; anything else is a house table).
3. **N08** — Gloss `pⁿ` and `np` on the glass or in the G drawer.
4. **VT05** — Expand or hover **RVF**; cut a line of density.
5. **VT06** — Hover or G-define *aequitas* (Vetus glossary currently omits it).
6. **VT07** — Gloss **K2 §5** (industry UQ gap, not a crate).
7. **Optional, lock hygiene** — Put “−EV / stake = 16.7% at 4:1” on the Coniunctio house-edge glossary line so every G drawer that names the term carries the check number.
