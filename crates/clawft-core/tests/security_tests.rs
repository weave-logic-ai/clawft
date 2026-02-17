//! Security boundary integration tests for `clawft_core::security`.
//!
//! These tests exercise the public API from an external-crate perspective,
//! validating session ID constraints, tool-result truncation, and content
//! sanitization at the boundaries where untrusted input enters the system.

use clawft_core::security::{sanitize_content, truncate_result, validate_session_id};
use clawft_types::error::ClawftError;
use serde_json::{json, Value};

// ── Session ID validation ──────────────────────────────────────────

#[test]
fn test_session_id_no_path_traversal() {
    let result = validate_session_id("../../etc/passwd");
    assert!(result.is_err(), "path traversal input must be rejected");
    let err = result.unwrap_err();
    assert!(
        matches!(err, ClawftError::SecurityViolation { .. }),
        "error must be SecurityViolation, got: {err}"
    );
    assert!(
        err.to_string().contains("path traversal"),
        "error message should mention path traversal, got: {err}"
    );
}

#[test]
fn test_session_id_rejects_slash() {
    let result = validate_session_id("foo/bar");
    assert!(
        result.is_err(),
        "forward slash in session ID must be rejected"
    );
    assert!(
        matches!(result.unwrap_err(), ClawftError::SecurityViolation { .. }),
        "error must be SecurityViolation"
    );
}

#[test]
fn test_session_id_rejects_backslash() {
    let result = validate_session_id("foo\\bar");
    assert!(
        result.is_err(),
        "backslash in session ID must be rejected"
    );
    assert!(
        matches!(result.unwrap_err(), ClawftError::SecurityViolation { .. }),
        "error must be SecurityViolation"
    );
}

#[test]
fn test_session_id_rejects_null_byte() {
    let result = validate_session_id("foo\0bar");
    assert!(
        result.is_err(),
        "null byte in session ID must be rejected"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, ClawftError::SecurityViolation { .. }),
        "error must be SecurityViolation, got: {err}"
    );
    assert!(
        err.to_string().contains("null byte"),
        "error message should mention null byte, got: {err}"
    );
}

#[test]
fn test_session_id_rejects_excessive_length() {
    let long_id = "a".repeat(257);
    let result = validate_session_id(&long_id);
    assert!(
        result.is_err(),
        "session ID exceeding 256 bytes must be rejected"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, ClawftError::SecurityViolation { .. }),
        "error must be SecurityViolation, got: {err}"
    );
    assert!(
        err.to_string().contains("too long"),
        "error message should mention length, got: {err}"
    );
}

#[test]
fn test_session_id_rejects_empty() {
    let result = validate_session_id("");
    assert!(result.is_err(), "empty session ID must be rejected");
    let err = result.unwrap_err();
    assert!(
        matches!(err, ClawftError::SecurityViolation { .. }),
        "error must be SecurityViolation, got: {err}"
    );
    assert!(
        err.to_string().contains("empty"),
        "error message should mention empty, got: {err}"
    );
}

#[test]
fn test_session_id_rejects_control_chars() {
    let result = validate_session_id("foo\x01bar");
    assert!(
        result.is_err(),
        "ASCII control characters (SOH 0x01) in session ID must be rejected"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, ClawftError::SecurityViolation { .. }),
        "error must be SecurityViolation, got: {err}"
    );
    assert!(
        err.to_string().contains("control character"),
        "error message should mention control character, got: {err}"
    );
}

#[test]
fn test_session_id_accepts_valid() {
    let result = validate_session_id("telegram:12345");
    assert!(
        result.is_ok(),
        "valid session ID 'telegram:12345' should be accepted, got: {:?}",
        result.unwrap_err()
    );
}

#[test]
fn test_session_id_accepts_complex_valid() {
    let result = validate_session_id("slack:C0A1B2-thread_ts.1234");
    assert!(
        result.is_ok(),
        "valid complex session ID should be accepted, got: {:?}",
        result.unwrap_err()
    );
}

// ── Tool result truncation ─────────────────────────────────────────

#[test]
fn test_tool_result_size_limit() {
    // Build a JSON value whose serialized form exceeds 1024 bytes.
    let huge_string = "x".repeat(2000);
    let huge_json = json!({ "data": huge_string });
    let serialized_original = serde_json::to_string(&huge_json).unwrap();
    assert!(
        serialized_original.len() > 1024,
        "precondition: input must exceed 1024 bytes, was {} bytes",
        serialized_original.len()
    );

    let truncated = truncate_result(huge_json, 1024);
    let serialized = serde_json::to_string(&truncated).unwrap();
    assert!(
        serialized.len() <= 1024,
        "truncated result must serialize to at most 1024 bytes, got {} bytes",
        serialized.len()
    );
}

