//! WeftOS agent service — daemon-side wrapper around
//! [`clawft_core::agent::AgentLoop`] that owns per-conversation locking,
//! cancellation tokens, and a single `dispatch` entry point shaped for
//! the `agent.chat` JSON-RPC handler.
//!
//! # Why this crate exists
//!
//! `clawft-core::agent::AgentLoop` is the production tool loop (the
//! Rust translation of nanobot referenced in the agent module headers).
//! Today (commit `e6f8c816`), the `agent.chat` RPC inlines a fresh
//! ~360-line tool loop in `clawft-weave::daemon::handle_agent_chat`
//! — the "vertical-slice spike". The plan in
//! `docs/plans/agent-core-v1.md` retires that spike in favor of having
//! the daemon delegate to a service that drives `AgentLoop::handle_turn`
//! per request, so the panel and the CLI share one execution core.
//!
//! This crate is the home of that service.
//!
//! # Crate layout
//!
//! - [`service`] — [`AgentService`], the per-daemon dispatcher with
//!   per-conv locks and cancel tokens.
//! - [`protocol`] — [`AgentChatParams`], [`AgentChatResult`], and the
//!   helper message/tool-call structs. Phase C2 will delete the
//!   duplicate definitions from `clawft-weave::protocol` and re-export
//!   from here.
//!
//! # Phasing
//!
//! - **C1 (this commit)** — skeleton: in-memory defaults
//!   ([`NoopGate`](clawft_core::agent::gate::NoopGate),
//!   [`InMemorySink`](clawft_core::agent::sink::InMemorySink) — both
//!   inherited from B1/B2). Public surface fixed; tests cover the
//!   lock / cancel / shutdown semantics with a stubbed
//!   [`AgentLoopHandle`](crate::service::AgentLoopHandle).
//! - **C2** — daemon wiring behind the `agent-core-chat` feature flag.
//! - **C3** — substrate-backed `ConversationSink`.
//! - **D2** — kernel-backed [`EffectGate`](clawft_core::agent::gate::EffectGate).
//! - **D3** — flag flip; spike deletion.

#![deny(rust_2018_idioms)]
#![warn(missing_docs)]

pub mod atom_registry;
pub mod enrich_queue;
pub mod dialogue_act;
pub mod enrichment_classifier;
pub mod interrupt_router;
pub mod kernel_gate;
pub mod protocol;
pub mod service;
mod session_forest;
pub mod session_tier;
pub mod spawn_registry;
pub mod subagent;
pub mod routing_log;
pub mod soul_journal;
pub mod substrate_budget;
pub mod substrate_sink;
pub mod text_structure;
pub mod turn_classifier;
pub mod verbalize;

pub use atom_registry::{
    AtomKey, AtomLocator, AtomRegistry, ConsistencyIssue, ConsistencyReport, Disposition,
    IssueSeverity, ProjectionAuditRow,
};
pub use enrich_queue::{EnrichJob, EnrichQueue};
pub use dialogue_act::{Act, Intent, RefinedAct, SpeechActClass};
pub use enrichment_classifier::{
    parse_enrichment_envelope, EnrichmentClassifier, DEFAULT_ENRICHMENT_MAX_TOKENS,
    ENRICHMENT_SYSTEM_PROMPT,
};
pub use interrupt_router::{
    InterruptAction, InterruptCtx, InterruptOutcome, InterruptRouter, InterruptSignals,
};
pub use kernel_gate::KernelEffectGate;
pub use text_structure::{Argument, EntityKind, EntitySpan, Structure, UtteranceShape};
pub use protocol::{
    AgentChatMessage, AgentChatParams, AgentChatResult, AgentChatToolCall, ChainAppendParams,
    ChainAppendResult, WitnessRecord,
};
pub use service::{AgentLoopHandle, AgentService, AgentServiceError, ReplySubmitter};
pub use session_tier::SessionTier;
pub use routing_log::SubstrateRouterDecisionLog;
pub use soul_journal::SubstrateSoulJournal;
pub use spawn_registry::{SharedSpawnRegistry, SpawnRegistry, TaskRecord};
pub use subagent::{DaemonSubagentSpawner, SubagentConfig, SubagentForest};
pub use substrate_budget::SubstrateBudgetStore;
pub use turn_classifier::{
    arousal_of, ClassificationVector, KeywordTurnClassifier, Tier, TurnClassifier, Vad,
};
pub use verbalize::{
    verbalize_classification, verbalize_classification_value, verbalize_turn,
    verbalize_voice_analysis,
};
pub use substrate_sink::{
    AudioRef, HEARTBEAT_PERIOD, KernelSubstrateClient, KernelTurnAnchor, NoopTurnAnchor,
    SubstrateClient, SubstrateConversationSink, TurnAnchor, TurnContent, TurnContentPart,
};
