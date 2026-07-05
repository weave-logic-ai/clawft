//! Ambient parent context for agent-initiated spawns (M4 seam).
//!
//! The [`Tool::execute`](crate::tools::registry::Tool::execute) signature
//! receives only the tool's JSON arguments — it has no handle to the
//! conversation or agent that invoked it. But a spawned subagent must carry
//! its parent's identity so the spawner can draw the `TriggeredBy` /
//! `EvidenceFor` forest edges (design D3) and enforce the recursion depth
//! cap (design D5). This is why the design says the `parent_*` fields of
//! [`SpawnSpec`](super::spawn::SpawnSpec) come from the *execution context*,
//! not from the LLM's arguments.
//!
//! This module is that seam. The agent loop installs a [`SpawnContext`]
//! around tool dispatch via [`SpawnContext::scope`]; the `agent_spawn` tool
//! reads it back with [`current`]. Outside a scope (the in-process CLI, unit
//! tests) [`current`] returns `None` and the spawn degrades gracefully to a
//! root-level spawn with no parent edges.
//!
//! Native-only: the task-local is backed by `tokio`, which the browser/WASM
//! build does not pull in. Subagent spawning is a daemon capability, so the
//! seam never needs to exist off-native.

use std::future::Future;

/// Parent conversation context captured around tool dispatch so the
/// `agent_spawn` tool can populate a [`SpawnSpec`](super::spawn::SpawnSpec)'s
/// `parent_*` fields without the LLM supplying them.
#[derive(Debug, Clone, Default)]
pub struct SpawnContext {
    /// The parent conversation id (`conv_id = P`).
    pub conv_id: Option<String>,
    /// The parent agent's principal id (for the spawn-time gate precheck).
    pub agent_id: Option<String>,
    /// Universal id of the parent turn making the spawn call (forest edges).
    pub turn_uid: Option<String>,
    /// The parent conversation's spawn depth. A spawned child is `depth + 1`.
    pub depth: u32,
}

tokio::task_local! {
    static SPAWN_CONTEXT: SpawnContext;
}

impl SpawnContext {
    /// Run `fut` with `self` installed as the ambient spawn context. The
    /// agent loop wraps its tool-dispatch future in this so any `agent_spawn`
    /// call inside sees the parent conversation's identity and depth.
    pub async fn scope<F>(self, fut: F) -> F::Output
    where
        F: Future,
    {
        SPAWN_CONTEXT.scope(self, fut).await
    }
}

/// The ambient [`SpawnContext`] for the current task, or `None` when no
/// [`SpawnContext::scope`] is active (in-process CLI, unit tests).
pub fn current() -> Option<SpawnContext> {
    SPAWN_CONTEXT.try_with(|c| c.clone()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn current_is_none_outside_scope() {
        assert!(current().is_none());
    }

    #[tokio::test]
    async fn scope_installs_and_restores() {
        let ctx = SpawnContext {
            conv_id: Some("P".into()),
            agent_id: Some("agent-P".into()),
            turn_uid: Some("turn-1".into()),
            depth: 2,
        };
        ctx.scope(async {
            let seen = current().expect("context visible inside scope");
            assert_eq!(seen.conv_id.as_deref(), Some("P"));
            assert_eq!(seen.agent_id.as_deref(), Some("agent-P"));
            assert_eq!(seen.turn_uid.as_deref(), Some("turn-1"));
            assert_eq!(seen.depth, 2);
        })
        .await;
        // Restored to empty once the scope ends.
        assert!(current().is_none());
    }
}
