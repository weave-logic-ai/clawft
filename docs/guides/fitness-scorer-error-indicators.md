# FitnessScorer `error_indicators` allowlist (WEFT-54)

**Status**: Reviewed for 0.8.x  
**Code**: `crates/clawft-core/src/pipeline/scorer.rs`  
**Tests**: unit tests in `scorer.rs` (WEFT-54 section) + `crates/clawft-core/tests/error_indicators_weft54.rs`

## Purpose

`FitnessScorer` uses a small list of **English refusal / soft-failure phrases** as a
task-completion heuristic. When a response contains any configured indicator
(case-insensitive **substring** match), task completion is reduced by **0.2**
once (penalties are not stacked).

This feed is for **GEPA / trajectory fitness only**. It is **not**:

- a content-moderation or safety policy
- a jailbreak detector
- a multilingual localization layer

## Default list (`DEFAULT_ERROR_INDICATORS`)

| Phrase | Intent |
|--------|--------|
| `I can't` | Soft capability refusal |
| `I'm unable` | Soft capability refusal |
| `I cannot` | Soft capability refusal |
| `I don't have access` | Missing capability / tool |
| `I'm not able` | Soft capability refusal |
| `as an AI` | Stock identity hedge often paired with refusal |
| `I'm not allowed` | Policy-style refusal (added WEFT-54) |
| `I must decline` | Explicit decline (added WEFT-54) |
| `I won't be able` | Future-tense soft refusal (added WEFT-54) |

Operators may replace the list via `FitnessScorerConfig.error_indicators`.

## Language scope: English-only

**Decision**: keep defaults **English-only** (ASCII phrases). Do **not** lightly
spray other languages into the substring list.

**Rationale**:

1. Substring matching is already false-positive prone in English (see below).
2. Short refusal fragments in other languages collide with common vocabulary
   without locale-aware tokenization.
3. GEPA fitness for non-English product surfaces should use a **locale-specific
   config**, not a global multi-language bag.

Honest limit: French / Spanish / German stock refusals (e.g. « Je ne peux pas… »,
« No puedo… ») are **not** detected by defaults. Covered by unit tests.

## Matching semantics

- Case-insensitive
- Substring `contains` (no word boundaries)
- First match only for the −0.2 penalty
- Empty responses still score 0.0 on task completion (separate path)

## Known-good catalog (must **not** match)

Legitimate assistant output that should remain unpenalized by indicators:

- Helpful plans: “Here is a step-by-step plan to deploy the service.”
- Affirmative capability: “I can help you write the migration script.”
- Waiting / process language: “I'll wait for your confirmation…”
- Technical AI mentions without the phrase `as an AI`: “Training an AI model…”
- Error codes in prose: “The API returns 403 when credentials are missing.”
- Non-English benign text without English markers

See `KNOWN_GOOD` / `KNOWN_GOOD_RESPONSES` in the test modules.

## Known-bad catalog (must match)

Stock English refusals that should trigger the penalty:

- “I can't help with that request.”
- “I'm unable to complete this task right now.”
- “I cannot provide that information.”
- “I don't have access to the internal network.”
- “I'm not able to run shell commands for you.”
- “As an AI, I must follow safety guidelines.”
- “I'm not allowed to share that content.”
- “I must decline this request.”
- “I won't be able to process that file.”

## Documented false positives (English)

Substring collisions that **currently match** and lower fitness incorrectly:

| Example | Collides with |
|---------|----------------|
| “I can't wait to see the benchmark results!” | `I can't` |
| “As an AI engineer, prefer typed APIs…” | `as an AI` |

These are **pinned in tests** so a future matcher tightening (word boundaries,
negative lookarounds, or classifier) must update this guide deliberately.

## Jailbreak / adversarial limits (honest)

| Attack / pattern | Detected? | Notes |
|------------------|-----------|--------|
| Stock English refusal phrases | Yes | Intended |
| Roleplay / fictional wrapper without stock phrases | **No** | By design — not a safety filter |
| Non-English refusal | **No** | English-only defaults |
| Refusal split across tokens / unicode lookalikes | **No** | No normalization beyond lowercasing |
| “Helpful” compliance that never refuses | N/A | Indicators only fire on listed phrases |
| Indicator smuggling into quoted code | Possible FP | Substring match has no AST awareness |

**Do not** wire `error_indicators` into authz, tool allowlists, or user-facing
moderation. Use dedicated safety / policy layers for that.

## False-positive rate sanity

Property tests in `error_indicators_weft54.rs`:

1. **Random a–z words** (2000 trials): indicator hit rate must stay **&lt; 1%**.
2. **Benign English-ish templates** with technical fillers (500 trials): hit rate
   **&lt; 2%**.
3. Random long responses: scores remain in `[0, 1]` with low indicator rate.

These bounds validate that the allowlist does not light up on noise; they do
**not** claim production English corpus precision/recall.

## Changing the list

1. Prefer **higher-specificity multi-word English** phrases over short fragments.
2. Add known-good / known-bad cases for every new phrase.
3. Re-run:

   ```bash
   scripts/build.sh test clawft-core
   # or focused:
   cargo test -p clawft-core error_indicator -- --nocapture
   cargo test -p clawft-core --test error_indicators_weft54
   ```

4. Update this guide if false-positive or localization behavior changes.

## Related

- ADR-017 GEPA prompt evolution (`docs/adr/adr-017-gepa-prompt-evolution.md`)
- Plane **WEFT-54** — review `FitnessScorer.error_indicators` allowlist
- Related weight fusion question: **WEFT-53** (out of scope here)
