//! Security boundary validation functions.

use clawft_types::error::ClawftError;
use serde_json::Value;

const MAX_SESSION_ID_LEN: usize = 256;

/// Validate a session identifier against security policies.
///
/// Rejects empty, overlong, path-traversal, directory-separator, null-byte,
/// and control-character identifiers (tab is permitted).
pub fn validate_session_id(id: &str) -> Result<(), ClawftError> {
    if id.is_empty() {
        return Err(ClawftError::SecurityViolation {
            reason: "session ID must not be empty".into(),
        });
    }
    if id.len() > MAX_SESSION_ID_LEN {
        return Err(ClawftError::SecurityViolation {
            reason: format!("session ID too long ({} bytes, max {MAX_SESSION_ID_LEN})", id.len()),
        });
    }
    if id.contains("..") {
        return Err(ClawftError::SecurityViolation {
            reason: "session ID contains path traversal sequence '..'".into(),
        });
    }
    if id.contains('/') || id.contains('\\') {
        return Err(ClawftError::SecurityViolation {
            reason: "session ID contains directory separator".into(),
        });
    }
    if id.contains('\0') {
        return Err(ClawftError::SecurityViolation {
            reason: "session ID contains null byte".into(),
        });
    }
    for byte in id.bytes() {
        if byte <= 0x1F && byte != 0x09 {
            return Err(ClawftError::SecurityViolation {
                reason: format!("session ID contains control character 0x{byte:02X}"),
            });
        }
    }
    Ok(())
}

/// Truncate a JSON value so its serialized form fits within `max_bytes`.
///
/// Strategy varies by type: strings get a truncation suffix, arrays keep
/// leading elements with a sentinel, and other values get raw-JSON wrapping.
pub fn truncate_result(value: Value, max_bytes: usize) -> Value {
    let serialized = serde_json::to_string(&value).unwrap_or_default();
    if serialized.len() <= max_bytes {
        return value;
    }
    let original_len = serialized.len();
    match value {
        Value::String(s) => truncate_string(s, original_len, max_bytes),
        Value::Array(arr) => truncate_array(arr, original_len, max_bytes),
        _ => truncate_raw(serialized, original_len, max_bytes),
    }
}

fn truncate_string(s: String, original_len: usize, max_bytes: usize) -> Value {
    let suffix = format!("... [truncated, original size: {original_len} bytes]");
    let available = max_bytes.saturating_sub(2 + suffix.len()); // 2 = JSON quotes
    let truncated: String = s.chars().take(available).collect();
    let mut result = Value::String(format!("{truncated}{suffix}"));
    // Shrink further if JSON escaping pushed us over.
    while serde_json::to_string(&result).unwrap_or_default().len() > max_bytes {
        let chars: Vec<char> = result.as_str().unwrap().chars().collect();
        if chars.len() <= suffix.len() + 1 {
            result = Value::String(suffix.clone());
            break;
        }
        let keep = chars.len() - suffix.len() - 1;
        let shortened: String = chars[..keep].iter().collect();
        result = Value::String(format!("{shortened}{suffix}"));
    }
    result
}

fn truncate_array(arr: Vec<Value>, original_count: usize, max_bytes: usize) -> Value {
    let sentinel = serde_json::json!({"_truncated": true, "original_count": original_count});
    let sentinel_ser = serde_json::to_string(&sentinel).unwrap_or_default();
    let mut kept: Vec<Value> = Vec::new();
    let mut current_len = 2 + sentinel_ser.len(); // `[` + sentinel + `]`
    for elem in arr {
        let elem_ser = serde_json::to_string(&elem).unwrap_or_default();
        let added = elem_ser.len() + 1; // comma separator
        if current_len + added > max_bytes {
            break;
        }
        current_len += added;
        kept.push(elem);
    }
    kept.push(sentinel);
    Value::Array(kept)
}

fn truncate_raw(serialized: String, original_len: usize, max_bytes: usize) -> Value {
    let template_overhead =
        r#"{"_truncated_json":"...","original_bytes":}"#.len() + original_len.to_string().len();
    let available = max_bytes.saturating_sub(template_overhead);
    let prefix: String = serialized.chars().take(available).collect();
    let mut result =
        serde_json::json!({"_truncated_json": format!("{prefix}..."), "original_bytes": original_len});
    while serde_json::to_string(&result).unwrap_or_default().len() > max_bytes {
        let cur = match result.get("_truncated_json").and_then(Value::as_str) {
            Some(s) => s.to_owned(),
            None => break,
        };
        let chars: Vec<char> = cur.chars().collect();
        if chars.len() <= 4 {
            break;
        }
        let shortened: String = chars[..chars.len() - 4].iter().collect();
        result["_truncated_json"] = Value::String(format!("{shortened}..."));
    }
    result
}

