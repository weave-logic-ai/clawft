//! Token counting for context-window budgeting (ADR-058 Phase 2.1).
//!
//! Replaces the old whitespace `count_tokens` heuristic with a *real*
//! subword tokenizer so the live-window budget (ADR-060: cap to 32k) is
//! computed against actual token counts rather than word counts.
//!
//! # Backends (`real-tokenizer` feature, native default)
//!
//! Resolved once, in priority order, by [`backend`]:
//!
//! 1. **Exact (`CLAWFT_HERMES_TOKENIZER`)** — when that env var points at a
//!    valid `tokenizer.json`, it is loaded via the HF `tokenizers` crate and
//!    used directly. This is the served model's own tokenizer, so token
//!    counts match the runtime window exactly.
//! 2. **`o200k_base` stand-in** — `tiktoken`'s GPT-4o BPE (~200k entries),
//!    bundled in the crate (no model download). It counts punctuation, code,
//!    and non-space-delimited text far better than the whitespace heuristic,
//!    but it is *not* the served tokenizer (see "Exactness" below).
//!
//! Without the feature (browser / WASM) the fallback is an improved char+word
//! heuristic; the bundled BPE vocab is heavy and not WASM-friendly, so browser
//! builds accept a looser budget.
//!
//! # Exactness vs. the served model
//!
//! The agent-loop model (ADR-060/061) is **Hermes-4.3-36B**, a
//! **`SeedOssForCausalLM`** model (ByteDance Seed-OSS base, 155 136-entry
//! vocab) — *not* a Llama-3 or GPT family model. Its `tokenizer.json` is
//! published in the non-GGUF base repo `NousResearch/Hermes-4.3-36B` and was
//! verified to reproduce the live `llama.cpp` server's `POST /tokenize`
//! counts **exactly** (10/10 fixtures, 0% delta).
//!
//! `o200k_base` is therefore only an *estimator* for this model. Measured
//! against the live server (`:8090 /tokenize`) over a 10-fixture set spanning
//! prose, dense code, URLs, digits, and CJK/emoji:
//!
//! | metric | o200k_base vs Seed-OSS |
//! |--------|------------------------|
//! | mean Δ | **14.8 %** |
//! | median Δ | **12.5 %** |
//! | max Δ | **56.4 %** (digit-heavy text) |
//! | within 5 % | **3 / 10 fixtures** |
//!
//! So o200k_base is **outside** the ADR-058 ~5% target. It remains the bundled
//! default because it needs no model download and is deterministic across
//! environments (critical: budgeting and its tests must not silently change
//! with which files happen to be on disk). Operators who need exact budgeting
//! set `CLAWFT_HERMES_TOKENIZER` to the served model's `tokenizer.json`
//! (e.g. fetched with `~/llm/bin/pull NousResearch/Hermes-4.3-36B --include
//! "*tokenizer*"`). The `tests/hermes_tokenizer_parity.rs` `#[ignore]`'d probe
//! re-measures both deltas against a live server.

/// Approximate token count for a string, using the best available tokenizer.
///
/// With the `real-tokenizer` feature (native default) this delegates to the
/// resolved [`backend`] — the exact Seed-OSS tokenizer when
/// `CLAWFT_HERMES_TOKENIZER` is configured, otherwise the bundled `o200k_base`
/// BPE estimator. Without the feature it returns a char/word heuristic biased
/// slightly high so budgeting never *under*counts.
///
/// The function is infallible and cheap to call per message; the tokenizer is
/// constructed once and cached for the process lifetime.
pub fn count_tokens(text: &str) -> usize {
    #[cfg(feature = "real-tokenizer")]
    {
        backend().count(text)
    }
    #[cfg(not(feature = "real-tokenizer"))]
    {
        heuristic_count_tokens(text)
    }
}

/// Resolved token-counting backend (real-tokenizer builds).
#[cfg(feature = "real-tokenizer")]
enum Backend {
    /// The served model's own tokenizer, loaded from a `tokenizer.json`.
    Hermes(Box<tokenizers::Tokenizer>),
    /// The bundled `o200k_base` BPE estimator (no model download required).
    O200k(tiktoken_rs::CoreBPE),
}

