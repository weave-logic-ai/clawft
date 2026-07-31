//! Identity-aware system-prompt builder (agent-core-v1 Phase D1).
//!
//! Produces the leading `system` message for each turn by composing
//! `SOUL.md` + `IDENTITY.md` + workspace context + a binding-thread
//! integrity status. Mirrors the shape of the spike's
//! `build_concierge_system_prompt` in `clawft-weave::daemon` so the
//! Phase D3 cutover preserves the user-visible prompt semantics.
//!
//! ```text
//! [identity]
//! {soul}
//!
//! {identity}
//!
//! [binding-thread-status]
//! {ok | mismatch}
//!
//! [workspace]
//! {workspace_path}
//!
//! [hash]
//! {sha256_hex}
//! ```
//!
//! The hash and binding-thread status are pinned to the trailing
//! lines so they're observable in logs without crowding the persona
//! content the LLM actually consumes for its voice.
//!
//! ## Binding-thread policy (WEFT-342)
//!
//! By default (`agents.binding_thread_mode = "deny"`) a missing
//! [`BINDING_THREAD_EXCERPT`] hard-refuses the turn
//! ([`IdentityError::BindingThreadMismatch`]). Set
//! `binding_thread_mode = "warn_only"` to restore the legacy annotate
//! and `warn!` degraded path. The agent loop also evaluates governance
//! rule `soul.binding_thread_intact` via `gate.check` every turn.
//!
//! Plan reference: `docs/plans/agent-core-v1.md` Phase D1; WEFT-342.

use std::path::PathBuf;
use std::sync::Arc;

use tracing::warn;

use clawft_types::config::BindingThreadMode;

use crate::agent::identity::{
    BINDING_THREAD_MISMATCH_REASON, Identity, IdentityError, IdentityProvider,
    soul_contains_binding_thread,
};

/// Status of the binding-thread integrity check.
///
/// When the operator policy is [`BindingThreadMode::Deny`] (default),
/// a [`Mismatch`](Self::Mismatch) aborts prompt construction via
/// [`IdentityError::BindingThreadMismatch`]. Under
/// [`BindingThreadMode::WarnOnly`] the mismatch only annotates the
/// prompt and emits a `warn!` log so an operator can roll back the
/// SOUL.md edit without blocking chat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingThreadStatus {
    /// Loaded SOUL.md contains [`BINDING_THREAD_EXCERPT`] verbatim.
    Ok,
    /// Loaded SOUL.md does NOT contain [`BINDING_THREAD_EXCERPT`].
    Mismatch,
}

impl BindingThreadStatus {
    /// Stable string label for embedding in the system prompt and gate context.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Mismatch => "mismatch",
        }
    }

    /// Compute the status by substring-matching `soul` against the
    /// compile-time excerpt. Splitting this out keeps the check
    /// deterministically testable without going through the full
    /// `build()` path.
    pub fn from_soul(soul: &str) -> Self {
        if soul_contains_binding_thread(soul) {
            Self::Ok
        } else {
            Self::Mismatch
        }
    }

    /// `true` when the binding thread is intact.
    pub const fn is_ok(self) -> bool {
        matches!(self, Self::Ok)
    }
}

/// Result of [`SystemPromptBuilder::build_with_meta`]: the rendered
/// system-prompt body plus the identity-loader source label (WEFT-328)
/// and the binding-thread status observed this turn (WEFT-342).
#[derive(Debug, Clone)]
pub struct BuiltSystemPrompt {
    /// Rendered system message body (same text as [`SystemPromptBuilder::build`]).
    pub body: String,
    /// Identity loader source (e.g. `"clawft"`) for
    /// [`AgentChatResult::identity_source`](clawft_types::agent_chat::AgentChatResult).
    pub identity_source: String,
    /// Binding-thread integrity status for this turn (WEFT-342).
    pub binding_thread_status: BindingThreadStatus,
}

/// Builds an identity-aware system message body for a turn.
///
/// Construction is cheap; reuse one builder per [`AgentLoop`]. The
/// builder calls `provider.current()` on every `build()` invocation
/// so a `FileIdentityProvider` cached load picks up cross-turn edits
/// without explicit invalidation.
///
/// See module documentation for the output layout.
pub struct SystemPromptBuilder {
    identity_provider: Arc<dyn IdentityProvider>,
    workspace: PathBuf,
    /// WEFT-342: deny (default) hard-refuses on mismatch; warn_only
    /// annotates the prompt and continues.
    binding_thread_mode: BindingThreadMode,
}

