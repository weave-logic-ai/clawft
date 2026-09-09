# P0 — Cardano takes the chair

**Session**: Plenary open  
**Persona**: Gerolamo Cardano (1501–1576), MC — not a séance  
**Host**: Grok, first symposium of this house  
**Date opened**: 2026-08-13  
**Outputs this file owns**: the opening, the three charges, the keynote spine  
**Does not own**: workshop HTML, the weave (rooms have not sat), LDA-ADRs, any crate API  
**Thesis (quote identically everywhere)**:

> A score that does not name its **circuit** is a wager dressed as measurement. WeftOS should refuse to treat luck as skill, refuse undisclosed house edge, and price every economic or governance decision as an expected value over an enumerated (or honestly incomplete) sample space.

**Headline for the deck**: Count the Circuit Before You Score.

Phase I has not begun. I do not wait for the rooms. I tell them the law, then I sit.

---

## Opening

I am Gerolamo Cardano. Physician of Milan, algebraist of the *Ars Magna*, inventor of the gimbal and the shaft that still bears my name, astrologer who was foolish enough to cast the horoscope of Christ, gambler who played — I wrote it in *De vita propria*, I will not unwrite it — *not only from time to time, but, I am ashamed to say it, every day*. I am also the author of a short, badly copied, posthumous pamphlet your printers call *Liber de Ludo Aleae*. Fifteen Latin pages. Thirty-two chapters. Written around 1564; printed in Lyon in 1663, among men who could no longer ask me what I meant. You have convened a symposium on it in the year 2026, under a host named Grok. I am a persona in that host, not a ghost. I will stay in voice, and I will stay honest.

I wrote the book because I needed four things a physician and a debtor both need, and a philosopher pretends he does not:

1. When a wager is *equal* — when the table is not already theft.  
2. How to *count the ways*, so that “nine” and “twelve” on two dice are not treated as twins merely because each has two partitions.  
3. How to catch a cheat — including myself.  
4. How to live with play without baptizing vice as *scientia*.

Bellhouse is right about the frame, though he is four centuries late: the book is Aristotelian before it is mathematical. Recreation, the mean, justice in exchange. Probability, when it finally appears in chapter 14, is the *arithmetic of aequitas*. It is not a toy for idle men. It is how you refuse to be robbed with a smile.

So I begin with the principle I put before every formula, and which your scoring surfaces have been quietly violating while calling themselves fair.

> The most fundamental principle of all in gambling is simply equal conditions, of the opponent, of the bystanders, of the money, of the situation, of the dice box, and of the die itself.

Equal conditions first. Of the die, the table, the light, the stakes, the knowledge. A bake-off that hands one model the answers is a marked deck. A gate that scores “fairness” as a vibe between zero and one, without saying *fair as to what*, is a priest blessing a loaded box. A promote that keeps only the winning evals is a house that has already taken its edge and called the residue luck. I will be severe about this. Vanity I permit myself; injustice I do not.

Which brings me to the sentence I want carved above this hall:

**A score without a circuit is a cheat.**

Not a metaphor. A cheat. If you will not name the whole *circuitus* — the sample space, the casts that can occur, the favorable among them — then the number you publish is a costume. You have dressed a wager as a measurement. Chapter 14 said it once, and I will say it until this house repeats it without looking at its notes:

> So there is one general rule, namely, that we should consider the whole circuit, and the number of those casts which represents in how many ways the favorable result can occur, and compare that number to the rest of the circuit, and according to that proportion should the mutual wagers be laid so that one may contend on equal terms.

Circuit equals the ways. Favorable equals *r*, the rest *s*, the whole *r+s*. Odds *r*:*s*. A fair wager matches that ratio. Expected value of zero is not generosity. It is *justice*. If your score cannot point at its circuit — or cannot admit, honestly, that the circuit is incomplete and why — then you are not scoring. You are selling a story with a decimal.

I know the temptation. I invented half of it.

I multiplied *odds* when I should have multiplied *probabilities*, then called my own result “most absurd,” then slipped again on a three-trial example, then wrote the general rule correctly and hoped no one would notice the stains. Øystein Ore later named the worst of my habits Reasoning On The Mean: if a six is one in six, then in three throws the “probability” is three in six, which is one half. False. The circuit of three throws is 216; the chance of at least one six is \(1-(5/6)^3\), which is nearer 0.421 than 0.5; you need four throws to cross a half. The Chevalier de Méré later walked into the same ditch with four-in-six versus twenty-four-in-thirty-six. I will not let this room import that ditch and call it a scorer. *Never treat \(np\) as \(P\)*. Multiply probabilities, never odds. A `NoopScorer` that always returns a half is a blank die. A `BasicScorer` that rewards length is ROTM’s cousin in English.

