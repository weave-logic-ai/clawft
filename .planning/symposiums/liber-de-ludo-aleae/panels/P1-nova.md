# P1 — Room Nova: Doctrine from *Liber de Ludo Aleae*

**Room**: Nova (Phase I). Book and new doctrine only.  
**Leads**: `probability-historian` + `decision-theorist` (Team Circuitus).  
**Status**: complete for Phase I. Does not visit Vetus. Does not retrofit any runtime.  
**Inputs**: `sources/chapter-digest.md`; Gould in Ore (1953); Bellhouse (2005); Gorroochurn (2012); 1663 *Opera Omnia* I, pp. 262–276.  
**Outputs**: this paper; workshop deck at `workshops/nova/index.html`.  
**Thesis (identical to symposium README)**:

> A score that does not name its **circuit** is a wager dressed as measurement.

Nova extracts a usable, named doctrine from Cardano and from the historiography that recovered him. Primitive names are locked here so Coniunctio can reuse them without drift. Errors are named so they cannot be imported as folklore.

Claims that go beyond Gould, Ore, Bellhouse, Gorroochurn, or the 1663 chapter heads are marked **[INFERRED]**.

---

## 0. What this room is doing

Gerolamo Cardano (1501–1576) wrote a gambler’s manual that accidentally states classical probability. He needed four practical things:

1. When a wager is *equal* (fair).
2. How to count the ways.
3. How to catch a cheat.
4. How to live with play without calling vice a science.

Nova’s job is to keep the questions and the primitives, correct the arithmetic he himself sometimes botched, and drop the ghost he still named *Prince*. The result is a decision kit: circuit, equality, odds, power, frequency, fraud, remaining-work, small stakes. It is not a crate API and not a score inventory.

---

## 1. The book Cardano wrote

### 1.1 Composition and publication

| Fact | Source |
|------|--------|
| Latin title | *Liber de Ludo Aleae* |
| English | *The Book on Games of Chance* (Gould) |
| Composition | Late life, ~1564. Chapter 20 dates a 1526 episode with the clause “thirty-eight years have passed” (digest; Ore 1953). Scattered notes may begin as early as the mid-1520s (Ekert 2008, citing the Padua years). |
| Publication | Posthumous. *Hieronymi Cardani Mediolanensis Opera Omnia*, tomus I (Lyon: Huguetan & Ravaud, 1663), pp. 262–276. |
| Scale | ~32 short chapters, ~15 pages of Latin (Gorroochurn 2012; Bellhouse 2005). |
| English text | Sydney Henry Gould, printed as part of Øystein Ore, *Cardano: The Gambling Scholar* (Princeton, 1953); reissued Dover 1961 / 2015 as *The Book on Games of Chance*. |

Cardano did not list the *Liber* among his mathematical works. Bellhouse’s central claim is that the book is a Renaissance humanist text on the *morality* of play, written in the intellectual frame of Aristotle’s *Nicomachean Ethics*. Probability, when it appears, is the arithmetic of *aequitas* — justice in exchange — not a new branch of *mathesis* seeking a name (Bellhouse 2005, *Historia Mathematica* 32: 180–202).

He played. In *De vita propria liber* he writes that he gambled “not only from time to time… but I am ashamed to say it, everyday” (Gould/Ore via Gorroochurn 2012). The medical epigraph Ore puts on the treatise is the right tone:

> Even if gambling were altogether an evil, still, on account of the very large number of people who play, it would seem to be a natural evil. For that very reason it ought to be discussed by a medical doctor like one of the incurable diseases. (Cardano, as given by Ore 1953, front matter)

### 1.2 Historiographic position

Three readings must be held at once.

**Ore (1953).** The *Liber* is so extensive, and in certain questions so successful, that “it would seem much more just to date the beginnings of probability theory from Cardano’s treatise rather than the customary reckoning from Pascal’s discussions with his gambling friend de Méré.” Ore is a partisan, and usefully so: he is the first modern reader to translate the combinatorics into symbols we can check.

**Edwards (quoted by Gorroochurn 2012).** “Before Pascal and Fermat no more had been achieved than the enumeration of the fundamental probability set in various games with dice or cards.” This is too small. Chapter 14 is not an enumeration. It is a *rule for laying wagers* from an enumeration.

**Gigerenzer et al. (1989), *The Empire of Chance*.** Classical probability arrives when luck is banished — when variable events can be treated as expressions of stable underlying proportions. Cardano almost makes that step and then names a leftover supernatural “authority of the Prince.” We keep his detector (systematic lean is not chance). We drop the Prince.

Nova’s position: Cardano *anticipated* the classical definition, the multiplication rule for independent trials, a rudimentary law of large numbers, the right *question* in the problem of points, and the ethical frame that makes a fair price a matter of justice rather than generosity. He did not found the mature theory. Pascal, Fermat, Huygens, Jacob Bernoulli, de Moivre, and Laplace wrote more cleanly, and they did not have his book (1663 is after Huygens 1657 and after the 1654 letters). What they still owed him is listed in §8.

### 1.3 How to read 32 short chapters

Gould’s running heads (Ore 1953 / Dover) are the English of the 1663 *capita*. The table below groups them as the symposium digest does. Individual Gould titles that are confirmed in Ore/Gorroochurn/Google-Books contents are marked **Gould**. Latin *capita* confirmed on the 1663 plates (as reproduced by Gorroochurn fig. 3) are marked **1663**. Titles reconstructed from the digest’s grouping, Bellhouse’s chapter citations, or the subject of the *caput* are marked **[INFERRED]**.