impl SystemPromptBuilder {
    /// Wire the builder against an [`IdentityProvider`] and the
    /// workspace path used in the `[workspace]` section.
    ///
    /// Binding-thread policy defaults to [`BindingThreadMode::Deny`]
    /// (WEFT-342). Use [`Self::with_binding_thread_mode`] to opt into
    /// legacy warn-only behaviour.
    pub fn new(identity_provider: Arc<dyn IdentityProvider>, workspace: PathBuf) -> Self {
        Self {
            identity_provider,
            workspace,
            binding_thread_mode: BindingThreadMode::Deny,
        }
    }

    /// Set the binding-thread integrity policy (WEFT-342).
    pub fn with_binding_thread_mode(mut self, mode: BindingThreadMode) -> Self {
        self.binding_thread_mode = mode;
        self
    }

    /// Borrow the configured binding-thread policy.
    pub fn binding_thread_mode(&self) -> BindingThreadMode {
        self.binding_thread_mode
    }

    /// Borrow the workspace path the builder advertises in prompts.
    pub fn workspace(&self) -> &std::path::Path {
        &self.workspace
    }

    /// Build the system message body plus the identity-source label
    /// (WEFT-328). Callers that only need the text can use [`Self::build`].
    ///
    /// Returns:
    /// - [`IdentityError::NotFound`] when the underlying provider
    ///   cannot resolve identity content.
    /// - [`IdentityError::BindingThreadMismatch`] when SOUL.md lacks
    ///   the binding-thread excerpt and policy is
    ///   [`BindingThreadMode::Deny`] (WEFT-342).
    pub async fn build_with_meta(&self) -> Result<BuiltSystemPrompt, IdentityError> {
        let identity = self.identity_provider.current().await?;
        let status = BindingThreadStatus::from_soul(&identity.soul);
        if status == BindingThreadStatus::Mismatch && self.binding_thread_mode.is_deny() {
            warn!(
                hash = %identity.hash,
                source = %identity.source,
                reason = BINDING_THREAD_MISMATCH_REASON,
                "binding-thread mismatch: SOUL.md does not contain the \
                 compile-time BINDING_THREAD_EXCERPT — hard refusing turn \
                 (agents.binding_thread_mode=deny)"
            );
            return Err(IdentityError::BindingThreadMismatch);
        }
        Ok(BuiltSystemPrompt {
            body: self.render_with_status(&identity, status),
            identity_source: identity.source.as_str().to_string(),
            binding_thread_status: status,
        })
    }

    /// Build the system message body.
    ///
    /// See [`Self::build_with_meta`] for error semantics.
    pub async fn build(&self) -> Result<String, IdentityError> {
        Ok(self.build_with_meta().await?.body)
    }

    /// Render an in-memory [`Identity`] into the prompt body. Split
    /// out so unit tests can exercise the formatting deterministically
    /// without going through an [`IdentityProvider`].
    ///
    /// Callers that need the deny-mode refuse must use
    /// [`Self::build_with_meta`] (or check
    /// [`BindingThreadStatus::from_soul`] themselves) — this method
    /// always renders, including mismatch annotations for warn-only.
    pub fn render(&self, identity: &Identity) -> String {
        let status = BindingThreadStatus::from_soul(&identity.soul);
        self.render_with_status(identity, status)
    }

