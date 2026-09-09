# SCORE-pass4-3 — independent scorer 3 of 4

Read-only. Did not edit any HTML. Did not open `SCORECARD.md`, `SCORE-pass2-*`, or `SCORE-pass3-*`.

**Audience used:** WeftOS / clawft engineers, governance, MetaHarness operators (keynote `OUTLINE.md`); workshop cold-reader who can walk the room in five minutes (`AGENDA.md`).

**How deductions were applied** (rubric in `~/.grok/skills/deck/references/rubric.md`):

- Start at 100. Score each page against **that page’s goals** (keynote: `data-primary-goal` / `data-secondary-goals`; workshops: title + lede + room charter, because those files omit goal metadata).
- First-use gloss = on-slide parenthetical, `abbr`/`title` hover, or a definition in the same sentence. A **G** glossary counts only for terms that actually live in *that* deck’s `<dl>`. Chrome that says “G glossary” is a pointer, not a definition of missing entries.
- Interactive controls and live charts need **Try / See / Why** on glass. Static labeled SVGs do not.
- So-what must be on the slide, not only in `N` notes.
- Unexplained Latin, ticket IDs used as words, crate-layer acronyms, and symposium shorthand fail the cold-reader test.

**Pass line:** page ≥ 95 and whole-deck ≥ 95. Merge later will be **min**, not average.

---

## Keynote — `decks/keynote/index.html` (K01–K16)

