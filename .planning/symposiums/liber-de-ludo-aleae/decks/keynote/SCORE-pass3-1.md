# SCORE-pass3-1 — independent scorer 1

**Date:** 2026-08-13  
**Scorer:** Pass 3 scorer 1  
**Scope:** all four Liber de Ludo Aleae decks. Read-only. Did not edit HTML. Did not read `SCORE-pass2-*.md` or other `SCORE-pass3-*.md`.

**Method:** Full HTML/CSS/JS of each `index.html`, the three keynote SVGs, `demos/circuit-ev.html`, keynote `OUTLINE.md`, `SCORECARD.md` expert-consult line, and `adrs/LDA-ADR-003-house-edge-index.md`. Visual judgment is from source (layout, type, contrast, overflow rules), not live screenshots.

**Gates:** page pass ≥ 95 against *that slide’s* goals. Whole-deck is a separate narrative score, **not** a page average. Merge later = minimum across the three pass-3 scorers.

---

## Claimed-fix verification

| Claim | Verdict | Evidence |
|-------|---------|----------|
| K11 / E5 say Pascal **7:1**, not 4:1 | **Yes** | K11 third card: `7 : 1`, “B wins the match only as BBB (1/8)… Not 4:1, not Cardano’s 6:1.” K14 E5: “Match odds are 7:1 for A. (0.5:0.125 is not the match.)” Nova N06 and Nova glossary `7:1` match. Arithmetic is correct (race-to-6 at 5–3; 8 equally likely 3-cast paths). |
| K09 house edge = expected loss / stake (**16.7%** at 4:1), matching E1 and `circuit-ev.html` | **Partial** | **Numbers agree:** K09 JS is `ed = ev < 0 ? -ev : 0` with `ev = (1/6)·payout + (5/6)·(−1)` → at 4:1, EV = −0.167, edge = 16.7%. Try/See says exactly that. E1: “EV = −1/6. Edge 16.7%.” `circuit-ev.html` uses `edge = −ev / stake`. **Words do not:** K09 `<abbr title="1 − (offered payout ÷ fair payout)">` is the **20%** payout-shortfall formula. Keynote glossary still: “How much worse the offered payout is than the fair payout.” Coniunctio glossary *is* the approved definition. LDA-ADR-003 text is still the 20% formula (not a deck file). |
| Glossaries include first-use terms | **Mostly** | All four decks have a G drawer. Keynote hovers cover circuit / aequitas / ROTM / MH / sidecar / genesis / Plane. Residuals below. |
| Try / See / Why on inventory and sliders | **Mostly** | Present on K03 die, K04 throws, K05 chips, K09 payout, K10 *n*, K14 tabs, Nova N03 calculator, Vetus VT01 dice and VT03 inventory. Vetus VT01 T/S/W says “red memory face”; every `.die .face` is accent gold — the caption does not match the glass. |
| So-what on every board | **Almost** | Dedicated `.so` on every keynote, Nova, and Coniunctio section, and on 7/8 Vetus boards. **Vetus VT06 has no `.so`.** Nova N03’s `.so` is the calculator output, not a decision sentence. |

**First-use residuals (not a full fail list):** keynote “house edge” definition contradicts the widget; “triangular numbers” (K11 / N06) is only glossed by its wrong 6:1 result; Nova N08 dumps `pⁿ` / `np` as kit names; Vetus VT07 “K2 §5” has no gloss; “marked deck” is used on K05 V17 before K08 explains it.

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
| K08 | Fairness is not yet aequitas | 95 | yes |
| K09 | House edge on opportunities | 88 | **no** |
| K10 | Luck is not a method | 97 | yes |
| K11 | Pay what remains | 96 | yes |
| K12 | Lay the wager | 96 | yes |
| K13 | Sidecar, not genesis | 97 | yes |
| K14 | Make it grokable | 96 | yes |
| K15 | What we will not do | 97 | yes |
| K16 | Three moves | 97 | yes |

### Page notes

**K01 — 97.** Title *is* the thesis; a cold reader has it in five seconds. Lede names Cardano as chair and glosses *circuit*; eyebrow carries “First Grok symposium”; `.so` lands the WeftOS claim (published numbers without a sample space are bets). The 1564 / 1663 / 2026 cards only highlight — they do not reveal more — but they still carry the secondary timeline.

**K02 — 96.** Nova / Vetus / Coniunctio are hovered and sequenced; `.so` states the fairness-of-inquiry rule. T/S/W is present. Click only toggles a gold border on text that is already visible, so “See the charge stay on its side of the wall” is slightly overclaimed, not false.

**K03 — 97.** Interactive die plus `circuitus.svg` *is* chapter 14. Odds line reports `r:s`, `p=r/6`, and fair payout; T/S/W and EV=0 so-what are on-slide. Diagram is a readable four-box flow.