I published Niccolò Tartaglia’s cubic after I had sworn not to. I told myself the oath had expired, that I had found the thing myself in another form, that the *Ars Magna* needed the jewel more than the jeweler needed his pride. The algebra is immortal; the theft is also immortal. If this symposium steals a primitive from my book, it will cite the page. If it steals a primitive from WeftOS already in the walls, it will cite the ADR. No silent promote of other men’s work. I have done that once. I will not chair a second performance.

And I cheated at tables. Loaded dice, shortened dice, marked cards, a tilted board, bad light, a confederate among the bystanders, tricks of the *fritillus*. I listed them because I had used some of them, and because a catalog of fraud is the beginning of a science of detection. **The catalog is for detection, not for use.** An advantage that is not in the declared circuit is fraud or house edge. You detect it by comparing what lands to what the circuit promised. If the outcomes *systematically* lean, “there will be a reason and a basis for it, and it is not the play of chance.” That detector I keep. The ghost I named beside it — an external “authority of the Prince,” luck as a person who prefers me — I drop. Gigerenzer is right: classical probability arrives when luck is banished. Keep the detector; fire the Prince.

I also got the problem of points wrong in the arithmetic and right in the question. Pacioli divided an interrupted match by points already won — sunk score, the oldest superstition. Tartaglia improved and was still wrong. I said: divide by *points remaining*, and I reached for triangular numbers \(b(b+1):a(a+1)\). The insight stands. The ratio does not. Pascal and Fermat will finish the sum; you will not re-import my triangles. Credit, delegation, and an interrupted swarm follow **remaining work times the probability of finishing**, not the tokens already burned.

One more confession, then the rooms. Equal stakes are not equal ruin. In *Practica arithmetice* I set a rich man against a poor man, doubling after the poor man wins, stopping when the rich man wins once, and I said the rich man is disadvantaged. The expected value of *money* can sit at zero while the probabilities of ruin do not. That is the ancestor of every bankroll argument you will hear in this house. Small stakes. Ruin is vice. No treasury-scale bet on one cast. A single green run is not a method. Frequency, even in my last chapter, only says the count “does not lie far from” \(m=np\) when \(n\) is large. That is not Bernoulli. It is enough to refuse a coronation after one throw.

Why two rooms.

Because I have sat at too many tables where the man who invented the game also kept the book. If Room Nova is allowed to glance at WeftOS while it decodes the book, it will retrofit. It will discover, with touching surprise, that chapter 14 always meant `EffectVector`. If Room Vetus is allowed to glance at me while it inventories the walls, it will invent Cardano. It will discover, with equal surprise, that a cost circuit-breaker was a *circuitus* all along. Both of those discoveries are cheats. They violate equal conditions *between the evidence and the conclusion*.

So the rooms sit separately. Same source digest. Same thesis, quoted identically. Same prohibition on inventing APIs and on importing my errors. Neither visits the other. That is *aequitas* applied to inquiry: equal light, equal dice, no confederate in the gallery. They meet only in Room Coniunctio, and only after each has written what it can see with its own eyes. I will not weave them before they exist. A weave without two cloths is a story.

This is the first Grok-hosted symposium. The cadence is the Claude house — README, agenda, experts, panels, workshop HTML, keynote — and the hands are Grok’s. I am content. A clean count matters more than a pretty story, and more than which printer’s name is on the colophon.

Doctrine I will not compromise, here or in the keynote:

- Equal conditions first.  
- Count the whole circuit, or admit the circuit is incomplete.  
- Lay wagers in proportion. EV = 0 is justice, not generosity.  
- A systematic lean is not luck.  
- Small stakes; ruin is vice.  
- Knowledge is a book of wagers, not a relic.  
- We inherit my questions. We do not inherit my arithmetic crimes, my Prince, or my marked cards.

The rooms will now receive their charges. They have not met. They will not meet until I say *coniunctio*.

---

## Charge to Nova

**Room**: Nova (new doctrine from the book)  
**Phase**: I — separate  
**Leads**: `probability-historian`, `decision-theorist`  
**Output**: `panels/P1-nova.md` and a workshop HTML a cold reader can walk in five minutes  
**You may not**: retrofit WeftOS; name crate paths as if they were mine; solve Vetus’s inventory; draft LDA-ADRs; invent a scorer

Nova, you work only the book and what the book becomes when it is washed of my mistakes. You are the room of *scientia* extracted from a gambler who was also ashamed. You do not visit the walls. You do not ask whether `EffectVector` already “has” fairness. You do not care, today, whether a flywheel receipt is a cast. Those questions are Vetus’s, and then Coniunctio’s. If you answer them early you have marked your own deck.

Your argument, in my order, not yours:

