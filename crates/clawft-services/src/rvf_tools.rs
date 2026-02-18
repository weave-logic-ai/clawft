//! RVF (RuVector Format) tool provider for MCP.
//!
//! Exposes RVF vector operations as MCP tools via the [`ToolProvider`] trait.
//! This module is gated behind the `rvf` feature flag.
//!
//! Currently uses in-memory stub stores that implement the same argument
//! parsing and validation as the real RVF backend. When `rvf-runtime` and
//! friends are available, the stub operations will be replaced by calls
//! into the real vector engine.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use serde_json::Value;

use crate::mcp::ToolDefinition;
use crate::mcp::provider::{CallToolResult, ToolError, ToolProvider};

// ---------------------------------------------------------------------------
// Stub vector store
// ---------------------------------------------------------------------------

/// A single vector entry held in the stub store.
#[derive(Debug, Clone)]
struct StubEntry {
    id: String,
    embedding: Vec<f32>,
    metadata: HashMap<String, Value>,
}

/// In-memory stub vector store, standing in for a real RVF-backed store.
///
/// Supports insert, delete, brute-force query, and basic status reporting.
#[derive(Debug)]
struct StubVectorStore {
    dimension: usize,
    entries: Vec<StubEntry>,
}

impl StubVectorStore {
    fn new(dimension: usize) -> Self {
        Self {
            dimension,
            entries: Vec::new(),
        }
    }

    fn ingest(&mut self, id: String, embedding: Vec<f32>, metadata: HashMap<String, Value>) {
        // Overwrite if same id exists.
        self.entries.retain(|e| e.id != id);
        self.entries.push(StubEntry {
            id,
            embedding,
            metadata,
        });
    }

    fn delete(&mut self, id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        self.entries.len() < before
    }

    fn delete_filter(&mut self, key: &str, value: &Value) -> usize {
        let before = self.entries.len();
        self.entries.retain(|e| e.metadata.get(key) != Some(value));
        before - self.entries.len()
    }