/// Strip null bytes, DEL, and ASCII control chars except `\n`, `\r`, `\t`.
/// Preserves all valid UTF-8 including emoji, CJK, and RTL text.
pub fn sanitize_content(content: &str) -> String {
    content
        .chars()
        .filter(|&c| {
            if c == '\0' || c == '\x7F' {
                return false;
            }
            let b = c as u32;
            !(b <= 0x1F && b != 0x0A && b != 0x0D && b != 0x09)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate_session_id ────────────────────────────────────────

    #[test]
    fn valid_simple_id() {
        assert!(validate_session_id("abc-123_XYZ").is_ok());
    }

    #[test]
    fn valid_id_with_allowed_chars() {
        assert!(validate_session_id("user@host:session+1.0").is_ok());
    }

    #[test]
    fn valid_id_with_space_and_tab() {
        assert!(validate_session_id("hello world\there").is_ok());
    }

    #[test]
    fn reject_empty() {
        let err = validate_session_id("").unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn reject_too_long() {
        let err = validate_session_id(&"a".repeat(257)).unwrap_err();
        assert!(err.to_string().contains("too long"));
    }

    #[test]
    fn accept_max_length() {
        assert!(validate_session_id(&"b".repeat(256)).is_ok());
    }

    #[test]
    fn reject_path_traversal() {
        let err = validate_session_id("foo/../bar").unwrap_err();
        assert!(err.to_string().contains("path traversal"));
    }

    #[test]
    fn reject_forward_slash() {
        let err = validate_session_id("foo/bar").unwrap_err();
        assert!(err.to_string().contains("directory separator"));
    }

    #[test]
    fn reject_backslash() {
        let err = validate_session_id("foo\\bar").unwrap_err();
        assert!(err.to_string().contains("directory separator"));
    }

    #[test]
    fn reject_null_byte() {
        let err = validate_session_id("foo\0bar").unwrap_err();
        assert!(err.to_string().contains("null byte"));
    }

    #[test]
    fn reject_control_char() {
        let err = validate_session_id("foo\x01bar").unwrap_err();
        assert!(err.to_string().contains("control character"));
    }

    // ── truncate_result ────────────────────────────────────────────

    #[test]
    fn no_truncation_when_fits() {
        let val = Value::String("short".into());
        assert_eq!(truncate_result(val.clone(), 1000), val);
    }

    #[test]
    fn truncate_long_string() {
        let result = truncate_result(Value::String("x".repeat(500)), 100);
        let ser = serde_json::to_string(&result).unwrap();
        assert!(ser.len() <= 100, "got {} bytes", ser.len());
        assert!(result.as_str().unwrap().contains("truncated"));
    }

    #[test]
    fn truncate_large_array() {
        let arr: Vec<Value> = (0..100).map(|i| Value::Number(i.into())).collect();
        let result = truncate_result(Value::Array(arr), 100);
        let ser = serde_json::to_string(&result).unwrap();
        assert!(ser.len() <= 100, "got {} bytes", ser.len());
        let last = result.as_array().unwrap().last().unwrap();
        assert_eq!(last["_truncated"], Value::Bool(true));
    }

    #[test]
    fn truncate_object() {
        let mut map = serde_json::Map::new();
        for i in 0..50 {
            map.insert(format!("key_{i}"), Value::String("value".into()));
        }
        let result = truncate_result(Value::Object(map), 100);
        let ser = serde_json::to_string(&result).unwrap();
        assert!(ser.len() <= 100, "got {} bytes", ser.len());
        assert!(result.get("_truncated_json").is_some());
    }

    // ── sanitize_content ───────────────────────────────────────────

    #[test]
    fn sanitize_preserves_normal_text() {
        assert_eq!(sanitize_content("hello world"), "hello world");
    }

    #[test]
    fn sanitize_preserves_newlines_tabs() {
        assert_eq!(sanitize_content("a\nb\rc\td"), "a\nb\rc\td");
    }

    #[test]
    fn sanitize_strips_null() {
        assert_eq!(sanitize_content("a\0b"), "ab");
    }

    #[test]
    fn sanitize_strips_control_chars() {
        assert_eq!(sanitize_content("a\x01\x02\x1Fb"), "ab");
    }

    #[test]
    fn sanitize_strips_del() {
        assert_eq!(sanitize_content("a\x7Fb"), "ab");
    }

    #[test]
    fn sanitize_preserves_emoji() {
        let s = "Hello \u{1F600} world \u{1F30D}";
        assert_eq!(sanitize_content(s), s);
    }

    #[test]
    fn sanitize_preserves_cjk() {
        let s = "\u{4F60}\u{597D}\u{4E16}\u{754C}";
        assert_eq!(sanitize_content(s), s);
    }

    #[test]
    fn sanitize_preserves_rtl() {
        let s = "\u{0645}\u{0631}\u{062D}\u{0628}\u{0627}";
        assert_eq!(sanitize_content(s), s);
    }
}