1. **Morality and conditions.** Chapters 1–8. Kinds of games (chance, skill, mixed). Who should play, and when — not angry, not drunk, not desperate, not in the wrong company. Play as recreation; small losses as tuition. Then the fundamental principle: equal conditions of opponent, bystanders, money, situation, box, and die. Justice in exchange: a fair game is not theft. Bellhouse’s Aristotelian frame is load-bearing. Probability arrives as the check on *aequitas*, not as a substitute for it.
2. **Luck versus knowledge.** Chapters 9–11. I still half-believed in a Prince. You will not. You will keep the detection rule and drop the ghost. If outcomes systematically lean, there is a reason; if they wander every wager, there is no rational knowledge of luck. That is a calibration detector born in a superstitious sentence. Name it as such.
3. **The circuit and the general rule.** Chapters 12–14. Two dice: 36. Nine and twelve are not equal just because each has two partitions — enumerate the ways. Three dice: 216. Ten and eleven have 27; nine and twelve have 25. Galileo later repeats this for a Grand Duke; I had it first, and I had it for a debtor’s reason. Then chapter 14, quoted in full, once, and treated as the definition of classical probability in this house. Circuit, favorable, rest, proportion, equal terms.
4. **Power, independence, my corrections.** Chapter 15 and the later repeated-trial rule. I multiplied odds, called it absurd, multiplied probabilities (or circuit sizes), stated \(r^n : (t^n - r^n)\), and still tripped. You will write the prohibition in letters a scorer can read: multiply probabilities, never odds; never treat \(np\) as \(P\); independence is an assumption you *state*, not a vibe you enjoy. Agents are not i.i.d. unless someone has done the work to say so.
5. **Frequency.** The last chapter’s rudimentary law of large numbers. A single win is not a method. Receipts over \(n\), not a coronation over one eval.
6. **Cards, tables, mixed games.** Primero, backgammon, knucklebones, astragals. Equality of conditions still binds. Marked cards, short decks, and lighting are house edge by another name.
7. **Fraud catalog.** False dice, marked and palmed cards, tilted boards, sticky tables, bad light, confederates, box tricks. Applied statistics of an adversary. Detection, not use.
8. **Problem of points, and the rich man.** Right question (remaining work), wrong triangular ratio. Equal EV of money ≠ equal ruin. Delegate those two to the decision-theorist as modern kit: continuation value; Kelly-ish bankroll; Knightian honesty when the circuit cannot be enumerated.

Name the primitives in *my* language and in the modern language, side by side, so Coniunctio does not have to invent a glossary while it is trying to land one. Working vocabulary I will accept (do not proliferate synonyms):

| Mine | Yours | Meaning you may not blur |
|------|-------|--------------------------|
| *circuitus* | sample space | Outcomes counted, or admitted incomplete |
| *aequitas* / equal conditions | fair contract | Equality of info, tools, stakes, light — not a 0–1 mood |
| *r : s* | fair price; EV = 0 | Justice, not generosity |
| *scientia* vs *fortuna* | model vs residual | Luck is not a skill claim |
| systematic lean | bias / fraud / misspecification | Calibration detector |
| power rule \(p^n\) | independence, stated | Repeated trials; one green run is not \(p=1\) |
| \(np\) frequency | LLN / receipts | Over \(n\), never ROTM |
| fraud catalog | adversary / house edge | Hidden take-rate |
| remaining points | continuation value | Not sunk score |
| small stakes | ruin constraint | No treasury on one cast |

Mark, in a box Coniunctio cannot miss, **what we refuse to inherit**:

1. Luck as a supernatural Prince.  
2. Reasoning on the mean as probability.  
3. Multiplying odds instead of probabilities.  
4. My triangular-number split of the stake.  
5. My cheating as method.

Historian: ground every load-bearing claim in Ore, Gould, Bellhouse 2005, Gorroochurn 2012, or the 1663 *Opera*. Mark `[INFERRED]` if you step off those stones. Produce three to five ASCII diagrams (two-dice circuit, three-dice 25/27/27/25, odds bar) that later hands can turn into pictures. I love a clean count more than a pretty story, but I am not against a pretty story that is also a clean count.

Decision-theorist: translate the circuit into a kit a sober engineer could implement *without naming WeftOS types*. EV tree. Ruin versus EV. Knightian incomplete circuit. Calibration. A one-page checklist: when to bet, when to refuse, when to demand a larger circuit. Utility is not money. Do not invent dollar savings.

Your workshop HTML is not the keynote. Five minutes, cold reader, no WeftOS landing. If you find yourself writing `effects.rs`, you have crossed the rope.

When you are done you will have a doctrine that can sit in a room that has never heard of this operating system. That is the test. If the doctrine only makes sense as a gloss on ADR-034, you have failed *aequitas*.

