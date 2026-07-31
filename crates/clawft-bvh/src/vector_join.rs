//! BVH × HNSW Phase F join helpers (WEFT-723 / ADR-093).
//!
//! The BVH answers **where** (spatial query → leaf ids). Feature similarity
//! stays on HNSW / `VectorBackend`. This module only composes handles:
//!
//! 1. **Spatial-first**: after a spatial hit list, decode optional
//!    [`VectorRef`] from each leaf's CBOR payload and attach them.
//! 2. **Feature-first**: filter HNSW hits whose metadata / key maps to a
//!    leaf id that also appears in a spatial candidate set.
//!
//! No embeddings are stored or interpreted inside the BVH.

use crate::leaf::LeafId;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use weftos_leaf_types::spatial::VectorRef;

/// One leaf after a spatial query, with optional external feature handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpatialJoinHit {
    /// Leaf id from the BVH.
    pub leaf_id: LeafId,
    /// Decoded payload handle (ADR-088); `None` = pure geometry.
    pub vector: Option<VectorRef>,
}

/// Minimal CBOR map shape: only the optional `vector` field.
#[derive(Debug, Deserialize)]
struct PayloadVectorField {
    #[serde(default)]
    vector: Option<VectorRef>,
}

/// Decode an optional [`VectorRef`] from a spatial leaf CBOR payload.
///
/// Compatible with every `weftos-leaf-types` spatial payload that carries
/// `vector: Option<VectorRef>` (serde default / skip-if-none).
pub fn vector_ref_from_payload(payload: &[u8]) -> Option<VectorRef> {
    if payload.is_empty() {
        return None;
    }
    weftos_leaf_types::decode::<PayloadVectorField>(payload)
        .ok()
        .and_then(|p| p.vector)
}

/// Spatial-first join: attach `VectorRef` handles to spatial hits.
///
/// `payloads` maps leaf id → opaque CBOR bytes (as stored on `Leaf.payload`).
pub fn join_spatial_with_vector_refs(
    spatial_hits: &[LeafId],
    payloads: &HashMap<LeafId, Vec<u8>>,
) -> Vec<SpatialJoinHit> {
    spatial_hits
        .iter()
        .map(|&leaf_id| {
            let vector = payloads
                .get(&leaf_id)
                .and_then(|bytes| vector_ref_from_payload(bytes));
            SpatialJoinHit { leaf_id, vector }
        })
        .collect()
}

/// A feature-index hit that may be joined back to a leaf.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureHit {
    /// Opaque vector id within an index (matches `VectorRef.vector_id`).
    pub vector_id: u64,
    /// Optional leaf id when HNSW metadata recorded it.
    pub leaf_id: Option<LeafId>,
    /// Distance from the query embedding (lower is closer).
    pub distance: f32,
}

/// Dual-index hit after composition.
#[derive(Debug, Clone, PartialEq)]
pub struct DualIndexHit {
    /// Leaf id (spatial identity).
    pub leaf_id: LeafId,
    /// Feature handle when known.
    pub vector: Option<VectorRef>,
    /// Feature distance when this row came from (or matched) HNSW.
    pub feature_distance: Option<f32>,
}

/// Feature-first join: keep HNSW hits whose leaf is in the spatial set.
///
/// Use after a spatial prefilter (e.g. `query_aabb` / `query_knn`) to
/// implement "similar features **near here**".
pub fn filter_feature_hits_by_spatial(
    feature_hits: &[FeatureHit],
    spatial_leaf_ids: &HashSet<LeafId>,
    default_index_id: u32,
) -> Vec<DualIndexHit> {
    feature_hits
        .iter()
        .filter_map(|h| {
            let leaf_id = h.leaf_id?;
            if !spatial_leaf_ids.contains(&leaf_id) {
                return None;
            }
            Some(DualIndexHit {
                leaf_id,
                vector: Some(VectorRef::new(default_index_id, h.vector_id)),
                feature_distance: Some(h.distance),
            })
        })
        .collect()
}