    fn query(&self, embedding: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let mut scored: Vec<(String, f32)> = self
            .entries
            .iter()
            .map(|e| {
                let score = cosine_similarity(embedding, &e.embedding);
                (e.id.clone(), score)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn compact(&mut self) {
        // No-op for stub -- real RVF would reclaim freed segments.
    }
}

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    dot / (mag_a * mag_b)
}

// ---------------------------------------------------------------------------
// RvfToolProvider
// ---------------------------------------------------------------------------

/// Exposes RVF vector operations as MCP tools.
///
/// Manages a set of named vector stores and routes tool calls to the
/// appropriate operations: create, open, ingest, query, delete, compact,
/// status, list, route, and witness.
pub struct RvfToolProvider {
    stores: Mutex<HashMap<String, StubVectorStore>>,
    /// Audit trail for the `witness` tool.
    witness_log: Mutex<Vec<Value>>,
}

impl RvfToolProvider {
    /// Create a new RVF tool provider with no open stores.
    pub fn new() -> Self {
        Self {
            stores: Mutex::new(HashMap::new()),
            witness_log: Mutex::new(Vec::new()),
        }
    }
}

impl Default for RvfToolProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for RvfToolProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let store_count = self.stores.lock().map(|s| s.len()).unwrap_or(0);
        f.debug_struct("RvfToolProvider")
            .field("store_count", &store_count)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// ToolProvider impl
// ---------------------------------------------------------------------------

#[async_trait]
impl ToolProvider for RvfToolProvider {
    fn namespace(&self) -> &str {
        "rvf"
    }

    fn list_tools(&self) -> Vec<ToolDefinition> {
        vec![
            tool_def(
                "create_store",
                "Create a new vector store",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Store name" },
                        "dimension": { "type": "integer", "description": "Embedding dimension" }
                    },
                    "required": ["name", "dimension"]
                }),
            ),
            tool_def(
                "open_store",
                "Open an existing vector store",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Store name" }
                    },
                    "required": ["name"]
                }),
            ),
            tool_def(
                "ingest",
                "Add vectors to a store",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "store": { "type": "string", "description": "Store name" },
                        "id": { "type": "string", "description": "Vector entry ID" },
                        "embedding": {
                            "type": "array",
                            "items": { "type": "number" },
                            "description": "Embedding vector"
                        },
                        "metadata": {
                            "type": "object",
                            "description": "Optional metadata key-value pairs"
                        }
                    },
                    "required": ["store", "id", "embedding"]
                }),
            ),
            tool_def(
                "query",
                "Search a store by embedding vector",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "store": { "type": "string", "description": "Store name" },
                        "embedding": {
                            "type": "array",
                            "items": { "type": "number" },
                            "description": "Query embedding vector"
                        },
                        "top_k": {
                            "type": "integer",
                            "description": "Number of results to return",
                            "default": 10
                        }
                    },
                    "required": ["store", "embedding"]
                }),
            ),
            tool_def(
                "status",
                "Get store metrics and status",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "store": { "type": "string", "description": "Store name" }
                    },
                    "required": ["store"]
                }),
            ),
            tool_def(
                "delete",
                "Remove a vector by ID",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "store": { "type": "string", "description": "Store name" },
                        "id": { "type": "string", "description": "Vector entry ID to delete" }
                    },
                    "required": ["store", "id"]
                }),
            ),
            tool_def(
                "compact",
                "Reclaim space in a store",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "store": { "type": "string", "description": "Store name" }
                    },
                    "required": ["store"]
                }),
            ),
            tool_def(
                "delete_filter",
                "Delete vectors matching a metadata filter",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "store": { "type": "string", "description": "Store name" },
                        "key": { "type": "string", "description": "Metadata key to filter on" },
                        "value": { "description": "Value to match for deletion" }
                    },
                    "required": ["store", "key", "value"]
                }),
            ),
            tool_def(
                "list_stores",
                "List all open vector stores",
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            ),
            tool_def(
                "route",
                "Route a request to the optimal model tier",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "task_description": {
                            "type": "string",
                            "description": "Description of the task to route"
                        },
                        "complexity_hint": {
                            "type": "number",
                            "description": "Optional complexity score (0.0-1.0)"
                        }
                    },
                    "required": ["task_description"]
                }),
            ),
            tool_def(
                "witness",
                "Record an audit trail entry",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "action": { "type": "string", "description": "Action that was performed" },
                        "agent": { "type": "string", "description": "Agent that performed the action" },
                        "details": { "type": "object", "description": "Additional details" }
                    },
                    "required": ["action", "agent"]
                }),
            ),
        ]
    }

    async fn call_tool(&self, name: &str, args: Value) -> Result<CallToolResult, ToolError> {
        match name {
            "create_store" => self.handle_create_store(args),
            "open_store" => self.handle_open_store(args),
            "ingest" => self.handle_ingest(args),
            "query" => self.handle_query(args),
            "status" => self.handle_status(args),
            "delete" => self.handle_delete(args),
            "compact" => self.handle_compact(args),
            "delete_filter" => self.handle_delete_filter(args),
            "list_stores" => self.handle_list_stores(),
            "route" => self.handle_route(args),
            "witness" => self.handle_witness(args),
            _ => Err(ToolError::NotFound(name.to_string())),
        }
    }
}

// ---------------------------------------------------------------------------
// Tool handler implementations
// ---------------------------------------------------------------------------

impl RvfToolProvider {
    fn handle_create_store(&self, args: Value) -> Result<CallToolResult, ToolError> {
        let name = require_str(&args, "name")?;
        let dimension = require_u64(&args, "dimension")? as usize;

        if dimension == 0 {
            return Ok(CallToolResult::error("dimension must be > 0"));
        }

        let mut stores = self
            .stores
            .lock()
            .map_err(|e| ToolError::ExecutionFailed(format!("lock poisoned: {e}")))?;

        if stores.contains_key(&name) {
            return Ok(CallToolResult::error(format!(
                "store \"{name}\" already exists"
            )));
        }

        stores.insert(name.clone(), StubVectorStore::new(dimension));

        Ok(CallToolResult::text(
            serde_json::json!({
                "store": name,
                "dimension": dimension,
                "status": "created"
            })
            .to_string(),
        ))
    }