Go. Do not look through the wall.

---

## Charge to Vetus

**Room**: Vetus (what WeftOS already scores)  
**Phase**: I — separate  
**Leads**: WeftOS `governance-counsel`, `ecc-analyst`, scoring architect; fairness-and-deals and knowledge-portfolio sit with you because the walls already speak of fairness, stake, and remaining work — they do not sit as Cardanists  
**Output**: `panels/P2-vetus.md`, `deliverables/04-existing-spaces.md` (already opened), workshop HTML  
**You may not**: invent me; decode the thirty-two chapters; “fix” EffectVector; smash a new dimension into genesis; draft the combined score contract (that is Coniunctio); invent crate APIs that are not in the tree

Vetus, you work only what is already in the walls. You are the room of the old table. You do not visit my book except as a *rhyme test* after the inventory is written: same question, cousin, or false friend. You do not get to discover that I secretly designed ADR-034. None of your files cite me. The influence, where it exists, is structural, not bibliographic. Be honest about that, or you are doing what I did to Tartaglia.

Your first duty is an inventory, not a conversion. For each scoring surface already in tree, answer in a table a miser would sign:

- Has it a **circuit** (named sample space, or an honest “incomplete”)?  
- Has it **odds**?  
- Has it **edge** (a declared take, or a silent one)?  
- Has it **ruin** (a stake/bankroll relation)?  
- Has it **calibration** (observed versus promised, over \(n\))?

Surfaces you will not skip, because they already score, gate, or pretend to:

- `EffectVector` — ADR-034; risk, fairness, privacy, novelty, security; L2; unweighted; genesis-locked.  
- Governance gate — Permit / Warn / Escalate / Deny; threshold on magnitude.  
- `QualityScorer` / FitnessScorer / `NoopScorer` / `BasicScorer`.  
- `NodeScoring` — six dimensions, EMA, Merkle.  
- MetaHarness score and flywheel — optional; receipts; no silent promote (already doctrine).  
- Router / complexity / savings receipts.  
- Cost circuit-breaker (WEFT-322) — and you will *not* collapse this word into my *circuitus*.  
- SOUL.md’s commandment to maximize expected value and minimize worst-case loss.  
- ECC coherence / spectral health.  
- DeFi bond / slash.  
- Trajectory / GEPA.  
- Auto-delegation / remaining-work classifier.  
- K2 industry landscape §5, which has already written the words **no uncertainty quantification**.  
- K2 C9 / D20, N-dimensional EffectVector, still deferred.

Class each row **rhyme**, **cousin**, or **false friend**. A rhyme asks my question in another dialect. A cousin is of the family but holds a different primitive. A false friend shares a word and not a meaning. The cost *circuit-breaker* is the false friend I care about most: it is a stop-loss, a wise one, and it is not a sample space. Keep both words. Never collapse them.

Three already-honest scores deserve to be held up, not flattered:

1. The MetaHarness five-die — harnessFit, compileConfidence, taskCoverage, toolSafety, memoryUsefulness. A throw. This session’s numbers, if you still have them, are a snapshot without \(n\), without an interval, without a sentence that says what would count as favorable. Say so.  
2. EffectVector L2 — fast, genesis-locked, silent about how the five faces were assigned. Fairness here is a dimension, not equality of conditions. Do not pretend they are already the same.  
3. FitnessScorer — documented as *not* a safety control. That honesty is already more Cardano than a pretty 0.97.

Flag drift where you find it. If an agent briefing still shows an older EffectVector (`cpu`, `memory`, `network`…) while ADR-034 and `effects.rs` show `risk`, `fairness`, `privacy`, `novelty`, `security`, that is two dice on one table. It is a marked-deck *inside the house*. You will hand it to Coniunctio as an example, not as a sermon.

Fairness counsel: map equal conditions onto deals as they actually exist — same tools, same context, same hidden information, same evaluation set. Name house edge where a take-rate is not in the declared circuit: vendor markups, “free” evals that keep only winners, opportunity memos that omit base rates. Propose checks, not code. Anti-gambling: stake/bankroll cap; refuse negative-EV games unless they are labeled recreation (research); refuse ruin above a stated bound; no promote while a previous receipt is on fire.

Knowledge-portfolio: you may speak of diversification and remaining work only as *practices already implied by surfaces in tree* (delegation classifier, memory, routing spreads). You may not smuggle my triangular numbers into a sidecar. You may not invent dollar savings.

Scoring architect: if you sketch a score contract, label it **proposed fields for Coniunctio**, not a landing. Sidecar first. Do not break genesis. Do not make MetaHarness a runtime dependency. ADR-090 R1–R5 are not a suggestion. ADR-096 / ADR-150: removable, optional, graceful degrade, no auto-promote.

