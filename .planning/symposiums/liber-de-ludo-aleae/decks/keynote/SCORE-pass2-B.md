# SCORE pass 2 — Scorer B

Independent cold read. Did not open `SCORECARD.md` or `SCORE-pass2-A.md`. Did not edit HTML.

**Audience assumed:** WeftOS / clawft engineers, governance, MetaHarness operators who have **not** sat the symposium. Each deck is scored as if it were the first file opened.

**Rules applied (rubric + deck skill):** start 100; pass ≥ 95. In-file glossary (G) counts as a first-use gloss **only for terms that glossary actually defines**. Hover/`<abbr title>` and inline parentheticals count. Latin, crate/type names, ADR/LDA/WEFT numbers, and house jargon that are **not** on the slide and **not** in that deck’s glossary deduct −8 to −15. Interactive controls and charts need Try / See / Why on the slide (−10 to −20 if missing). So-what must be on the slide, not only in notes (−8). Whole-deck is **not** a page average.

Workshop HTML has no `data-id`. IDs below are board order: Nova `N01–N08`, Vetus `V01–V08`, Coniunctio `C01–C05`.

---

## Page scores

| deck | id | score | evidence |
|------|----|------:|----------|
| keynote | K01 | 97 | Thesis is readable in one glance; `circuit` and *Liber de Ludo Aleae* are hover-glossed; so-what names sample space vs measurement. Tiny: “gates, quality, harness” is insider shorthand, not fatal. |
| keynote | K02 | 95 | Two-room method and Nova / Vetus / Coniunctio land in 5s; Latin and class words are hover-glossed; so-what is on-slide. Click-to-highlight does not show “the charge stays on its side of the wall,” but Try/See/Why exist and the idea does not depend on the click. |
| keynote | K03 | 97 | Chapter 14 is a holdable die: faces → `r:s`, `p`, fair payout. Try/See/Why present; EV hover-glossed; flow SVG is readable. Diagram repeats CIRCUITUS already glossed on K01. |
| keynote | K04 | 84 | ROTM slider does the primary job (3 → 0.50 false / 0.421 real; four throws to cross ½) with Try/See/Why. Fail: “luck-as-Prince” and “triangular problem-of-points” have no on-slide or glossary gloss (−8). Three-dice SVG (25 vs 27) never says why partitions ≠ ways, so the figure is an unread second idea (−8). |
| keynote | K05 | 90 | Dedicated wall: chips carry class; placeholder defines rhyme / cousin / false friend; Try/See/Why and so-what present. Fail: first glance is a crate wall (FitnessScorer, NodeScoring, ECC, SOUL, K2 UQ) with no gloss until click; EffectVector is in G, the rest are not (−8). Bar claims V1–V31 while 13 chips show. |
| keynote | K06 | 98 | Homonym split is the whole slide and it works: sample space vs stop-loss, both named, SVG readable, so-what on-slide (“perfect stop-loss, crooked table”). WEFT-322 sits in the hover. |
| keynote | K07 | 82 | Live 75/100/65/90/53 and `n = 1` serve the primary goal; MetaHarness hover-glossed; so-what on-slide. Fail: bar chart has See/Why but no Try (−10); hover-faces promised by outline do not exist. ADR-096 is an unexplained citation (−8). |
| keynote | K08 | 97 | Fairness-dim ≠ aequitas is the one idea; both terms hover-glossed; L2 written as distance from zero; 0.7 vs 0.8 and briefing-vs-live 5D make the marked-deck point. Sidecar is in this deck’s glossary. So-what is a decision: do not rename, add a check. |
| keynote | K09 | 88 | Slider prices a 5:1 fair table; house-edge hover matches `1 − offered/fair`; Try/See/Why and so-what present. Fail: Try says “edge ≈ 17%” at payout 4; the widget prints **20.0%** (`1 − 4/5`). Same table is 16.7% \|EV\| on K14/E1. Internal contradiction (−12). |
| keynote | K10 | 95 | `n` slider moves fortuna → mixed → scientia with Try/See/Why; promote and scientia/fortuna glossed; so-what is no silent promote. 5 and 30 are house cutovers, not derived, but they are presented as this slide’s claim types, not as Cardano’s theorem. |
| keynote | K11 | 72 | Sunk 5:3 vs remaining 1-vs-3 is the right question; swarm so-what is on-slide. Fail: lede invokes Pascal remaining paths, then “P(A wins the **match**) : P(B) ≈ 0.5 : 0.125” = **4:1**. Race-to-6 at 5–3 is **7:1**. That is the error this symposium exists to refuse (−20). “Wrong triangles” is unglossed (−8). |
| keynote | K12 | 96 | Launch pad does its job: link + Try/See/Why (payout 4 → negative EV). So-what (knowledge as a book of wagers) is on-slide. Second card is a file list, not a demo. |
| keynote | K13 | 97 | Sidecar vs genesis is one decision; both hover-glossed; four field cards are the contract; LDA-ADR hover-glossed; so-what is ship JSON this quarter, no sixth face. |
| keynote | K14 | 82 | Tabs + Try/See/Why make E1/E2/E7 grokable; so-what on-slide. Fail: E1 prints edge **16.7%** for the table K09 just called ~17% while showing 20% (−8). E5 repeats “about 4:1 continuation” with no 7:1 race figure (−10). |
| keynote | K15 | 84 | Locked vs sidecar columns meet the refusal goal; so-what on-slide (no silent self-promote). Fail: “0.024 USD” never appears on any prior keynote slide (−8). Footer ADR-090 is first use, not in G (−8). |
| keynote | K16 | 84 | Three numbered moves are the ask; Plane is parenthetically glossed; thesis is restated. Fail: no on-slide so-what for *these* three moves (the only “if we are just, we will count” line is in notes) (−8). “deliverable 03” is an unexplained pointer (−8). |
| nova | N01 | 96 | Book-only charge is immediate; so-what assigns G as the primitive dictionary. Cardano is used as a proper name without a one-line who, acceptable once the title is *Liber* but slightly cold (−4). |
| nova | N02 | 98 | Aequitas hover + the condition list; probability is a check, not a trick. So-what (marked-deck bake-off) is the product landing. |
| nova | N03 | 95 | Circuit hover; r/s calculator; Try/See/Why (27:25 → p, odds, even-money EV, p²). Default 27:25 is named as sums 10 vs 9. `.mono` is referenced but never defined in this file (cosmetic). |
| nova | N04 | 97 | ROTM hover; false-claim vs circuit cards; so-what is n×p is not P. No widget needed. |
| nova | N05 | 97 | Lean-as-detector vs Prince (hover-glossed); so-what (a score that always “equals what it should” is as suspicious as a lean) is sharp. |
| nova | N06 | 90 | Remaining-not-sunk is clear; so-what (don’t credit burned tokens) is on-slide; 6:1 ≠ 7:1 is the **correct** refusal. Fail: “triangles” has no gloss (−8). No worked 5–3 on this board. |
| nova | N07 | 97 | Ruin hover; equal stake ≠ equal chance of going broke; so-what is stop-loss ≠ sample space. |
| nova | N08 | 76 | Handoff list is the goal, but it is a dense one-paragraph dump (−10). `scientia/fortuna` is first use and **absent from this glossary** (−8). Coniunctio is unglossed Latin (−6). “crate types” is unglossed (−0 already folded into density). |
| vetus | V01 | 90 | Inventory charge + live MH cast (including `.024` as a price tag, not a sixth die); rhyme and MetaHarness glossed; Try/See/Why on the red 53. Fail: so-what sends the reader to Coniunctio with no gloss, and Coniunctio is not in this G (−8). |
| vetus | V02 | 80 | Phase I rule is on a card. Fail: no product so-what (−10). Four voice names (ecc-analyst, defi-networker, …) and “coherence homonym” are unglossed roles/jargon (−8). |
| vetus | V03 | 76 | 31-row expander is the inventory, and classes are pill-coded. Fail: this is the deck’s main interactive and it has **no Try/See/Why** (−14). No so-what (−10). A cold 5s scan is a database, not a board. |
| vetus | V04 | 95 | Breaker vs circuitus is the one idea; keep-both is the decision; in-tree identifiers are labeled with their job. WEFT-322 is attached to spend/tokens/iters. Pass on the line. |
| vetus | V05 | 76 | Title (“still no circuit”) is the idea. Fail: no so-what (−10). Three scores at once, each a crate/ticket pile (WEFT-54, NoopScorer, RVF, `agent_spawn`, ADR-034) with no first-use gloss (−10). |
| vetus | V06 | 92 | Marked-deck split (cpu/memory/… vs risk/fairness/…) is unmistakable. Lede is a decision (don’t “fix” it in this room). Fail: ADR-034 and `effects.rs` are unglossed (−8). |
| vetus | V07 | 80 | Four coherences + twins is the warning. Fail: no so-what (−10). K2 §5, EML, ruvllm, `GateDecision` land as crate noise (−8). |
| vetus | V08 | 84 | Handoff bullets name the artifacts and the genesis refusal. Fail: sidecar and C9 are first use and not in this G (−8). No explicit so-what line (−8). |
| coniunctio | C01 | 97 | Four landing tests are the slide; Nova/Vetus are glossed in-line as book-only / walls-only; genesis is in this G; so-what (“orphan claims are honorable”) is the ethic. |
| coniunctio | C02 | 96 | Five primitives × named Vetus gap × LDA is the combining table; circuitus and aequitas check are hover-glossed; sidecar and LDA-ADR are in G; so-what is “sidecar, not a sixth genesis face.” |
| coniunctio | C03 | 76 | Title + so-what (pretty forgeries fail the four tests) are gettable. Fail: WEFT-322, EffectVector, ROTM, $0.024 are a cold-reader wall — none are in this glossary (−15). Without the other rooms the examples do not parse in 5s (−9). |
| coniunctio | C04 | 92 | Field list is the contract; Try/See/Why points at the EV demo. Fail: no on-slide so-what for shipping the sidecar (−8). |
| coniunctio | C05 | 85 | Three numbered moves are parseable. Fail: one undifferentiated paragraph (−7). MetaHarness, ADR-034, Plane, `governance-counsel.md` are unglossed here (−8). So-what is “minutes / keynote is next,” not why these three. |