| Block | ## | Title / sense | Load-bearing claim |
|-------|----|---------------|--------------------|
| I. Morality | 1 | On kinds of games **Gould** | Chance vs skill vs mixed (dice, cards, chess, backgammon). |
| | 2 | Who should play and when **Gould** | Not when angry, drunk, desperate, or in the wrong company. Play “arouses anger and disturbs the mind.” |
| | 3 | The utility of play and losses **Gould** | Recreation; small losses as tuition. |
| | 4–6 | Conditions of play; equal conditions **Gould** (one head is “On conditions of play”) | **Fundamental principle.** Equal conditions of opponent, bystanders, money, situation, dice-box, die. |
| | 7–8 | Why play can be honest **[INFERRED]** | Justice in exchange: a fair game is not theft (Bellhouse / *EN* V). |
| II. Luck | 9–11 | On luck in play **Gould** (Gorroochurn cites the title) | *Fortuna* / *sors* seems to rule. Cardano still half-believes an external “authority of the Prince.” **Detection rule** (quoted §2.2). |
| III. Circuit | 12 | On the cast of two dice / *De duorum iactu* **Gould / 1663** | Enumerate 36 outcomes. Partitions are not equiprobable. |
| | 13 | On the cast of three dice / *De trium Aleorum iactu* **Gould / 1663** | 216 outcomes. 10 and 11 have 27 ways; 9 and 12 have 25. Galileo later repeats this for the Grand Duke. |
| | 14 | The general rule **Gould** (Gorroochurn) | **Classical probability, first stated.** Whole circuit; favorable to the rest; wagers in that proportion. |
| IV. Power | 15 | On an error which is made about this **Gould** | First he multiplies *odds* (“most absurd”). Then he multiplies probabilities (circuit sizes). Later he states *pⁿ* correctly, and slips again on a 3-trial example. |
| | later | Repeated trials **Gould** (closing combinatorics) | If single-trial odds for are *r* : (*t − r*), then *n* independent successes have odds *rⁿ* : (*tⁿ − rⁿ*). |
| V. Frequency | last / closing | Rudimentary LLN **Ore** | If the event has probability *p*, in a large number *n* of repetitions the count “does not lie far from” *m = np*. |
| VI. Mixed games | mid–late | Primero / *fluxus*, backgammon, *tali*, astragals, *sbaraino*, *tocadiglio* **Gould** (word-cloud + digest) | Combinatorics of suits and face cards. Skill + chance. Equality of conditions still binds. |
| VII. Fraud | several | False dice, marked cards, light, confederates, *fritillus* / *pyrgus* **Gould** | An advantage not in the declared circuit is fraud or house edge. Cardano used some of these. The catalog is for detection. |
| VIII. Tali | 32 | Throw of the *tali* **Bellhouse** | Bellhouse notes that in ch. 32 Cardano counts chances on knucklebones rather than computing a probability. The circuit habit survives even when the objects are not cubes. |

A complete 1663 running-head transcription of all 32 Latin titles is on the Lyon plates (pp. 262–276) and in Gould. This sitting does not pretend to have re-keyed every *caput* from the facsimile. The load-bearing claims above are the ones the doctrine needs; they are the ones Gould, Ore, Bellhouse, and Gorroochurn actually quote.

---

## 2. What Cardano said — the load-bearing passages

### 2.1 *Aequitas*: equal conditions as first principle

Bellhouse’s reading, and Gould’s sentence:

> The most fundamental principle of all in gambling is simply equal conditions, of the opponent, of the bystanders, of the money, of the situation, of the dice box, and of the die itself.

Ekert (2008) gives the same Gould sentence with the clause that makes the ethics explicit:

> To the extent to which you depart from that equality, if it is in your opponent’s favour, you are a fool, and if in your own, you are unjust.

This is not a vibe. It is a contract:

| Condition | What must be equal | What a departure is |
|-----------|--------------------|---------------------|
| Opponent | Knowledge, skill admitted, sobriety | A bake-off that hands one side the answers |
| Bystanders | No confederate | A silent partner in the gallery |
| Money | Stakes, solvency, the unit of account | Doubling into ruin; unequal bankrolls treated as equal risk |
| Situation | Light, time, anger, haste | Play while disturbed (ch. 2) |
| Dice-box | *Fritillus* / *pyrgus* not gimmicked | A box that prefers a face |
| Die | Geometry, faces, weight | Loaded, shortened, favoured side |

Cardano himself notes that “every die, even if it is acceptable, has its favoured side” (Gould via Ekert 2008). Equality is an ideal the circuit *assumes* and the fraud catalog *polices*. Probability is how you check the principle (Bellhouse: the arithmetic of *aequitas*).

Aristotle in the background (*Nicomachean Ethics* II on the mean; V on justice in exchange): recreation sits between idleness and vice; a fair exchange is an arithmetic proportion. **[INFERRED from Bellhouse 2005, not from a fresh reading of the Greek.]** Cardano’s “small losses as tuition” (ch. 3) is the mean applied to the purse.

### 2.2 *Scientia* versus *fortuna*

Gorroochurn (2012) quotes the luck chapter in Gould:

> In these matters, luck seems to play a very great role, so that some meet with unexpected success while others fail in what they might expect…

And the detection rule (digest; Gorroochurn 2012):

> If anyone should throw with an outcome tending more in one direction than it should and less in another, or else it is always just equal to what it should be, then, in the case of a fair game there will be a reason and a basis for it, and it is not the play of chance; but if there are diverse results at every placing of the wagers, then some other factor is present to a greater or less extent; there is no rational knowledge of luck to be found in this, though it is necessarily luck.

Two claims live in one paragraph.

1. **The detector (keep).** A *systematic* lean against the circuit is not chance. It has a reason: bias, fraud, misspecification. A wander that refuses to settle is not knowledge of luck either — it is residual. Modern names: calibration test, bias detector, specification check.
2. **The ghost (drop).** Cardano still assigns the residual to an external force, “the authority of the Prince.” Gigerenzer: he thereby relinquishes a claim to founding the mathematical theory. Classical probability requires a climate in which variable events are expressions of stable proportions, at least in the long run.