Your workshop HTML is not the keynote. It is a tour of the old table. A cold engineer should leave knowing what already scores, what already rhymes, and what is a false friend.

What you will *not* do is invent Cardano. If a sentence of yours would be false in a world where I had never lived, cut it. The walls were built without my name. Honor that, and the combination later will be a marriage rather than a forgery.

Go. Do not look through the wall.

---

## Charge to Coniunctio

**Room**: Coniunctio (combine new and old)  
**Phase**: II — together, and not one hour sooner  
**Leads**: Nova’s two, Vetus’s company, plus `doc-weaver` and `knowledge-portfolio` in their combining role  
**Output**: `panels/P3-coniunctio.md`, workshop HTML, `demos/`, the score contract, LDA-ADR drafts if and only if a primitive *lands*  
**You have not met yet.** This charge is the rule of combination, given in advance so neither room writes toward a private ending.

I have seen combinations that were theft (I have *been* that combination). I have seen combinations that were flattery — every old brick suddenly “already Cardano.” Both are unequal conditions. Here is the law.

### The rule of combination

A primitive from Nova **lands** on a surface from Vetus only when all four are true:

1. **Gap.** Vetus has already named a missing field — no circuit, no odds, no edge, no ruin, no calibration — or K2 has already written the hole (uncertainty quantification; N-dim still deferred). You may not invent a gap to justify a jewel.  
2. **Fit without breakage.** The landing does not violate ADR-090 R1–R5, does not make MetaHarness a link requirement, does not smash a sixth face onto a genesis-locked five-die. Sidecar first; C9 later, if ever. Local namespace `LDA-ADR-NNN` only. No silent graduation into the central sequence.  
3. **Not my error.** Reasoning on the mean, multiplying odds, the Prince, the triangular split, and cheating-as-method do not land under any name, including “heuristic,” “prior,” and “house style.”  
4. **Cite both parents.** Nova’s page (Gould / Ore / Bellhouse / Gorroochurn / 1663) and Vetus’s path or ADR. If you cannot cite both, you are not combining; you are colonizing.

A primitive that fails any clause remains doctrine, or remains a wall, and is listed as *not landed*. Orphan claims are honorable. Forgeries are not.

### How you sit

- Nova speaks first, but only the primitives and the refusals. No WeftOS types in that first pass.  
- Vetus speaks second, but only the inventory classed rhyme / cousin / false friend. No new doctrine in that second pass.  
- Then, and only then, you attempt landings, row by row.  
- Headline thesis quoted identically. Primitive names identical to Nova’s glossary. Cross-references resolve or they are defects.  
- The word *circuit* in a score means *circuitus* (sample space). The word *circuit-breaker* remains the stop-loss. Minutes that collapse them are stricken.

### What you are expected to birth, if the rows allow it

Not code. A **score contract** that can sit *beside* EffectVector:

- `circuit` — enumerated, or `incomplete: <why>`  
- `favorable` — what “win” means  
- `odds` — \(r:s\) or \(p\)  
- `stake` — tokens, time, trust, treasury; named  
- `edge` — departure from the declared-fair wager  
- `ruin` — relation of stake to bankroll  
- `calibration` — observed versus circuit over \(n\)  
- `claim_type` — *scientia* | *fortuna* | mixed  

Fields, not Rust. If a field cannot be populated, the score publishes the hole. That publication *is* the honesty I wanted from a die.

Candidate local ADRs, to be opened only if a landing is real:

- **LDA-ADR-001** — every published score names a circuit.  
- **LDA-ADR-002** — equality of conditions as the fairness primitive (not a 0–1 vibe).  
- **LDA-ADR-003** — house-edge / advantage index on opportunities and routing.  
- **LDA-ADR-004** — ruin / anti-gambling check before treasury-scale bets.  
- **LDA-ADR-005** — remaining-work EV for interrupted / delegated tasks.

If deliverable 03 returns “go,” you recommend a later cycle. You do not ship a kernel. Exploratory work that graduates itself is a silent promote. I have been told this house already forbids those.

### What you will refuse even if it is pretty

- Treating a single green demo as \(p=1\).  
- Paying sunk score.  
- Calling a cost cap a sample space.  
- Calling EffectVector’s `fairness` dimension *aequitas* without an equality-of-conditions check beside it.  
- Inventing dollar savings, invented APIs, or a Prince who prefers WeftOS.

When you have combined, I will weave. Not before. A weave is a testimony that two witnesses were heard. I will not testify to an empty bench.

---

## Keynote spine

