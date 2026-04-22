//! State-delta vocabulary — the three verbs every adapter speaks.
//!
//! ADR-017 §1: `Append`, `Replace`, `Remove`. The substrate is a flat
//! path-keyed map, so `path` here is the **absolute** topic-rooted path
//! (`substrate/kernel/processes/by-pid/1412`) rather than the
//! JSON-Pointer relative form sketched in the ADR. Adapters emit
//! absolute paths so the composer never has to know which adapter a
//! delta came from.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One mutation of the substrate state tree.
///
/// The composer applies these in order to [`crate::Substrate`]. The
/// ADR-004 state-diff tree consumes the same vocabulary, so adapter
/// deltas and composer-authored deltas are interchangeable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum StateDelta {
    /// Append a value to a list-shaped path. Used for logs and other
    /// append-only streams. If the path does not yet exist, a new array
    /// is created. If the path exists and is not an array, the delta is
    /// treated as [`StateDelta::Replace`] to avoid losing data.
    Append {
        /// Absolute topic-rooted path (`substrate/kernel/logs`).
        path: String,
        /// The value to append.
        value: Value,
    },
    /// Replace the value at a path wholesale. This is the default for
    /// singletons and list-by-id collections where individual entries
    /// carry a stable key (`.../by-pid/1412`, `.../by-name/mesh`).
    Replace {
        /// Absolute topic-rooted path.
        path: String,
        /// The replacement value.
        value: Value,
    },
    /// Drop the value at a path. For list-by-id collections this
    /// removes a single entry; for singletons it clears the cell.
    Remove {
        /// Absolute topic-rooted path.
        path: String,
    },
}

impl StateDelta {
    /// The path this delta targets. Convenience accessor for composers
    /// that dispatch on path prefix.
    pub fn path(&self) -> &str {
        match self {
            StateDelta::Append { path, .. }
            | StateDelta::Replace { path, .. }
            | StateDelta::Remove { path } => path,
        }
    }
}