/// Spatial-first rank: among spatial hits that have handles, order by
/// optional feature distance map (`vector_id → distance`).
///
/// Leaves without a handle sort last (geometry-only).
pub fn rank_spatial_hits_by_feature_distance(
    spatial: &[SpatialJoinHit],
    feature_distance: &HashMap<u64, f32>,
) -> Vec<DualIndexHit> {
    let mut out: Vec<DualIndexHit> = spatial
        .iter()
        .map(|h| {
            let feature_distance = h
                .vector
                .and_then(|v| feature_distance.get(&v.vector_id).copied());
            DualIndexHit {
                leaf_id: h.leaf_id,
                vector: h.vector,
                feature_distance,
            }
        })
        .collect();
    out.sort_by(|a, b| match (a.feature_distance, b.feature_distance) {
        (Some(da), Some(db)) => da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.leaf_id.0.cmp(&b.leaf_id.0),
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use weftos_leaf_types::spatial::{AabbWire, Vec3Wire, WmObjectPayload, VectorRef};
    use weftos_leaf_types::{decode, encode};

    fn unit_aabb() -> AabbWire {
        AabbWire::from_min_max(Vec3Wire::new(0.0, 0.0, 0.0), Vec3Wire::new(1.0, 1.0, 1.0))
    }

    #[test]
    fn decode_vector_ref_from_wm_object_payload() {
        let p = WmObjectPayload {
            label: Some("chair".into()),
            confidence: Some(0.9),
            bound: unit_aabb(),
            vector: Some(VectorRef::ecc_hnsw(42)),
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(vector_ref_from_payload(&bytes), Some(VectorRef::ecc_hnsw(42)));
        // Round-trip full payload still works.
        assert_eq!(decode::<WmObjectPayload>(&bytes).unwrap().vector, p.vector);
    }

    #[test]
    fn missing_vector_is_none() {
        let p = WmObjectPayload {
            label: None,
            confidence: None,
            bound: unit_aabb(),
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(vector_ref_from_payload(&bytes), None);
    }

    #[test]
    fn spatial_first_then_rank_by_feature() {
        let id0 = LeafId(0);
        let id1 = LeafId(1);
        let id2 = LeafId(2);
        let mut payloads = HashMap::new();
        let p0 = WmObjectPayload {
            label: Some("a".into()),
            confidence: None,
            bound: unit_aabb(),
            vector: Some(VectorRef::ecc_hnsw(100)),
        };
        let p1 = WmObjectPayload {
            label: Some("b".into()),
            confidence: None,
            bound: unit_aabb(),
            vector: Some(VectorRef::ecc_hnsw(200)),
        };
        let p2 = WmObjectPayload {
            label: Some("c".into()),
            confidence: None,
            bound: unit_aabb(),
            vector: None,
        };
        payloads.insert(id0, encode(&p0).unwrap());
        payloads.insert(id1, encode(&p1).unwrap());
        payloads.insert(id2, encode(&p2).unwrap());

        let joined = join_spatial_with_vector_refs(&[id0, id1, id2], &payloads);
        assert_eq!(joined.len(), 3);
        assert_eq!(joined[0].vector, Some(VectorRef::ecc_hnsw(100)));
        assert_eq!(joined[2].vector, None);

        let mut dist = HashMap::new();
        dist.insert(100, 0.9); // farther
        dist.insert(200, 0.1); // closer
        let ranked = rank_spatial_hits_by_feature_distance(&joined, &dist);
        assert_eq!(ranked[0].leaf_id, id1); // distance 0.1
        assert_eq!(ranked[1].leaf_id, id0); // distance 0.9
        assert_eq!(ranked[2].leaf_id, id2); // no feature
        assert_eq!(ranked[0].feature_distance, Some(0.1));
    }

    #[test]
    fn feature_first_filtered_by_spatial_set() {
        let spatial: HashSet<LeafId> = [LeafId(1), LeafId(2)].into_iter().collect();
        let hits = vec![
            FeatureHit {
                vector_id: 10,
                leaf_id: Some(LeafId(1)),
                distance: 0.2,
            },
            FeatureHit {
                vector_id: 11,
                leaf_id: Some(LeafId(99)), // outside spatial set
                distance: 0.05,
            },
            FeatureHit {
                vector_id: 12,
                leaf_id: None,
                distance: 0.01,
            },
        ];
        let dual = filter_feature_hits_by_spatial(&hits, &spatial, 0);
        assert_eq!(dual.len(), 1);
        assert_eq!(dual[0].leaf_id, LeafId(1));
        assert_eq!(dual[0].vector, Some(VectorRef::ecc_hnsw(10)));
    }
}
