//! Exo-Resource-Tree: hierarchical resource namespace for WeftOS.
//!
//! Provides a tree-structured resource namespace with:
//! - CRUD operations on typed resource nodes
//! - Merkle hash integrity (bottom-up recomputation)
//! - DAG-backed mutation log for audit trail
//! - Bootstrap from checkpoint or fresh namespace
//!
//! # K0 Scope
//! Tree CRUD, Merkle, mutation log, bootstrap.
//!
//! # K1 Scope (stubs in K0)
//! Permission engine, delegation certificates.

pub mod boot;
pub mod delegation;
pub mod error;
pub mod model;
pub mod mutation;
pub mod permission;
pub mod scoring;
pub mod tree;

pub use boot::{bootstrap_fresh, from_checkpoint, to_checkpoint};
pub use delegation::DelegationCert;
pub use error::{TreeError, TreeResult};
pub use model::{Action, ResourceId, ResourceKind, ResourceNode, Role};
pub use mutation::{MutationEvent, MutationLog};
pub use permission::{check as check_permission, Decision};
pub use scoring::NodeScoring;
pub use tree::ResourceTree;