**K04 — 95.** Primary refusal is unmistakable: slider at *n*=3 shows ROTM 0.50 vs circuit 0.421, and the copy names four throws to cross ½. T/S/W and so-what are correct. The right card also packs Prince / multiply-odds / triangular split / 25-vs-27; hierarchy still leads with ROTM, so this stays at the gate rather than under it.

**K05 — 96.** Dedicated wall: 13 chips, expand-in-place class + gap, pointer to `04-existing-spaces.md`. T/S/W names V1 / V6 / V7 / V17 and those chips exist. “Marked deck” on V17 is still first-use without a gloss (K08 is later).

**K06 — 98.** Homonym split is the whole slide. Lede + `homonym.svg` + so-what (“perfect stop-loss, crooked table”) are one idea. WEFT-322 is in the hover. Notes’ “4:1 house game” is the *payout* example, not the Pascal error.

**K07 — 96.** Five bars are 75 / 100 / 65 / 90 / 53 with the stated color rule (memory crit). Lede explains *n*=1 and missing interval; so-what forbids promote-on-one-cast. Prior throw and $0.024-as-price-not-face keep the secondary honest.

**K08 — 95.** Fairness-dim vs *aequitas* is the title and the so-what (“do not rename; add a check”). Left card glosses L2 and the 0.7 / 0.8 dual bar; right card is the marked briefing. Two supporting facts, one decision. Outline’s weight-drag was not built; HTML goals do not require it.

**K09 — 88.** Primary *pricing* works: drag-to-4 matches Try/See (EV −0.167, edge 16.7% of stake) and matches E1 / `circuit-ev.html`. So-what (router / winner-only eval) is on-slide. **Fail:** the same lede hovers house edge as `1 − (offered ÷ fair)` — 20% at 4:1 — and the G drawer still defines edge as payout-shortfall. Two formulas on one glass. Deduct wrong-science (−12).

**K10 — 97.** *n*-slider moves fortuna → mixed → scientia at 5 and 30; T/S/W says so. Promote is hovered; flywheel receipts are named as signed eval records. So-what is the release rule.

**K11 — 96.** Claimed 7:1 fix is on the glass, with BBB = 1/8 and an explicit refusal of 4:1 and Cardano 6:1. Sunk 5:3 vs still-needed 1-vs-3 is the right question. Swarm so-what is on-slide. “Triangular numbers” is only glossed as “wrong arithmetic.”

**K12 — 96.** Primary goal is to open the demo; the link and T/S/W (six selected, payout 4, negative EV) do that. Book-of-wagers so-what is on-slide. No in-deck portfolio widget; HTML goals do not ask for one.

**K13 — 97.** Four sidecar fields are the contract; genesis is hovered as “do not smash a sixth face.” LDA-001–005 are listed as a local namespace. So-what is ship-this-quarter, not kernel.

**K14 — 96.** Tabs + T/S/W work. E1 is 16.7% EV-edge (aligned with the K09 *widget*). E5 is 7:1 and correctly warns that 0.5:0.125 is not the match. Seven of ten examples are enough for “grokable.”

**K15 — 97.** Locked vs sidecar is scannable in five seconds. So-what names silent self-graduation as already-forbidden auto-promote. Refusals match the doctrine (no genesis smash, no breaker rename, no ROTM prior, no invented $ savings).

**K16 — 97.** Three pressable moves match the secondary goals (counsel drift, one sidecar eval, do not touch genesis). Plane is glossed as the work board. Thesis returns as a closer, not a new idea.

### Keynote whole-deck — **91** (not an average)

| Dimension | /weight | Score | Ask |
|-----------|--------:|------:|-----|
| Narrative arc | 25 | 24 | Thesis is slide 1. Method → doctrine → walls → pricing → demo → contract → ask. |
| One idea per slide | 20 | 17 | K04 carries a second error-family; K08 is L2 + marked deck; K09 is one slider with two edge definitions. |
| Visual system | 15 | 14 | Gold/ink, heroes, three SVGs, glossary, `.so` / T/S/W pattern. Radar is crude CSS bars. |
| Fidelity to source | 20 | 17 | 7:1 is now Pascal. ROTM, homonym, MH cast, sidecar-not-genesis match the rooms. House-edge *numbers* match the expert consult; house-edge *words* (K09 title + keynote glossary) still describe LDA-003’s 20% formula. |
| Decision value | 20 | 19 | Viewer knows the three moves and what is locked. |

**91 = 90–94:** do not ship; rewrite the leftover K09/glossary definition so it names −EV/stake. Do not polish around it.

---

## 2. Nova — `workshops/nova/index.html`

Book-only room. 8 boards. Goals inferred from title + so-what + panel job (no `data-primary-goal` on these files).

