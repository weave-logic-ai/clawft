//! Phase 2 integration tests.
//!
//! Verifies that Phase 2 features (channels, services, security) work
//! together correctly.

use clawft_core::security::{sanitize_content, truncate_result, validate_session_id};
use clawft_types::error::ClawftError;

/// Test 1: Security validation integrates with session operations.
#[test]
fn test_security_validation_blocks_path_traversal() {
    let result = validate_session_id("../../../etc/shadow");
    assert!(result.is_err());
    if let Err(ClawftError::SecurityViolation { reason }) = result {
        assert!(
            reason.contains("path traversal") || reason.contains(".."),
            "expected path traversal reason, got: {reason}"
        );
    } else {
        panic!("expected SecurityViolation error");
    }
}

/// Test 2: Sanitize content removes dangerous bytes.
#[test]
fn test_sanitize_removes_control_characters() {
    let input = "hello\x00world\x01\x02\x03\nkeep\tnewline\r";
    let output = sanitize_content(input);
    assert!(output.contains("helloworld"));
    assert!(output.contains("\n"));
    assert!(output.contains("\t"));
    assert!(output.contains("\r"));
    assert!(!output.contains('\0'));
    assert!(!output.contains('\x01'));
}

/// Test 3: Tool result truncation enforces size limits.
#[test]
fn test_truncation_enforces_limit() {
    let big = serde_json::json!({"data": "x".repeat(10000)});
    let result = truncate_result(big, 256);
    let serialized = serde_json::to_string(&result).unwrap();
    assert!(
        serialized.len() <= 256,
        "truncated result should be <= 256 bytes, got {}",
        serialized.len()
    );
}

/// Test 4: Valid session IDs pass validation.
#[test]
fn test_valid_session_ids_accepted() {
    assert!(validate_session_id("telegram:12345").is_ok());
    assert!(validate_session_id("slack:C0A1B2-thread").is_ok());
    assert!(validate_session_id("discord:guild_channel").is_ok());
    assert!(validate_session_id("cli-session").is_ok());
    assert!(validate_session_id("user@email.com").is_ok());
}

/// Test 5: Memory fixture file exists and is valid markdown.
#[test]
fn test_memory_fixture_exists() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/MEMORY.md"
    );
    let content = std::fs::read_to_string(path).unwrap();
    assert!(content.contains("# Project Context"));
    assert!(content.contains("MSRV"));
    assert!(content.contains("clawft-llm"));
}

/// Test 6: Directory separator in session ID is rejected.
#[test]
fn test_directory_separator_rejected() {
    let result = validate_session_id("foo/bar");
    assert!(result.is_err());
    if let Err(ClawftError::SecurityViolation { reason }) = result {
        assert!(
            reason.contains("directory separator"),
            "expected directory separator reason, got: {reason}"
        );
    } else {
        panic!("expected SecurityViolation error");
    }
}

/// Test 7: Backslash in session ID is rejected.
#[test]
fn test_backslash_rejected() {
    let result = validate_session_id("foo\\bar");
    assert!(result.is_err());
    if let Err(ClawftError::SecurityViolation { reason }) = result {
        assert!(
            reason.contains("directory separator"),
            "expected directory separator reason, got: {reason}"
        );
    } else {
        panic!("expected SecurityViolation error");
    }
}

/// Test 8: Empty session ID is rejected.
#[test]
fn test_empty_session_id_rejected() {
    let result = validate_session_id("");
    assert!(result.is_err());
    if let Err(ClawftError::SecurityViolation { reason }) = result {
        assert!(
            reason.contains("empty"),
            "expected empty reason, got: {reason}"
        );
    } else {
        panic!("expected SecurityViolation error");
    }
}

/// Test 9: Truncation of arrays works correctly.
#[test]
fn test_truncation_array() {
    let arr: Vec<serde_json::Value> = (0..100)
        .map(|i| serde_json::Value::Number(i.into()))
        .collect();
    let big = serde_json::Value::Array(arr);
    let result = truncate_result(big, 128);
    let serialized = serde_json::to_string(&result).unwrap();
    assert!(
        serialized.len() <= 128,
        "truncated array should be <= 128 bytes, got {}",
        serialized.len()
    );
    // Should contain the truncation sentinel.
    let arr = result.as_array().unwrap();
    let last = arr.last().unwrap();
    assert_eq!(last["_truncated"], serde_json::Value::Bool(true));
}

/// Test 10: Sanitize preserves valid Unicode.
#[test]
fn test_sanitize_preserves_unicode() {
    let input = "Hello world \u{1F600}\u{4F60}\u{597D}";
    let output = sanitize_content(input);
    assert_eq!(output, input, "valid unicode must be preserved");
}

/// Test 11: Null byte in session ID is rejected.
#[test]
fn test_null_byte_rejected() {
    let result = validate_session_id("session\0id");
    assert!(result.is_err());
    if let Err(ClawftError::SecurityViolation { reason }) = result {
        assert!(
            reason.contains("null byte"),
            "expected null byte reason, got: {reason}"
        );
    } else {
        panic!("expected SecurityViolation error");
    }
}

/// Test 12: Session ID length limit is enforced.
#[test]
fn test_session_id_length_limit() {
    let long_id = "a".repeat(257);
    let result = validate_session_id(&long_id);
    assert!(result.is_err());
    if let Err(ClawftError::SecurityViolation { reason }) = result {
        assert!(
            reason.contains("too long"),
            "expected too long reason, got: {reason}"
        );
    } else {
        panic!("expected SecurityViolation error");
    }

    // Max length (256) should be accepted.
    let exact = "b".repeat(256);
    assert!(validate_session_id(&exact).is_ok());
}
