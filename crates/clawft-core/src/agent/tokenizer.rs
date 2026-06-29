//! Token counting for context-window budgeting (ADR-058 Phase 2.1).
//!
//! Replaces the old whitespace `count_tokens` heuristic with a *real*
//! subword tokenizer so the live-window budget (ADR-060: cap to 32k) is
//! computed against actual token counts rather than word counts.
//!
//! # Backends
//!
//! * **`real-tokenizer` feature (native default)** — a real BPE tokenizer,
//!   `tiktoken`'s `o200k_base` vocab (the GPT-4o family, ~200k entries).
//!   It is bundled in the crate, so there is no model download, and it is a
//!   large-vocab BPE comparable in granularity to the served Hermes /
//!   Seed-OSS tokenizer. Crucially it counts punctuation, code, and
//!   non-space-delimited text correctly — exactly where the whitespace
//!   heuristic undercounts and risks overflowing the real window.
//! * **fallback (browser / `real-tokenizer` off)** — an improved char+word
//!   heuristic. The bundled BPE vocab is heavy and not WASM-friendly, so
//!   browser builds use the estimator and accept a looser budget.
//!
//! # Exactness vs. the served model
//!
//! Exact parity with the *served* Hermes tokenizer requires that model's
//! `tokenizer.json`, which ships with the GGUF/HF weights (download blocked
//! at the time of writing — ADR-060 0.1). `o200k_base` is the production
//! stand-in until then; the abstraction here is the single seam through
//! which the exact model tokenizer can later be plugged (load a
//! `tokenizer.json` via the HF `tokenizers` crate) without touching callers.

/// Approximate token count for a string, using the best available tokenizer.
///
/// With the `real-tokenizer` feature (native default) this returns the exact
/// `o200k_base` BPE token count for `text`. Without it, it returns a
/// char/word heuristic biased slightly high so budgeting never *under*counts.
///
/// The function is infallible and cheap to call per message; the BPE vocab is
/// parsed once and cached for the process lifetime.
pub fn count_tokens(text: &str) -> usize {
    #[cfg(feature = "real-tokenizer")]
    {
        bpe().encode_ordinary(text).len()
    }
    #[cfg(not(feature = "real-tokenizer"))]
    {
        heuristic_count_tokens(text)
    }
}

/// The lazily-initialized, process-wide BPE tokenizer.
///
/// `o200k_base` construction parses the bundled vocab once; subsequent calls
/// reuse the same `CoreBPE`.
#[cfg(feature = "real-tokenizer")]
fn bpe() -> &'static tiktoken_rs::CoreBPE {
    use std::sync::OnceLock;
    static BPE: OnceLock<tiktoken_rs::CoreBPE> = OnceLock::new();
    BPE.get_or_init(|| tiktoken_rs::o200k_base().expect("bundled o200k_base vocab must load"))
}

/// Heuristic token estimate used when no real tokenizer is available.
///
/// Takes the **max** of a char-based estimate (~4 chars/token, the standard
/// rule of thumb) and a word-based estimate (~4/3 tokens/word for English
/// prose). The max keeps the estimate safe on both ends: code and structured
/// text (few spaces, many tokens) are caught by the char term, while ordinary
/// prose is caught by the word term. Empty input is zero.
///
/// Compiled only when it can actually be reached: the fallback (no real
/// tokenizer) build, or any test build (the tests exercise it directly).
#[cfg(any(not(feature = "real-tokenizer"), test))]
pub(crate) fn heuristic_count_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    let chars = text.chars().count();
    let by_chars = chars.div_ceil(4);
    let words = text.split_whitespace().count();
    let by_words = (words * 4).div_ceil(3);
    by_chars.max(by_words).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_zero() {
        assert_eq!(count_tokens(""), 0);
        assert_eq!(heuristic_count_tokens(""), 0);
    }

    #[test]
    fn nonempty_is_positive() {
        assert!(count_tokens("hello world") >= 1);
        assert!(heuristic_count_tokens("hello world") >= 1);
    }

    /// The heuristic must not wildly *under*count code-shaped text — the
    /// failure mode of the old whitespace counter (which saw `fn` blocks with
    /// almost no spaces as a handful of tokens). Char-based term dominates.
    #[test]
    fn heuristic_handles_code_without_undercount() {
        let code = "fn main(){let x=vec![1,2,3];println!(\"{:?}\",x);}";
        let words = code.split_whitespace().count(); // ~3
        let old_whitespace_estimate = (words * 4).div_ceil(3);
        let est = heuristic_count_tokens(code);
        // The improved estimate is meaningfully larger than the pure
        // whitespace count on dense code.
        assert!(
            est > old_whitespace_estimate * 2,
            "code estimate {est} should dwarf whitespace estimate {old_whitespace_estimate}"
        );
    }

    /// Fixture set with reference counts from the real `o200k_base` BPE.
    /// `count_tokens` (real-tokenizer on) must land within ~5% of each
    /// reference. This guards that budgeting uses a real subword tokenizer
    /// and never silently regresses to a word/char heuristic. References were
    /// captured from `tiktoken_rs::o200k_base().encode_ordinary(..).len()`.
    #[cfg(feature = "real-tokenizer")]
    #[test]
    fn real_tokenizer_within_tolerance_on_fixtures() {
        // (input, reference o200k token count)
        let fixtures: &[(&str, usize)] = &[
            ("The quick brown fox jumps over the lazy dog.", 10),
            (
                "Rust is a systems programming language focused on safety and performance.",
                12,
            ),
            (
                "fn main() {\n    let x = vec![1, 2, 3];\n    println!(\"{:?}\", x);\n}",
                28,
            ),
            (
                "ExoChain is the single source of truth; the agent gets a query/graft layer.",
                18,
            ),
        ];
        for (text, reference) in fixtures {
            let actual = count_tokens(text);
            let tol = (*reference as f64 * 0.05).ceil() as usize;
            let lo = reference.saturating_sub(tol.max(1));
            let hi = reference + tol.max(1);
            assert!(
                actual >= lo && actual <= hi,
                "token count {actual} for {text:?} outside 5% of reference {reference} (allowed {lo}..={hi})"
            );
        }
    }
}