| id | title | score | ≥95 |
|----|-------|------:|:---:|
| N01 | Doctrine from the book | 96 | yes |
| N02 | Equal conditions first | 97 | yes |
| N03 | Count the whole circuit | 90 | **no** |
| N04 | Refuse ROTM | 97 | yes |
| N05 | Lean is not luck | 96 | yes |
| N06 | Pay remaining work | 97 | yes |
| N07 | Small stakes, named ruin | 96 | yes |
| N08 | What Coniunctio may use | 92 | **no** |

### Page notes

**N01 — 96.** Room rule is the slide: book only, crimes stay on the table. So-what (if it is not in G, it is not a primitive) is the right discipline for this room.

**N02 — 97.** *Aequitas* hovered; probability is a check, not a trick. Bake-off / marked-deck so-what is a decision sentence a cold engineer can use.

**N03 — 90.** Calculator + T/S/W do chapter 14 (default 27:25, *p*, fair odds, even-money EV, *p*²). **Fail:** `id="out"` uses class `so`, so the live line reads “So what. p = r/(r+s) = …”. There is no product/decision so-what; the prefix is attached to arithmetic. Deduct −10.

**N04 — 97.** 3×1/6 = ½ vs 1−(5/6)³ ≈ 0.421 is correct and dual-carded. So-what is the np-is-not-P rule. *ROTM* hovered.

**N05 — 96.** Detector stays, Prince (hovered) goes. So-what: a score that always “just equals” the mean is as suspect as a lean. One idea.

**N06 — 97.** Claimed 7:1 fix is here and worked: BBB = 1/8, pot 7:1, not 6:1 and not 4:1. Cardano’s triangular 6:1 is named as the wrong arithmetic. Swarm so-what (do not credit burned tokens) is on-slide. Slight book-room leak of “swarm,” required by the so-what rule.

**N07 — 96.** Ruin hovered. So-what splits stop-loss from sample space — the homonym the other rooms will need. Five-second read holds.

**N08 — 92.** May-leave / may-not is the handoff idea and is visible at a glance. The leave column is a token dump (`pⁿ · np` especially) a cold reader cannot unpack without N03–N04 in working memory. Deduct unexplained-term (−8).

### Nova whole-deck — **90** (not an average)

| Dimension | /weight | Score | Ask |
|-----------|--------:|------:|-----|
| Narrative arc | 25 | 23 | *Aequitas* → circuit → ROTM → lean → remaining → ruin → kit. Thesis of the room is slide 1. |
| One idea per slide | 20 | 18 | Holds until the N08 kit list. |
| Visual system | 15 | 13 | Same tokens as the keynote; no diagram, no texture, no notes. A pamphlet, consistently so. |
| Fidelity to source | 20 | 19 | Book wall holds. 7:1 is right. ROTM numbers are right. |
| Decision value | 20 | 17 | N08 is the decision board; N03 never says what WeftOS should *do* with *r:s*. |

**90 = 90–94:** targeted rewrite of N03 so-what and N08 unpacking. Do not ship.

---

## 3. Vetus — `workshops/vetus/index.html`

Inventory room. 8 boards.

| id | title | score | ≥95 |
|----|-------|------:|:---:|
| VT01 | What WeftOS already scores | 90 | **no** |
| VT02 | Four voices, one rule | 96 | yes |
| VT03 | Thirty-one surfaces, three classes | 95 | yes |
| VT04 | Circuit vs circuit-breaker | 97 | yes |
| VT05 | Casts you can recount — still no circuit | 95 | yes |
| VT06 | Governance-counsel EffectVector drift | 90 | **no** |
| VT07 | Words already spent twice | 92 | **no** |
| VT08 | What Vetus hands Coniunctio | 97 | yes |

### Page notes

**VT01 — 90.** Inventory-not-combining so-what is clear; the six-die MH cast plus “price tag, not a sixth face” meets the live-instrument secondary. T/S/W is present (claimed fix). **Fail:** Try says “Read the **red** memory face”; CSS paints every `.die .face` `--accent` gold. The caption describes a keynote convention this board does not use. Deduct demo-support (−10).

**VT02 — 96.** Four seats + Phase I wall are one rule. So-what (Coniunctio cannot claim nobody looked) is on-slide. Class words appear here before the table.

**VT03 — 95.** Claimed inventory T/S/W and `.so` are here: open V1 / V6 / V17, see rhyme vs false friend vs marked deck, this table is the only landing strip. Thirty-one expandable rows are the job, not a 5-second matrix; the idea (“classification is not a merge”) is in the lede. Gate, not a showcase.

**VT04 — 97.** Side-by-side breaker vs *circuitus*, including the unshipped Level-2 stub. So-what is the crooked-table line. Static comparison; T/S/W not required.