**Pass (≥ 95):** K01, K02, K03, K06, K08, K10, K12, K13; N01, N02, N03, N04, N05, N07; V04; C01, C02.

**Fail:** every other row above. None of the four decks can ship on pages alone.

---

## Whole-deck effectiveness

Not an average of the page column. Dimensions are narrative 25 / one idea 20 / visual 15 / fidelity 20 / decision 20.

### keynote — **86** (fail)

| dimension | /weight | note |
|-----------|--------:|------|
| Narrative arc | 21/25 | Thesis is sentence-one. Rooms → rule → refusals → walls → live dice → edge → luck → remaining → contract → ask is the right spine. K09/K11/K14 number fights bruise the close. |
| One idea per slide | 16/20 | K04 is ROTM plus an unexplained 25/27 chart plus a refuse-list. K05 is a chip census. K08 is L2 *and* marked deck. |
| Visual system | 14/15 | One chrome: gold rule, serif, `.so`, Try/See/Why, three SVGs, two heroes. Progress dots sit on the same baseline as the footer (risk of collision). |
| Fidelity to source | 13/20 | Sidecar-not-genesis and ROTM are honest. **K11 states Pascal then prints 4:1 match odds (true race odds 7:1).** House edge is 20% (widget), ≈17% (Try), and 16.7% (K14 E1) for one table. Nova’s own board refuses 6:1 ≠ 7:1. |
| Decision value | 18/20 | Viewer knows what to believe (name the circuit) and the Monday moves (align briefing, sidecar one MH throw, do not touch genesis). |