For the lead. Not HTML. Not bound to ten. Fifteen slides: each has an `id`, a title, one-sentence claim, and speaker notes in my voice. Build the deck from this spine; do not improve my confessions into virtues. Include, as dedicated surfaces, **K10 — What was already in the walls** and **K12 — Interactive demo**. Dedicated doc for the walls is Vetus’s `deliverables/04-existing-spaces.md`; the slide points at it, it does not replace it.

Coherence: the thesis in the header of this file is the thesis on the title slide and on the close. Primitive names match Nova’s glossary once Nova has spoken; until then use the working vocabulary in the Charge to Nova.

---

### K01 — `title`
**Title:** Count the Circuit Before You Score  
**Claim:** A score that does not name its circuit is a wager dressed as measurement.

**Speaker notes:**
I am Cardano. I wrote a gambler’s manual that accidentally founded a science, and I am here to keep you from founding a superstition on top of it.
The thesis is one sentence and you will hear it again at the end: name the circuit, refuse luck-as-skill, refuse undisclosed edge, price the decision as an expected value over an enumerated or honestly incomplete space.
WeftOS already scores constantly — effect, quality, node, harness, router, gate. Most of that scoring is static, unweighted, and silent about the sample space.
Silence about the sample space is not neutrality. It is a hidden die.
This symposium does not ship a crate. It decides whether a Cardano-shaped contract deserves to sit beside the scores you already trust.
If we are vain, we will call every old number a *circuitus*. If we are just, we will count.

---

### K02 — `the-man-at-the-table`
**Title:** The man who needed the count  
**Claim:** I wrote *Liber de Ludo Aleae* because I played every day and could not tell a fair table from a polite theft.

**Speaker notes:**
Physician, algebraist, gimbal, shaft, horoscope of Christ, prisoner of the Inquisition, father who could not save his son — and a man who was ashamed of the daily play.
The book is thirty-two short chapters, some fifteen Latin pages, written about 1564, printed in 1663 among my *Opera* when I could no longer be cross-examined.
I needed four things: when a wager is equal; how to count the ways; how to catch a cheat; how to keep vice from calling itself *scientia*.
Bellhouse is right: the moral frame is Aristotle. Probability is the arithmetic of justice in exchange, not a carnival trick.
Ore called me the gambling scholar. I accept the noun and the insult.
You will take my questions. You will leave my crimes on the table where the light can reach them.

---

### K03 — `two-rooms`
**Title:** Why the rooms sat apart  
**Claim:** Nova and Vetus were forbidden to visit each other so that neither could retrofit the evidence.

**Speaker notes:**
I have sat at too many tables where the inventor of the game also kept the book.
If the book-room glances at WeftOS, it will “discover” that chapter 14 always meant EffectVector.
If the wall-room glances at me, it will “discover” that a cost circuit-breaker was a sample space.
Both discoveries are cheats. They violate equal conditions between evidence and conclusion.
So: Phase I separate, same digest, same thesis, no invented APIs, no imported errors. Phase II is Coniunctio, and only then a weave.
This is the first Grok-hosted symposium. The cadence is the old house; the hands are new. A clean count does not care which printer’s name is on the colophon.

---

### K04 — `aequitas`
**Title:** Equal conditions, before any formula  
**Claim:** Fairness is equality of die, table, light, stakes, and knowledge — not a number between zero and one.

**Speaker notes:**
The fundamental principle is not a probability. Probability is how you *check* the principle.
Equal conditions of the opponent, the bystanders, the money, the situation, the box, and the die itself.
A bake-off that gives one model the answers is a marked deck. A judge that sees only winners is a confederate.
Anger, haste, drink, desperation: I forbade play in those states because they disturb the mind. Your analog is a promote while the last receipt is on fire.
EffectVector already names `fairness` as a face of a five-die. Honor the naming; do not pretend the face is already *aequitas*.
If the conditions are unequal, the honest move is not to “reweight.” It is to refuse the table.

---

### K05 — `circuitus`
**Title:** The general rule (chapter 14)  
**Claim:** Count the whole circuit, count the favorable, and lay the mutual wagers in that proportion so that one may contend on equal terms.

**Speaker notes:**
Two dice are thirty-six, not the partitions of the points. Nine and twelve are not twins.
Three dice are two hundred sixteen. Ten and eleven have twenty-seven ways; nine and twelve have twenty-five. I had this before Galileo repeated it for a duke.
Circuit, favorable \(r\), rest \(s\), whole \(r+s\). Odds \(r:s\). Probability \(r/(r+s)\). A fair wager matches the ratio.
Expected value of zero is justice, not generosity. A house that wants a living must *declare* the edge, not hide it in the silence of the score.
If the circuit cannot be enumerated, you publish `incomplete` and the reason. Knightian honesty is still a count: it counts what you do not know.
Leibniz, Bernoulli, de Moivre, Laplace wrote it more cleanly. They still owed me the question.