**VT05 — 95.** Three nearest recountable casts, each still missing a sample space — that *is* the Coniunctio gap. Dense but carded. So-what is explicit.

**VT06 — 90.** The marked deck is visually the best board in the room (cpu/… vs risk/…). **Fail:** no `.so`. Lede is process (“Coniunctio may use this; Vetus does not fix it”) rather than a product decision (do not treat the briefing as the live five-die). Deduct missing so-what (−10). Claimed “so-what on every board” is false here.

**VT07 — 92.** Four *coherence* formulas + twins make the “say which formula” point; so-what is on-slide. “K2 §5” and several crate twins land without a first-use gloss. Deduct unexplained-term (−8).

**VT08 — 97.** Handoff list is the decision: V1–V31, homonym, coherence/twins, three live scores, counsel drift, no genesis smash. So-what: Coniunctio lands only on named gaps; Vetus wrote no contract. C9 / sidecar / genesis are in G.

### Vetus whole-deck — **90** (not an average)

| Dimension | /weight | Score | Ask |
|-----------|--------:|------:|-----|
| Narrative arc | 25 | 23 | Cast → who sat → table → homonym → three scores → marked deck → word-spend → handoff. |
| One idea per slide | 20 | 18 | VT03 is a tool-table (acceptable). VT07 is two clusters. |
| Visual system | 15 | 13 | Best workshop chrome (pills, inventory, sticky footer). Still no texture/diagram system like the keynote. |
| Fidelity to source | 20 | 19 | Surfaces and classes match the inventory job. No 4:1 / edge-arithmetic error. |
| Decision value | 20 | 17 | VT08 is actionable. VT06 never writes the decision on the glass. |

**90 = 90–94:** color the VT01 memory face or drop “red”; give VT06 a so-what; gloss K2. Do not ship.

---

## 4. Coniunctio — `workshops/coniunctio/index.html`

Combining room. 5 boards.

| id | title | score | ≥95 |
|----|-------|------:|:---:|
| C01 | A primitive lands only on a named gap | 96 | yes |
| C02 | What landed | 97 | yes |
| C03 | What did not land | 96 | yes |
| C04 | The sidecar contract | 95 | yes |
| C05 | Three moves | 97 | yes |

### Page notes

**C01 — 96.** Four tests are the landing rule; so-what (orphans honorable, forgeries not) is the ethic. Glossary defines *landing*. Thin, but one idea.

**C02 — 97.** Five primitives × named Vetus gap × LDA-001–005. So-what: sidecar fields, not a sixth genesis face. *circuitus* / *aequitas* hovered.

**C03 — 96.** Refusals match keynote K15 (breaker≠circuitus, fairness≠aequitas, no ROTM prior, no $0.024 savings, no auto-promote). So-what: pretty forgeries fail the four tests.

**C04 — 95.** Field list is the contract; T/S/W opens `circuit-ev.html` and says edge appears when payout ≠ fair. So-what (“shipping the hole is the honesty”) is right; tying it only to “LDA-003 as a die” narrows a multi-field contract to the edge demo. Gate.

**C05 — 97.** Same three moves as K16, with Plane hovered and a minutes link. So-what: do these before a central ADR.

### Coniunctio whole-deck — **93** (not an average)

| Dimension | /weight | Score | Ask |
|-----------|--------:|------:|-----|
| Narrative arc | 25 | 24 | Rule → landed → refused → contract → ask. Tightest of the four. |
| One idea per slide | 20 | 19 | Holds. C04 slightly splits contract vs demo. |
| Visual system | 15 | 12 | Same pamphlet shell as Nova. Table is clean; no diagram of the four tests. |
| Fidelity to source | 20 | 19 | Landings match LDA-001–005. Glossary house edge is the approved −EV/stake wording. No 4:1-as-Pascal. |
| Decision value | 20 | 19 | Three moves + four tests. Viewer knows what may be filed. |

**93 = 90–94:** pages individually pass; whole-deck still short of 95 on visual thinness. Do not ship on whole-deck alone.

---

## Ship verdict (this scorer)

| Deck | Pages all ≥95? | Whole-deck | Ship? |
|------|:--------------:|----------:|:-----:|
| Keynote | no (K09 88) | 91 | **no** |
| Nova | no (N03 90, N08 92) | 90 | **no** |
| Vetus | no (VT01 90, VT06 90, VT07 92) | 90 | **no** |
| Coniunctio | yes | 93 | **no** |

Pass 3 does not ship for scorer 1. The 7:1 Pascal fix and the 16.7% *numeric* unification are real. The leftover must-fix is the keynote house-edge *definition* (K09 hover + keynote glossary still speak 20% payout-shortfall), plus the three workshop residuals (N03 so-what hijack, VT01 “red” face, VT06 missing so-what).
