//! Daemon RPC handlers for `ecc.spatial.*` (WEFT-720 / ADR-056 Phase E).
//!
//! Holds a process-local [`BvhStore`] (scaffold). Phase C moves this behind
//! `SpatialService` + ExoChain `ChainSink`; the CLI method names stay stable.

use std::sync::{Arc, Mutex, OnceLock};

use clawft_bvh::{
    Aabb, BranchId, BranchMeta, BvhStore, Leaf, MembershipFilter, Ray, Vec3,
    apply_membership_filter, parse_aabb6, parse_identity, parse_tag, parse_vec3,
};

use crate::protocol::Response;

static DAEMON_SPATIAL: OnceLock<Arc<Mutex<BvhStore>>> = OnceLock::new();

/// Shared spatial store for the daemon (lazy-init empty main branch).
pub fn spatial_store() -> Arc<Mutex<BvhStore>> {
    DAEMON_SPATIAL
        .get_or_init(|| Arc::new(Mutex::new(BvhStore::new())))
        .clone()
}

/// Reset store to empty (integration tests).
#[cfg(test)]
pub fn reset_spatial_store_for_tests() {
    if let Some(s) = DAEMON_SPATIAL.get() {
        let mut g = s.lock().expect("spatial lock");
        *g = BvhStore::new();
    }
}

fn branch_id(v: Option<&serde_json::Value>) -> BranchId {
    BranchId(v.and_then(|x| x.as_u64()).unwrap_or(0))
}

fn parse_filter_tags(params: &serde_json::Value) -> Result<MembershipFilter, String> {
    match params.get("filter_tags").and_then(|v| v.as_str()) {
        None | Some("") => Ok(MembershipFilter::none()),
        Some(s) => {
            let mut tags = Vec::new();
            for part in s.split(',') {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                tags.push(parse_tag(part)?);
            }
            Ok(MembershipFilter::tags(tags))
        }
    }
}

fn status_json(store: &BvhStore) -> serde_json::Value {
    let branches: Vec<serde_json::Value> = store
        .list_branches()
        .into_iter()
        .map(|(id, meta, leaves)| {
            serde_json::json!({
                "id": id.0,
                "name": meta.name,
                "note": meta.note,
                "leaves": leaves,
            })
        })
        .collect();
    serde_json::json!({
        "backend": store.backend_name(),
        "epoch": store.epoch(),
        "len": store.len(),
        "chain_events": store.events().len(),
        "branches": branches,
        "oq4": "membership filters applied as post-query MembershipFilter wrapper",
    })
}

