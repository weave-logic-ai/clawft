//! Error types for the exo-resource-tree crate.

use thiserror::Error;

use crate::model::ResourceId;

#[derive(Debug, Error)]
pub enum TreeError {
    #[error("resource not found: {id}")]
    NotFound { id: ResourceId },

    #[error("resource already exists: {id}")]
    AlreadyExists { id: ResourceId },

    #[error("parent not found: {parent_id}")]
    ParentNotFound { parent_id: ResourceId },

    #[error("cannot remove non-empty node: {id} has {child_count} children")]
    NotEmpty { id: ResourceId, child_count: usize },

    #[error("invalid path: {reason}")]
    InvalidPath { reason: String },

    #[error("checkpoint error: {0}")]
    Checkpoint(String),
}

pub type TreeResult<T> = Result<T, TreeError>;
