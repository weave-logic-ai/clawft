//! Minimal ontology snapshot for the composer runtime.
//!
//! An [`OntologySnapshot`] maps slash-delimited topic paths
//! (`substrate/kernel/processes`) to JSON values. The real kernel
//! adapter (sibling M1.5-C stream) owns the live topic bus; this
//! crate accepts a pre-snapshotted view so the surface composer can
//! run without taking a dependency on the adapter trait.
//!
//! TODO(m1.6+): replace this with the real `clawft-kernel-adapter`
//! substrate handle when that crate lands.

use std::collections::BTreeMap;

use serde_json::Value as Json;

/// Keyed by full topic path (`substrate/kernel/services`). Values are
/// opaque JSON trees; the evaluator reads them via [`crate::eval`].
#[derive(Clone, Debug, Default)]
pub struct OntologySnapshot {
    topics: BTreeMap<String, Json>,
}

impl OntologySnapshot {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn put(&mut self, key: impl Into<String>, value: Json) -> &mut Self {
        self.topics.insert(key.into(), value);
        self
    }

    pub fn with(mut self, key: impl Into<String>, value: Json) -> Self {
        self.put(key, value);
        self
    }

    /// Read a path. The path is looked up as a single key first
    /// (direct topic). If that misses, we walk prefix/suffix splits
    /// so a binding like `$substrate/kernel/services/mesh/cpu` can
    /// resolve against a topic `substrate/kernel/services` whose
    /// JSON contains `{"mesh": {"cpu": …}}`.
    pub fn read(&self, path: &str) -> Option<Json> {
        if let Some(v) = self.topics.get(path) {
            return Some(v.clone());
        }
        // Try progressively shorter prefixes.
        let segs: Vec<&str> = path.split('/').collect();
        for cut in (1..segs.len()).rev() {
            let prefix = segs[..cut].join("/");
            if let Some(v) = self.topics.get(&prefix) {
                let tail = &segs[cut..];
                return walk_json(v, tail).cloned();
            }
        }
        None
    }

    pub fn topics(&self) -> impl Iterator<Item = (&String, &Json)> {
        self.topics.iter()
    }
}

fn walk_json<'a>(v: &'a Json, path: &[&str]) -> Option<&'a Json> {
    let mut cur = v;
    for seg in path {
        cur = match cur {
            Json::Object(map) => map.get(*seg)?,
            Json::Array(arr) => {
                let idx: usize = seg.parse().ok()?;
                arr.get(idx)?
            }
            _ => return None,
        };
    }
    Some(cur)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn read_direct_topic() {
        let mut s = OntologySnapshot::empty();
        s.put("substrate/kernel/status", json!({"state": "healthy"}));
        let v = s.read("substrate/kernel/status").unwrap();
        assert_eq!(v, json!({"state": "healthy"}));
    }

    #[test]
    fn read_nested_via_prefix() {
        let mut s = OntologySnapshot::empty();
        s.put(
            "substrate/kernel/services",
            json!({"mesh": {"cpu": 42}, "ws": {"cpu": 8}}),
        );
        let v = s.read("substrate/kernel/services/mesh/cpu").unwrap();
        assert_eq!(v, json!(42));
    }

    #[test]
    fn missing_returns_none() {
        let s = OntologySnapshot::empty();
        assert!(s.read("substrate/nope").is_none());
    }
}