| id | score | fail? | evidence |
|----|------:|:-----:|----------|
| K01 | 85 | FAIL | Thesis is the title and the lede; Cardano chairs; so-what is on glass; `circuit` and *Liber* are hover-glossed. **−15 overclaim:** the 1564 card says the book “founds a science.” The symposium’s own Nova paper refuses that Ore line (anticipated classical probability; did not found the mature theory; 1663 is after Huygens). Flourish on glass, not hedged. |
| K02 | 93 | FAIL | Two-room method is the three cards; Nova / Vetus / genesis / rhyme-cousin-false-friend are glossed; Try/See/Why is present; so-what is the fairness-of-inquiry sentence. **−5 contrast:** hero is a light parchment half under a 45% top overlay; title and `#AAA8B4` lede sit on that half. Dim text on mid-tan is the readable-but-not-AA beat. **−2 Coniunctio** is defined only as “the combining table” (enough) while the Latin headword itself has no hover. |
| K03 | 98 | | Chapter 14 is a die you can hold: click faces, read `r:s`, `p`, fair payout. Diagram `circuitus.svg` is the same rule in four boxes. EV = 0 is the so-what. Try/See/Why sits next to the faces. Tiny: fair-payout units are not restated as “net odds, stake already down.” |
| K04 | 96 | | ROTM vs circuit is the slider; `n×p` vs `P` is on the lede; 3 → 0.50 vs 0.421 and “four throws to cross ½” match the JS. Try/See/Why present. **−4 secondary:** “other inherited errors” are a footnote (“Prince, interrupted-match, cheating-as-method live later”), so the primary “refusals” plural is only half on this glass. The 25/27 chart is a second idea on the same board; it is labeled, so not a full overload hit. |
| K05 | 92 | FAIL | Dedicated wall: 13 chips expand to class + gap; 31-total honesty is in the lede; Try/See/Why names V1/V6/V7/V17; so-what is “name the rhyme, do not invent a green field.” **−8 unexplained first-use in the bodies the demo opens:** GEPA, ruvllm, K2 — none are in the keynote glossary, none are expanded on click. A cold engineer can class the chip and still not say what those three strings mean. |
| K06 | 98 | | Homonym is the title, the lede, and `homonym.svg` (sample space vs stop-loss, WEFT-322 named). So-what: perfect breaker, crooked table. WEFT-322 is in the glossary and in the breaker hover. No widget, so no Try/See/Why required. |
| K07 | 97 | | Five named faces 75/100/65/90/53, `n = 1`, prior 2026-07-31 75/65/51, cost as a price tag not a sixth die. So-what: do not promote on one cast. Try/See/Why explains the red/amber/gold paint. **−3 title friction:** “unlabeled dice” vs lede “each bar is one named face” — the unlabeled thing is the *circuit*, not the faces; one extra second to parse. |
| K08 | 97 | | Fairness-dim vs aequitas is the title; L2 is unpacked as Euclidean distance, √5, 0.7 vs 0.8; counsel `cpu/memory/…` vs live `risk/fairness/…` is the marked deck. So-what: add a check, do not rename the face. No control, so no Try/See/Why required. (Outline’s weight-drag is not on this HTML; not scored as a missing goal.) |
| K09 | 99 | | Live 4:1 slider: EV −0.167, house edge 16.7% of stake — matches the glossary’s own correction (not the 20% shortfall). Try/See/Why says “drag to 4.” So-what maps to router / vendor / judge. Fair 5:1 → EV 0. |
| K10 | 98 | | `n` slider moves fortuna → mixed → scientia at 5 and 30; those cutovers are labeled as **this house’s rule, not Bernoulli**. Promote is hover-glossed. Flywheel = “signed records of each eval.” Try/See/Why present. So-what: never silently. |
| K11 | 95 | | 5–3 of 6 is three columns: sunk 5:3 / still-needed 1 vs 3 / Pascal 7:1 (BBB = 1/8). So-what is leftover swarm budget, on glass. Triangular numbers hover-glossed. **−5 secondary buried:** LDA-005 exists only in speaker notes. Pass on the line, no slack. |
| K12 | 96 | | Primary is “open the demo”: link + Try/See/Why (six selected, payout 4, negative EV). Secondary “knowledge as a book of wagers” is the so-what sentence, not a worked portfolio. Honest “teaching toy.” Thin, but the declared goal is a doorway and the doorway is labeled. |
| K13 | 97 | | Sidecar vs genesis is the title; four field cards cover circuit / odds-edge / stake-ruin / calibration; LDA 001–005 listed on glass; so-what is ship-the-hole this quarter. `sidecar`, `genesis`, `LDA-ADR` hover-glossed. Cards are not a demo. |
| K14 | 97 | | Tabs E1, E2, E4, E5, E6, E7, E10 with Try/See/Why. So-what: if you cannot replay it on a six-die, keep it out of the kernel. Seven of ten examples; the HTML goal does not demand E1–E10 complete. |
| K15 | 98 | | Locked vs sidecar is one glance. Genesis smash, breaker rename, fairness-as-aequitas, ROTM-as-prior, $0.024 savings, auto-promote are all on the locked list. So-what: self-graduating exploration is already a silent promote. |
| K16 | 98 | | Three moves = drift fix / one sidecar eval / do not touch genesis. Plane and “deliverable 03” are glossed in the lede. Thesis restated. So-what: do these before a central ADR. |

### Keynote whole-deck: **93** — FAIL (90–94: rewrite weak beats, do not polish)

| Dimension | /weight | Score | Ask |
|-----------|--------:|------:|-----|
| Narrative arc | 25 | 23 | Thesis is slide 1. Rooms → rule → refusals → walls → money → contract → ask. K12 is a hinge that does not earn a full beat. |
| One idea per slide | 20 | 18 | K04 is ROTM *and* 25/27. K08 is L2 *and* marked deck. K12 is demo door *and* portfolio aphorism. |
| Visual system | 15 | 14 | DESIGN tokens, gold rail, three SVGs, two heroes. K02 parchment under type is the only contrast miss. |
| Fidelity to source | 20 | 18 | Faces, 16.7%, 7:1, 25/27, counsel drift, 0.7/0.8 all match P2/P3. K01 “founds a science” fights P1. |
| Decision value | 20 | 19 | Viewer leaves with three moves and a sidecar contract. |

**Ship blockers (pages):** K01 85, K02 93, K05 92.  
**Targeted rewrites:** hedge the 1564 card to P1’s anticipated-not-founded line; darken the K02 overlay (or keep type off the parchment); gloss GEPA / ruvllm / K2 in the chip bodies (or drop those tokens from the 13-chip set).