Nova’s cut: *scientia* is the named circuit plus the wager laid in proportion. *Fortuna* is whatever remains after the circuit is honest. Luck is not a skill claim. A single win is not a method.

### 2.3 *Circuitus*: two dice, three dice, the general rule

**Two dice (ch. 12).** The circuit has 36 equally likely ordered pairs. Integer *partitions* are not outcomes. The point 9 is 3+6 and 4+5 (two partitions, four ordered pairs). The point 10 is 4+6 and 5+5 (two partitions, three ordered pairs). They are not equal. The digest’s shorthand “points 9 and 12 are not equal just because each has two partitions” is the same lesson; the clean two-dice pairing in Gould/Gorroochurn is 9 against 10 (4 ways against 3). **[INFERRED: the digest’s “9 and 12” is the three-dice pairing; we do not force it onto ch. 12.]**

**Three dice (ch. 13).** The circuit has 216 ordered triples. The Grand Duke’s later question to Galileo — why are 10 and 11 more frequent than 9 and 12, when each has six partitions? — is already solved here, almost a century earlier (Gorroochurn 2012; Galileo, *Sopra le Scoperte dei Dadi*). Cardano’s own plate (Gorroochurn fig. 3; 1663, *Caput XIII*) prints the last rows 9, 12, 25 and 10, 11, 27.

Partitions and permutations (Gorroochurn, Table 1):

```
 score 12          score 11          score 10          score  9
 6-5-1  ×6         6-4-1  ×6         6-3-1  ×6         6-2-1  ×6
 6-4-2  ×6         6-3-2  ×6         6-2-2  ×3         5-3-1  ×6
 6-3-3  ×3         5-5-1  ×3         5-4-1  ×6         5-2-2  ×3
 5-5-2  ×3         5-4-2  ×6         5-3-2  ×6         4-4-1  ×3
 5-4-3  ×6         5-3-3  ×3         4-4-2  ×3         4-3-2  ×6
 4-4-4  ×1         4-4-3  ×3         4-3-3  ×3         3-3-3  ×1
 ──────────        ──────────        ──────────        ──────────
        25                27                27                25
```

Rule of the permutations: three distinct faces → 3! = 6; a pair and a singleton → 3; a triple → 1. Anyone who counts partitions as if they were casts is offering even money on an uneven circuit. That is the first house edge that looks like arithmetic.

**Chapter 14, the general rule** (Gould via Gorroochurn 2012; digest):

> So there is one general rule, namely, that we should consider the whole circuit, and the number of those casts which represents in how many ways the favorable result can occur, and compare that number to the rest of the circuit, and according to that proportion should the mutual wagers be laid so that one may contend on equal terms.

Let the circuit have *t* equally likely casts, of which *r* are favorable and *s = t − r* are not. Then:

| Cardano | Symbol | Modern |
|---------|--------|--------|
| Whole circuit | *t = r + s* | Sample space, \|Ω\| |
| Favorable casts | *r* | \|A\| |
| The rest of the circuit | *s* | \|Aᶜ\| |
| Mutual wagers in proportion | *r : s* | Fair odds for *A* |
| Contention on equal terms | EV = 0 | Fair contract |

Probability in the classical sense is *p = r / (r + s)*. Cardano does not use the word *probabilitas* as a technical term (Ekert 2008, n. 7: the modern word, detached from gambling, is Jacob Bernoulli’s). He uses *circuitus*, *aequitas*, and the proportion of wagers. That is enough.

Gorroochurn lines this sentence up with Leibniz (1678), Jacob Bernoulli (*Ars Conjectandi*, 1713), de Moivre (*De Mensura Sortis*, 1711), and Laplace (1774 / *Essai philosophique*). The later definitions are cleaner. They are the same idea.

### 2.4 Power rule, and Cardano catching himself

Gorroochurn reconstructs “Cardano’s formula.” One trial: *t* equally likely outcomes, *r* favorable, odds *r : (t − r)*. Then *n* independent successes have odds

\[
r^{n} : (t^{n} - r^{n})
\]

which is *pⁿ* written as a wager. This is the multiplication rule for independent events.

He did not arrive there on the first try.

**Error (multiply odds).** For an event with odds 1 : 1, multiplying the odds still yields 1 : 1 after two, three, four trials. Chapter 15, “On an Error Which Is Made About This,” calls this “most absurd” (Gould via Gorroochurn):

> But this reasoning seems to be false, even in the case of equality, as, for example, the chance of getting one of any three chosen faces in one cast of one die is equal to the chance of getting one of the other three, but according to this reasoning there would be an even chance of getting a chosen face each time in two casts, and thus in three, and four, which is most absurd. For if a player with two dice can with equal chances throw an even and an odd number, it does not follow that he can with equal fortune throw an even number in each of three successive casts.

**Correction (multiply the circuit).** For even/odd, *p* = ½:

> Therefore, in comparisons where the probability is one half, as of even faces with odd, we shall multiply the number of casts by itself and subtract one from the product, and the proportion which the remainder bears to unity will be the proportion of the wagers to be staked. Thus, in successive casts we shall multiply 2 by itself, which will be 4; we shall subtract 1; the remainder is 3; therefore a player will rightly wager 3 against 1… Thus he loses three times and wins once.

Circuit 4, favorable 1, odds 1 : 3. Correct.

**Slip, then the general rule.** Gorroochurn notes that in the next sentence Cardano mis-handles a 3-trial example with non-even odds, then later in the book states the rule correctly for ace-or-deuce (*r* = 2, *t* = 6):

- two trials: 2² : (6² − 2²) = 4 : 32 = 1 : 8
- three trials: 2³ : (6³ − 2³) = 8 : 208 = 1 : 26