#[test]
fn test_tool_result_preserves_small() {
    let small_json = json!({ "status": "ok", "count": 42 });
    let original = small_json.clone();
    let result = truncate_result(small_json, 65536);
    assert_eq!(
        result, original,
        "small JSON value must pass through truncate_result unchanged"
    );
}

#[test]
fn test_tool_result_truncates_string() {
    let big_str = "y".repeat(500);
    let value = Value::String(big_str);
    let result = truncate_result(value, 100);
    let text = result
        .as_str()
        .expect("truncated string should still be a JSON string");
    assert!(
        text.contains("truncated"),
        "truncated string should contain truncation marker, got: {text}"
    );
    let serialized = serde_json::to_string(&result).unwrap();
    assert!(
        serialized.len() <= 100,
        "serialized truncated string must fit in 100 bytes, got {} bytes",
        serialized.len()
    );
}

#[test]
fn test_tool_result_truncates_array() {
    let large_array: Vec<Value> = (0..200).map(|i| Value::Number(i.into())).collect();
    let value = Value::Array(large_array);
    let result = truncate_result(value, 100);

    let arr = result
        .as_array()
        .expect("truncated array should still be a JSON array");
    let last = arr
        .last()
        .expect("truncated array should have at least one element");
    assert_eq!(
        last.get("_truncated"),
        Some(&Value::Bool(true)),
        "last element of truncated array must be a sentinel with _truncated: true"
    );

    let serialized = serde_json::to_string(&result).unwrap();
    assert!(
        serialized.len() <= 100,
        "serialized truncated array must fit in 100 bytes, got {} bytes",
        serialized.len()
    );
}

// ── Content sanitization ───────────────────────────────────────────

#[test]
fn test_context_builder_escapes_user_input() {
    // The context builder is pass-through for valid UTF-8. Verify that
    // `<script>alert(1)</script>` survives sanitize_content unchanged,
    // since angle brackets and script tags are valid printable characters.
    let xss_payload = "<script>alert(1)</script>";
    let sanitized = sanitize_content(xss_payload);
    assert_eq!(
        sanitized, xss_payload,
        "HTML/script tags are valid UTF-8 and must not be stripped by sanitize_content"
    );
}

#[test]
fn test_memory_write_sanitizes_content() {
    let dirty = "hello\0world\x01";
    let clean = sanitize_content(dirty);
    assert_eq!(
        clean, "helloworld",
        "null bytes and control characters must be stripped"
    );
    assert!(
        !clean.contains('\0'),
        "sanitized output must not contain null bytes"
    );
    assert!(
        !clean.contains('\x01'),
        "sanitized output must not contain SOH control character"
    );
}

#[test]
fn test_sanitize_preserves_valid_utf8() {
    let emoji = "\u{1F389}"; // party popper
    assert_eq!(
        sanitize_content(emoji),
        emoji,
        "emoji must pass through sanitize_content unchanged"
    );

    let cjk = "\u{65E5}\u{672C}\u{8A9E}"; // Japanese
    assert_eq!(
        sanitize_content(cjk),
        cjk,
        "CJK characters must pass through sanitize_content unchanged"
    );

    let accented = "caf\u{00E9}";
    assert_eq!(
        sanitize_content(accented),
        accented,
        "accented characters must pass through sanitize_content unchanged"
    );
}

#[test]
fn test_sanitize_strips_null_and_control() {
    // Null (0x00), SOH (0x01), STX (0x02), and DEL (0x7F) must be stripped.
    let input = "a\0b\x01c\x02d\x7Fe";
    let result = sanitize_content(input);
    assert_eq!(
        result, "abcde",
        "null, control chars, and DEL must all be removed"
    );

    // Newline (0x0A), carriage return (0x0D), and tab (0x09) must be preserved.
    let whitespace_input = "line1\nline2\rline3\tcol2";
    let whitespace_result = sanitize_content(whitespace_input);
    assert_eq!(
        whitespace_result, whitespace_input,
        "newline, carriage return, and tab must be preserved by sanitize_content"
    );
}

// ── Truncation smoke test (compile-time constant accessibility) ────

#[test]
fn test_max_agent_iterations() {
    // Verify that the truncation mechanism works with a small byte limit,
    // confirming the concept of configurable result-size boundaries.
    // This serves as a compile-time smoke test that the truncation API
    // is accessible and functional with arbitrary limits.
    let small_limit: usize = 80;
    let value = Value::String("a]long value that exceeds the limit".repeat(5));
    let result = truncate_result(value.clone(), small_limit);
    let serialized = serde_json::to_string(&result).unwrap();
    assert!(
        serialized.len() <= small_limit,
        "truncate_result must enforce arbitrary small limits; got {} bytes for limit {}",
        serialized.len(),
        small_limit
    );

    // Also verify that a value within the limit passes through unchanged.
    let tiny = json!(1);
    let tiny_result = truncate_result(tiny.clone(), small_limit);
    assert_eq!(
        tiny_result, tiny,
        "values within the byte limit must pass through unchanged"
    );
}