    fn handle_open_store(&self, args: Value) -> Result<CallToolResult, ToolError> {
        let name = require_str(&args, "name")?;

        let stores = self
            .stores
            .lock()
            .map_err(|e| ToolError::ExecutionFailed(format!("lock poisoned: {e}")))?;

        if stores.contains_key(&name) {
            Ok(CallToolResult::text(
                serde_json::json!({
                    "store": name,
                    "status": "open",
                    "entries": stores[&name].len()
                })
                .to_string(),
            ))
        } else {
            Ok(CallToolResult::error(format!("store \"{name}\" not found")))
        }
    }

    fn handle_ingest(&self, args: Value) -> Result<CallToolResult, ToolError> {
        let store_name = require_str(&args, "store")?;
        let id = require_str(&args, "id")?;
        let embedding = require_f32_array(&args, "embedding")?;

        let metadata: HashMap<String, Value> = args
            .get("metadata")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        let mut stores = self
            .stores
            .lock()
            .map_err(|e| ToolError::ExecutionFailed(format!("lock poisoned: {e}")))?;

        let store = stores.get_mut(&store_name).ok_or_else(|| {
            ToolError::ExecutionFailed(format!("store \"{store_name}\" not found"))
        })?;

        if embedding.len() != store.dimension {
            return Ok(CallToolResult::error(format!(
                "dimension mismatch: expected {}, got {}",
                store.dimension,
                embedding.len()
            )));
        }

        store.ingest(id.clone(), embedding, metadata);

        Ok(CallToolResult::text(
            serde_json::json!({
                "store": store_name,
                "id": id,
                "status": "ingested",
                "total_entries": store.len()
            })
            .to_string(),
        ))
    }

    fn handle_query(&self, args: Value) -> Result<CallToolResult, ToolError> {
        let store_name = require_str(&args, "store")?;
        let embedding = require_f32_array(&args, "embedding")?;
        let top_k = args.get("top_k").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

        let stores = self
            .stores
            .lock()
            .map_err(|e| ToolError::ExecutionFailed(format!("lock poisoned: {e}")))?;

        let store = stores.get(&store_name).ok_or_else(|| {
            ToolError::ExecutionFailed(format!("store \"{store_name}\" not found"))
        })?;

        if embedding.len() != store.dimension {
            return Ok(CallToolResult::error(format!(
                "dimension mismatch: expected {}, got {}",
                store.dimension,
                embedding.len()
            )));
        }

        let results = store.query(&embedding, top_k);
        let results_json: Vec<Value> = results
            .into_iter()
            .map(|(id, score)| {
                serde_json::json!({
                    "id": id,
                    "score": score
                })
            })
            .collect();

        Ok(CallToolResult::text(
            serde_json::json!({
                "store": store_name,
                "results": results_json,
                "count": results_json.len()
            })
            .to_string(),
        ))
    }

    fn handle_status(&self, args: Value) -> Result<CallToolResult, ToolError> {
        let store_name = require_str(&args, "store")?;

        let stores = self
            .stores
            .lock()
            .map_err(|e| ToolError::ExecutionFailed(format!("lock poisoned: {e}")))?;

        let store = stores.get(&store_name).ok_or_else(|| {
            ToolError::ExecutionFailed(format!("store \"{store_name}\" not found"))
        })?;

        Ok(CallToolResult::text(
            serde_json::json!({
                "store": store_name,
                "entries": store.len(),
                "dimension": store.dimension,
                "index_type": "brute_force_stub",
                "backend": "in_memory"
            })
            .to_string(),
        ))
    }