Both are *pⁿ*. Symposium rule, taken from his correction and not from his slip: **multiply probabilities, never odds.**

Independence is an assumption, not a gift of the table. The power rule is legal only when the next cast does not remember the last. **[INFERRED as a modern constraint; Cardano treats successive casts as independent once the die and the box are honest.]**

### 2.5 *np* frequency, and the error Ore named ROTM

**Rudimentary LLN (Ore 1953, as quoted by Gorroochurn 2012):**

> It is clear … that he [Cardano] is aware of the so-called law of large numbers in its most rudimentary form. Cardano’s mathematics belongs to the period antedating the expression by means of formulas, so that he is not able to express the law explicitly in this way, but he uses it as follows: when the probability for an event is *p* then by a large number *n* of repetitions the number of times it will occur does not lie far from the value *m = np*.

This is not Bernoulli’s theorem. It is enough to say: a single win is not a method; receipts are taken over *n*.

**Reasoning on the mean (ROTM).** Ore’s name (1953, p. 150) for a second habit Cardano does *not* fully kill. If an event has probability *p* on one trial, ROTM treats *np* as if it were the probability that the event occurs at least once in *n* trials.

| Question | ROTM | Truth |
|----------|------|-------|
| At least one six in 3 throws of one die | 3 × 1/6 = 1/2 | 1 − (5/6)³ = 91/216 ≈ 0.421. Four throws to exceed 1/2: 1 − (5/6)⁴ ≈ 0.518. Cardano himself, once he counts, finds 91 rather than 108 (Williams 2005; Gorroochurn 2012). |
| At least one double-six in *n* throws of two dice | *n*/36 = 1/2 ⇒ *n* = 18 | 1 − (35/36)ⁿ > 1/2 ⇒ *n* ≥ 25. Cardano’s ROTM answer is 18 (Gorroochurn 2012). |
| De Méré, a century later | 4 × 1/6 ≟ 24 × 1/36 | 1 − (5/6)⁴ ≈ 0.518 versus 1 − (35/36)²⁴ ≈ 0.491. Same family of error. |

ROTM is the error we refuse. Frequency *np* is a *count we expect*. It is not *P*(at least one).

### 2.6 Fraud catalog

Cardano lists, as a player who used some of them (digest; Gould word-cloud: *fritillus*, *pyrgus*, marked cards, deceive):

- False, loaded, or shortened dice; a die that “has its favoured side.”
- Marked and palmed cards; short decks.
- Tilted boards, sticky tables, bad light.
- Confederates among bystanders.
- Dice-box tricks (*fritillus*, *pyrgus*).

**Symposium use.** An advantage that is not in the declared circuit is **fraud or house edge**. Detect it by comparing observed frequencies to the circuit (luck chapter + fraud chapters). The catalog is for detection, not use. His own cheating is not a method we inherit.

### 2.7 Remaining-work: the problem of points

The interrupted match is older than Pascal. It is in Italian manuscripts of the fourteenth century and in print in Pacioli’s *Summa de arithmetica* (1494). Two players agree to play until one has won *N* games. They stop early. How is the stake divided?

| Author | Rule | Worked case: play to 6, stop at 5–3. A needs *a* = 1, B needs *b* = 3. |
|--------|------|------------------------------------------------------------------------|
| Pacioli 1494 | Divide by points *already won* | 5 : 3. Pays sunk score. |
| Tartaglia | Improved, still wrong | (digest). |
| Cardano, *Practica arithmetice* (1539) | Divide by points *remaining*, using triangular numbers | *b(b+1) : a(a+1)* = 3·4 : 1·2 = 12 : 2 = **6 : 1**. |
| Fermat 1654 | Enumerate remaining equally likely sequences of length *a+b−1* | 8 sequences; A loses only on BBB; **7 : 1**. |
| Pascal 1654 | Arithmetic triangle, row *a+b−1* | Sum of the first *b* entries : sum of the last *a*. Same **7 : 1**. |

Cardano’s *insight* is the primitive: **credit follows remaining work, not sunk score.** His *arithmetic* is the triangular-number shortcut, and it is wrong. 6 : 1 is not 7 : 1. We keep the question. We refuse the ratio.

(The *Liber* itself treats a related “Problem of Dice” with ROTM, as Gorroochurn notes; the clean remaining-points statement is in the 1539 *Practica*. Nova treats both as Cardano’s.)

### 2.8 Small stakes, and the rich man against the poor man

Chapters 2–3: play at the right time, for small stakes, as recreation. Losses may teach; they must not ruin.

A sharper ancestor sits in *Practica arithmetice* (Gorroochurn 2012). A rich man and a poor man play for equal stakes. If the poor man wins, stakes double the next day and they continue. If the rich man wins once, play ends.

Let the unit stake be 1 and the coin be fair. If the rich man ever wins, his net is +1 (the doubling refunds prior losses plus one). If the poor man wins *n* times running, the rich man is down 1 + 2 + … + 2ⁿ⁻¹ = 2ⁿ − 1. Expected money is zero:

\[
\mathbb{E}[\text{rich}] = (1 - 2^{-n})\cdot(+1) + 2^{-n}\cdot(-(2^{n}-1)) = 0.
\]

Cardano says the rich man is at a great disadvantage. Gorroochurn notes that the *probability* the rich man ends the series is 1 − 2⁻ⁿ, which *rises* with *n* — so if “advantage” means “who is more likely to pocket the +1,” Cardano’s qualitative claim is backwards. If “advantage” means “who bears a left-tail that can swallow a treasury,” Cardano is pointing at the right object: **equal stakes do not imply equal ruin.** The payoff is a lottery ticket sold by the rich man. EV of money can be zero while ruin probabilities, and utilities, are not.

