//! In-memory environment for WASM/WASI.
//!
//! Provides a [`WasiEnvironment`] using an in-memory [`HashMap`]. This is
//! immediately usable for testing and sandboxed execution without requiring
//! real WASI environment variable support.
//!
//! The backing store is protected by a [`std::sync::Mutex`] so it is `Send + Sync`.
//!
//! This module is fully decoupled from `clawft-platform` so it can compile for
//! `wasm32-wasip1` without pulling in tokio or reqwest.

use std::collections::HashMap;
use std::sync::Mutex;

/// In-memory environment variable store for WASM/WASI.
///
/// Uses a [`HashMap`] behind a [`Mutex`] to provide thread-safe, in-memory
/// environment variable access. This works in any environment (native or WASM)
/// and is useful for testing and sandboxed agent execution where real OS
/// environment variables are either unavailable or undesirable.
pub struct WasiEnvironment {
    vars: Mutex<HashMap<String, String>>,
}

impl WasiEnvironment {
    /// Create a new empty in-memory environment.
    pub fn new() -> Self {
        Self {
            vars: Mutex::new(HashMap::new()),
        }
    }

    /// Create a new in-memory environment pre-populated with the given variables.
    pub fn with_vars(vars: HashMap<String, String>) -> Self {
        Self {
            vars: Mutex::new(vars),
        }
    }

    /// Get the value of an environment variable, or `None` if it is not set.
    pub fn get_var(&self, name: &str) -> Option<String> {
        self.vars
            .lock()
            .expect("WasiEnvironment mutex poisoned")
            .get(name)
            .cloned()
    }

    /// Set an environment variable.
    pub fn set_var(&self, name: &str, value: &str) {
        self.vars
            .lock()
            .expect("WasiEnvironment mutex poisoned")
            .insert(name.to_string(), value.to_string());
    }

    /// Remove (unset) an environment variable.
    pub fn remove_var(&self, name: &str) {
        self.vars
            .lock()
            .expect("WasiEnvironment mutex poisoned")
            .remove(name);
    }
}

impl Default for WasiEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_environment_is_empty() {
        let env = WasiEnvironment::new();
        assert!(env.get_var("ANY_KEY").is_none());
    }

    #[test]
    fn default_environment_is_empty() {
        let env = WasiEnvironment::default();
        assert!(env.get_var("ANY_KEY").is_none());
    }

    #[test]
    fn with_vars_pre_populates() {
        let mut initial = HashMap::new();
        initial.insert("KEY1".to_string(), "value1".to_string());
        initial.insert("KEY2".to_string(), "value2".to_string());

        let env = WasiEnvironment::with_vars(initial);
        assert_eq!(env.get_var("KEY1"), Some("value1".to_string()));
        assert_eq!(env.get_var("KEY2"), Some("value2".to_string()));
        assert!(env.get_var("KEY3").is_none());
    }

    #[test]
    fn set_and_get_var() {
        let env = WasiEnvironment::new();
        env.set_var("MY_VAR", "hello");
        assert_eq!(env.get_var("MY_VAR"), Some("hello".to_string()));
    }

    #[test]
    fn set_var_overwrites() {
        let env = WasiEnvironment::new();
        env.set_var("KEY", "first");
        assert_eq!(env.get_var("KEY"), Some("first".to_string()));

        env.set_var("KEY", "second");
        assert_eq!(env.get_var("KEY"), Some("second".to_string()));
    }

    #[test]
    fn remove_var_deletes() {
        let env = WasiEnvironment::new();
        env.set_var("KEY", "value");
        assert!(env.get_var("KEY").is_some());

        env.remove_var("KEY");
        assert!(env.get_var("KEY").is_none());
    }

    #[test]
    fn remove_nonexistent_var_is_noop() {
        let env = WasiEnvironment::new();
        // Should not panic.
        env.remove_var("DOES_NOT_EXIST");
    }

    #[test]
    fn get_missing_var_returns_none() {
        let env = WasiEnvironment::new();
        assert!(env.get_var("NONEXISTENT").is_none());
    }

    #[test]
    fn wasi_environment_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WasiEnvironment>();
    }

    #[test]
    fn empty_key_and_value_work() {
        let env = WasiEnvironment::new();
        env.set_var("", "empty_key");
        assert_eq!(env.get_var(""), Some("empty_key".to_string()));

        env.set_var("KEY", "");
        assert_eq!(env.get_var("KEY"), Some(String::new()));
    }
}
