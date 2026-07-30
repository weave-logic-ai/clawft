//! Browser environment implementation using an in-memory key-value store.
//!
//! Since WASM environments do not have OS-level environment variables,
//! [`BrowserEnvironment`] maintains a [`HashMap`] behind a [`Mutex`] to
//! satisfy the `Send + Sync` bounds required by the [`Environment`] trait.
//! In practice, WASM is single-threaded so the mutex is uncontended.
//!
//! The browser WASM runtime holds an [`std::sync::Arc`] clone of this
//! store (WEFT-391) so `set_env` can mutate live env state after
//! [`BrowserPlatform`](super::BrowserPlatform) is moved into `AgentLoop`.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::env::Environment;

/// In-memory environment variable store for browser/WASM targets.
///
/// Variables are stored in a `HashMap` and are scoped to the lifetime of
/// this struct. There is no persistence across page reloads.
///
/// Designed to be shared via [`std::sync::Arc`] so the browser WASM
/// `set_env` entry point and the platform-owned agent loop observe the
/// same map (WEFT-391).
pub struct BrowserEnvironment {
    vars: Mutex<HashMap<String, String>>,
}

impl BrowserEnvironment {
    /// Create a new empty browser environment.
    pub fn new() -> Self {
        Self {
            vars: Mutex::new(HashMap::new()),
        }
    }

    /// Create a browser environment pre-populated with the given variables.
    pub fn with_vars(vars: HashMap<String, String>) -> Self {
        Self {
            vars: Mutex::new(vars),
        }
    }
}

impl Default for BrowserEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment for BrowserEnvironment {
    fn get_var(&self, name: &str) -> Option<String> {
        self.vars
            .lock()
            .expect("BrowserEnvironment mutex poisoned")
            .get(name)
            .cloned()
    }

    fn set_var(&self, name: &str, value: &str) {
        self.vars
            .lock()
            .expect("BrowserEnvironment mutex poisoned")
            .insert(name.to_string(), value.to_string());
    }

    fn remove_var(&self, name: &str) {
        self.vars
            .lock()
            .expect("BrowserEnvironment mutex poisoned")
            .remove(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn set_get_remove_round_trip() {
        let env = BrowserEnvironment::new();
        assert!(env.get_var("K").is_none());
        env.set_var("K", "v1");
        assert_eq!(env.get_var("K"), Some("v1".to_string()));
        env.set_var("K", "v2");
        assert_eq!(env.get_var("K"), Some("v2".to_string()));
        env.remove_var("K");
        assert!(env.get_var("K").is_none());
    }

    #[test]
    fn with_vars_pre_populates() {
        let mut vars = HashMap::new();
        vars.insert("A".to_string(), "1".to_string());
        vars.insert("B".to_string(), "2".to_string());
        let env = BrowserEnvironment::with_vars(vars);
        assert_eq!(env.get_var("A"), Some("1".to_string()));
        assert_eq!(env.get_var("B"), Some("2".to_string()));
        assert!(env.get_var("C").is_none());
    }

    /// WEFT-391 core invariant: Arc clones share the same Mutex map.
    #[test]
    fn arc_clone_sees_set_var_mutations() {
        let env = Arc::new(BrowserEnvironment::new());
        let sibling = Arc::clone(&env);
        env.set_var("CLAWFT_WEFT391", "shared");
        assert_eq!(
            sibling.get_var("CLAWFT_WEFT391"),
            Some("shared".to_string())
        );
        sibling.set_var("CLAWFT_WEFT391", "updated");
        assert_eq!(
            env.get_var("CLAWFT_WEFT391"),
            Some("updated".to_string())
        );
    }
}