### nova — **90** (fail; targeted rewrite, do not re-outline)

| dimension | /weight | note |
|-----------|--------:|------|
| Narrative arc | 23/25 | Aequitas → circuit → ROTM → lean → remaining → ruin → frozen names. Doctrine kit is complete in eight boards. |
| One idea per slide | 18/20 | N01–N07 are one-beat. N08 is a suitcase. |
| Visual system | 12/15 | Same gold/dark as the keynote but thinner: no diagrams, no notes, no dots, Georgia only. Calculator is the only texture. |
| Fidelity to source | 19/20 | ROTM numbers are right. Remaining-work correctly keeps 7:1 and refuses triangles. Book-only wall holds. |
| Decision value | 17/20 | Decision is “these names may leave the room; no crate types.” Clear for Coniunctio; light for a product owner who only opens this file. |

### vetus — **84** (fail; restructure the inventory beat)

| dimension | /weight | note |
|-----------|--------:|------|
| Narrative arc | 21/25 | Live cast → who sat → census → homonym → three scores → marked deck → spent words → handoff. Thesis (classify, do not invent Cardano) is on board 1. |
| One idea per slide | 14/20 | V03 is a 31-row application, not a slide. V05 is three scores. V07 is two homonym families. |
| Visual system | 13/15 | Internally consistent console chrome (system-ui, pills, sticky footer) but **not** the keynote/Nova serif system. Reads as a different product. |
| Fidelity to source | 18/20 | Stub honesty (ruvllm, Level-2 breaker), 0.7 vs 0.8, two Noops, counsel drift: this is the careful room. It does not invent Cardano. |
| Decision value | 16/20 | “Sidecar first, do not smash genesis” is on the last board. Most middle boards never say what to do. A cold engineer gets a catalog, not a contract. |

