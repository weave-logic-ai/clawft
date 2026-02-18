//! JSON deep merge and key normalization utilities.
//!
//! Used by the workspace config loader to merge defaults, global config,
//! and workspace-level config into a single resolved [`Config`].

use serde_json::Value;

/// Deep merge two JSON [`Value`]s.
///
/// Rules (per consensus P1 #21):
/// - **Objects** are recursively merged (right keys override left).
/// - **Arrays** are REPLACED (not concatenated).
/// - **Scalars**: right overrides left.
/// - **Null** on right deletes the key from the base.
pub fn deep_merge(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                if value.is_null() {
                    base_map.remove(key);
                } else if let Some(base_value) = base_map.get_mut(key) {
                    deep_merge(base_value, value);
                } else {
                    base_map.insert(key.clone(), value.clone());
                }
            }
        }
        (base, overlay) => {
            *base = overlay.clone();
        }
    }
}

/// Recursively normalize JSON object keys from `camelCase` to `snake_case`.
///
/// For example, `"mcpServers"` becomes `"mcp_servers"`, `"maxTokens"` becomes
/// `"max_tokens"`. Non-object values and already-snake_case keys pass through
/// unchanged.
pub fn normalize_keys(value: &mut Value) {
    match value {
        Value::Object(map) => {
            // Collect keys that need renaming to avoid borrow issues.
            let entries: Vec<(String, Value)> = map
                .iter()
                .map(|(k, v)| (camel_to_snake(k), v.clone()))
                .collect();

            map.clear();
            for (key, mut val) in entries {
                normalize_keys(&mut val);
                map.insert(key, val);
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                normalize_keys(item);
            }
        }
        _ => {}
    }
}

/// Convert a single `camelCase` string to `snake_case`.
///
/// Handles consecutive uppercase letters (e.g. `"HTMLParser"` -> `"html_parser"`).
fn camel_to_snake(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    let chars: Vec<char> = s.chars().collect();

    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_uppercase() {
            // Insert underscore before uppercase letter if:
            // - Not the first character, AND
            // - Previous character is lowercase, OR
            // - Next character exists and is lowercase (handles "HTMLParser" -> "html_parser")
            if i > 0 {
                let prev_lower = chars[i - 1].is_lowercase();
                let next_lower = chars.get(i + 1).is_some_and(|c| c.is_lowercase());
                if prev_lower || next_lower {
                    result.push('_');
                }
            }
            result.push(ch.to_lowercase().next().unwrap());
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── deep_merge tests ─────────────────────────────────────────────

    #[test]
    fn merge_objects_recursively() {
        let mut base = json!({"a": {"x": 1, "y": 2}, "b": 10});
        let overlay = json!({"a": {"y": 3, "z": 4}, "c": 20});
        deep_merge(&mut base, &overlay);

        assert_eq!(base["a"]["x"], 1); // untouched
        assert_eq!(base["a"]["y"], 3); // overridden
        assert_eq!(base["a"]["z"], 4); // added
        assert_eq!(base["b"], 10); // untouched
        assert_eq!(base["c"], 20); // added
    }

    #[test]
    fn merge_arrays_replaced_not_concatenated() {
        let mut base = json!({"list": [1, 2, 3]});
        let overlay = json!({"list": [4, 5]});
        deep_merge(&mut base, &overlay);

        assert_eq!(base["list"], json!([4, 5]));
    }

    #[test]
    fn merge_null_deletes_key() {
        let mut base = json!({"a": 1, "b": 2, "c": 3});
        let overlay = json!({"b": null});
        deep_merge(&mut base, &overlay);

        assert_eq!(base["a"], 1);
        assert!(base.get("b").is_none());
        assert_eq!(base["c"], 3);
    }

    #[test]
    fn merge_scalars_overridden() {
        let mut base = json!({"x": "old", "y": 42});
        let overlay = json!({"x": "new", "y": 99});
        deep_merge(&mut base, &overlay);

        assert_eq!(base["x"], "new");
        assert_eq!(base["y"], 99);
    }

    #[test]
    fn merge_nested_null_deletes_nested_key() {
        let mut base = json!({"a": {"x": 1, "y": 2}});
        let overlay = json!({"a": {"x": null}});
        deep_merge(&mut base, &overlay);

        assert!(base["a"].get("x").is_none());
        assert_eq!(base["a"]["y"], 2);
    }

    #[test]
    fn merge_empty_overlay_preserves_base() {
        let mut base = json!({"a": 1});
        let overlay = json!({});
        deep_merge(&mut base, &overlay);
        assert_eq!(base["a"], 1);
    }

    #[test]
    fn merge_into_empty_base() {
        let mut base = json!({});
        let overlay = json!({"a": 1, "b": {"c": 2}});
        deep_merge(&mut base, &overlay);
        assert_eq!(base["a"], 1);
        assert_eq!(base["b"]["c"], 2);
    }

    // ── camel_to_snake tests ─────────────────────────────────────────

    #[test]
    fn camel_to_snake_simple() {
        assert_eq!(camel_to_snake("mcpServers"), "mcp_servers");
        assert_eq!(camel_to_snake("maxTokens"), "max_tokens");
        assert_eq!(camel_to_snake("allowFrom"), "allow_from");
    }

    #[test]
    fn camel_to_snake_already_snake() {
        assert_eq!(camel_to_snake("already_snake"), "already_snake");
        assert_eq!(camel_to_snake("simple"), "simple");
    }

    #[test]
    fn camel_to_snake_consecutive_uppercase() {
        assert_eq!(camel_to_snake("HTMLParser"), "html_parser");
        assert_eq!(camel_to_snake("getHTTPResponse"), "get_http_response");
    }

    #[test]
    fn camel_to_snake_single_char() {
        assert_eq!(camel_to_snake("A"), "a");
        assert_eq!(camel_to_snake("x"), "x");
    }

    // ── normalize_keys tests ─────────────────────────────────────────

    #[test]
    fn normalize_camel_to_snake() {
        let mut val = json!({"mcpServers": {"testServer": {"apiKey": "k"}}});
        normalize_keys(&mut val);

        assert!(val.get("mcp_servers").is_some());
        assert!(val["mcp_servers"].get("test_server").is_some());
        assert!(val["mcp_servers"]["test_server"].get("api_key").is_some());
    }

    #[test]
    fn normalize_preserves_scalar_values() {
        let mut val = json!({"maxTokens": 8192, "apiKey": "secret"});
        normalize_keys(&mut val);

        assert_eq!(val["max_tokens"], 8192);
        assert_eq!(val["api_key"], "secret");
    }

    #[test]
    fn normalize_handles_arrays() {
        let mut val = json!({"allowFrom": [{"userId": "u1"}]});
        normalize_keys(&mut val);

        assert!(val.get("allow_from").is_some());
        assert_eq!(val["allow_from"][0]["user_id"], "u1");
    }

    #[test]
    fn normalize_empty_object() {
        let mut val = json!({});
        normalize_keys(&mut val);
        assert_eq!(val, json!({}));
    }

    #[test]
    fn normalize_non_object_is_noop() {
        let mut val = json!(42);
        normalize_keys(&mut val);
        assert_eq!(val, json!(42));
    }
}