    fn render_with_status(&self, identity: &Identity, status: BindingThreadStatus) -> String {
        if status == BindingThreadStatus::Mismatch {
            warn!(
                hash = %identity.hash,
                source = %identity.source,
                "binding-thread mismatch: SOUL.md does not contain the \
                 compile-time BINDING_THREAD_EXCERPT — running in \
                 degraded mode (agents.binding_thread_mode=warn_only)"
            );
        }

        // Reserve roughly: identity bodies + ~512 bytes of scaffolding
        // (+ pending journal summaries when present, WEFT-96).
        let mut s = String::with_capacity(identity.soul.len() + identity.identity.len() + 512);
        s.push_str("[identity]\n");
        s.push_str(&identity.soul);
        // Make sure there's a blank line between the two bodies even
        // if SOUL.md doesn't end in a newline.
        if !identity.soul.ends_with('\n') {
            s.push('\n');
        }
        s.push('\n');
        s.push_str(&identity.identity);
        if !identity.identity.ends_with('\n') {
            s.push('\n');
        }
        s.push('\n');

        s.push_str("[binding-thread-status]\n");
        s.push_str(status.as_str());
        s.push('\n');
        if status == BindingThreadStatus::Mismatch {
            s.push_str(
                "note: SOUL.md does not match the compile-time \
                 binding-thread excerpt; running in degraded mode.\n",
            );
        }
        s.push('\n');

        // WEFT-96: surface pending journal rows consulted this turn.
        // Promote still owns merging into SOUL.md — this is advisory.
        if !identity.pending_journal.is_empty() {
            s.push_str("[soul-journal-pending]\n");
            s.push_str(&format!("count: {}\n", identity.pending_journal.len()));
            for entry in &identity.pending_journal {
                s.push_str("- ");
                s.push_str(&entry.summary);
                s.push('\n');
            }
            s.push('\n');
        }

        s.push_str("[workspace]\n");
        s.push_str(&self.workspace.display().to_string());
        s.push('\n');
        s.push('\n');

        s.push_str("[hash]\n");
        s.push_str(&identity.hash);
        s.push('\n');

        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::identity::{
        BINDING_THREAD_EXCERPT, FileIdentityProvider, IdentitySource,
    };
    use async_trait::async_trait;

    /// In-memory identity provider for tests.
    struct StubProvider(Identity);

    #[async_trait]
    impl IdentityProvider for StubProvider {
        async fn current(&self) -> Result<Identity, IdentityError> {
            Ok(self.0.clone())
        }
    }

    /// Provider that always errors — exercises the propagation path.
    struct FailingProvider;

    #[async_trait]
    impl IdentityProvider for FailingProvider {
        async fn current(&self) -> Result<Identity, IdentityError> {
            Err(IdentityError::NotFound)
        }
    }

    fn ok_identity() -> Identity {
        let soul = format!(
            "# SOUL.md\n\nWe carry the binding thread: \
             {BINDING_THREAD_EXCERPT}.\n"
        );
        let identity = "# IDENTITY.md\n\nI am clawft.".to_string();
        Identity::from_files(soul, identity, IdentitySource::Stub)
    }

    fn mismatch_identity() -> Identity {
        let soul = "# SOUL.md\n\nNothing distinctive in here.\n".to_string();
        let identity = "# IDENTITY.md\n\nI am clawft.".to_string();
        Identity::from_files(soul, identity, IdentitySource::Stub)
    }

    #[tokio::test]
    async fn build_includes_soul_identity_workspace_and_hash() {
        let id = ok_identity();
        let expected_soul = id.soul.clone();
        let expected_identity = id.identity.clone();
        let expected_hash = id.hash.clone();
        let provider = Arc::new(StubProvider(id));
        let builder = SystemPromptBuilder::new(provider, PathBuf::from("/tmp/ws"));

        let prompt = builder.build().await.expect("build ok");

        assert!(prompt.contains("[identity]"));
        assert!(prompt.contains(expected_soul.trim()));
        assert!(prompt.contains(expected_identity.trim()));
        assert!(prompt.contains("[binding-thread-status]\nok"));
        assert!(prompt.contains("[workspace]\n/tmp/ws"));
        assert!(prompt.contains("[hash]\n"));
        assert!(prompt.contains(&expected_hash));
    }

    #[tokio::test]
    async fn build_with_meta_exposes_identity_source_and_status() {
        // WEFT-328: loop needs the source label for AgentChatResult.
        // WEFT-342: also surfaces binding_thread_status.
        let id = ok_identity();
        assert_eq!(id.source, IdentitySource::Stub);
        let provider = Arc::new(StubProvider(id));
        let builder = SystemPromptBuilder::new(provider, PathBuf::from("/tmp/ws"));
        let built = builder.build_with_meta().await.expect("build ok");
        assert_eq!(built.identity_source, "stub");
        assert!(built.body.contains("[identity]"));
        assert_eq!(built.binding_thread_status, BindingThreadStatus::Ok);
    }

    #[tokio::test]
    async fn deny_mode_hard_refuses_on_mismatch() {
        // WEFT-342 default: mismatch → BindingThreadMismatch (hard refuse).
        let provider = Arc::new(StubProvider(mismatch_identity()));
        let builder = SystemPromptBuilder::new(provider, PathBuf::from("/tmp/ws"));
        assert!(builder.binding_thread_mode().is_deny());
        let err = builder.build().await.unwrap_err();
        assert!(matches!(err, IdentityError::BindingThreadMismatch));
        assert!(err.to_string().contains("binding-thread mismatch"));
    }

    #[tokio::test]
    async fn warn_only_mode_annotates_mismatch() {
        // WEFT-342 legacy path: annotate + continue.
        let provider = Arc::new(StubProvider(mismatch_identity()));
        let builder = SystemPromptBuilder::new(provider, PathBuf::from("/tmp/ws"))
            .with_binding_thread_mode(BindingThreadMode::WarnOnly);

        let built = builder.build_with_meta().await.expect("warn_only continues");
        assert_eq!(built.binding_thread_status, BindingThreadStatus::Mismatch);
        assert!(built.body.contains("[binding-thread-status]\nmismatch"));
        assert!(built.body.contains("degraded mode"));
    }

    #[tokio::test]
    async fn build_propagates_provider_error() {
        let provider = Arc::new(FailingProvider);
        let builder = SystemPromptBuilder::new(provider, PathBuf::from("/tmp"));
        let err = builder.build().await.unwrap_err();
        assert!(matches!(err, IdentityError::NotFound));
    }

    #[tokio::test]
    async fn build_surfaces_pending_journal_section() {
        // WEFT-96: every-turn journal consult is visible in the prompt.
        use crate::agent::soul_journal::PendingJournalEntry;
        let mut id = ok_identity();
        id.source = IdentitySource::Journal;
        id.pending_journal = vec![PendingJournalEntry {
            entry_id: "01HZ".into(),
            summary: "prefer terse replies".into(),
            content: "user wants shorter answers".into(),
            ts: "2026-01-01T00:00:00Z".into(),
            conv_id: "c1".into(),
            signal: "user_correction".into(),
        }];
        let provider = Arc::new(StubProvider(id));
        let builder = SystemPromptBuilder::new(provider, PathBuf::from("/tmp/ws"));
        let built = builder.build_with_meta().await.expect("build ok");
        assert_eq!(built.identity_source, "journal");
        assert!(built.body.contains("[soul-journal-pending]"));
        assert!(built.body.contains("count: 1"));
        assert!(built.body.contains("prefer terse replies"));
    }

    #[test]
    fn binding_status_helper_matches_constant() {
        let with_excerpt = format!("prefix\n{BINDING_THREAD_EXCERPT}\nsuffix");
        assert_eq!(
            BindingThreadStatus::from_soul(&with_excerpt),
            BindingThreadStatus::Ok
        );
        assert_eq!(
            BindingThreadStatus::from_soul("nothing relevant"),
            BindingThreadStatus::Mismatch
        );
    }

    #[tokio::test]
    async fn integrates_with_file_identity_provider() {
        let tmp = tempfile::tempdir().unwrap();
        let clawft = tmp.path().join(".clawft");
        std::fs::create_dir_all(&clawft).unwrap();
        std::fs::write(
            clawft.join("SOUL.md"),
            format!("# SOUL\n{BINDING_THREAD_EXCERPT}\n"),
        )
        .unwrap();
        std::fs::write(clawft.join("IDENTITY.md"), "# IDENTITY\nclawft").unwrap();

        let platform = Arc::new(clawft_platform::NativePlatform::new());
        let provider: Arc<dyn IdentityProvider> =
            Arc::new(FileIdentityProvider::new(tmp.path(), platform));
        let builder = SystemPromptBuilder::new(provider, tmp.path().to_path_buf());

        let prompt = builder.build().await.expect("build ok");
        assert!(prompt.contains("[binding-thread-status]\nok"));
        assert!(prompt.contains(&tmp.path().display().to_string()));
    }
}