---

## Nova — `workshops/nova/index.html` (N01–N08)

No `data-primary-goal` attributes. Goals inferred from title + room charter (book only; 5-minute kit; no crate types).

| id | score | fail? | evidence |
|----|------:|:-----:|----------|
| N01 | 96 | | Room rule in one glance: book only, arithmetic crimes stay on the table. So-what points at G as the primitive gate. Glossary is complete enough for the rest of the walk. No demo. |
| N02 | 97 | | Aequitas is hover-glossed as equal conditions; the six-part list is in the sentence; so-what is the marked-deck bake-off. 5s works. |
| N03 | 96 | | Live `r:s` calculator; default 27:25 named as three-dice 10 vs 9; output is p, fair odds, even-money EV, p². Try/See/Why on glass. **−4:** p² on a 10-vs-9 *subset* is a slightly different circuit than “repeat the same trial,” and 27/25 is not motivated by a picture. |
| N04 | 97 | | ROTM 3×1/6 vs 1−(5/6)³ ≈ 0.421; two cards False claim / Circuit; so-what is never treat `n×p` as `P`. Matches K04 without the extra chart. |
| N05 | 97 | | Detector stays, Prince is fired (hover-glossed). So-what: a score that always “just equals” is as bad as a lean. One idea. |
| N06 | 90 | FAIL | Title is pay remaining work; so-what (do not credit tokens already burned) is on glass. **−10 missing diagram:** Pacioli 5:3, triangular 6:1, Pascal 7:1, BBB 1/8, “not 4:1” are one paragraph with no splitter. Keynote K11 proves this beat needs three cards. 5s gets the slogan, not the count. Slight Phase-I leak: “tokens already burned” is Vetus dialect. |
| N07 | 97 | | Ruin hover-glossed; rich vs thin purse; so-what is stop-loss ≠ sample space. Clean. |
| N08 | 93 | FAIL | Handoff is the goal: may-leave / may-not-leave. So-what sends combining next door. **−7 term dump:** scientia/fortuna, pⁿ, np, fraud catalog land as a comma list. They are *in* the glossary, so not a full −8 unexplained, but a cold reader cannot say what each primitive *does* after this slide without opening G and reconstructing N02–N07. |

### Nova whole-deck: **88** — FAIL (< 90: restructure, do not polish)

| Dimension | /weight | Score | Ask |
|-----------|--------:|------:|-----|
| Narrative arc | 25 | 21 | Kit order is right (equality → circuit → errors → lean → remaining → ruin → handoff). Charter also asked for a 32-chapter reconstruction. There is no chapter map. |
| One idea per slide | 20 | 18 | N06 is the only overloaded board. N08 is a list, not an idea. |
| Visual system | 15 | 12 | Same gold/dark tokens, G panel, Try/See/Why on the one widget. No SVG, no texture, no chapter spine. Thinner than Vetus on purpose, too thin for the arithmetic. |
| Fidelity to source | 20 | 17 | Arithmetic and refusals match P1. Missing power-rule, frequency/LLN, and fraud-catalog as walked beats (they only appear in the N08 bag). |
| Decision value | 20 | 18 | Coniunctio knows what may leave the room. A cold reader cannot yet *use* remaining-work or the primitive list without the paper. |

**Ship blockers:** N06 90, N08 93. Whole-deck 88.  
**Restructure, not polish:** add a one-screen chapter spine (morality → circuit → power → frequency → fraud); split N06 to the K11 three-column; expand N08 to one line of meaning per primitive.

---

## Vetus — `workshops/vetus/index.html` (V01–V08)

No `data-primary-goal` attributes. Goals inferred from title + Phase I charter (inventory only; class not merge; do not invent Cardano).

