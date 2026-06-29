//! Live parity probe for context-budgeting token counts (ADR-058 2.1).
//!
//! Re-measures `agent::tokenizer::count_tokens` against the *served* model's
//! true tokenizer via llama.cpp's `POST /tokenize`. It documents — and guards
//! against regressions in — the two facts established when the seam was wired:
//!
//!   * `o200k_base` (the bundled default) is an *estimator*: ~13% median, up
//!     to ~56% on digit-heavy text, off the served Seed-OSS tokenizer.
//!   * the served model's own `tokenizer.json` (env `CLAWFT_HERMES_TOKENIZER`,
//!     from `NousResearch/Hermes-4.3-36B`) reproduces the server exactly.
//!
//! `#[ignore]`'d: it needs a live Hermes server on `127.0.0.1:8090` and uses
//! `curl` (no HTTP client dev-dependency). Run with:
//!
//!   cargo test -p clawft-core --test hermes_tokenizer_parity -- --ignored --nocapture
//!
//! To exercise the exact backend, set the env var first:
//!
//!   CLAWFT_HERMES_TOKENIZER=/path/to/hermes/tokenizer.json \
//!     cargo test -p clawft-core --test hermes_tokenizer_parity -- --ignored --nocapture

use std::process::Command;

use clawft_core::agent::context::count_tokens;

const SERVER: &str = "http://127.0.0.1:8090/tokenize";

/// Representative fixtures: prose, dense code, URL, digits, CJK/emoji.
const FIXTURES: &[&str] = &[
    "The quick brown fox jumps over the lazy dog.",
    "fn main() {\n    let x = vec![1, 2, 3];\n    println!(\"{:?}\", x);\n}",
    "https://example.com/path?query=value&other=123#fragment and user@host.domain.tld",
    "1234567890 0xDEADBEEF 3.14159265358979 1e-10 0b1010 0o777 == != >= <= && || ??",
    "Café déjà vu — naïve façade, 北京, emoji test 🚀🔥",
];

/// Ask the live server for the true token count of `text` via `/tokenize`.
fn server_token_count(text: &str) -> usize {
    let body = serde_json::json!({ "content": text }).to_string();
    let out = Command::new("curl")
        .args([
            "-s",
            "-m",
            "5",
            SERVER,
            "-H",
            "Content-Type: application/json",
            "--data-binary",
            &body,
        ])
        .output()
        .expect("curl must be on PATH");
    assert!(out.status.success(), "curl to {SERVER} failed");
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("server returned JSON with a `tokens` array");
    v["tokens"]
        .as_array()
        .expect("response must contain a `tokens` array")
        .len()
}

#[test]
#[ignore = "requires a live Hermes server on 127.0.0.1:8090 (ADR-060)"]
fn token_count_parity_against_live_server() {
    let exact = std::env::var_os("CLAWFT_HERMES_TOKENIZER").is_some();
    println!(
        "\n>>> count_tokens backend: {}",
        if exact {
            "exact Hermes/Seed-OSS (CLAWFT_HERMES_TOKENIZER set)"
        } else {
            "bundled o200k_base estimator"
        }
    );
    println!("{:<6}{:<8}{:<8}fixture", "local", "server", "Δ%");

    let mut deltas = Vec::new();
    for text in FIXTURES {
        let local = count_tokens(text);
        let server = server_token_count(text);
        let pct = (local as f64 - server as f64).abs() / server as f64 * 100.0;
        deltas.push(pct);
        let snip: String = text.replace('\n', "\\n").chars().take(40).collect();
        println!("{local:<6}{server:<8}{pct:<8.1}{snip}");
    }
    let mean = deltas.iter().sum::<f64>() / deltas.len() as f64;
    let max = deltas.iter().cloned().fold(0.0_f64, f64::max);
    println!(">>> mean Δ = {mean:.1}%   max Δ = {max:.1}%");

    if exact {
        // The served model's own tokenizer must match the server within
        // rounding (off-by-one on a fixture is the most a future template
        // tweak should ever produce).
        assert!(
            max <= 2.0,
            "exact tokenizer should match the server (max Δ {max:.1}% > 2%)"
        );
    } else {
        // Pin the documented finding: o200k_base is a *real* tokenizer (not
        // the whitespace heuristic) yet materially diverges from the served
        // model. If this band shifts, the module docs' measured numbers need
        // refreshing.
        assert!(
            max > 5.0,
            "o200k_base was expected to diverge >5% from the served tokenizer; \
             got max Δ {max:.1}% — has the served model changed?"
        );
    }
}
