//! Secret string wrapper that prevents accidental exposure.
//!
//! [`SecretString`] wraps sensitive values (API keys, passwords, tokens) and
//! ensures they never appear in logs, Debug output, or serialized JSON.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A string value that should not appear in logs, Debug output, or serialized JSON.
///
/// - `Debug` prints `[REDACTED]` (or `""` if empty)
/// - `Serialize` emits an empty string (never the actual value)
/// - `Deserialize` accepts a plain string (backward compatible with old configs)
/// - `Display` prints `[REDACTED]` (or empty if the value is empty)
/// - [`expose()`](SecretString::expose) returns the inner value for actual use
#[derive(Clone, Default)]
pub struct SecretString(String);

impl SecretString {
    /// Create a new `SecretString` wrapping the given value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Get the actual secret value. Use sparingly and only where needed
    /// (e.g., HTTP Authorization headers, API calls).
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Returns `true` if the wrapped value is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            write!(f, "\"\"")
        } else {
            write!(f, "\"[REDACTED]\"")
        }
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            write!(f, "")
        } else {
            write!(f, "[REDACTED]")
        }
    }
}

impl Serialize for SecretString {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Never serialize the actual secret value.
        serializer.serialize_str("")
    }
}

impl<'de> Deserialize<'de> for SecretString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(SecretString(s))
    }
}

impl From<String> for SecretString {
    fn from(s: String) -> Self {
        SecretString(s)
    }
}

impl From<&str> for SecretString {
    fn from(s: &str) -> Self {
        SecretString(s.to_string())
    }
}

impl PartialEq for SecretString {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_non_empty() {
        let s = SecretString::new("my-secret-key");
        assert_eq!(format!("{:?}", s), "\"[REDACTED]\"");
    }

    #[test]
    fn debug_shows_empty_for_empty() {
        let s = SecretString::default();
        assert_eq!(format!("{:?}", s), "\"\"");
    }

    #[test]
    fn display_redacts_non_empty() {
        let s = SecretString::new("secret");
        assert_eq!(format!("{}", s), "[REDACTED]");
    }

    #[test]
    fn display_empty_for_empty() {
        let s = SecretString::default();
        assert_eq!(format!("{}", s), "");
    }

    #[test]
    fn expose_returns_actual_value() {
        let s = SecretString::new("actual-secret");
        assert_eq!(s.expose(), "actual-secret");
    }

    #[test]
    fn is_empty_works() {
        assert!(SecretString::default().is_empty());
        assert!(!SecretString::new("x").is_empty());
    }

    #[test]
    fn serialize_emits_empty_string() {
        let s = SecretString::new("my-api-key");
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"\"");
        assert!(!json.contains("my-api-key"));
    }

    #[test]
    fn deserialize_accepts_plain_string() {
        let s: SecretString = serde_json::from_str("\"my-api-key\"").unwrap();
        assert_eq!(s.expose(), "my-api-key");
    }

    #[test]
    fn from_string() {
        let s: SecretString = "test".into();
        assert_eq!(s.expose(), "test");
    }

    #[test]
    fn from_owned_string() {
        let s: SecretString = String::from("test").into();
        assert_eq!(s.expose(), "test");
    }

    #[test]
    fn equality() {
        let a = SecretString::new("same");
        let b = SecretString::new("same");
        let c = SecretString::new("different");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
