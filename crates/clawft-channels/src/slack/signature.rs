//! Slack request signature verification.
//!
//! Implements HMAC-SHA256 signature verification for Slack HTTP event
//! subscriptions. This is the standard Slack signing secret flow:
//!
//! 1. Concatenate `v0:{timestamp}:{body}` as the base string.
//! 2. Compute `HMAC-SHA256(signing_secret, base_string)`.
//! 3. Compare `v0={hex_digest}` against the provided signature.
//!
//! An anti-replay check rejects requests with timestamps older than
//! 5 minutes.

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Maximum age of a request timestamp in seconds before it is rejected
/// as a potential replay attack.
const MAX_TIMESTAMP_AGE_SECS: i64 = 300; // 5 minutes

type HmacSha256 = Hmac<Sha256>;

/// Verify a Slack request signature.
///
/// # Arguments
///
/// * `signing_secret` -- The app's signing secret from Slack.
/// * `timestamp` -- The `X-Slack-Request-Timestamp` header value.
/// * `body` -- The raw request body.
/// * `signature` -- The `X-Slack-Signature` header value (e.g. `v0=abcdef...`).
///
/// # Returns
///
/// `true` if the signature is valid and the timestamp is fresh.
pub fn verify_signature(
    signing_secret: &str,
    timestamp: &str,
    body: &str,
    signature: &str,
) -> bool {
    // Anti-replay: reject if timestamp is too old.
    if let Ok(ts) = timestamp.parse::<i64>() {
        let now = chrono::Utc::now().timestamp();
        if (now - ts).abs() > MAX_TIMESTAMP_AGE_SECS {
            return false;
        }
    } else {
        // Non-numeric timestamp is invalid.
        return false;
    }

    // Build the base string: v0:{timestamp}:{body}
    let base_string = format!("v0:{timestamp}:{body}");

    // Compute HMAC-SHA256.
    let Ok(mut mac) = HmacSha256::new_from_slice(signing_secret.as_bytes()) else {
        return false;
    };
    mac.update(base_string.as_bytes());
    let result = mac.finalize();
    let computed = hex::encode(result.into_bytes());

    let expected = format!("v0={computed}");

    // Constant-time comparison via the hmac crate's verify method is
    // preferred, but since we already finalized, we do a simple
    // byte-by-byte comparison. For webhook verification this is
    // acceptable.
    constant_time_eq(expected.as_bytes(), signature.as_bytes())
}

/// Compute the expected signature for a given request (useful for testing).
///
/// Returns the full `v0={hex}` string.
pub fn compute_signature(signing_secret: &str, timestamp: &str, body: &str) -> String {
    let base_string = format!("v0:{timestamp}:{body}");
    let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes()).expect("HMAC key is valid");
    mac.update(base_string.as_bytes());
    let result = mac.finalize();
    format!("v0={}", hex::encode(result.into_bytes()))
}

/// Constant-time byte comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// We use a minimal hex encoding to avoid adding the `hex` crate
// as a dependency. The `hex::encode` call above is actually our own
// module-level function.
mod hex {
    /// Encode bytes as lowercase hexadecimal.
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes.as_ref().iter().fold(String::new(), |mut acc, b| {
            use std::fmt::Write;
            let _ = write!(acc, "{b:02x}");
            acc
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "8f742231b10e8888abcd99yyyzzz85a5";

    #[test]
    fn valid_signature() {
        let now = chrono::Utc::now().timestamp();
        let timestamp = now.to_string();
        let body = r#"{"token":"Jhj5dZrVaK7ZwHHjRyZWjbDl"}"#;

        let sig = compute_signature(TEST_SECRET, &timestamp, body);
        assert!(verify_signature(TEST_SECRET, &timestamp, body, &sig));
    }

    #[test]
    fn invalid_signature() {
        let now = chrono::Utc::now().timestamp();
        let timestamp = now.to_string();
        let body = "some body";

        assert!(!verify_signature(
            TEST_SECRET,
            &timestamp,
            body,
            "v0=0000000000000000000000000000000000000000000000000000000000000000"
        ));
    }

    #[test]
    fn wrong_body_fails() {
        let now = chrono::Utc::now().timestamp();
        let timestamp = now.to_string();
        let body = "original body";

        let sig = compute_signature(TEST_SECRET, &timestamp, body);
        assert!(!verify_signature(
            TEST_SECRET,
            &timestamp,
            "tampered body",
            &sig
        ));
    }

    #[test]
    fn replay_attack_old_timestamp() {
        // Timestamp from 10 minutes ago.
        let old = chrono::Utc::now().timestamp() - 600;
        let timestamp = old.to_string();
        let body = "body";

        let sig = compute_signature(TEST_SECRET, &timestamp, body);
        assert!(!verify_signature(TEST_SECRET, &timestamp, body, &sig));
    }

    #[test]
    fn replay_attack_future_timestamp() {
        // Timestamp 10 minutes in the future.
        let future = chrono::Utc::now().timestamp() + 600;
        let timestamp = future.to_string();
        let body = "body";

        let sig = compute_signature(TEST_SECRET, &timestamp, body);
        assert!(!verify_signature(TEST_SECRET, &timestamp, body, &sig));
    }

    #[test]
    fn timestamp_within_window_succeeds() {
        // Timestamp 2 minutes ago (within 5 minute window).
        let recent = chrono::Utc::now().timestamp() - 120;
        let timestamp = recent.to_string();
        let body = "body";

        let sig = compute_signature(TEST_SECRET, &timestamp, body);
        assert!(verify_signature(TEST_SECRET, &timestamp, body, &sig));
    }

    #[test]
    fn non_numeric_timestamp_fails() {
        assert!(!verify_signature(
            TEST_SECRET,
            "not-a-number",
            "body",
            "v0=abc"
        ));
    }

    #[test]
    fn empty_signature_fails() {
        let now = chrono::Utc::now().timestamp().to_string();
        assert!(!verify_signature(TEST_SECRET, &now, "body", ""));
    }

    #[test]
    fn signature_without_v0_prefix_fails() {
        let now = chrono::Utc::now().timestamp();
        let timestamp = now.to_string();
        let body = "body";
        let sig = compute_signature(TEST_SECRET, &timestamp, body);
        // Strip the v0= prefix.
        let bare = &sig[3..];
        assert!(!verify_signature(TEST_SECRET, &timestamp, body, bare));
    }

    #[test]
    fn different_secrets_produce_different_signatures() {
        let now = chrono::Utc::now().timestamp().to_string();
        let body = "body";
        let sig1 = compute_signature("secret-one", &now, body);
        let sig2 = compute_signature("secret-two", &now, body);
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn constant_time_eq_works() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hell"));
        assert!(!constant_time_eq(b"", b"x"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn hex_encode() {
        assert_eq!(hex::encode([0x00, 0xff, 0x0a, 0xbc]), "00ff0abc");
        assert_eq!(hex::encode([]), "");
    }
}