| id | score | fail? | evidence |
|----|------:|:-----:|----------|
| V01 | 97 | | Inventory room + live MH cast 75/100/65/90/53 + .024 price tag. `n` explained as trial count. Try/See/Why on the red memory face. So-what: combining is next door. Rhyme / MetaHarness hover-glossed. |
| V02 | 96 | | Four seats + Phase I rule on one grid. So-what: Coniunctio cannot claim the walls were uninspected. Seat names are in-house; `coherence` is in this glossary. No widget. |
| V03 | 91 | FAIL | 31-row expander is the inventory; Try/See/Why names V1/V6/V17; so-what: classification is not a merge. **−8 unexplained in the bodies the demo opens:** ROTM (not in the Vetus glossary — a Cardano import on a Phase I board), GEPA, DEMOCRITUS, ruvllm, RVF. The table chrome is clean; the click payload is not cold-readable. |
| V04 | 98 | | Circuit vs circuit-breaker is two cards and a ≠. WEFT-322 is cited as the spend trip, meaning on the same line. So-what: perfect stop-loss, crooked table. `stake` / `fairness` pills tell the other homonym. |
| V05 | 92 | FAIL | Three recountable casts, still no circuit — primary is the heading. MH / L2 / Fitness weights match P2. So-what is the Coniunctio landing strip. **−8:** `RVF` is an unexplained crate-layer acronym on first use; not in the glossary. Binding-thread is also unglossed but secondary. |
| V06 | 91 | FAIL | Marked deck is two faces of one name; so-what is “fix our briefing before we lecture.” **−8:** that so-what punches on **aequitas**, which is not in the Vetus glossary and is Nova Latin this room was forbidden to invent. The inequality is clear without the word; the word is the unexplained term on the decision line. **−1** Phase I leak. |
| V07 | 92 | FAIL | Four coherences + twins. So-what: always say which formula; do not inventory a stub. Fiedler / λ₂ are in *this* glossary (“spectral lean”). **−8:** `K2 §5` is not glossed (K2 is a prior symposium, not a word a cold WeftOS engineer must know). EML is adjacent and also unglossed; one deduction for the cluster. |
| V08 | 97 | | Handoff list is the goal: V1–V31, homonym, twins, three live scores, counsel drift, sidecar-not-genesis, C9 deferred. So-what: Coniunctio lands only on these gaps. Non-delivery is explicit. C9 / sidecar / genesis are in the glossary. |

### Vetus whole-deck: **94** — FAIL (90–94: targeted rewrite)

| Dimension | /weight | Score | Ask |
|-----------|--------:|------:|-----|
| Narrative arc | 25 | 24 | Live cast → who sat → table → homonym → three honest scores → marked deck → spent words → handoff. Best workshop arc of the three. |
| One idea per slide | 20 | 19 | V07 is two clusters (coherence + twins) under one “do not collapse.” Still one instruction. |
| Visual system | 15 | 14 | Dice, expander table, vs-cards, glossary, DESIGN tokens. More textured than Nova/Coniunctio. |
| Fidelity to source | 20 | 19 | V1–V31, two gates, two breakers, 0.7/0.8, counsel 5D, stub 7-factor — matches P2. ROTM/aequitas on glass are the only doctrine leaks. |
| Decision value | 20 | 18 | Coniunctio gets a closed landing list. Cold reader still trips on V03/V05/V06/V07 jargon in the click payload. |

**Ship blockers:** V03 91, V05 92, V06 91, V07 92.  
**Targeted rewrite:** add ROTM, RVF, K2, GEPA, DEMOCRITUS, ruvllm (or drop the tokens) to the Vetus glossary *and* to the first sentence that uses them; replace V06’s aequitas with “equal conditions” so Phase I stays clean.

---

## Coniunctio — `workshops/coniunctio/index.html` (C01–C05)

No `data-primary-goal` attributes. Goals inferred from P3 (four tests; land or refuse; sidecar; three moves).

