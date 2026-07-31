//! Optional link from a spatial leaf payload to an external feature index.
//!
//! **ADR-088 / design note `docs/design/bvh_schema_updates.md`**: leaves stay
//! geometric; similarity lives in HNSW (or another vector backend). A
//! [`VectorRef`] is an *optional handle* so WEFT-709 world-model leaves can
//! later attach embeddings without a payload migration.
//!
//! - `None` — pure geometry (default; all existing CBOR remains valid).
//! - `Some` — opaque `(index_id, vector_id)` for a join outside the BVH.

use serde::{Deserialize, Serialize};

/// Well-known [`VectorRef::index_id`] values (extensible; do not renumber).
pub mod index_ids {
    /// Default ECC / kernel HNSW service (`hnsw_service` / `VectorBackend`).
    pub const ECC_HNSW: u32 = 0;
    /// Reserved for future visual / DINO-style feature indices.
    pub const VISUAL_FEATURES: u32 = 1;
    /// Reserved for language-grounded CLIP/SigLIP-style indices.
    pub const LANGUAGE_FEATURES: u32 = 2;
}

/// Handle into an external feature index (not an inline embedding).
///
/// The BVH never interprets `vector_id`; consumers resolve it against the
/// named index. Dimensionality and embedder choice are **out of scope**
/// for the leaf schema (ADR-088).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VectorRef {
    /// Which index / backend namespace (see [`index_ids`]).
    #[serde(default)]
    pub index_id: u32,
    /// Opaque id within that index (HNSW entry key, store row, …).
    pub vector_id: u64,
}

impl VectorRef {
    /// Construct a ref into the default ECC HNSW index.
    pub const fn ecc_hnsw(vector_id: u64) -> Self {
        Self {
            index_id: index_ids::ECC_HNSW,
            vector_id,
        }
    }

    /// Construct a ref into an explicit index namespace.
    pub const fn new(index_id: u32, vector_id: u64) -> Self {
        Self {
            index_id,
            vector_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{decode, encode};

    #[test]
    fn vector_ref_roundtrip() {
        let v = VectorRef::ecc_hnsw(42);
        let bytes = encode(&v).unwrap();
        assert_eq!(decode::<VectorRef>(&bytes).unwrap(), v);
    }

    #[test]
    fn option_none_is_default_serde() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Wrapper {
            #[serde(default, skip_serializing_if = "Option::is_none")]
            vector: Option<VectorRef>,
        }
        let empty = Wrapper { vector: None };
        let bytes = encode(&empty).unwrap();
        assert_eq!(decode::<Wrapper>(&bytes).unwrap(), empty);
        // Field omitted (skip_serializing) round-trips as None.
        let again = decode::<Wrapper>(&bytes).unwrap();
        assert_eq!(again.vector, None);
    }
}