This is the ancestor of St. Petersburg (N. Bernoulli 1713 / D. Bernoulli 1738), of gambler’s ruin (Huygens and after), and of bankroll constraints. Nova keeps the constraint: small stakes; never the treasury on one cast.

---

## 3. Named primitives

These names are the Room Nova vocabulary. Coniunctio must not rename them without a recorded reason.

| Primitive | Cardano’s language | Modern name | Operational content |
|-----------|-------------------|-------------|---------------------|
| **circuitus** | *circuitus*, “the whole circuit” (ch. 14) | Sample space Ω | Every claim names the outcomes it counted. If the circuit is incomplete, say so. |
| **aequitas** | equal conditions (fundamental principle) | Fair contract | Equality of information, tools, stakes, situation, box, die — not a mood. |
| **odds *r* : *s*** | mutual wagers in proportion | Fair price; EV = 0 | Stake *s* to win *r*, or the inverse, so that the contract has expectation zero. |
| **scientia vs fortuna** | knowledge vs luck / *sors* / Prince | Model vs residual | Luck is not a skill claim. Residual is what remains after an honest circuit. |
| **systematic lean** | “tending more in one direction than it should” | Calibration / bias / fraud | Persistent departure from the circuit has a reason. It is not chance. |
| **power rule** | *rⁿ* : (*tⁿ − rⁿ*) | Independence, *pⁿ* | Repeated trials multiply *probabilities*. Independence must be stated. |
| ***np* frequency** | count “does not lie far from” *np* (Ore) | Rudimentary LLN | Receipts over *n*. A single win is not a method. |
| **fraud catalog** | loaded dice, marked cards, light, confederates, *fritillus* | Adversary / house edge | Advantage not in the declared circuit is priced or refused. Detect; do not use. |
| **remaining-work** | points yet to win (*Practica*; problem of points) | Continuation value | Pay what is left to do, weighted by the chance of finishing. Do not pay sunk score. |
| **small stakes** | recreation; right time; not desperate | Ruin constraint | Equal stakes ≠ equal ruin. Never a treasury-scale bet on one cast. |

**Anti-primitive (named so it cannot sneak back):**

| Error | Cardano’s use | Ban |
|-------|---------------|-----|
| **ROTM** | *np* treated as *P*(at least one) | Forbidden in any scorer, any promo gate, any “we are halfway there.” |

---

## 4. Errors we refuse

Five refusals. Four are Cardano’s. One is the misuse of Cardano.

### 4.1 Luck as a supernatural Prince

He names an “authority of the Prince” for the residual. We do not. Residual after an honest circuit is *fortuna* in the thin sense: not yet knowledge. It is not a person, not a blessing, and not a line you get to write on a skill evaluation.

### 4.2 Reasoning on the mean

*np* is an expected *count*. It is not a probability. 3 × 1/6 is not *P*(at least one six in three throws). 18 × 1/36 is not *P*(at least one double-six in eighteen). De Méré’s 4-versus-24 confusion is the same family. If a later room writes a “coverage” score as a sum of marginals and reads it as a chance of success, that room has imported ROTM.

### 4.3 Multiplying odds instead of probabilities

Odds 1 : 1, multiplied, stay 1 : 1. Cardano called this “most absurd” and then did it again on a 3-trial page. We take his correction, not his slip. Conversion is mandatory: odds *r* : *s* → *p* = *r*/(*r*+*s*) → combine → convert back if a wager must be laid.

### 4.4 The triangular-number split of an interrupted stake

Remaining-work is the right object. *b(b+1) : a(a+1)* is the wrong number. Pacioli’s sunk-score split is worse. The Fermat–Pascal enumeration (or the Pascal triangle, or the modern binomial) is the arithmetic we use when a division of an interrupted pot is actually required.

### 4.5 His cheating as method

He catalogued fraud because he practiced it. The symposium inherits the *catalog*, for detection. It does not inherit the practice. A hidden take-rate is either priced in the open or the table is refused.

---

## 5. From circuit to decision theory

Cardano almost had a modern kit. The missing pieces have names. This section is the decision-theorist’s half. It does not invent a scoring API.

### 5.1 Circuit → sample space; incomplete circuit → Knightian

Frank Knight, *Risk, Uncertainty, and Profit* (1921): **risk** is the case where the circuit can be named and the chances known; **uncertainty** is the case where they cannot. Cardano computes only when he can count. When he cannot, he reaches for *fortuna*. Nova’s translation: if you cannot enumerate (or honestly sample) the circuit, you do not have a probability. You have a prior, a vibe, or a story. **Do not pretend a prior is a circuit.** Label the claim *incomplete circuit* and either enlarge the count, buy information, or refuse the wager.

Savage (1954) and de Finetti give a personal-probability reading in which any coherent willingness-to-bet *is* a probability. That reading is available, and Cardano’s “wagers in proportion” is already a Dutch-book instinct: mis-price *r* : *s* and someone has an edge. Nova still insists on the *count* when a count is possible. Subjective coherence is what you use when the circuit is admitted incomplete — and you say so.

### 5.2 Odds *r* : *s* → fair price; EV = 0; utility ≠ money

A contract that pays 1 on *A* and 0 otherwise, bought for price *q*, has expectation *p − q* if *p* = *r*/(*r*+*s*). Fairness is *q = p*, i.e. stakes in the ratio *r* : *s*. EV = 0 is justice, not generosity (Bellhouse’s Aristotelian frame).

Daniel Bernoulli (1738) on St. Petersburg: the expectation of *money* is not the expectation of *use*. Cardano’s rich-man game is the same lesson in a cheaper costume. A system that prices every decision in dollars and ignores bankroll will happily sell the treasury for a +1 with a left tail of 2ⁿ − 1.

### 5.3 Systematic lean → calibration

Modern names: reliability diagram, Brier score, Hosmer–Lemeshow, Spiegelhalter. Cardano’s version needs no machinery: compare the observed frequency to the circuit. If it leans, and keeps leaning, it is not chance.

