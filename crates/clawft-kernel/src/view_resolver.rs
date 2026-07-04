//! View resolution seam for the multiplexed [`TalkModeLoop`](crate::talk_loop).
//!
//! M2 D4: the loop stops holding a single `Arc<SessionView>` and instead holds
//! an injected [`ViewResolver`], so one daemon-hosted loop can multiplex all
//! conversations — committing each turn onto the *same* per-conversation
//! `SessionView` that `SessionTier` grafts from. `SessionTier` implements
//! `ViewResolver` (production), while [`SingleViewResolver`] keeps voice's
//! single-conversation `TalkForest` and the unit tests green.

use std::sync::Arc;

use crate::context_graft::SessionView;

/// Resolve the per-conversation [`SessionView`] the loop must commit onto.
///
/// Returns `None` when the conversation has been reaped (its view dropped); the
/// loop treats that as a logged no-op rather than committing onto a stale view.
pub trait ViewResolver: Send + Sync {
    /// The live view for `conv_id`, or `None` if the conversation is gone.
    fn view_for(&self, conv_id: &str) -> Option<Arc<SessionView>>;
}

/// A trivial resolver that returns one fixed view for any `conv_id`.
///
/// Used by voice's single-conversation `TalkForest` (Decision 4 "voice stays
/// green" adapter) and by kernel unit tests that exercise a single view.
pub struct SingleViewResolver(Arc<SessionView>);

impl SingleViewResolver {
    /// Wrap a single view; every `view_for` call returns it.
    pub fn new(view: Arc<SessionView>) -> Self {
        Self(view)
    }
}

impl ViewResolver for SingleViewResolver {
    fn view_for(&self, _conv_id: &str) -> Option<Arc<SessionView>> {
        Some(self.0.clone())
    }
}
