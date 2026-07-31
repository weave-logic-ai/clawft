//! Decentralized Identifier ([`Did`]) principals for the K1 ACL engine (WEFT-554).
//!
//! Full W3C DID resolution via an `exo_identity` crate is not yet in-tree.
//! This module provides a validated DID *string* type that:
//!
//! - Accepts standard `did:<method>:<id>` forms (preferred)
//! - Accepts opaque non-empty identifiers for scaffold / migration
//!   compatibility with WEFT-130 `Principal(String)` call sites
//!
//! Callers should migrate to `did:weft:…` / `did:key:…` forms as identity
//! infrastructure lands.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Error constructing or parsing a [`Did`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DidError {
    /// Empty or whitespace-only input.
    #[error("DID must be non-empty")]
    Empty,
    /// `did:` prefix present but method/id malformed.
    #[error("malformed DID: {0}")]
    Malformed(String),
}

/// A decentralized identifier used as an ACL principal.
///
/// Canonical form is `did:<method>:<id-string>` (W3C DID Core). Opaque
/// strings (no `did:` prefix) are accepted so existing string principals
/// keep working during the K1 migration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Did(String);

impl Did {
    /// Construct a DID, rejecting empty strings.
    ///
    /// Prefer [`Self::parse`] when you need structured validation of the
    /// `did:method:id` shape. This constructor accepts opaque ids.
    pub fn new(id: impl Into<String>) -> Result<Self, DidError> {
        let s = id.into();
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(DidError::Empty);
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Infallible constructor for trusted / test inputs. Panics on empty.
    pub fn from_str_unchecked(id: impl Into<String>) -> Self {
        Self::new(id).expect("Did::from_str_unchecked requires non-empty id")
    }

    /// Parse a DID. When the input starts with `did:`, method and id
    /// segments are required (`did:method:id` with non-empty parts).
    pub fn parse(s: impl AsRef<str>) -> Result<Self, DidError> {
        let s = s.as_ref().trim();
        if s.is_empty() {
            return Err(DidError::Empty);
        }
        if let Some(rest) = s.strip_prefix("did:") {
            let mut parts = rest.splitn(2, ':');
            let method = parts.next().unwrap_or("");
            let id = parts.next().unwrap_or("");
            if method.is_empty() || id.is_empty() {
                return Err(DidError::Malformed(
                    "expected did:<method>:<id> with non-empty method and id".into(),
                ));
            }
            if !method
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                return Err(DidError::Malformed(format!(
                    "invalid DID method characters: {method}"
                )));
            }
            return Ok(Self(format!("did:{method}:{id}")));
        }
        Ok(Self(s.to_string()))
    }

    /// Build a WeftOS-namespaced DID: `did:weft:<id>`.
    pub fn weft(id: impl AsRef<str>) -> Result<Self, DidError> {
        let id = id.as_ref().trim();
        if id.is_empty() {
            return Err(DidError::Empty);
        }
        Self::parse(format!("did:weft:{id}"))
    }

    /// Raw identifier string (may or may not have a `did:` prefix).
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether this is a fully-qualified `did:…` URI.
    pub fn is_did_uri(&self) -> bool {
        self.0.starts_with("did:")
    }

    /// DID method (`weft` for `did:weft:…`), if this is a DID URI.
    pub fn method(&self) -> Option<&str> {
        let rest = self.0.strip_prefix("did:")?;
        rest.split(':').next().filter(|m| !m.is_empty())
    }

    /// Method-specific id segment, if this is a DID URI.
    pub fn method_id(&self) -> Option<&str> {
        let rest = self.0.strip_prefix("did:")?;
        let mut parts = rest.splitn(2, ':');
        let _method = parts.next()?;
        parts.next().filter(|id| !id.is_empty())
    }
}

impl fmt::Display for Did {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Did {
    type Err = DidError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl AsRef<str> for Did {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<Did> for String {
    fn from(d: Did) -> Self {
        d.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_did_uri() {
        let d = Did::parse("did:weft:alice").unwrap();
        assert!(d.is_did_uri());
        assert_eq!(d.method(), Some("weft"));
        assert_eq!(d.method_id(), Some("alice"));
        assert_eq!(d.as_str(), "did:weft:alice");
    }

    #[test]
    fn opaque_principal_id() {
        let d = Did::parse("alice").unwrap();
        assert!(!d.is_did_uri());
        assert_eq!(d.method(), None);
        assert_eq!(d.as_str(), "alice");
    }

    #[test]
    fn rejects_empty_and_malformed() {
        assert_eq!(Did::parse(""), Err(DidError::Empty));
        assert_eq!(Did::parse("   "), Err(DidError::Empty));
        assert!(matches!(
            Did::parse("did:"),
            Err(DidError::Malformed(_))
        ));
        assert!(matches!(
            Did::parse("did:weft:"),
            Err(DidError::Malformed(_))
        ));
        assert!(matches!(
            Did::parse("did::alice"),
            Err(DidError::Malformed(_))
        ));
    }

    #[test]
    fn weft_helper() {
        let d = Did::weft("agent-007").unwrap();
        assert_eq!(d.as_str(), "did:weft:agent-007");
    }

    #[test]
    fn serde_roundtrip() {
        let d = Did::parse("did:key:z6MkhaXg").unwrap();
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, "\"did:key:z6MkhaXg\"");
        let back: Did = serde_json::from_str(&json).unwrap();
        assert_eq!(back, d);
    }
}