Caveats he did not write and we must: sample size; multiple comparisons; optional stopping. A wander of twenty casts is not a verdict. A lean that survives *n* large enough that *np* and *n(1−p)* are both comfortable is a verdict. **[INFERRED: the numerical hygiene is ours; the detector is his.]**

### 5.4 Power rule → independence, stated

*pⁿ* is legal when trials are independent and identically distributed. Agents, evals, and “another run of the same prompt” are usually not. Shared weights, shared data, shared judge, shared table — the next cast remembers the last. Nova’s constraint: **the power rule carries an independence receipt.** If you cannot write that receipt, you may not raise *p* to *n*.

### 5.5 *np* → frequency / receipts; ROTM forbidden

Bernoulli’s golden theorem (1713) is the grown form of Ore’s sentence. Use *np* as a planned count of receipts, never as a probability of eventual success. Coverage bars that sum marginals and call the sum a chance are ROTM.

### 5.6 Remaining points → continuation value

Sunk-cost fallacy, option value, expected remaining work. Pacioli pays the past. Cardano asks the future and then mis-counts it. The modern object is

\[
V = \sum_{\omega \in \Omega_{\text{remaining}}} p(\omega)\, u(\text{payoff}(\omega)).
\]

Delegation, interruption, and “how much of the pot does this worker hold” are problem-of-points problems. They are not scoreboard problems.

### 5.7 Small stakes → Kelly / ruin

Kelly (1956): if the edge is *b* and the chance is *p*, the log-optimal fraction of bankroll is *f\* = (bp − q)/b*, and *f\* = 0* when EV ≤ 0. Cardano has no logarithm. He has the qualitative rule: small stakes; not when desperate; recreation, not livelihood.

Huygens and later gambler’s-ruin calculations make the other half precise: two players, equal stakes, unequal fortunes — the poorer is ruined with probability proportional to the other’s fortune. **Equal stakes ≠ equal ruin.** A decision kit that reports EV and not ruin is Cardano’s rich man congratulating himself on a zero-mean ticket.

### 5.8 Fraud → misspecification + adversary

House edge is a take-rate not written in the declared circuit. It can be a loaded die, a marked card, a tilted board, a confederate — or a silent vig, a selected eval, a judge who has seen the answer. Detection is ch. 11 plus the fraud catalog: compare frequencies to the circuit; inspect the box. Response is binary: **price it in the open, or refuse the table.** Using the catalog as a playbook is the fifth error.

### 5.9 Map (one page)

```
  Cardano                         Decision theory                 Watch-out
  ----------------                -------------------------       -------------------------
  circuitus                       sample space Ω                  a prior is not a circuit
  incomplete count                Knightian uncertainty           say so; refuse or enlarge
  odds r : s                      fair price; EV = 0              utility ≠ money
  systematic lean                 calibration / bias test         n; multiple comparisons
  power rule p^n                  independence assumption         agents are not i.i.d.
  np frequency                    LLN / receipts                  ROTM: np is not P
  remaining points                continuation value              do not pay sunk score
  small stakes                    Kelly / ruin probability        equal stakes ≠ equal ruin
  fraud catalog                   misspec + adversary             price the edge or walk
```

---

## 6. Decision checklist

One page. When to bet, when to refuse, when to ask for a larger circuit. Written so a cold reader can run it against any proposed wager, score, or promotion.

**Before the cast**

1. **Name the circuit.** Write the outcomes you are counting, or write *incomplete circuit*. If you cannot do either, you do not have a score. You have a costume.
2. **Check *aequitas*.** Opponent, bystanders, money, situation, box, die — or their modern analogues: tools, context, hidden information, evaluation set, lighting of the test. A departure in their favour: you are a fool to sit. A departure in yours: you are unjust to sit.
3. **Convert to *r* : *s*.** Favorable, unfavorable, total. Price the contract so EV = 0 at those odds. If the offered price differs, that difference is *edge*. Name it.
4. **Size the stake to ruin, not to EV.** Small stakes. A positive EV with a left tail that can swallow the bankroll is the rich man’s ticket. If you cannot state a ruin bound, you have not finished the checklist.
5. **State independence, or do not raise *p* to *n*.** Shared state kills the power rule.

**During / after**

6. **Pay remaining work, not sunk score.** An interrupted task is divided by what is left and the chance of finishing, not by points already on the board.
7. **Compare frequency to the circuit.** A systematic lean is not luck. Recalibrate, inspect the box, or stand up. A wander at small *n* is not yet a verdict.
8. **Do not read *np* as a probability.** Planned receipts are counts. “At least one success” is 1 − (1 − *p*)ⁿ, and only under independence.
9. **Treat a hidden take-rate as fraud or house edge.** Price it or refuse. Do not file it under *fortuna*.
10. **Refuse** if any of: no circuit and no admission of incompleteness; unequal conditions you will not name; a stake that can ruin; luck claimed as skill; a single green run offered as *p* = 1.

**The three doors**

| Door | When | Act |
|------|------|-----|
| **Bet** | Circuit named, conditions equal (or the edge priced), stake inside the ruin bound, independence honest | Lay *r* : *s*. Record *n*. |
| **Refuse** | Unequal conditions, hidden take-rate, ruin-scale stake, luck-as-skill, ROTM dressed as a score | Walk. Say why. |
| **Enlarge** | Incomplete circuit, *n* too small to calibrate, independence in doubt | Buy more count. Do not fill the hole with a Prince. |

---

## 7. ASCII diagrams

Drawn for later `ascii-to-svg`. Do not restyle the primitive names.

### 7.1 Circuitus — two honest dice