| id | score | fail? | evidence |
|----|------:|:-----:|----------|
| C01 | 97 | | Four tests are the lede (named gap, no genesis break, not his error, both parents). So-what: orphans honorable, forgeries not. `landing` / `genesis` live in this glossary. 5s works. |
| C02 | 92 | FAIL | Five primitives × Vetus gap × LDA is the landing table; so-what is sidecar, not a sixth face. circuitus / aequitas hover-glossed. **−8:** **ruin** is a doctrine word here (P(bust)), not in this glossary, and the gap column (“breaker exists, is stop-loss”) names the cousin, not the primitive. Remaining work is English enough to pass. |
| C03 | 96 | | Refusals are a single paragraph: WEFT-322 ≠ circuitus, fairness ≠ aequitas without the check, ROTM-as-prior, $0.024 savings, auto-promote. ROTM / WEFT-322 / EffectVector are in this glossary. So-what: pretty forgeries fail the four tests. |
| C04 | 95 | | Full field list on glass (circuit … equal_conditions). Try/See/Why points at `circuit-ev.html` and says what to see (edge when payout ≠ fair). So-what is publish-the-hole. **On the line:** the so-what then calls that “LDA-003 as a die,” which is the *edge* ADR, not the whole contract. One-point wobble, not a miss. |
| C05 | 97 | | Three moves match keynote K16 (counsel briefing, sidecar the next MH throw, leave genesis/Plane). Plane is in this glossary. So-what: do these before a central ADR. Minutes link is a citation, not a hidden so-what. |

### Coniunctio whole-deck: **90** — FAIL (90–94: rewrite weak beats)

| Dimension | /weight | Score | Ask |
|-----------|--------:|------:|-----|
| Narrative arc | 25 | 23 | Rule → landed → refused → contract → ask. Correct combining-room spine. Too short to show a worked landing. |
| One idea per slide | 20 | 20 | Each board is one instruction. |
| Visual system | 15 | 11 | Gold rail + one table + paragraphs. No contract schema, no E1–E10, no both-parents column. Workshop-thin past the point of “on glass.” |
| Fidelity to source | 20 | 17 | LDA 001–005 and the refusal list match P3. P3’s landing rule was **cite both parents** — the table has no Gould / ADR-034 / WEFT cites. E1–E10 are off-deck. |
| Decision value | 20 | 19 | Three moves are the same ask as the keynote. Viewer knows what to do. |

**Ship blockers:** C02 92. Whole-deck 90.  
**Targeted rewrite:** gloss ruin (and claim_type if you keep it); add a “Nova parent / Vetus parent” column so the four tests are visible, not asserted; one worked landing (E1 or E7) on glass so C04 is not only a link.

---

## Cross-deck (this scorer only)

| Deck | Pages < 95 | Whole-deck | Ship? |
|------|------------|----------:|:-----:|
| Keynote | K01 85, K02 93, K05 92 | 93 | No |
| Nova | N06 90, N08 93 | 88 | No |
| Vetus | V03 91, V05 92, V06 91, V07 92 | 94 | No |
| Coniunctio | C02 92 | 90 | No |

None of the four decks ship under this pass.

**Systemic, not local:**

1. **Unexplained terms still fail after a glossary exists.** Keynote G is strong; chip *bodies* and Vetus expander *bodies* dump GEPA / ruvllm / K2 / RVF / DEMOCRITUS / ROTM. Coniunctio forgot `ruin`. Vetus uses `aequitas` in a so-what. Glossary-in-another-panel does not save a term that is not in that panel.
2. **Demos that are tables/chips count.** K05 and V03 have Try/See/Why chrome and then open unexplained jargon. The rubric bites the payload, not the caption.
3. **So-what is mostly on glass** — this pass is much cleaner than a first expert draft would be. Exceptions: K11’s LDA-005 (notes only); V06’s so-what is on glass but the load-bearing word is unexplained.
4. **Nova and Coniunctio are under-built relative to their charters.** Nova never shows the 32-chapter spine it was asked to reconstruct. Coniunctio never shows both parents on a landing. Keynote and Vetus are the load-bearing decks; the two short rooms still have to clear 95 on their own glass.

**What already meets the bar (do not reopen):** K03 die, K06 homonym, K09 16.7% edge, K10 honest 5/30 cutovers, K15 locked-vs-sidecar, K16 three moves, N02 aequitas, N04 ROTM, N07 ruin, V01 live cast, V04 breaker split, V08 closed handoff, C01 four tests, C05 three moves.
