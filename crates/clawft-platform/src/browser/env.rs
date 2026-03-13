//! Browser environment implementation using an in-memory key-value store.
//!
//! Since WASM environments do not have OS-level environment variables,
//! [`BrowserEnvironment`] maintains a [`HashMap`] behind a [`Mutex`] to
//! satisfy the `Send + Sync` bounds required by the [`Environment`] trait.
//! In practice, WASM is single-threaded so the mutex is uncontended.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::env::Environment;

/// In-memory environment variable store for browser/WASM targets.
///
/// Variables are stored in a `HashMap` and are scoped to the lifetime of
/// this struct. There is no persistence across page reloads.
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
