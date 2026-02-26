//! Configuration file discovery and loading.
//!
//! Ports the config discovery algorithm from Python `nanobot/config/loader.py`.
//!
//! The discovery order is:
//! 1. `CLAWFT_CONFIG` environment variable (absolute path).
//! 2. `~/.clawft/config.json`
//! 3. `~/.nanobot/config.json` (legacy fallback).
//! 4. If none found, return an empty JSON object (`{}`).
//!
//! JSON keys are normalized from camelCase to snake_case before returning,
//! matching the Python behavior where Pydantic models use snake_case field names.

use std::path::PathBuf;

use serde_json::Value;

/// Discover the config file path using the fallback chain.
///
/// Returns `None` if no config file exists at any of the candidate locations.
/// The discovery order is:
/// 1. Path from `CLAWFT_CONFIG` environment variable.
/// 2. `~/.clawft/config.json`
/// 3. `~/.nanobot/config.json`
///
/// On native targets, candidate paths are checked for existence using
/// synchronous `Path::exists()`. On non-native targets (WASM), the first
/// candidate path is returned without a filesystem existence check --
/// the caller's async `fs.exists()` handles validation.
pub fn discover_config_path(
    env: &dyn super::env::Environment,
    home_dir: Option<PathBuf>,
) -> Option<PathBuf> {
    // Step 1: Check CLAWFT_CONFIG env var
    if let Some(env_path) = env.get_var("CLAWFT_CONFIG") {
        let path = PathBuf::from(env_path);
        return Some(path);
    }

    // Step 2 & 3: Check home directory paths
    if let Some(home) = home_dir {
        let clawft_path = home.join(".clawft").join("config.json");

        #[cfg(feature = "native")]
        {
            if clawft_path.exists() {
                return Some(clawft_path);
            }

            let nanobot_path = home.join(".nanobot").join("config.json");
            if nanobot_path.exists() {
                return Some(nanobot_path);
            }
        }

        // On non-native (WASM) targets, return the preferred path without
        // synchronous filesystem checks. The caller validates asynchronously.
        #[cfg(not(feature = "native"))]
        {
            return Some(clawft_path);
        }
    }

    None
}

/// Load raw JSON configuration using the discovery algorithm.
///
/// Returns the parsed and key-normalized JSON value. The caller (typically
/// `clawft-types`) deserializes this into a typed `Config` struct.
///
/// If no config file is found, returns an empty JSON object.
pub async fn load_config_raw(
    fs: &dyn super::fs::FileSystem,
    env: &dyn super::env::Environment,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let home = fs.home_dir();
    let path = discover_config_path(env, home);

    let Some(path) = path else {
        tracing::info!("no config file found, using defaults");
        return Ok(Value::Object(serde_json::Map::new()));
    };

    if !fs.exists(&path).await {
        tracing::warn!(
            path = %path.display(),
            "config path does not exist, using defaults"
        );
        return Ok(Value::Object(serde_json::Map::new()));
    }

    tracing::debug!(path = %path.display(), "loading config file");
    let contents = fs
        .read_to_string(&path)
        .await
        .map_err(|e| format!("failed to read config file {}: {}", path.display(), e))?;

    let value: Value = serde_json::from_str(&contents)
        .map_err(|e| format!("failed to parse config file {}: {}", path.display(), e))?;

    Ok(normalize_keys(value))
}

/// Convert camelCase JSON keys to snake_case recursively.
///
/// Processes objects and arrays recursively. Non-object/array values are
/// returned unchanged. This matches the Python `convert_keys()` function
/// from `nanobot/config/loader.py`.
pub fn normalize_keys(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (key, val) in map {
                let snake_key = camel_to_snake(&key);
                new_map.insert(snake_key, normalize_keys(val));
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(normalize_keys).collect()),
        other => other,
    }
}