```
                    CIRCUITUS  t = 36
                    ordered pairs (d1, d2)

              1     2     3     4     5     6
           .-----------------------------------.
         1 |  2  |  3  |  4  |  5  |  6  |  7  |
           |-----+-----+-----+-----+-----+-----|
         2 |  3  |  4  |  5  |  6  |  7  |  8  |
           |-----+-----+-----+-----+-----+-----|
         3 |  4  |  5  |  6  |  7  |  8  |  9  |
           |-----+-----+-----+-----+-----+-----|
         4 |  5  |  6  |  7  |  8  |  9  | 10  |
           |-----+-----+-----+-----+-----+-----|
         5 |  6  |  7  |  8  |  9  | 10  | 11  |
           |-----+-----+-----+-----+-----+-----|
         6 |  7  |  8  |  9  | 10  | 11  | 12  |
           '-----------------------------------'

         point  9 = (3,6)(4,5)(5,4)(6,3)           r = 4
         point 10 = (4,6)(5,5)(6,4)                 r = 3
         two partitions each; not the same wager.
         fair odds for 9:   4 : 32
         fair odds for 10:  3 : 33
```

### 7.2 Three dice — 25 / 27 / 27 / 25

```
   THREE DICE   t = 216
   partitions are not casts; permutations are.

   9   ############ 25 / 216     p = 25/216 ≈ 0.116
  10   ############# 27 / 216    p = 27/216 = 0.125
  11   ############# 27 / 216    p = 27/216 = 0.125
  12   ############ 25 / 216     p = 25/216 ≈ 0.116

   even money on "10 versus 9" is a fool's seat.
   Galileo (Sopra le Scoperte dei Dadi) is later.
   Cardano, Caput XIII, already has 25 and 27.

         3 distinct  ->  6 casts
         a pair      ->  3 casts
         a triple    ->  1 cast
```

### 7.3 Odds bar — *r* : *s*

```
   circuit  t = r + s
   |<-------------- r -------------->|<------------- s ------------->|
   |########### FAVORABLE ###########|........... UNFAVORABLE .......|
   0                                 r                               t

   odds for  A   =  r : s
   odds against  =  s : r
   p(A)          =  r / t
   fair stake    :  lay s to win r   (or lay r to win s, other side)

   NEVER multiply the odds.
   convert to p, multiply, convert back.
```

### 7.4 EV tree — one cast, then stop

```
                      stake 1, offered even money
                      true circuit  r : s
                              |
              .---------------+---------------.
              | p = r/(r+s)                   | q = s/(r+s)
           [ A hits ]                      [ A misses ]
           +1  (net)                       -1  (net)
              |                               |
              '---------------+---------------'
                              |
                     EV = (r - s) / (r + s)

   EV = 0  iff  r = s          aequitas
   EV > 0  iff  r > s          edge in your favour (unjust if hidden)
   EV < 0  iff  r < s          you are the fool

   even money on three-dice "10 vs 9":
     p(10|10 or 9) = 27/52
     EV per unit  = (27 - 25)/52 = +1/26   for the 10-side
```

### 7.5 Ruin versus EV — rich man, poor man

```
   equal stakes, double after poor-man wins, stop when rich-man wins once
   coin fair.  EV of money = 0.  Ruin is not.

   path                  P           rich net      note
   R                     1/2         +1            stop
   P then R              1/4         +1            stop
   PP then R             1/8         +1            stop
   ...
   P^n                   2^{-n}      -(2^n - 1)    treasury event

   +1 +1 +1 +1 +1 +1 +1 +1 +1                    -(2^n - 1)
   |-------- frequent, small --------------------|---- rare, lethal ----|

   Kelly / small-stakes reading:
     do not sell this ticket with the treasury as the left tail.
     equal stakes ≠ equal ruin.
```

### 7.6 Remaining-work, not sunk score

```
   play to 6.  stop at 5 -- 3.
   Pacioli pays the past:     5 : 3
   Cardano pays the future,
     wrong triangle:          b(b+1) : a(a+1) = 6 : 1
   Fermat / Pascal pay the
     remaining circuit:       7 : 1

                 remaining games (length 3)
                 AAA AAB ABA BAA ABB BAB BBA BBB
                 A   A   A   A   A   A   A   B
                 |<---------- A, 7 ---------->|<>|

   primitive we keep:  remaining work × P(finish)
   ratio we refuse:    Cardano's 6 : 1
```

---

## 8. What Pascal, Fermat, Huygens, and Bernoulli still owed him

They did not have the book. Huygens, *De ratiociniis in ludo aleae* (1657), is the first *published* treatise; the *Liber* is 1663. The 1654 letters solve the problem of points correctly and launch the mature theory. Honesty about chronology is not denigration.

What the later theory still owed the questions in this book:

| Later result | Cardano already had | What he lacked |
|--------------|---------------------|----------------|
| Classical definition (Leibniz, Bernoulli, de Moivre, Laplace) | Ch. 14: whole circuit, favorable to the rest, wagers in proportion | The word; the axiomatic hygiene; the will to banish the Prince |
| Equiprobable enumeration | 36 and 216; 25/27/27/25; partitions ≠ permutations | Galileo’s later polish; a notation for |Ω| |
| Multiplication of independent trials | Power rule *rⁿ* : (*tⁿ − rⁿ*), after he corrected himself | A clean algebra; a stated independence axiom |
| Law of large numbers | *np* “does not lie far from” the count (Ore) | Bernoulli’s proof; a distinction between count and probability (he still commits ROTM) |
| Problem of points | Remaining work, not sunk score | The binomial / Fermat enumeration / Pascal triangle applied to the split |
| Fair price as justice | *Aequitas*; EV = 0 as “equal terms” | Formal expectation (Huygens); utility (D. Bernoulli) |
| House edge | Fraud catalog + calibration detector | A priced vig as a first-class object |
| Ruin | Small stakes; rich-man vs poor-man | Kelly; gambler’s-ruin formulae |