### coniunctio — **89** (fail; tighten C03–C05, do not re-outline)

| dimension | /weight | note |
|-----------|--------:|------|
| Narrative arc | 23/25 | Rule → landed → refused → contract → three moves. Combining thesis is sentence-one. |
| One idea per slide | 18/20 | Five boards, five jobs. C05 packs three asks into one paragraph. |
| Visual system | 12/15 | Matches Nova (thin, gold rule, serif). No diagram of the four tests or the sidecar JSON. C05 has no cards. |
| Fidelity to source | 18/20 | Five landings match LDA-001–005. Refusals match the doctrine. Does **not** repeat K11’s 4:1-as-Pascal error. Cold-reader examples on C03 assume the other rooms. |
| Decision value | 17/20 | Same three moves as the keynote, plus “both parents cited.” Actionable if you already know the gaps; C03–C05 leak jargon. |

---

## Whole-deck gate

| deck | whole-deck | pages all ≥ 95? | ship? |
|------|----------:|:---------------:|:-----:|
| keynote | 86 | no (8/16) | no |
| nova | 90 | no (6/8) | no |
| vetus | 84 | no (1/8) | no |
| coniunctio | 89 | no (2/5) | no |

Ship rule: every page ≥ 95 **and** every scorer’s whole-deck ≥ 95. All four fail both clauses.

---

## Highest-leverage fixes (notes for the lead; this scorer did not edit)

1. **K11 / K14 E5 / examples E5:** Separate the race (Pascal **7:1**) from independent remaining-work × P(finish) (**0.5 : 0.125 = 4:1**). Do not write “P(A wins the match)” next to 4:1.
2. **K09 vs K14 E1 vs `circuit-ev.html`:** Pick one house-edge definition (`1 − offered/fair` = 20% **or** \|EV\|/stake = 16.7%) and make Try, widget, and E1 print the same number.
3. **V03 / V02 / V05 / V07 / V08 / C04 / K16:** Put a so-what sentence on the board. Add Try/See/Why on the inventory table and on the K07 bars.
4. **First-use holes not in that deck’s G:** Prince, triangles, ADR-090, ADR-096, deliverable 03, $0.024, C9, sidecar (Vetus), scientia/fortuna (Nova), WEFT-322 / ROTM / EffectVector (Coniunctio C03).
5. **N08 / C05 / K04 refuse-list:** Split or card the dumps. One idea per board.

---

Scorer B · pass 2 · 2026-08-13 · read-only