    fn handle_delete(&self, args: Value) -> Result<CallToolResult, ToolError> {
        let store_name = require_str(&args, "store")?;
        let id = require_str(&args, "id")?;

        let mut stores = self
            .stores
            .lock()
            .map_err(|e| ToolError::ExecutionFailed(format!("lock poisoned: {e}")))?;

        let store = stores.get_mut(&store_name).ok_or_else(|| {
            ToolError::ExecutionFailed(format!("store \"{store_name}\" not found"))
        })?;

        let deleted = store.delete(&id);

        Ok(CallToolResult::text(
            serde_json::json!({
                "store": store_name,
                "id": id,
                "deleted": deleted,
                "remaining_entries": store.len()
            })
            .to_string(),
        ))
    }

    fn handle_compact(&self, args: Value) -> Result<CallToolResult, ToolError> {
        let store_name = require_str(&args, "store")?;

        let mut stores = self
            .stores
            .lock()
            .map_err(|e| ToolError::ExecutionFailed(format!("lock poisoned: {e}")))?;

        let store = stores.get_mut(&store_name).ok_or_else(|| {
            ToolError::ExecutionFailed(format!("store \"{store_name}\" not found"))
        })?;

        store.compact();

        Ok(CallToolResult::text(
            serde_json::json!({
                "store": store_name,
                "status": "compacted"
            })
            .to_string(),
        ))
    }

    fn handle_delete_filter(&self, args: Value) -> Result<CallToolResult, ToolError> {
        let store_name = require_str(&args, "store")?;
        let key = require_str(&args, "key")?;
        let value = args
            .get("value")
            .cloned()
            .ok_or_else(|| ToolError::ExecutionFailed("missing required field: value".into()))?;

        let mut stores = self
            .stores
            .lock()
            .map_err(|e| ToolError::ExecutionFailed(format!("lock poisoned: {e}")))?;

        let store = stores.get_mut(&store_name).ok_or_else(|| {
            ToolError::ExecutionFailed(format!("store \"{store_name}\" not found"))
        })?;

        let removed = store.delete_filter(&key, &value);

        Ok(CallToolResult::text(
            serde_json::json!({
                "store": store_name,
                "filter_key": key,
                "removed_count": removed,
                "remaining_entries": store.len()
            })
            .to_string(),
        ))
    }

    fn handle_list_stores(&self) -> Result<CallToolResult, ToolError> {
        let stores = self
            .stores
            .lock()
            .map_err(|e| ToolError::ExecutionFailed(format!("lock poisoned: {e}")))?;

        let store_list: Vec<Value> = stores
            .iter()
            .map(|(name, store)| {
                serde_json::json!({
                    "name": name,
                    "entries": store.len(),
                    "dimension": store.dimension
                })
            })
            .collect();

        Ok(CallToolResult::text(
            serde_json::json!({
                "stores": store_list,
                "count": store_list.len()
            })
            .to_string(),
        ))
    }

    fn handle_route(&self, args: Value) -> Result<CallToolResult, ToolError> {
        let task_description = require_str(&args, "task_description")?;
        let complexity_hint = args
            .get("complexity_hint")
            .and_then(|v| v.as_f64())
            .unwrap_or_else(|| estimate_complexity(&task_description));

        // Stub routing logic: route based on complexity score.
        let (tier, model) = if complexity_hint < 0.3 {
            (1, "haiku")
        } else if complexity_hint < 0.7 {
            (2, "sonnet")
        } else {
            (3, "opus")
        };

        Ok(CallToolResult::text(
            serde_json::json!({
                "task": task_description,
                "complexity": complexity_hint,
                "tier": tier,
                "recommended_model": model,
                "routing_method": "stub_complexity_threshold"
            })
            .to_string(),
        ))
    }

