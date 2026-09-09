# Liber de Ludo Aleae — Source Digest

**Text**: Girolamo Cardano, *Liber de Ludo Aleae* (written ~1564; ch. 20 dates a 1526 episode as "thirty-eight years have passed"). Published posthumously in *Opera Omnia* I (Lyon: Huguetan & Ravaud, 1663), pp. 262–276. ~32 short chapters, ~15 pages Latin.

**English**: Sydney Henry Gould, in Øystein Ore, *Cardano: The Gambling Scholar* (Princeton, 1953); Dover reprints 1961 / 2015 as *The Book on Games of Chance*.

**This digest is a working corpus for the symposium**, assembled from Gould/Ore excerpts, Bellhouse (2005), Gorroochurn (2012), and the 1663 chapter heads. It is **not** a complete translation. Quoted English follows Gould as cited by Ore and Gorroochurn. Latin titles from the 1663 *Opera*.

---

## 0. What the book is

A gambler's manual that accidentally founds classical probability. Cardano played "not only from time to time… but I am ashamed to say it, everyday" (*De vita propria*). He needed:

1. When a wager is *equal* (fair).
2. How to count the ways.
3. How to catch a cheat.
4. How to live with play without calling vice a science.

Bellhouse (2005): the moral frame is Aristotelian — *Nicomachean Ethics* on recreation, mean, and justice in exchange. Probability is the arithmetic of *aequitas*.

---

## 1. Chapter map (32 chapters, grouped)

Gould's running heads (Ore 1953 / Dover). Grouped for the workshops.

### I. Morality and conditions of play

| # | Title (Gould / sense) | Load-bearing claim |
|---|----------------------|--------------------|
| 1 | On kinds of games | Chance vs skill vs mixed (dice, cards, chess, backgammon). |
| 2 | Who should play and when | Not when angry, drunk, desperate, or in the wrong company. |
| 3 | The utility of play and losses | Play as recreation; losses as tuition if small. |
| 4–6 | Conditions of play; equal conditions | **Fundamental principle: equal conditions** — of the die, the table, the light, the stakes, the knowledge. |
| 7–8 | Why play can be honest | Justice in exchange: a fair game is not theft. |

**Quote (fundamental principle, Bellhouse/Gould):**

> The most fundamental principle of all in gambling is simply equal conditions, of the opponent, of the bystanders, of the money, of the situation, of the dice box, and of the die itself.

**Quote (anger):** gambling "arouses anger and disturbs the mind."

### II. Luck vs knowledge

| # | Title | Load-bearing claim |
|---|-------|--------------------|
| 9–11 | On luck in play | Luck (*fortuna*, *sors*) seems to rule. Cardano still half-believes an external "authority of the Prince." |
| | | **Detection rule**: if outcomes *systematically* lean, it is not chance — "there will be a reason and a basis for it." If they wander every wager, there is "no rational knowledge of luck." |

**Quote (Gorroochurn, Gould):**

> If anyone should throw with an outcome tending more in one direction than it should and less in another, or else it is always just equal to what it should be, then, in the case of a fair game there will be a reason and a basis for it, and it is not the play of chance; but if there are diverse results at every placing of the wagers, then some other factor is present to a greater or less extent; there is no rational knowledge of luck to be found in this, though it is necessarily luck.

**Symposium use**: this is a **calibration / bias detector**, even while Cardano still names a ghost. Gigerenzer: classical probability arrives when luck is banished. We keep the detector; we drop the ghost.

### III. The circuit (sample space) and the general rule

| # | Title | Load-bearing claim |
|---|-------|--------------------|
| 12 | On the cast of two dice / *De duorum iactu* | Enumerate 36 outcomes. Points 9 and 12 are not equal just because each has two partitions. |
| 13 | On the cast of three dice / *De trium Aleorum iactu* | 216 outcomes. 10 and 11 have 27 ways; 9 and 12 have 25. Galileo later repeats this for the Grand Duke. |
| 14 | The general rule | **Classical probability, first stated.** |

**Quote — Chapter 14, the general rule (Gould via Gorroochurn):**

> So there is one general rule, namely, that we should consider the whole circuit, and the number of those casts which represents in how many ways the favorable result can occur, and compare that number to the rest of the circuit, and according to that proportion should the mutual wagers be laid so that one may contend on equal terms.

**Modern**: circuit = sample space. Favorable = *r*, unfavorable = *s*, total = *r+s*. Odds for the event = *r*:*s*. Probability = *r*/(*r+s*). A **fair wager** matches that ratio.

This is the same idea Leibniz (1678), Bernoulli (*Ars Conjectandi*), de Moivre (*De Mensura Sortis*), and Laplace later write more cleanly.

### IV. Power rule, independence, and Cardano's own corrections

| # | Title | Load-bearing claim |
|---|-------|--------------------|
| 15 | On an error which is made about this | First he multiplies *odds*. That is "most absurd." Then he multiplies *probabilities* (or circuit sizes). |
| | | For even/odd with two dice (odds 1:1), two successive even-or-odd matches are not 1:1. Correct: multiply the circuit, subtract the favorable product. |
| later | Repeated trials | If single-trial odds for are *r*:(*t−r*), then *n* independent successes have odds *rⁿ* : (*tⁿ − rⁿ*). Equivalently *pⁿ*. |

**Error we do not import — Reasoning On The Mean (ROTM, Ore):**

If *P*(six) = 1/6, then in 3 throws "probability" = 3/6 = 1/2. False. Correct: 1 − (5/6)³ ≈ 0.421; four throws to exceed 1/2. De Méré's later confusion is the same family: 4 × 1/6 vs 24 × 1/36.

