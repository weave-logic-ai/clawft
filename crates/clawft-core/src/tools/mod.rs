//! Tool system: registry, trait, execution, per-tool permit proofs.

pub mod permit;
pub mod registry;

pub use permit::{
    present_proof, present_proof_at, PermitIssuer, PermitProofError, ToolPermitToken,
    DEFAULT_PERMIT_TTL_NS,
};
