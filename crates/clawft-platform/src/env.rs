//! Environment variable abstraction and native implementation.
//!
//! Provides a platform-agnostic [`Environment`] trait for reading and writing
//! environment variables. The native implementation delegates to [`std::env`].
//! A WASM implementation could use a config map or browser-based storage.

/// Platform-agnostic environment variable access.
///
/// Implementations provide read/write access to environment-style key-value
/// configuration. The native implementation maps directly to OS environment
/// variables; alternative implementations can use in-memory maps for testing
/// or WASM compatibility.
pub trait Environment: Send + Sync {
    /// Get the value of an environment variable, or `None` if it is not set.
    fn get_var(&self, name: &str) -> Option<String>;

    /// Set an environment variable.
    fn set_var(&self, name: &str, value: &str);

    /// Remove (unset) an environment variable.
    fn remove_var(&self, name: &str);
}

/// Native environment implementation using [`std::env`].
#[cfg(feature = "native")]
pub struct NativeEnvironment;

#[cfg(feature = "native")]
impl Environment for NativeEnvironment {
    fn get_var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn set_var(&self, name: &str, value: &str) {
        // SAFETY: This is the standard library function. We accept the caveats
        // around concurrent mutation of the environment in multi-threaded programs.
        // In practice, env var mutation is done during initialization only.
        unsafe {
            std::env::set_var(name, value);
        }
    }

    fn remove_var(&self, name: &str) {
        // SAFETY: Same caveat as set_var above.
        unsafe {
            std::env::remove_var(name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests mutate process-global environment variables, so they
    // use unique variable names to avoid interference with other tests.

    #[test]
    fn test_get_var_existing() {
        let env = NativeEnvironment;
        // PATH is universally available on all platforms
        assert!(env.get_var("PATH").is_some());
    }

    #[test]
    fn test_get_var_missing() {
        let env = NativeEnvironment;
        assert!(env.get_var("CLAWFT_DEFINITELY_NOT_SET_12345").is_none());
    }

    #[test]
    fn test_set_and_get_var() {
        let env = NativeEnvironment;
        let key = "CLAWFT_TEST_SET_GET_VAR";

        env.set_var(key, "test_value");
        assert_eq!(env.get_var(key), Some("test_value".to_string()));

        // Cleanup
        env.remove_var(key);
    }

    #[test]
    fn test_remove_var() {
        let env = NativeEnvironment;
        let key = "CLAWFT_TEST_REMOVE_VAR";

        env.set_var(key, "to_remove");
        assert!(env.get_var(key).is_some());

        env.remove_var(key);
        assert!(env.get_var(key).is_none());
    }

    #[test]
    fn test_set_var_overwrites() {
        let env = NativeEnvironment;
        let key = "CLAWFT_TEST_OVERWRITE_VAR";

        env.set_var(key, "first");
        assert_eq!(env.get_var(key), Some("first".to_string()));

        env.set_var(key, "second");
        assert_eq!(env.get_var(key), Some("second".to_string()));

        // Cleanup
        env.remove_var(key);
    }
}