**Partial error**: he sometimes multiplies odds instead of probabilities, then catches himself, then slips again on a 3-trial example, then states the general rule correctly. Symposium rule: **multiply probabilities, never odds; never treat *np* as *P*.**

### V. Frequency (rudimentary LLN)

Last chapter / closing remarks (Ore): if an event has probability *p*, in a large number *n* of repetitions the count "does not lie far from" *m = np*.

Not Bernoulli's theorem. Enough to say: **a single win is not a method.**

### VI. Cards, tables, mixed games

Chapters on primero / fluxus, backgammon, knucklebones (*tali*), astragals, *sbaraino*, *tocadiglio*. Combinatorics of suits and face cards. Skill + chance. **Equality of conditions still binds** — marked cards, short decks, and lighting are house edge by another name.

### VII. Fraud catalog (cheating as applied statistics)

Cardano lists, as a player who used some of them:

- False / loaded / shortened dice
- Marked and palmed cards
- Tilted boards, sticky tables, bad light
- Confederates among bystanders
- Dice-box tricks (*fritillus*, *pyrgus*)

**Symposium use**: an advantage that is not in the declared circuit is **fraud or house edge**. Detect it by comparing observed frequencies to the circuit (ch. 11 + fraud chapters).

### VIII. Problem of points (also in *Practica arithmetice*, 1539)

Interrupted match. Pacioli divided by points already won (wrong). Tartaglia improved, still wrong. Cardano: divide by **points remaining**, using triangular numbers *b(b+1) : a(a+1)*. Insight correct, arithmetic not (Pascal/Fermat).

**Symposium use**: credit and delegation follow **remaining work × probability of finishing**, not sunk score.

### IX. Related: rich man vs poor man (*Practica arithmetice*)

Equal stakes, doubling after poor-man wins, stop if rich man wins once. Cardano says the rich man is disadvantaged. The EV of *money* can be zero while **ruin probabilities are not**. This is the ancestor of St. Petersburg / Kelly / bankroll.

---

## 2. Named primitives (working vocabulary)

| Cardano | Modern | WeftOS landing (draft) |
|---------|--------|------------------------|
| *circuitus* | Sample space | Every score names its circuit (outcomes counted or admitted incomplete) |
| *aequitas* / equal conditions | Fair contract | Fairness dim = equality of info, tools, stakes — not a vibe |
| *r : s* odds; wagers in proportion | Fair price / EV = 0 | Price routing, deals, promote gates as wagers |
| *scientia* vs *fortuna* | Model vs residual | Luck is not a skill claim; residual after the circuit |
| Systematic lean ≠ chance | Bias / fraud / misspecification | Calibration detector |
| Power rule *pⁿ* | Independence | Repeated agent trials; don't treat one green run as *p*=1 |
| *np* frequency | LLN | Flywheel receipts over *n*, not one eval |
| Fraud catalog | Adversary / house edge | Opportunity analysis; hidden take-rate |
| Remaining points | Continuation value | Delegation / interrupt / problem of points |
| Small stakes, right time | Ruin constraint | Anti-gambling: no treasury-scale bets on one cast |
| ROTM (error) | *np* ≠ *P* | Forbidden in scorers |

---

## 3. What we refuse to inherit

1. Luck as a supernatural "Prince."
2. Reasoning on the mean as probability.
3. Multiplying odds instead of probabilities.
4. Cardano's triangular-number split of the stake.
5. His own cheating as method. The catalog is for **detection**, not use.

---

## 4. Bibliographic anchors

- Cardano, *Opera Omnia* I (1663), pp. 262–276. [IA: imgmar3940MiscellaneaOpal](https://archive.org/details/imgmar3940MiscellaneaOpal)
- Ore, Øystein. *Cardano: The Gambling Scholar*. Princeton, 1953.
- Gould, S.H., trans. *The Book on Games of Chance*. Dover, 1961/2015.
- Bellhouse, David. "Decoding Cardano's *Liber de Ludo Aleae*." *Historia Mathematica* 32 (2005): 180–202.
- Gorroochurn, Prakash. "Some Laws and Problems of Classical Probability and How Cardano Anticipated Them." *Chance* 25, no. 4 (2012): 13–20.
- Gigerenzer et al., *The Empire of Chance* (1989), on banishing luck.
- Wikipedia / SEP: Gerolamo Cardano (life, *Ars Magna*, Inquisition, *De vita propria*).

---

## 5. WeftOS scoring inventory (for Team Tabula)

| Surface | Dims | Circuit? | Uncertainty? | Notes |
|---------|------|----------|--------------|-------|
| `EffectVector` | 5: risk, fairness, privacy, novelty, security | No | No | L2 magnitude; unweighted; genesis-locked (ADR-034) |
| `QualityScorer` | 1 scalar 0–1 | No | No | `NoopScorer`=0.5; `BasicScorer` length/error/tools |
| `NodeScoring` | 6: trust, performance, difficulty, reward, reliability, velocity | No | No | Merkle-hashed; EMA update |
| MetaHarness score | harnessFit, taskCoverage, memoryUsefulness | Partial (OIA) | Partial | Optional; receipts |
| Router / complexity | 7-factor (ruvllm) | Heuristic | No | Cost routing |
| Governance gate | Permit / Warn / Escalate / Deny | Threshold on magnitude | No | K2 landscape: "no uncertainty quantification" |

K2 C9 / D20: N-dimensional named EffectVector still deferred.
