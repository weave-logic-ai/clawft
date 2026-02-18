//! Schema conversion utilities for tool delegation.
//!
//! Converts between OpenAI function-calling format and Anthropic tool format,
//! enabling tools defined in one schema to be used with the other provider.

use serde_json::{Value, json};

/// Convert an array of OpenAI function-calling tool schemas to Anthropic
/// tool format.
///
/// # OpenAI format (input)
/// ```json
/// {
///   "type": "function",
///   "function": {
///     "name": "get_weather",
///     "description": "Get the current weather",
///     "parameters": { "type": "object", "properties": { ... } }
///   }
/// }
/// ```
///
/// # Anthropic format (output)
/// ```json
/// {
///   "name": "get_weather",
///   "description": "Get the current weather",
///   "input_schema": { "type": "object", "properties": { ... } }
/// }
/// ```
///
/// Tools that cannot be converted (missing `function` field, etc.) are
/// silently skipped.
pub fn openai_to_anthropic(schemas: &[Value]) -> Vec<Value> {
    schemas
        .iter()
        .filter_map(|tool| {
            let func = tool.get("function")?;
            let name = func.get("name")?.as_str()?;
            let description = func
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("");

            let input_schema = func
                .get("parameters")
                .cloned()
                .unwrap_or_else(|| json!({"type": "object", "properties": {}}));

            Some(json!({
                "name": name,
                "description": description,
                "input_schema": input_schema,
            }))
        })
        .collect()
}

/// Build an Anthropic `tool_result` content block.
///
/// # Anthropic tool_result format
/// ```json
/// {
///   "type": "tool_result",
///   "tool_use_id": "toolu_abc123",
///   "content": "The weather in SF is 72F and sunny."
/// }
/// ```
pub fn tool_result_block(tool_use_id: &str, result: &str) -> Value {
    json!({
        "type": "tool_result",
        "tool_use_id": tool_use_id,
        "content": result,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn openai_to_anthropic_single_tool() {
        let openai = vec![json!({
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get the current weather for a location",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "City name"
                        }
                    },
                    "required": ["location"]
                }
            }
        })];

        let anthropic = openai_to_anthropic(&openai);
        assert_eq!(anthropic.len(), 1);

        let tool = &anthropic[0];
        assert_eq!(tool["name"], "get_weather");
        assert_eq!(
            tool["description"],
            "Get the current weather for a location"
        );
        assert_eq!(tool["input_schema"]["type"], "object");
        assert_eq!(
            tool["input_schema"]["properties"]["location"]["type"],
            "string"
        );
        assert_eq!(tool["input_schema"]["required"][0], "location");
    }

    #[test]
    fn openai_to_anthropic_multiple_tools() {
        let openai = vec![
            json!({
                "type": "function",
                "function": {
                    "name": "tool_a",
                    "description": "Tool A",
                    "parameters": {"type": "object", "properties": {}}
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "tool_b",
                    "description": "Tool B",
                    "parameters": {"type": "object", "properties": {}}
                }
            }),
        ];

        let anthropic = openai_to_anthropic(&openai);
        assert_eq!(anthropic.len(), 2);
        assert_eq!(anthropic[0]["name"], "tool_a");
        assert_eq!(anthropic[1]["name"], "tool_b");
    }

    #[test]
    fn openai_to_anthropic_missing_description() {
        let openai = vec![json!({
            "type": "function",
            "function": {
                "name": "no_desc",
                "parameters": {"type": "object", "properties": {}}
            }
        })];

        let anthropic = openai_to_anthropic(&openai);
        assert_eq!(anthropic.len(), 1);
        assert_eq!(anthropic[0]["name"], "no_desc");
        // Missing description defaults to empty string.
        assert_eq!(anthropic[0]["description"], "");
    }

    #[test]
    fn openai_to_anthropic_missing_parameters() {
        let openai = vec![json!({
            "type": "function",
            "function": {
                "name": "no_params",
                "description": "No parameters"
            }
        })];

        let anthropic = openai_to_anthropic(&openai);
        assert_eq!(anthropic.len(), 1);
        assert_eq!(anthropic[0]["input_schema"]["type"], "object");
    }

    #[test]
    fn openai_to_anthropic_skips_invalid() {
        let openai = vec![
            json!({"type": "function"}), // no "function" key content (name missing)
            json!({"not_a_tool": true}), // no "function" key at all
            json!({
                "type": "function",
                "function": {
                    "name": "valid",
                    "description": "A valid tool",
                    "parameters": {"type": "object", "properties": {}}
                }
            }),
        ];

        let anthropic = openai_to_anthropic(&openai);
        assert_eq!(anthropic.len(), 1);
        assert_eq!(anthropic[0]["name"], "valid");
    }

    #[test]
    fn openai_to_anthropic_empty_input() {
        let anthropic = openai_to_anthropic(&[]);
        assert!(anthropic.is_empty());
    }

    #[test]
    fn tool_result_block_format() {
        let block = tool_result_block("toolu_abc123", "The answer is 42.");
        assert_eq!(block["type"], "tool_result");
        assert_eq!(block["tool_use_id"], "toolu_abc123");
        assert_eq!(block["content"], "The answer is 42.");
    }

    #[test]
    fn tool_result_block_empty_result() {
        let block = tool_result_block("toolu_xyz", "");
        assert_eq!(block["type"], "tool_result");
        assert_eq!(block["tool_use_id"], "toolu_xyz");
        assert_eq!(block["content"], "");
    }

    #[test]
    fn tool_result_block_json_content() {
        let result = r#"{"status":"ok","data":[1,2,3]}"#;
        let block = tool_result_block("toolu_json", result);
        assert_eq!(block["content"], result);
    }
}