/// Dispatch one `ecc.spatial.*` method.
pub fn handle_spatial(method: &str, params: &serde_json::Value) -> Response {
    let store = spatial_store();
    let mut guard = match store.lock() {
        Ok(g) => g,
        Err(e) => return Response::error(format!("spatial lock poisoned: {e}")),
    };

    match method {
        "ecc.spatial.status" => Response::success(status_json(&guard)),
        "ecc.spatial.insert" => {
            let tag_s = params
                .get("tag")
                .and_then(|v| v.as_str())
                .unwrap_or("wm_object");
            let aabb_s = match params.get("aabb").and_then(|v| v.as_str()) {
                Some(s) => s,
                None => return Response::error("missing aabb"),
            };
            let identity_s = params
                .get("identity")
                .and_then(|v| v.as_str())
                .unwrap_or("object");
            let branch = branch_id(params.get("branch"));

            let tag = match parse_tag(tag_s) {
                Ok(t) => t,
                Err(e) => return Response::error(format!("tag: {e}")),
            };
            let bound = match parse_aabb6(aabb_s) {
                Ok(b) => b,
                Err(e) => return Response::error(format!("aabb: {e}")),
            };
            let identity = match parse_identity(identity_s) {
                Ok(i) => i,
                Err(e) => return Response::error(e),
            };
            let payload = match params.get("payload").and_then(|v| v.as_str()) {
                None | Some("") => Vec::new(),
                Some(hex) => match hex::decode(hex.trim_start_matches("0x")) {
                    Ok(b) => b,
                    Err(e) => return Response::error(format!("payload hex: {e}")),
                },
            };
            let leaf = Leaf::new(bound, identity, tag, payload);
            match guard.insert(branch, leaf) {
                Ok(id) => Response::success(serde_json::json!({
                    "leaf_id": id.0,
                    "branch": branch.0,
                    "epoch": guard.epoch(),
                })),
                Err(e) => Response::error(e.to_string()),
            }
        }
        "ecc.spatial.query" => {
            let kind = params
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("aabb");
            let branch = branch_id(params.get("branch"));
            let filter = match parse_filter_tags(params) {
                Ok(f) => f,
                Err(e) => return Response::error(e),
            };
            let limit = params
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            let result = match kind {
                "point" => {
                    let at = match params.get("at").and_then(|v| v.as_str()) {
                        Some(s) => match parse_vec3(s) {
                            Ok(p) => p,
                            Err(e) => return Response::error(e),
                        },
                        None => return Response::error("missing at"),
                    };
                    guard.query_point(branch, at)
                }
                "aabb" => {
                    let region = match params.get("region").and_then(|v| v.as_str()) {
                        Some(s) => match parse_aabb6(s) {
                            Ok(b) => b,
                            Err(e) => return Response::error(e),
                        },
                        None => return Response::error("missing region"),
                    };
                    guard.query_aabb(branch, region)
                }
                "sphere" => {
                    let center = match params.get("center").and_then(|v| v.as_str()) {
                        Some(s) => match parse_vec3(s) {
                            Ok(p) => p,
                            Err(e) => return Response::error(e),
                        },
                        None => return Response::error("missing center"),
                    };
                    let radius = params
                        .get("radius")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.0) as f32;
                    guard.query_sphere(branch, center, radius)
                }
                "ray" => {
                    let origin = match params.get("origin").and_then(|v| v.as_str()) {
                        Some(s) => match parse_vec3(s) {
                            Ok(p) => p,
                            Err(e) => return Response::error(e),
                        },
                        None => return Response::error("missing origin"),
                    };
                    let dir = match params.get("dir").and_then(|v| v.as_str()) {
                        Some(s) => match parse_vec3(s) {
                            Ok(p) => p,
                            Err(e) => return Response::error(e),
                        },
                        None => return Response::error("missing dir"),
                    };
                    let max_t = params
                        .get("max_t")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.0e6) as f32;
                    match guard.query_ray(branch, Ray::new(origin, dir), max_t) {
                        Ok(hits) => {
                            let hits: Vec<_> = hits
                                .into_iter()
                                .map(|h| {
                                    serde_json::json!({
                                        "leaf_id": h.id.0,
                                        "t": h.t,
                                    })
                                })
                                .collect();
                            return Response::success(serde_json::json!({
                                "kind": "ray",
                                "branch": branch.0,
                                "hits": hits,
                            }));
                        }
                        Err(e) => return Response::error(e.to_string()),
                    }
                }
                "knn" => {
                    let at = match params.get("at").and_then(|v| v.as_str()) {
                        Some(s) => match parse_vec3(s) {
                            Ok(p) => p,
                            Err(e) => return Response::error(e),
                        },
                        None => return Response::error("missing at"),
                    };
                    let k = params.get("k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
                    guard.query_knn(branch, at, k)
                }
                other => return Response::error(format!("unknown query kind {other:?}")),
            };

            match result {
                Ok(mut ids) => {
                    ids = apply_membership_filter(&guard, branch, ids, &filter);
                    if limit > 0 && ids.len() > limit {
                        ids.truncate(limit);
                    }
                    let hits: Vec<_> = ids
                        .into_iter()
                        .map(|id| serde_json::json!({"leaf_id": id.0}))
                        .collect();
                    Response::success(serde_json::json!({
                        "kind": kind,
                        "branch": branch.0,
                        "hits": hits,
                    }))
                }
                Err(e) => Response::error(e.to_string()),
            }
        }
        "ecc.spatial.branch.derive" => {
            let parent = branch_id(params.get("parent"));
            let name = params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("branch")
                .to_string();
            let note = params
                .get("note")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let meta = BranchMeta { name: name.clone(), note };
            match guard.derive_branch(parent, meta) {
                Ok(child) => Response::success(serde_json::json!({
                    "branch_id": child.0,
                    "parent": parent.0,
                    "name": name,
                    "epoch": guard.epoch(),
                })),
                Err(e) => Response::error(e.to_string()),
            }
        }
        "ecc.spatial.branch.list" => {
            let branches: Vec<_> = guard
                .list_branches()
                .into_iter()
                .map(|(id, meta, leaves)| {
                    serde_json::json!({
                        "id": id.0,
                        "name": meta.name,
                        "leaves": leaves,
                    })
                })
                .collect();
            Response::success(serde_json::json!({ "branches": branches }))
        }
        "ecc.spatial.diff" => {
            let a = branch_id(params.get("a"));
            let b = branch_id(params.get("b"));
            let region = match params.get("region").and_then(|v| v.as_str()) {
                Some(s) => match parse_aabb6(s) {
                    Ok(bb) => bb,
                    Err(e) => return Response::error(e),
                },
                None => {
                    // Full space default
                    Aabb::from_min_max(
                        Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
                        Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
                    )
                }
            };
            match guard.branch_diff(a, b, region) {
                Ok(entries) => {
                    let entries: Vec<_> = entries
                        .into_iter()
                        .map(|e| {
                            serde_json::json!({
                                "leaf_id": e.leaf_id.0,
                                "kind": e.kind,
                            })
                        })
                        .collect();
                    Response::success(serde_json::json!({
                        "a": a.0,
                        "b": b.0,
                        "entries": entries,
                    }))
                }
                Err(e) => Response::error(e.to_string()),
            }
        }
        "ecc.spatial.events" => {
            let limit = params
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let events = guard.events();
            let slice = if limit == 0 || limit >= events.len() {
                events
            } else {
                &events[events.len() - limit..]
            };
            Response::success(serde_json::json!({
                "count": events.len(),
                "events": slice,
            }))
        }
        "ecc.spatial.replay" => {
            let events = guard.events().to_vec();
            let n = events.len();
            let replayed = BvhStore::replay(&events);
            let ok = replayed.len() == guard.len()
                && replayed.list_branches().len() == guard.list_branches().len();
            // Replace live store with replayed state to prove round-trip.
            let epoch = replayed.epoch();
            let len = replayed.len();
            *guard = replayed;
            Response::success(serde_json::json!({
                "ok": ok,
                "events_replayed": n,
                "epoch": epoch,
                "len": len,
            }))
        }
        "ecc.spatial.remove" => {
            let branch = branch_id(params.get("branch"));
            let leaf_id = match params.get("leaf_id").and_then(|v| v.as_u64()) {
                Some(id) => clawft_bvh::LeafId(id),
                None => return Response::error("missing leaf_id"),
            };
            match guard.remove(branch, leaf_id) {
                Ok(removed) => Response::success(serde_json::json!({
                    "removed": removed,
                    "leaf_id": leaf_id.0,
                    "branch": branch.0,
                })),
                Err(e) => Response::error(e.to_string()),
            }
        }
        _ => Response::error(format!("unknown spatial method {method}")),
    }
}