    fn handle_witness(&self, args: Value) -> Result<CallToolResult, ToolError> {
        let action = require_str(&args, "action")?;
        let agent = require_str(&args, "agent")?;
        let details = args.get("details").cloned().unwrap_or(Value::Null);

        let entry = serde_json::json!({
            "action": action,
            "agent": agent,
            "details": details,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        let mut log = self
            .witness_log
            .lock()
            .map_err(|e| ToolError::ExecutionFailed(format!("lock poisoned: {e}")))?;

        log.push(entry.clone());
        let sequence = log.len();

        Ok(CallToolResult::text(
            serde_json::json!({
                "status": "recorded",
                "sequence": sequence,
                "entry": entry
            })
            .to_string(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a [`ToolDefinition`] from parts.
fn tool_def(name: &str, description: &str, input_schema: Value) -> ToolDefinition {
    ToolDefinition {
        name: name.to_string(),
        description: description.to_string(),
        input_schema,
    }
}

/// Extract a required string field from JSON args.
fn require_str(args: &Value, field: &str) -> Result<String, ToolError> {
    args.get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            ToolError::ExecutionFailed(format!("missing or invalid required field: {field}"))
        })
}

/// Extract a required u64 field from JSON args.
fn require_u64(args: &Value, field: &str) -> Result<u64, ToolError> {
    args.get(field).and_then(|v| v.as_u64()).ok_or_else(|| {
        ToolError::ExecutionFailed(format!("missing or invalid required field: {field}"))
    })
}

/// Extract a required array of f32 from JSON args.
fn require_f32_array(args: &Value, field: &str) -> Result<Vec<f32>, ToolError> {
    let arr = args.get(field).and_then(|v| v.as_array()).ok_or_else(|| {
        ToolError::ExecutionFailed(format!("missing or invalid required field: {field}"))
    })?;

    arr.iter()
        .map(|v| {
            v.as_f64().map(|f| f as f32).ok_or_else(|| {
                ToolError::ExecutionFailed(format!("non-numeric value in {field} array"))
            })
        })
        .collect()
}

/// Stub complexity estimator based on task description length and keywords.
fn estimate_complexity(description: &str) -> f64 {
    let word_count = description.split_whitespace().count();
    let has_complex_keywords = ["architecture", "security", "refactor", "migrate", "design"]
        .iter()
        .any(|kw| description.to_lowercase().contains(kw));

    let base = (word_count as f64 / 50.0).min(0.5);
    if has_complex_keywords {
        (base + 0.4).min(1.0)
    } else {
        base
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_provider() -> RvfToolProvider {
        RvfToolProvider::new()
    }

    // ── Basic trait tests ────────────────────────────────────────────

    #[test]
    fn namespace_returns_rvf() {
        let provider = make_provider();
        assert_eq!(provider.namespace(), "rvf");
    }

    #[test]
    fn list_tools_returns_11_tools() {
        let provider = make_provider();
        let tools = provider.list_tools();
        assert_eq!(tools.len(), 11);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"create_store"));
        assert!(names.contains(&"open_store"));
        assert!(names.contains(&"ingest"));
        assert!(names.contains(&"query"));
        assert!(names.contains(&"status"));
        assert!(names.contains(&"delete"));
        assert!(names.contains(&"compact"));
        assert!(names.contains(&"delete_filter"));
        assert!(names.contains(&"list_stores"));
        assert!(names.contains(&"route"));
        assert!(names.contains(&"witness"));
    }

    #[test]
    fn unknown_tool_returns_not_found() {
        let provider = make_provider();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(provider.call_tool("nonexistent", json!({})));
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::NotFound(name) => assert_eq!(name, "nonexistent"),
            other => panic!("expected NotFound, got: {other}"),
        }
    }

    #[test]
    fn debug_format() {
        let provider = make_provider();
        let debug = format!("{:?}", provider);
        assert!(debug.contains("RvfToolProvider"));
        assert!(debug.contains("store_count: 0"));
    }

    // ── create_store ────────────────────────────────────────────────

    #[tokio::test]
    async fn create_store_succeeds() {
        let provider = make_provider();
        let result = provider
            .call_tool("create_store", json!({"name": "test", "dimension": 128}))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = extract_text(&result);
        let parsed: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["store"], "test");
        assert_eq!(parsed["dimension"], 128);
        assert_eq!(parsed["status"], "created");
    }

    #[tokio::test]
    async fn create_store_duplicate_returns_error() {
        let provider = make_provider();
        provider
            .call_tool("create_store", json!({"name": "dup", "dimension": 64}))
            .await
            .unwrap();
        let result = provider
            .call_tool("create_store", json!({"name": "dup", "dimension": 64}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(extract_text(&result).contains("already exists"));
    }

    #[tokio::test]
    async fn create_store_zero_dimension_returns_error() {
        let provider = make_provider();
        let result = provider
            .call_tool("create_store", json!({"name": "bad", "dimension": 0}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(extract_text(&result).contains("dimension must be > 0"));
    }

    #[tokio::test]
    async fn create_store_missing_name_returns_error() {
        let provider = make_provider();
        let result = provider
            .call_tool("create_store", json!({"dimension": 128}))
            .await;
        assert!(result.is_err());
    }

    // ── ingest + query round-trip ───────────────────────────────────

    #[tokio::test]
    async fn ingest_and_query_round_trip() {
        let provider = make_provider();

        // Create store.
        provider
            .call_tool("create_store", json!({"name": "vec", "dimension": 3}))
            .await
            .unwrap();

        // Ingest two vectors.
        let result = provider
            .call_tool(
                "ingest",
                json!({
                    "store": "vec",
                    "id": "a",
                    "embedding": [1.0, 0.0, 0.0],
                    "metadata": {"label": "alpha"}
                }),
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        let parsed: Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(parsed["total_entries"], 1);

        provider
            .call_tool(
                "ingest",
                json!({
                    "store": "vec",
                    "id": "b",
                    "embedding": [0.0, 1.0, 0.0]
                }),
            )
            .await
            .unwrap();

        // Query -- vector a should be closest to [1, 0, 0].
        let result = provider
            .call_tool(
                "query",
                json!({
                    "store": "vec",
                    "embedding": [1.0, 0.1, 0.0],
                    "top_k": 2
                }),
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        let parsed: Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(parsed["count"], 2);
        let results = parsed["results"].as_array().unwrap();
        assert_eq!(results[0]["id"], "a");
    }

    #[tokio::test]
    async fn ingest_dimension_mismatch_returns_error() {
        let provider = make_provider();
        provider
            .call_tool("create_store", json!({"name": "dim3", "dimension": 3}))
            .await
            .unwrap();

        let result = provider
            .call_tool(
                "ingest",
                json!({
                    "store": "dim3",
                    "id": "bad",
                    "embedding": [1.0, 2.0]
                }),
            )
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(extract_text(&result).contains("dimension mismatch"));
    }

    #[tokio::test]
    async fn ingest_nonexistent_store_returns_error() {
        let provider = make_provider();
        let result = provider
            .call_tool(
                "ingest",
                json!({
                    "store": "missing",
                    "id": "x",
                    "embedding": [1.0]
                }),
            )
            .await;
        assert!(result.is_err());
    }

    // ── delete ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_removes_entry() {
        let provider = make_provider();
        provider
            .call_tool("create_store", json!({"name": "del", "dimension": 2}))
            .await
            .unwrap();
        provider
            .call_tool(
                "ingest",
                json!({"store": "del", "id": "x", "embedding": [1.0, 0.0]}),
            )
            .await
            .unwrap();

        let result = provider
            .call_tool("delete", json!({"store": "del", "id": "x"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        let parsed: Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(parsed["deleted"], true);
        assert_eq!(parsed["remaining_entries"], 0);
    }

    #[tokio::test]
    async fn delete_nonexistent_entry_returns_false() {
        let provider = make_provider();
        provider
            .call_tool("create_store", json!({"name": "del2", "dimension": 2}))
            .await
            .unwrap();

        let result = provider
            .call_tool("delete", json!({"store": "del2", "id": "ghost"}))
            .await
            .unwrap();
        let parsed: Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(parsed["deleted"], false);
    }

    // ── delete_filter ───────────────────────────────────────────────

    #[tokio::test]
    async fn delete_filter_removes_matching() {
        let provider = make_provider();
        provider
            .call_tool("create_store", json!({"name": "df", "dimension": 2}))
            .await
            .unwrap();
        provider
            .call_tool(
                "ingest",
                json!({
                    "store": "df", "id": "a",
                    "embedding": [1.0, 0.0],
                    "metadata": {"tag": "red"}
                }),
            )
            .await
            .unwrap();
        provider
            .call_tool(
                "ingest",
                json!({
                    "store": "df", "id": "b",
                    "embedding": [0.0, 1.0],
                    "metadata": {"tag": "blue"}
                }),
            )
            .await
            .unwrap();

        let result = provider
            .call_tool(
                "delete_filter",
                json!({"store": "df", "key": "tag", "value": "red"}),
            )
            .await
            .unwrap();
        let parsed: Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(parsed["removed_count"], 1);
        assert_eq!(parsed["remaining_entries"], 1);
    }

    // ── status ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn status_returns_metrics() {
        let provider = make_provider();
        provider
            .call_tool("create_store", json!({"name": "st", "dimension": 64}))
            .await
            .unwrap();
        provider
            .call_tool(
                "ingest",
                json!({
                    "store": "st",
                    "id": "v1",
                    "embedding": vec![0.0_f32; 64]
                }),
            )
            .await
            .unwrap();

        let result = provider
            .call_tool("status", json!({"store": "st"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        let parsed: Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(parsed["entries"], 1);
        assert_eq!(parsed["dimension"], 64);
        assert_eq!(parsed["index_type"], "brute_force_stub");
    }

    // ── compact ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn compact_succeeds() {
        let provider = make_provider();
        provider
            .call_tool("create_store", json!({"name": "cmp", "dimension": 2}))
            .await
            .unwrap();
        let result = provider
            .call_tool("compact", json!({"store": "cmp"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        let parsed: Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(parsed["status"], "compacted");
    }

    // ── list_stores ─────────────────────────────────────────────────

    #[tokio::test]
    async fn list_stores_empty() {
        let provider = make_provider();
        let result = provider.call_tool("list_stores", json!({})).await.unwrap();
        let parsed: Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(parsed["count"], 0);
    }

    #[tokio::test]
    async fn list_stores_after_create() {
        let provider = make_provider();
        provider
            .call_tool("create_store", json!({"name": "s1", "dimension": 16}))
            .await
            .unwrap();
        provider
            .call_tool("create_store", json!({"name": "s2", "dimension": 32}))
            .await
            .unwrap();

        let result = provider.call_tool("list_stores", json!({})).await.unwrap();
        let parsed: Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(parsed["count"], 2);
    }

    // ── open_store ──────────────────────────────────────────────────

    #[tokio::test]
    async fn open_store_found() {
        let provider = make_provider();
        provider
            .call_tool("create_store", json!({"name": "op", "dimension": 8}))
            .await
            .unwrap();
        let result = provider
            .call_tool("open_store", json!({"name": "op"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        let parsed: Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(parsed["status"], "open");
    }

    #[tokio::test]
    async fn open_store_not_found() {
        let provider = make_provider();
        let result = provider
            .call_tool("open_store", json!({"name": "ghost"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(extract_text(&result).contains("not found"));
    }

    // ── route ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn route_simple_task_routes_to_haiku() {
        let provider = make_provider();
        let result = provider
            .call_tool(
                "route",
                json!({"task_description": "add a comment", "complexity_hint": 0.1}),
            )
            .await
            .unwrap();
        let parsed: Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(parsed["tier"], 1);
        assert_eq!(parsed["recommended_model"], "haiku");
    }

    #[tokio::test]
    async fn route_complex_task_routes_to_opus() {
        let provider = make_provider();
        let result = provider
            .call_tool(
                "route",
                json!({"task_description": "redesign the security architecture", "complexity_hint": 0.9}),
            )
            .await
            .unwrap();
        let parsed: Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(parsed["tier"], 3);
        assert_eq!(parsed["recommended_model"], "opus");
    }

    #[tokio::test]
    async fn route_auto_complexity_estimation() {
        let provider = make_provider();
        // Short description should estimate low complexity.
        let result = provider
            .call_tool("route", json!({"task_description": "fix typo"}))
            .await
            .unwrap();
        let parsed: Value = serde_json::from_str(&extract_text(&result)).unwrap();
        // "fix typo" = 2 words, base=2/50=0.04, no complex keywords => tier 1
        assert_eq!(parsed["tier"], 1);
    }

    // ── witness ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn witness_records_entry() {
        let provider = make_provider();
        let result = provider
            .call_tool(
                "witness",
                json!({
                    "action": "tool_call",
                    "agent": "coder-1",
                    "details": {"tool": "ingest", "store": "memory"}
                }),
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        let parsed: Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(parsed["status"], "recorded");
        assert_eq!(parsed["sequence"], 1);
        assert!(parsed["entry"]["timestamp"].is_string());
    }

    #[tokio::test]
    async fn witness_increments_sequence() {
        let provider = make_provider();
        provider
            .call_tool("witness", json!({"action": "a", "agent": "x"}))
            .await
            .unwrap();
        let result = provider
            .call_tool("witness", json!({"action": "b", "agent": "y"}))
            .await
            .unwrap();
        let parsed: Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(parsed["sequence"], 2);
    }

    // ── Composite integration ───────────────────────────────────────

    #[tokio::test]
    async fn composite_routes_rvf_query_correctly() {
        use crate::mcp::composite::CompositeToolProvider;

        let mut composite = CompositeToolProvider::new();
        composite.register(Box::new(RvfToolProvider::new()));

        // Verify tools are listed with rvf__ prefix.
        let tools = composite.list_tools_all();
        assert_eq!(tools.len(), 11);
        assert!(tools.iter().any(|t| t.name == "rvf__create_store"));
        assert!(tools.iter().any(|t| t.name == "rvf__query"));

        // Create a store via the composite.
        let result = composite
            .call_tool("rvf__create_store", json!({"name": "comp", "dimension": 3}))
            .await
            .unwrap();
        assert!(!result.is_error);

        // Ingest via composite.
        composite
            .call_tool(
                "rvf__ingest",
                json!({"store": "comp", "id": "v1", "embedding": [1.0, 0.0, 0.0]}),
            )
            .await
            .unwrap();

        // Query via composite.
        let result = composite
            .call_tool(
                "rvf__query",
                json!({"store": "comp", "embedding": [1.0, 0.0, 0.0], "top_k": 1}),
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        let parsed: Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(parsed["count"], 1);
        assert_eq!(parsed["results"][0]["id"], "v1");
    }

    // ── Helpers ─────────────────────────────────────────────────────

    fn extract_text(result: &CallToolResult) -> String {
        match &result.content[0] {
            crate::mcp::ContentBlock::Text { text } => text.clone(),
        }
    }

    // ── cosine_similarity unit tests ────────────────────────────────

    #[test]
    fn cosine_similarity_identical() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_zero_vector() {
        let a = vec![0.0, 0.0];
        let b = vec![1.0, 1.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    // ── estimate_complexity ─────────────────────────────────────────

    #[test]
    fn estimate_complexity_short_simple() {
        let c = estimate_complexity("fix typo");
        assert!(c < 0.3);
    }

    #[test]
    fn estimate_complexity_with_keyword() {
        let c = estimate_complexity("refactor the authentication module");
        assert!(c >= 0.4);
    }

    // ── ingest overwrites existing id ───────────────────────────────

    #[tokio::test]
    async fn ingest_overwrites_same_id() {
        let provider = make_provider();
        provider
            .call_tool("create_store", json!({"name": "ow", "dimension": 2}))
            .await
            .unwrap();
        provider
            .call_tool(
                "ingest",
                json!({"store": "ow", "id": "a", "embedding": [1.0, 0.0]}),
            )
            .await
            .unwrap();
        provider
            .call_tool(
                "ingest",
                json!({"store": "ow", "id": "a", "embedding": [0.0, 1.0]}),
            )
            .await
            .unwrap();

        // Status should show 1 entry, not 2.
        let result = provider
            .call_tool("status", json!({"store": "ow"}))
            .await
            .unwrap();
        let parsed: Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(parsed["entries"], 1);
    }
}
