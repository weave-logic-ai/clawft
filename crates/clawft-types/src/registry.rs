//! Unified registry trait for key-value registries across WeftOS.
//!
//! Many subsystems maintain registries that map a string key to some
//! value (services, tools, pipes, workspaces, etc.).  This module
//! provides a common read-side trait so GUI introspection, health
//! checks, and cross-crate tooling can enumerate registry contents
//! without depending on each concrete type.
//!
//! # Design notes
//!
//! The write side (`register`, `unregister`) is intentionally excluded
//! from the trait because the concrete registries diverge significantly:
//!
//! - Some derive the key from the value (`service.name()`).
//! - Some auto-generate the key (`ProcessTable::allocate_pid`).
//! - Some require `&mut self`, others use interior mutability (`DashMap`).
//!
//! The read side, however, is consistent: look up by key, list keys,
//! check membership, count entries.

/// Read-side interface shared by all key-value registries.
///
/// Implementors expose a uniform way to inspect registry contents
/// without coupling callers to concrete storage (DashMap, HashMap,
/// Vec, etc.).
///
/// The `Value` associated type is the *returned* value, which may
/// differ from the stored value (e.g., `Arc<dyn Trait>` vs `&T`).
///
/// # Examples
///
/// ```ignore
/// fn dump_registry(reg: &dyn Registry<Value = String>) {
///     for key in reg.list_keys() {
///         if let Some(val) = reg.get(&key) {
///             println!("{key}: {val}");
///         }
///     }
/// }
/// ```
pub trait Registry {
    /// The value returned by [`get`](Registry::get).
    ///
    /// Use an owned type (`Arc<T>`, `T: Clone`) when the registry
    /// uses interior mutability and cannot hand out references.
    type Value;

    /// Look up a value by its string key.
    fn get(&self, key: &str) -> Option<Self::Value>;

    /// Return all keys currently in the registry.
    ///
    /// The order is unspecified unless a concrete implementation
    /// documents otherwise.
    fn list_keys(&self) -> Vec<String>;

    /// Check whether `key` is present.
    ///
    /// Default implementation delegates to [`get`](Registry::get),
    /// but concrete types should override when a cheaper check exists.
    fn contains(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Number of entries in the registry.
    fn count(&self) -> usize {
        self.list_keys().len()
    }

    /// Whether the registry is empty.
    fn is_empty(&self) -> bool {
        self.count() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Minimal concrete registry for testing the trait.
    struct SimpleRegistry {
        map: BTreeMap<String, String>,
    }

    impl Registry for SimpleRegistry {
        type Value = String;

        fn get(&self, key: &str) -> Option<String> {
            self.map.get(key).cloned()
        }

        fn list_keys(&self) -> Vec<String> {
            self.map.keys().cloned().collect()
        }

        fn count(&self) -> usize {
            self.map.len()
        }
    }

    #[test]
    fn empty_registry() {
        let reg = SimpleRegistry {
            map: BTreeMap::new(),
        };
        assert!(reg.is_empty());
        assert_eq!(reg.count(), 0);
        assert!(reg.list_keys().is_empty());
        assert!(!reg.contains("anything"));
        assert!(reg.get("anything").is_none());
    }

    #[test]
    fn basic_operations() {
        let mut map = BTreeMap::new();
        map.insert("alpha".into(), "one".into());
        map.insert("beta".into(), "two".into());

        let reg = SimpleRegistry { map };

        assert!(!reg.is_empty());
        assert_eq!(reg.count(), 2);
        assert!(reg.contains("alpha"));
        assert!(!reg.contains("gamma"));
        assert_eq!(reg.get("beta"), Some("two".into()));
        assert_eq!(reg.list_keys(), vec!["alpha", "beta"]);
    }

    #[test]
    fn trait_object_works() {
        let mut map = BTreeMap::new();
        map.insert("key".into(), "val".into());
        let reg = SimpleRegistry { map };

        // Ensure it can be used as a trait object.
        let dyn_reg: &dyn Registry<Value = String> = &reg;
        assert_eq!(dyn_reg.count(), 1);
        assert_eq!(dyn_reg.get("key"), Some("val".into()));
    }
}
