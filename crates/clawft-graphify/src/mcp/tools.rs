//! MCP tool definitions for graphify (WEFT-369).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tool name constants (public wire names — no namespace prefix).
pub const TOOL_QUERY: &str = "graphify_query";
pub const TOOL_INGEST: &str = "graphify_ingest";
pub const TOOL_EXPORT: &str = "graphify_export";
pub const TOOL_DIFF: &str = "graphify_diff";

/// All tool names in list order.
pub const TOOL_NAMES: &[&str] = &[TOOL_QUERY, TOOL_INGEST, TOOL_EXPORT, TOOL_DIFF];

/// MCP tool definition (JSON Schema input).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Definitions advertised via `tools/list`.
pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: TOOL_QUERY.into(),
            description: "Search the knowledge graph with a natural-language or keyword question. \
                 Scores nodes by label/source match and optionally traverses BFS/DFS from top hits."
                .into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "Question or keyword query"
                    },
                    "graph": {
                        "type": "string",
                        "description": "Path to graph JSON (default: graphify-out/graph.json)",
                        "default": "graphify-out/graph.json"
                    },
                    "mode": {
                        "type": "string",
                        "description": "Traversal mode",
                        "enum": ["bfs", "dfs", "none"],
                        "default": "bfs"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Traversal depth (1-6)",
                        "default": 3,
                        "minimum": 1,
                        "maximum": 6
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max ranked matches to return",
                        "default": 10
                    }
                },
                "required": ["question"]
            }),
        },
        ToolDefinition {
            name: TOOL_INGEST.into(),
            description: "Ingest a local directory into the knowledge graph (file-level entities + \
                 clustering), or a URL when the http-client feature is enabled. Writes graphify-out/."
                .into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "target": {
                        "type": "string",
                        "description": "Local path (directory) or http(s) URL"
                    },
                    "output": {
                        "type": "string",
                        "description": "Output root for graph artifacts (default: <target>/graphify-out or ./graphify-out)",
                        "default": "graphify-out"
                    },
                    "contributor": {
                        "type": "string",
                        "description": "Optional contributor tag for URL ingest metadata"
                    }
                },
                "required": ["target"]
            }),
        },
        ToolDefinition {
            name: TOOL_EXPORT.into(),
            description: "Export a knowledge graph JSON file to a supported format (json, graphml, \
                 cypher, html, obsidian, wiki, svg, vowl — availability depends on features)."
                .into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "format": {
                        "type": "string",
                        "description": "Export format",
                        "enum": ["json", "graphml", "cypher", "html", "obsidian", "wiki", "svg", "vowl"]
                    },
                    "graph": {
                        "type": "string",
                        "description": "Source graph JSON path",
                        "default": "graphify-out/graph.json"
                    },
                    "output": {
                        "type": "string",
                        "description": "Output path (default depends on format)"
                    }
                },
                "required": ["format"]
            }),
        },
        ToolDefinition {
            name: TOOL_DIFF.into(),
            description: "Compare two knowledge graph JSON snapshots and summarize node/edge changes."
                .into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "old": {
                        "type": "string",
                        "description": "Previous graph JSON path",
                        "default": "graphify-out/graph.json.bak"
                    },
                    "current": {
                        "type": "string",
                        "description": "Current graph JSON path",
                        "default": "graphify-out/graph.json"
                    }
                }
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tools_have_object_schemas() {
        let defs = tool_definitions();
        assert_eq!(defs.len(), TOOL_NAMES.len());
        for def in &defs {
            assert!(TOOL_NAMES.contains(&def.name.as_str()));
            assert_eq!(def.input_schema["type"], "object");
            assert!(def.input_schema.get("properties").is_some());
        }
    }
}