Edwards’ line — that before 1654 there was only enumeration — is the myth this room exists to retire. Enumeration was the *method*. Chapter 14 is the *doctrine*.

---

## 9. The doctrine Room Nova hands to the table

Not an ADR. Not a crate. Ten sentences Coniunctio may pin, rewrite, or refuse.

1. A score that does not name its circuit is a wager dressed as measurement.
2. *Aequitas* is equality of conditions, inspected, not a fairness scalar.
3. Fair price is odds *r* : *s* from the circuit; EV = 0 is justice.
4. *Scientia* is the circuit plus the proportion. *Fortuna* is residual. Luck is not skill.
5. A systematic lean is a reason. It is not the Prince.
6. Multiply probabilities. Never odds. Never treat *np* as *P*.
7. Independence is a receipt. No receipt, no *pⁿ*.
8. Pay remaining work × chance of finishing. Do not pay sunk score. Do not use *b(b+1) : a(a+1)*.
9. Size stakes to ruin, not to EV. Small stakes. Equal stakes ≠ equal ruin.
10. A take-rate not in the declared circuit is fraud or house edge. Detect it. Price it or walk. Do not use it.

---

## 10. Bibliographic anchors

Primary and the four named readers. Secondary only where a quotation above depends on it.

- Cardano, Girolamo. *Liber de Ludo Aleae*. In *Hieronymi Cardani Mediolanensis Opera Omnia*, tomus I. Lyon: Huguetan & Ravaud, 1663, pp. 262–276. [Internet Archive: imgmar3940MiscellaneaOpal](https://archive.org/details/imgmar3940MiscellaneaOpal).
- Cardano, Girolamo. *Practica arithmetice et mensurandi singularis*. Milan, 1539. (Problem of points; rich man vs poor man.)
- Cardano, Girolamo. *De vita propria liber*. (Daily play; the shame sentence.)
- Gould, Sydney Henry, trans. *The Book on Games of Chance*. In Ore 1953; separately Holt, Rinehart and Winston, 1961; Dover, 2015.
- Ore, Øystein. *Cardano: The Gambling Scholar*. Princeton: Princeton University Press, 1953. (ROTM named at p. 150; rudimentary LLN; Gould’s English.)
- Bellhouse, David. “Decoding Cardano’s *Liber de Ludo Aleae*.” *Historia Mathematica* 32, no. 2 (2005): 180–202. (Aristotelian / humanist frame; *aequitas*; ch. 32 *tali*.)
- Gorroochurn, Prakash. “Some Laws and Problems of Classical Probability and How Cardano Anticipated Them.” *Chance* 25, no. 4 (2012): 13–20. (Ch. 14 quote; power-rule reconstruction; 25/27 table; ROTM on double-six; problem of points 6 : 1 vs 7 : 1; rich/poor game.)
- Gorroochurn, Prakash. *Classic Problems of Probability*. Hoboken: Wiley, 2012. Problem 1.
- Gigerenzer, Gerd, Zeno Swijtink, Theodore Porter, Lorraine Daston, John Beatty, and Lorenz Krüger. *The Empire of Chance: How Probability Changed Science and Everyday Life*. Cambridge: Cambridge University Press, 1989. (Banishing luck.)
- Galileo Galilei. *Sopra le Scoperte dei Dadi*. (Three-dice 9/10/11/12; later than Cardano.)
- Pacioli, Luca. *Summa de arithmetica, geometria, proportioni et proportionalita*. Venice, 1494. (Sunk-score split.)
- Pascal–Fermat correspondence, 1654. (Correct problem of points.)
- Huygens, Christiaan. *De ratiociniis in ludo aleae*. 1657. (First published treatise; expectation.)
- Bernoulli, Jacob. *Ars Conjectandi*. Basel, 1713. (Classical definition; golden theorem.)
- de Moivre, Abraham. *De Mensura Sortis*. 1711. (Classical definition.)
- Leibniz, Gottfried Wilhelm. 1678 fragment on expectation, as cited by Gorroochurn 2012.
- Laplace, Pierre-Simon. First probability paper, 1774; *Essai philosophique sur les probabilités*. (Classical definition in the textbook form.)
- Bernoulli, Daniel. “Specimen theoriae novae de mensura sortis.” *Commentarii Academiae Scientiarum Imperialis Petropolitanae*, 1738. (Utility; St. Petersburg.)
- Knight, Frank H. *Risk, Uncertainty, and Profit*. Boston: Houghton Mifflin, 1921.
- Kelly, J. L., Jr. “A New Interpretation of Information Rate.” *Bell System Technical Journal* 35, no. 4 (1956): 917–926.
- Savage, Leonard J. *The Foundations of Statistics*. New York: Wiley, 1954.
- Williams, L. “Cardano and the Gambler’s *Habitus*.” *Studies in History and Philosophy of Science* 36 (2005). (ROTM practice; cited by Gorroochurn.)
- Ekert, Artur. “Complex and Unpredictable Cardano.” arXiv:0806.0485, 2008. (Gould’s equal-conditions sentence with the fool/unjust clause; “favoured side.”)
- Edwards, A. W. F., as quoted by Gorroochurn 2012. (The “enumeration only” line Nova rejects as sufficient history.)
- Local digest: `sources/chapter-digest.md`.

---

## 11. Handoff

Room Vetus is not this paper. Room Coniunctio is not this paper.

Nova hands over: the ten primitives, the five refusals, the decision map, the checklist, and the six ASCII diagrams. Primitive names are frozen as in §3. The headline thesis is the sentence in the front matter.

If Coniunctio pins local ADRs, the candidates already named in `AGENDA.md` (LDA-ADR-001 circuit, 002 *aequitas*, 003 house edge, 004 ruin, 005 remaining-work) are the natural homes. Nova does not write them.