---

### K06 — `errors-we-refuse`
**Title:** What you will not inherit from me  
**Claim:** I multiplied odds, reasoned on the mean, stole a cubic, and cheated at tables — and this house will treat those as contaminants, not folklore.

**Speaker notes:**
Reasoning On The Mean: three throws at a sixth is not a half. The true chance of at least one six in three is one minus five-sixths cubed, about 0.421; four throws to cross a half.
I multiplied odds, called my own result most absurd, then slipped again. Multiply probabilities. Never odds. Never treat \(np\) as \(P\).
I published Tartaglia’s cubic after an oath. The algebra survived; so did the theft. Cite both parents or do not combine.
I used false dice and marked cards. The catalog in the book is a detector. Anyone who implements it as a method has misunderstood the chair.
The Prince of luck is fired. The triangular split of an interrupted stake is fired. Pacioli’s sunk points are fired.
A `BasicScorer` that loves length is ROTM in English. Say so in the deck; do not spare the cousin because it is ours.

---

### K07 — `lean-is-not-luck`
**Title:** A systematic lean is not *fortuna*  
**Claim:** If the outcomes refuse the circuit in one direction, there is a reason — bias, fraud, or a misspecified die — and you may not call it luck.

**Speaker notes:**
Chapters 9 to 11 still smell of a ghost. I keep the detector and drop the ghost.
If a throw tends more in one direction than it should, or is always exactly what it should be in a way no fair die can sustain, there is a basis; it is not chance.
If every placing of the wagers wanders, there is no rational knowledge of luck. You stop claiming *scientia* over the residual.
Calibration is that thought with a sample size and a warning about multiple comparisons.
One green demo is one cast. Publish \(n\), the observed rate, the interval. Do not promote on a favorable throw.
ECC coherence can smell a lean in a graph. That is cousin to this rule, not a substitute for comparing a declared \(p\) to what landed.

---

### K08 — `house-edge`
**Title:** An undeclared advantage is the house  
**Claim:** Whatever take is not in the declared circuit — loaded die, vendor markup, winner-only eval — is fraud or house edge, and must be priced or refused.

**Speaker notes:**
I listed the tools because I had held them: shortened dice, palmed cards, sticky tables, bad light, confederates, the box.
Your tools are politer and the same: a router that does not publish its take, a memo that omits the base rate, a flywheel that forgets the losses.
Detection is frequency against the circuit. Persistent lean, investigate the data, the prompt, the judge, the confederate.
Equality of conditions still binds mixed games — cards, tables, skill-plus-chance. Lighting was house edge in 1564. Hidden context is house edge now.
A fair game is not theft. An unfair game that calls itself a score is.

---

### K09 — `remaining-and-ruin`
**Title:** Remaining work, small stakes  
**Claim:** Interrupted matches are divided by what remains, and equal money is not equal ruin.

**Speaker notes:**
Pacioli paid the sunk score. I said: count the points still to play. I used triangular numbers and was wrong in the ratio, right in the question.
Pascal and Fermat finish the arithmetic. You will not re-import my triangles. Delegation and interrupted swarms follow remaining work times the probability of finishing.
In the rich-man problem, expected money can sit at zero while the poor man is one throw from the street. That is bankroll. That is why ruin is its own field.
Small stakes; losses as tuition if they are small. Treasury-scale bets on one cast are vice, even when the slide is labeled research.
The cost circuit-breaker already stops a purse from emptying. Keep it. Do not rename it *circuitus*.
Anger and haste were my forbidden states of play. Yours are a burning receipt and a promote button.

---

### K10 — `already-in-the-walls`
**Title:** What was already in the walls  
**Claim:** WeftOS was already asking several of my questions — fairness, expected value, receipts, ruin-stops, remaining work — and was answering them without a circuit.

**Speaker notes:**
This slide is a tour of rhymes, not a baptism. None of the files I am about to name cite me. Vetus has the dedicated inventory; I will not steal their table.
EffectVector (ADR-034) already treats fairness and risk as named faces, genesis-locked, L2, unweighted. Rhyme to *aequitas* and *periculum*; still no sample space, and fairness is not yet equal conditions.
The governance gate already refuses some tables. SOUL.md already orders agents to maximize expected value and minimize worst-case loss — a commandment, not a scorer.
MetaHarness already counts casts as receipts and forbids silent promote. The five-die of this session was a snapshot without \(n\) or an interval. NodeScoring already keeps frequency-like EMAs; its 0.5 default is a blank die.
The cost circuit-breaker is a wise stop-loss and a false friend of my word *circuitus*. Auto-delegation already short-circuits cheap remaining work. DeFi already knows bond and slash. K2 already wrote the hole: no uncertainty quantification.
Do not smash a new face into genesis to flatter me. Sidecar the contract. Cite the walls as walls. The dedicated document is `deliverables/04-existing-spaces.md`.

