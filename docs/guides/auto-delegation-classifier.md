# Auto-delegation classifier (WEFT-201)

## Status

**Documented fragility + regression corpus.** A LoRA / embedding classifier
is blocked on the micro-LoRA hybrid context router (cross-stream). Until
that lands, WeftOS uses:

1. **Ordered regex rules** from `delegation.rules` (first match wins)
2. **Auto complexity heuristic** when no rule matches
   (`DelegationEngine::complexity_estimate`)

Pre-LLM short-circuit: `AutoDelegationRouter` in
`crates/clawft-cli/src/commands/agent.rs` wraps the engine. Any non-`Local`
target becomes a direct `delegate_task` invocation.

## Fragility (known)

| Failure mode | Example | Impact |
|--------------|---------|--------|
| **False positive** | `"list the deploy status"` matches `(?i)deploy` → Claude | Extra cost / latency for a local-capable task |
| **False negative** | `"ship to prod"` without a rule → Auto may stay Local if short | Missed Claude path for ops work |
| **Substring traps** | keyword `review` in `"preview layout"` | Complexity score inflated |
| **Anchor gaps** | `(?i)list` vs `(?i)^list\b` | `checklist` / mid-sentence matches |
| **Language** | Non-English task text | Keywords are English-only |
| **Serde footgun** | `[delegation]` present without `claude_enabled` | Field defaults to `false` (not `Default::default()` true) |

False positives/negatives **directly affect routing** — auto-delegation
bypasses the local LLM, so a bad match is not self-corrected by model
judgment.

## Operator guidance

1. Prefer **anchored, specific** regexes (`(?i)^deploy\b`, `(?i)\bsecurity audit\b`).
2. Put **Local** rules **before** broad Claude rules when they share tokens.
3. Keep a short **golden corpus** in tests (this crate) when changing rules.
4. Run `weft doctor --multi-agent` (WEFT-197) to see rule count +
   `claude_enabled`.
5. When unsure, disable auto-delegation (`claude_enabled = false`) and
   rely on explicit `delegate_task` tool calls.

## Complexity heuristic (Auto)

Score ∈ [0, 1]:

- 30% length (saturates at 500 chars)
- 20% `?` density (saturates at 3)
- 50% keyword hits from a fixed English list (`deploy`, `refactor`,
  `architect`, …)

Threshold: **&lt; 0.3 → Local**, else Claude (if available + enabled).

## Regression corpus

Authoritative cases live in:

- `crates/clawft-services/src/delegation/mod.rs` (`classifier_corpus_*`)
- `crates/clawft-cli/src/commands/agent.rs` (`auto_delegation_tests`)

Add new fixtures there when changing patterns or keywords — do not rely
on ad-hoc manual checks alone.

## Follow-up

Replace regex+keyword with embedding / micro-LoRA routing when the hybrid
context router is available. Keep the corpus as an oracle for parity
tests against the new classifier.