#[cfg(feature = "real-tokenizer")]
impl Backend {
    fn count(&self, text: &str) -> usize {
        match self {
            // `encode` only errors on pathological tokenizer config, never on
            // ordinary UTF-8 input; fall back to the char/word heuristic rather
            // than panicking in the budgeting hot path.
            Backend::Hermes(tok) => match tok.encode(text, false) {
                Ok(enc) => enc.len(),
                Err(_) => heuristic_count_tokens(text),
            },
            Backend::O200k(bpe) => bpe.encode_ordinary(text).len(),
        }
    }
}

/// The lazily-initialized, process-wide token-counting backend.
///
/// Resolved once: if `CLAWFT_HERMES_TOKENIZER` names a loadable
/// `tokenizer.json`, that exact tokenizer is used; otherwise the bundled
/// `o200k_base` vocab is parsed once and reused.
#[cfg(feature = "real-tokenizer")]
fn backend() -> &'static Backend {
    use std::sync::OnceLock;
    static BACKEND: OnceLock<Backend> = OnceLock::new();
    BACKEND.get_or_init(|| {
        if let Some(path) = std::env::var_os("CLAWFT_HERMES_TOKENIZER") {
            match tokenizers::Tokenizer::from_file(&path) {
                Ok(tok) => {
                    tracing::info!(
                        path = ?path,
                        "tokenizer: using exact Hermes/Seed-OSS tokenizer.json for budgeting"
                    );
                    return Backend::Hermes(Box::new(tok));
                }
                Err(err) => {
                    tracing::warn!(
                        path = ?path,
                        error = %err,
                        "tokenizer: CLAWFT_HERMES_TOKENIZER failed to load; \
                         falling back to bundled o200k_base estimator"
                    );
                }
            }
        }
        Backend::O200k(tiktoken_rs::o200k_base().expect("bundled o200k_base vocab must load"))
    })
}

/// Heuristic token estimate used when no real tokenizer is available.
///
/// Takes the **max** of a char-based estimate (~4 chars/token, the standard
/// rule of thumb) and a word-based estimate (~4/3 tokens/word for English
/// prose). The max keeps the estimate safe on both ends: code and structured
/// text (few spaces, many tokens) are caught by the char term, while ordinary
/// prose is caught by the word term. Empty input is zero.
///
/// Always compiled: it is the no-feature fallback, the tests exercise it
/// directly, and (with `real-tokenizer` on) it is the infallible fallback for
/// the exact-tokenizer encode path in [`Backend::count`].
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
    /// With no `CLAWFT_HERMES_TOKENIZER` configured, `count_tokens`
    /// (real-tokenizer on) resolves to the bundled `o200k_base` backend and
    /// must land within ~5% of each reference. This guards that budgeting uses
    /// a real subword tokenizer and never silently regresses to a word/char
    /// heuristic. References were captured from
    /// `tiktoken_rs::o200k_base().encode_ordinary(..).len()`.
    #[cfg(feature = "real-tokenizer")]
    #[test]
    fn real_tokenizer_within_tolerance_on_fixtures() {
        // The exact-tokenizer override changes `count_tokens` to the Seed-OSS
        // vocab, whose counts differ from o200k by design (see module docs);
        // skip the o200k reference check when an exact tokenizer is configured.
        if std::env::var_os("CLAWFT_HERMES_TOKENIZER").is_some() {
            return;
        }
        // (input, reference o200k token count)
        let fixtures: &[(&str, usize)] = &[
            ("The quick brown fox jumps over the lazy dog.", 10),
            (
                "Rust is a systems programming language focused on safety and performance.",
                12,
            ),
            (
                "fn main() {\n    let x = vec![1, 2, 3];\n    println!(\"{:?}\", x);\n}",
                26,
            ),
            (
                "ExoChain is the single source of truth; the agent gets a query/graft layer.",
                19,
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
