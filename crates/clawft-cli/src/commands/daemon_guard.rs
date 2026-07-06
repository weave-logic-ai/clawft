//! Build-stamp mismatch guard (installer DX).
//!
//! A stale installed `weft` talking to a freshly-built daemon (or the
//! reverse) silently diverges — e.g. the CLI is missing a subcommand the
//! daemon now speaks, or vice-versa. This guard catches that: after
//! connecting to the daemon, callers run [`warn_on_build_mismatch`], which
//! reads the daemon's build hash from `kernel.status` and compares it to
//! the hash baked into this binary (`BUILD_GIT_HASH`, set by `build.rs`).
//! On divergence it prints ONE warning line to stderr, once per process,
//! and is otherwise silent. It never fails a command — a transport error,
//! an `unknown` stamp on either side, or a matching hash all no-op.
//!
//! Wire contract: the daemon reports `kernel.status` → `build.sha` and this
//! side reads exactly that key path. Both are the git short hash from the
//! same `--short=8` + `-dirty` convention (see the two `build.rs` files),
//! so a same-tree build of `weft` and `weaver` produces identical hashes.

use std::sync::atomic::{AtomicBool, Ordering};

use clawft_rpc::{DaemonClient, Request};

/// The build hash baked into this `weft` binary (set by `build.rs`).
const WEFT_SHA: &str = env!("BUILD_GIT_HASH");

/// Warn at most once per process invocation.
static WARNED: AtomicBool = AtomicBool::new(false);

/// True when a hash is absent or the git-unavailable sentinel, so a
/// comparison would be meaningless.
fn indeterminate(sha: &str) -> bool {
    sha.is_empty() || sha == "unknown"
}

/// Compare this binary's build hash against the connected daemon's and
/// print a single warning to stderr on mismatch.
///
/// Best-effort and non-fatal: any RPC/parse failure, an indeterminate
/// stamp on either side, or a matching hash returns silently. The warning
/// fires at most once per process even across multiple daemon connections.
pub async fn warn_on_build_mismatch(client: &mut DaemonClient) {
    // Cheap pre-check: if we already warned, or our own stamp is
    // indeterminate, skip the round-trip entirely.
    if WARNED.load(Ordering::Relaxed) || indeterminate(WEFT_SHA) {
        return;
    }

    let Ok(resp) = client.call(Request::new("kernel.status")).await else {
        return;
    };
    let Ok(value) = resp.into_result() else {
        return;
    };
    let daemon_sha = value
        .get("build")
        .and_then(|b| b.get("sha"))
        .and_then(|s| s.as_str())
        .unwrap_or("");

    if indeterminate(daemon_sha) || daemon_sha == WEFT_SHA {
        return;
    }

    // Set-and-check so concurrent callers still emit exactly one line.
    if WARNED.swap(true, Ordering::Relaxed) {
        return;
    }
    eprintln!(
        "warning: weft built at {WEFT_SHA} but daemon at {daemon_sha} — \
         run scripts/build.sh install (+ restart daemon)"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indeterminate_covers_empty_and_unknown() {
        assert!(indeterminate(""));
        assert!(indeterminate("unknown"));
        assert!(!indeterminate("14145f16"));
        assert!(!indeterminate("14145f16-dirty"));
    }

    #[test]
    fn weft_sha_is_baked_nonempty() {
        // build.rs always sets BUILD_GIT_HASH (git hash or the `unknown`
        // fallback), so the compile-time env is never missing.
        assert!(!WEFT_SHA.is_empty());
    }
}
