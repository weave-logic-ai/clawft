//! Obsidian vault cultivation — frontmatter enrichment, wikilink extraction,
//! link analysis, connection suggestion, and vault-domain hyperedge pipeline
//! (SUGGEST → ratify → CRDT apply; WEFT-378).

pub mod analyze;
pub mod crdt;
pub mod frontmatter;
pub mod hyperedge;
pub mod links;
pub mod pipeline;
pub mod suggest;

pub use analyze::{BrokenLink, VaultMetrics, VaultNode, analyze_vault};
pub use crdt::{CrdtDelta, CrdtEntry, CrdtError, CrdtHyperedgeStore, MergeStats};
pub use hyperedge::{
    HyperedgeKind, HyperedgeProposal, HyperedgeSuggestConfig, ProposalLifecycle, proposal_id,
    suggest_hyperedges,
};
pub use pipeline::{
    ApplyStats, RatifyView, VAULT_DOMAIN, VaultHyperedgePipeline, suggest_ratify_apply,
    vault_entity_id,
};
pub use suggest::{SuggestConfig, Suggestion, suggest_links};