/// Convert a single camelCase string to snake_case.
///
/// Handles consecutive uppercase letters (acronyms) correctly:
/// a run of uppercase letters like `"HTML"` is kept together, with an
/// underscore inserted only before the last uppercase letter if it is
/// followed by a lowercase letter (indicating the start of a new word).
///
/// # Examples
/// ```
/// # use clawft_platform::config_loader::camel_to_snake;
/// assert_eq!(camel_to_snake("camelCase"), "camel_case");
/// assert_eq!(camel_to_snake("systemPrompt"), "system_prompt");
/// assert_eq!(camel_to_snake("already_snake"), "already_snake");
/// assert_eq!(camel_to_snake("HTMLParser"), "html_parser");
/// assert_eq!(camel_to_snake("getHTMLParser"), "get_html_parser");
/// assert_eq!(camel_to_snake("simpleXML"), "simple_xml");
/// ```
pub fn camel_to_snake(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut result = String::with_capacity(name.len() + 4);

    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_uppercase() && i > 0 {
            let prev = chars[i - 1];
            let next = chars.get(i + 1).copied();

            // Insert underscore before:
            // 1. An uppercase letter preceded by a lowercase letter (camelCase boundary)
            // 2. An uppercase letter followed by a lowercase letter, when preceded
            //    by an uppercase letter (end of acronym: "HTMLParser" -> "html_parser")
            if prev.is_lowercase()
                || (prev.is_uppercase() && next.is_some_and(|c| c.is_lowercase()))
            {
                result.push('_');
            }
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── camel_to_snake tests ──────────────────────────────────────────

    #[test]
    fn test_camel_to_snake_basic() {
        assert_eq!(camel_to_snake("camelCase"), "camel_case");
    }

    #[test]
    fn test_camel_to_snake_multiple_words() {
        assert_eq!(camel_to_snake("systemPrompt"), "system_prompt");
        assert_eq!(camel_to_snake("contextWindow"), "context_window");
        assert_eq!(
            camel_to_snake("memoryConsolidation"),
            "memory_consolidation"
        );
    }

    #[test]
    fn test_camel_to_snake_already_snake() {
        assert_eq!(camel_to_snake("already_snake"), "already_snake");
    }

    #[test]
    fn test_camel_to_snake_single_word() {
        assert_eq!(camel_to_snake("model"), "model");
    }

    #[test]
    fn test_camel_to_snake_all_upper() {
        // Pure acronym stays together (no trailing lowercase to split on)
        assert_eq!(camel_to_snake("HTML"), "html");
    }

    #[test]
    fn test_camel_to_snake_acronym_then_word() {
        // Acronym followed by a word: split before the last uppercase
        assert_eq!(camel_to_snake("HTMLParser"), "html_parser");
        assert_eq!(camel_to_snake("getHTMLParser"), "get_html_parser");
        assert_eq!(camel_to_snake("simpleXML"), "simple_xml");
        assert_eq!(camel_to_snake("XMLHTTPRequest"), "xmlhttp_request");
    }

    #[test]
    fn test_camel_to_snake_leading_upper() {
        // Leading uppercase is just lowered, no underscore
        assert_eq!(camel_to_snake("Config"), "config");
    }

    #[test]
    fn test_camel_to_snake_empty() {
        assert_eq!(camel_to_snake(""), "");
    }

    // ── normalize_keys tests ──────────────────────────────────────────

    #[test]
    fn test_normalize_keys_flat_object() {
        let input = json!({
            "systemPrompt": "hello",
            "contextWindow": 4096
        });
        let expected = json!({
            "system_prompt": "hello",
            "context_window": 4096
        });
        assert_eq!(normalize_keys(input), expected);
    }

    #[test]
    fn test_normalize_keys_nested_object() {
        let input = json!({
            "agentsConfig": {
                "defaultModel": "gpt-4",
                "maxTokens": 1024
            }
        });
        let expected = json!({
            "agents_config": {
                "default_model": "gpt-4",
                "max_tokens": 1024
            }
        });
        assert_eq!(normalize_keys(input), expected);
    }

    #[test]
    fn test_normalize_keys_array() {
        let input = json!([
            {"firstName": "Alice"},
            {"firstName": "Bob"}
        ]);
        let expected = json!([
            {"first_name": "Alice"},
            {"first_name": "Bob"}
        ]);
        assert_eq!(normalize_keys(input), expected);
    }

    #[test]
    fn test_normalize_keys_primitives_unchanged() {
        assert_eq!(normalize_keys(json!(42)), json!(42));
        assert_eq!(normalize_keys(json!("hello")), json!("hello"));
        assert_eq!(normalize_keys(json!(true)), json!(true));
        assert_eq!(normalize_keys(json!(null)), json!(null));
    }

    #[test]
    fn test_normalize_keys_empty_object() {
        assert_eq!(normalize_keys(json!({})), json!({}));
    }

    // ── discover_config_path tests ────────────────────────────────────

    /// A minimal mock environment for config discovery tests.
    struct MockEnv {
        vars: std::collections::HashMap<String, String>,
    }

    impl MockEnv {
        fn new() -> Self {
            Self {
                vars: std::collections::HashMap::new(),
            }
        }

        fn with_var(mut self, key: &str, value: &str) -> Self {
            self.vars.insert(key.to_string(), value.to_string());
            self
        }
    }

    impl super::super::env::Environment for MockEnv {
        fn get_var(&self, name: &str) -> Option<String> {
            self.vars.get(name).cloned()
        }

        fn set_var(&self, _name: &str, _value: &str) {
            // No-op for mock
        }

        fn remove_var(&self, _name: &str) {
            // No-op for mock
        }
    }

    #[test]
    fn test_discover_env_var_takes_precedence() {
        let env = MockEnv::new().with_var("CLAWFT_CONFIG", "/custom/config.json");
        let result = discover_config_path(&env, Some(PathBuf::from("/home/user")));
        assert_eq!(result, Some(PathBuf::from("/custom/config.json")));
    }

    #[test]
    fn test_discover_no_home_no_env() {
        let env = MockEnv::new();
        let result = discover_config_path(&env, None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_discover_home_but_no_files() {
        // When home dir is given but no config files exist on disk,
        // discover returns None (both .clawft and .nanobot paths don't exist).
        let env = MockEnv::new();
        // Use a path that definitely does not contain config files.
        let result = discover_config_path(
            &env,
            Some(PathBuf::from("/tmp/clawft_test_nonexistent_home")),
        );
        assert_eq!(result, None);
    }
}