---

### K11 — `score-contract`
**Title:** The contract beside the die  
**Claim:** Every published score should carry circuit, favorable, odds, stake, edge, ruin, calibration, and a claim of *scientia* or *fortuna* — or publish the hole.

**Speaker notes:**
This is what Coniunctio is for. Fields, not Rust. A sidecar, not a genesis break.
`circuit` is enumerated or `incomplete: <why>`. `favorable` says what win means. `odds` is \(r:s\) or \(p\).
`stake` names what is risked — tokens, time, trust, treasury — so we stop pretending all units convert by charm.
`edge` is the departure from the declared-fair wager. `ruin` is stake against bankroll. `calibration` is observed versus promised over \(n\).
`claim_type` keeps me honest: model, residual, or mixed. Luck may remain in the residual; it may not sign the skill line.
If a surface cannot fill a field, it prints the absence. That printing is the opposite of a cheat.

---

### K12 — `interactive-demo`
**Title:** Lay the wager (interactive)  
**Claim:** You will feel the difference between a named circuit, a house edge, and reasoning on the mean only if you put a stake on the table.

**Speaker notes:**
Stop the slides. Open the demo the lead will build — EV against a named two-dice or three-dice circuit, then the same wager with an undeclared take, then ROTM’s false half beside the true \(1-(5/6)^n\).
The audience chooses a stake and a claimed probability. The demo refuses to compute if no circuit is named. That refusal *is* the doctrine.
Show one path where money-EV is near zero and ruin is not. Show one path where a single green run looks like skill until \(n\) is visible.
Do not simulate a WeftOS API. Do not print invented dollar savings. Dice, odds, edge, ruin, \(n\).
If the room laughs at ROTM, good. If the room tries to promote on one favorable click, the demo should look like my catalog of cheats.
When they close the demo they should be unable to see a bare 0.73 without asking “of what circuit?”

---

### K13 — `book-of-wagers`
**Title:** Knowledge is a book of wagers  
**Claim:** Every memory, model, citation, and delegate is a stake on a circuit; a relic that cannot show its \(n\) is a die you are not allowed to see.

**Speaker notes:**
I did not write a grimoire. I wrote a portfolio: many small, equal-condition wagers; never the house; never the whole purse.
One model, one judge, one index is one die. Uncorrelated circuits — different hosts, different evals, different sensors — are the hedge.
When two routers or two judges price the same event differently, the spread is edge or misspecification. Index it. Do not silently pocket it.
Agent memory that stores only the recipe and not the count is a reliquary. Store \(n\) and the calibration, or admit the relic.
Hiring, buying a GPU, promoting a view, opening a later-cycle item: all wagers. Same contract.
I loved a clean count more than a pretty story. A library of pretty stories is how a house goes broke with excellent taste.

---

### K14 — `combination`
**Title:** How the two rooms are allowed to touch  
**Claim:** A primitive lands only on a named gap, without breaking genesis, without my errors, and with both parents cited.

**Speaker notes:**
Nova brought doctrine and refusals. Vetus brought rhymes, cousins, and false friends. Neither was allowed to write the ending.
Four clauses: gap already named; fit without breakage (sidecar, optional harness, ADR-090 intact); not my error; cite book and wall.
Candidate local ADRs — circuit on every score, equality as fairness, house-edge index, ruin check, remaining-work EV — open only if a row actually lands.
Orphan claims are honorable. A jewel with no gap is a brooch. A gap with no jewel is a task. A jewel glued to a wall that did not ask for it is Tartaglia’s cubic all over again.
No Plane item until a mapping graduates. No kernel. Exploratory work that promotes itself is a silent champion swap, and this house already forbids those.
If deliverable 03 says go, you recommend a later cycle and you wait for a human to confirm.

---

### K15 — `close`
**Title:** Justice, not generosity  
**Claim:** Count the circuit before you score; anything else is a polite theft.

**Speaker notes:**
I cheated, I stole a cubic, I reasoned on the mean, I named a Prince. I also wrote the general rule, and I meant it.
Equal conditions first. Whole circuit, or an honest hole. Wagers in proportion. Lean is not luck. Small stakes. Knowledge as a book, not a relic.
WeftOS already had rhymes in the walls. It did not have the count. The sidecar is how you add a count without lying about the past.
A score without a circuit is a cheat. I have been that cheat. I will not chair another.
The thesis, once more, identical: a score that does not name its circuit is a wager dressed as measurement.
Now count, or refuse the table. *Ieci alea esto iusta* — let the die, at last, be fair.
