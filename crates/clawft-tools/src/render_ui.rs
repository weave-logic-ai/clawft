//! Render UI tool for pushing canvas elements to the Live Canvas.
//!
//! Agents call this tool to render, update, or remove UI elements
//! on the canvas. The tool parses the input as a [`CanvasCommand`]
//! and logs it. Real message bus integration will come in a later
//! milestone; for now this is a functional stub that validates
//! input and returns success.

use async_trait::async_trait;
use clawft_core::tools::registry::{Tool, ToolError};
use clawft_types::canvas::CanvasCommand;
use serde_json::json;
use tracing::{debug, info};

/// Tool that agents invoke to push UI elements to the canvas.
///
/// Accepts a JSON payload conforming to the [`CanvasCommand`] protocol
/// and (in the future) publishes it to the message bus for WebSocket
/// delivery to connected clients.
pub struct RenderUiTool;

impl RenderUiTool {
    /// Create a new `RenderUiTool`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for RenderUiTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for RenderUiTool {
    fn name(&self) -> &str {
        "render_ui"
    }

    fn description(&self) -> &str {
        "Render a UI element on the Live Canvas. Supports text, buttons, inputs, images, code blocks, tables, and forms."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The canvas command: render, update, remove, reset, or batch",
                    "enum": ["render", "update", "remove", "reset", "batch"]
                },
                "id": {
                    "type": "string",
                    "description": "Element ID (required for render, update, remove)"
                },
                "element": {
                    "type": "object",
                    "description": "The canvas element to render or update",
                    "properties": {
                        "type": {
                            "type": "string",
                            "description": "Element type: text, button, input, image, code, table, form",
                            "enum": ["text", "button", "input", "image", "code", "table", "form"]
                        }
                    }
                },
                "position": {
                    "type": "integer",
                    "description": "Optional position index for render command"
                },
                "commands": {
                    "type": "array",
                    "description": "Array of sub-commands for batch command",
                    "items": { "type": "object" }
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        // Parse the input as a CanvasCommand.
        let command: CanvasCommand = serde_json::from_value(args.clone()).map_err(|e| {
            ToolError::InvalidArgs(format!("invalid canvas command: {e}"))
        })?;

        // Extract the element ID for the response (if applicable).
        let element_id = match &command {
            CanvasCommand::Render { id, .. } => Some(id.clone()),
            CanvasCommand::Update { id, .. } => Some(id.clone()),
            CanvasCommand::Remove { id } => Some(id.clone()),
            CanvasCommand::Reset => None,
            CanvasCommand::Batch { commands } => {
                info!(count = commands.len(), "processing batch canvas command");
                None
            }
        };

        debug!(?command, "render_ui tool invoked");

        // In the future, this will publish to the message bus.
        // For now, return success with the element ID.
        Ok(json!({
            "status": "rendered",
            "element_id": element_id,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool() -> RenderUiTool {
        RenderUiTool::new()
    }

    #[test]
    fn name_is_render_ui() {
        assert_eq!(make_tool().name(), "render_ui");
    }

    #[test]
    fn description_not_empty() {
        assert!(!make_tool().description().is_empty());
    }

    #[test]
    fn parameters_has_command_field() {
        let params = make_tool().parameters();
        let required = params["required"].as_array().unwrap();
        assert!(required.contains(&json!("command")));
    }

    #[test]
    fn default_impl() {
        let tool = RenderUiTool::default();
        assert_eq!(tool.name(), "render_ui");
    }

    #[tokio::test]
    async fn render_text_element() {
        let result = make_tool()
            .execute(json!({
                "command": "render",
                "id": "el-1",
                "element": { "type": "text", "content": "Hello" }
            }))
            .await
            .unwrap();

        assert_eq!(result["status"], "rendered");
        assert_eq!(result["element_id"], "el-1");
    }

    #[tokio::test]
    async fn render_button_element() {
        let result = make_tool()
            .execute(json!({
                "command": "render",
                "id": "btn-1",
                "element": {
                    "type": "button",
                    "label": "Click me",
                    "action": "do_thing"
                }
            }))
            .await
            .unwrap();

        assert_eq!(result["status"], "rendered");
        assert_eq!(result["element_id"], "btn-1");
    }

    #[tokio::test]
    async fn update_element() {
        let result = make_tool()
            .execute(json!({
                "command": "update",
                "id": "el-1",
                "element": { "type": "text", "content": "Updated" }
            }))
            .await
            .unwrap();

        assert_eq!(result["status"], "rendered");
        assert_eq!(result["element_id"], "el-1");
    }

    #[tokio::test]
    async fn remove_element() {
        let result = make_tool()
            .execute(json!({
                "command": "remove",
                "id": "el-1"
            }))
            .await
            .unwrap();

        assert_eq!(result["status"], "rendered");
        assert_eq!(result["element_id"], "el-1");
    }

    #[tokio::test]
    async fn reset_canvas() {
        let result = make_tool()
            .execute(json!({ "command": "reset" }))
            .await
            .unwrap();

        assert_eq!(result["status"], "rendered");
        assert!(result["element_id"].is_null());
    }

    #[tokio::test]
    async fn batch_command() {
        let result = make_tool()
            .execute(json!({
                "command": "batch",
                "commands": [
                    { "command": "reset" },
                    {
                        "command": "render",
                        "id": "el-1",
                        "element": { "type": "text", "content": "Fresh" }
                    }
                ]
            }))
            .await
            .unwrap();

        assert_eq!(result["status"], "rendered");
    }

    #[tokio::test]
    async fn invalid_command_returns_error() {
        let err = make_tool()
            .execute(json!({ "command": "invalid_cmd" }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs(_)));
    }

    #[tokio::test]
    async fn missing_command_returns_error() {
        let err = make_tool()
            .execute(json!({ "id": "el-1" }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs(_)));
    }

    #[tokio::test]
    async fn render_with_position() {
        let result = make_tool()
            .execute(json!({
                "command": "render",
                "id": "el-top",
                "element": { "type": "text", "content": "First" },
                "position": 0
            }))
            .await
            .unwrap();

        assert_eq!(result["status"], "rendered");
        assert_eq!(result["element_id"], "el-top");
    }

    #[tokio::test]
    async fn render_code_element() {
        let result = make_tool()
            .execute(json!({
                "command": "render",
                "id": "code-1",
                "element": {
                    "type": "code",
                    "code": "fn main() {}",
                    "language": "rust"
                }
            }))
            .await
            .unwrap();

        assert_eq!(result["status"], "rendered");
        assert_eq!(result["element_id"], "code-1");
    }

    #[tokio::test]
    async fn render_table_element() {
        let result = make_tool()
            .execute(json!({
                "command": "render",
                "id": "tbl-1",
                "element": {
                    "type": "table",
                    "headers": ["Name", "Age"],
                    "rows": [["Alice", "30"]]
                }
            }))
            .await
            .unwrap();

        assert_eq!(result["status"], "rendered");
        assert_eq!(result["element_id"], "tbl-1");
    }

    #[tokio::test]
    async fn render_form_element() {
        let result = make_tool()
            .execute(json!({
                "command": "render",
                "id": "form-1",
                "element": {
                    "type": "form",
                    "fields": [{
                        "name": "username",
                        "label": "Username",
                        "field_type": "text",
                        "required": true
                    }],
                    "submit_action": "create_user"
                }
            }))
            .await
            .unwrap();

        assert_eq!(result["status"], "rendered");
        assert_eq!(result["element_id"], "form-1");
    }

    #[test]
    fn tool_is_object_safe() {
        fn accepts_tool(_t: &dyn Tool) {}
        accepts_tool(&make_tool());
    }
}
