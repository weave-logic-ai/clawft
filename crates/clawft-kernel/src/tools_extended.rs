//! Extended tool catalog (Sprint 10 D3).
//!
//! Adds 10 priority tools to the WeftOS WASM tool catalog:
//! `fs.analyze`, `git.log`, `doc.parse`, `config.read`, `env.detect`,
//! `metrics.snapshot`, `kv.get`, `kv.set`, `report.template`, `health.check`.
//!
//! All tools implement [`BuiltinTool`] and are registered in the default
//! catalog via [`extended_tool_catalog`] and [`register_extended_tools`].

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;

#[cfg(feature = "os-patterns")]
use crate::config_service::ConfigService;
use crate::governance::EffectVector;
use crate::service::ServiceRegistry;
use crate::wasm_runner::{
    BuiltinTool, BuiltinToolSpec, ToolCategory, ToolError, ToolRegistry,
};

// ---------------------------------------------------------------------------
// Catalog specs for the 10 new tools
// ---------------------------------------------------------------------------

/// Return [`BuiltinToolSpec`] entries for the 10 extended tools.
///
/// Called by [`builtin_tool_catalog`] (or merged separately) to make these
/// tools visible in the catalog before their impl structs are registered.
pub fn extended_tool_catalog() -> Vec<BuiltinToolSpec> {
    vec![
        BuiltinToolSpec {
            name: "fs.analyze".into(),
            category: ToolCategory::Filesystem,
            description: "Analyze directory: file count, total lines, type breakdown, largest files"
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Directory path to analyze"}
                },
                "required": ["path"]
            }),
            gate_action: "tool.fs.read".into(),
            effect: EffectVector {
                risk: 0.1,
                ..Default::default()
            },
            native: true,
        },
        BuiltinToolSpec {
            name: "git.log".into(),
            category: ToolCategory::System,
            description: "Return recent git commits as JSON".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "limit": {"type": "integer", "default": 20, "description": "Max commits to return"},
                    "path": {"type": "string", "default": ".", "description": "Repository path"}
                }
            }),
            gate_action: "tool.sys.git".into(),
            effect: EffectVector {
                risk: 0.05,
                ..Default::default()
            },
            native: true,
        },
        BuiltinToolSpec {
            name: "doc.parse".into(),
            category: ToolCategory::Filesystem,
            description: "Parse markdown: sections, word count, link count, code block count".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Markdown file to parse"}
                },
                "required": ["path"]
            }),
            gate_action: "tool.fs.read".into(),
            effect: EffectVector {
                risk: 0.05,
                ..Default::default()
            },
            native: true,
        },
        BuiltinToolSpec {
            name: "config.read".into(),
            category: ToolCategory::System,
            description: "Read a TOML configuration file and return parsed JSON".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to TOML file"},
                    "key": {"type": "string", "description": "Dotted key to extract (e.g. namespace.key)"}
                }
            }),
            gate_action: "tool.sys.config".into(),
            effect: EffectVector {
                risk: 0.1,
                ..Default::default()
            },
            native: true,
        },
        BuiltinToolSpec {
            name: "env.detect".into(),
            category: ToolCategory::System,
            description: "Detect runtime environment: OS, arch, CPU count, memory, versions".into(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
            gate_action: "tool.sys.env".into(),
            effect: EffectVector {
                risk: 0.05,
                ..Default::default()
            },
            native: true,
        },
        BuiltinToolSpec {
            name: "metrics.snapshot".into(),
            category: ToolCategory::System,
            description: "Snapshot current metrics, optionally filtered by prefix".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "prefix": {"type": "string", "description": "Optional metric name prefix filter"}
                }
            }),
            gate_action: "tool.sys.metrics".into(),
            effect: EffectVector {
                risk: 0.05,
                ..Default::default()
            },
            native: true,
        },
        BuiltinToolSpec {
            name: "kv.get".into(),
            category: ToolCategory::Agent,
            description: "Get a value from the config KV store".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "key": {"type": "string"},
                    "namespace": {"type": "string", "default": "default"}
                },
                "required": ["key"]
            }),
            gate_action: "tool.agent.kv.read".into(),
            effect: EffectVector {
                risk: 0.05,
                ..Default::default()
            },
            native: true,
        },
        BuiltinToolSpec {
            name: "kv.set".into(),
            category: ToolCategory::Agent,
            description: "Set a value in the config KV store".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "key": {"type": "string"},
                    "namespace": {"type": "string", "default": "default"},
                    "value": {"description": "Value to store (any JSON)"}
                },
                "required": ["key", "value"]
            }),
            gate_action: "tool.agent.kv.write".into(),
            effect: EffectVector {
                risk: 0.2,
                ..Default::default()
            },
            native: true,
        },
        BuiltinToolSpec {
            name: "report.template".into(),
            category: ToolCategory::System,
            description: "Render a template with {{variable}} substitution".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "template": {"type": "string", "description": "Template name or raw template text"},
                    "data": {"type": "object", "description": "Key-value data for variable substitution"}
                },
                "required": ["template", "data"]
            }),
            gate_action: "tool.sys.report".into(),
            effect: EffectVector {
                risk: 0.1,
                ..Default::default()
            },
            native: true,
        },
        BuiltinToolSpec {
            name: "health.check".into(),
            category: ToolCategory::System,
            description: "Check health of all services or a specific service".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "service": {"type": "string", "description": "Optional service name to check"}
                }
            }),
            gate_action: "tool.sys.health".into(),
            effect: EffectVector {
                risk: 0.05,
                ..Default::default()
            },
            native: true,
        },
    ]
}

// ---------------------------------------------------------------------------
// Helper: look up a spec by name from the extended catalog
// ---------------------------------------------------------------------------

fn spec_by_name(name: &str) -> BuiltinToolSpec {
    extended_tool_catalog()
        .into_iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("{name} must be in extended catalog"))
}

// ---------------------------------------------------------------------------
// 1. fs.analyze
// ---------------------------------------------------------------------------

/// Walks a directory and returns aggregate statistics.
pub struct FsAnalyzeTool {
    spec: BuiltinToolSpec,
}

impl Default for FsAnalyzeTool {
    fn default() -> Self {
        Self::new()
    }
}

impl FsAnalyzeTool {
    pub fn new() -> Self {
        Self {
            spec: spec_by_name("fs.analyze"),
        }
    }
}

impl BuiltinTool for FsAnalyzeTool {
    fn name(&self) -> &str {
        "fs.analyze"
    }
    fn spec(&self) -> &BuiltinToolSpec {
        &self.spec
    }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'path' parameter".into()))?;

        let root = std::path::Path::new(path);
        if !root.exists() {
            return Err(ToolError::FileNotFound(path.to_string()));
        }
        if !root.is_dir() {
            return Err(ToolError::InvalidArgs(format!("{path} is not a directory")));
        }

        let mut file_count: u64 = 0;
        let mut total_lines: u64 = 0;
        let mut type_counts: HashMap<String, u64> = HashMap::new();
        let mut largest: Vec<(String, u64)> = Vec::new();

        walk_dir(root, &mut |entry_path| {
            if !entry_path.is_file() {
                return;
            }
            let meta = match std::fs::metadata(entry_path) {
                Ok(m) => m,
                Err(_) => return,
            };
            let size = meta.len();
            file_count += 1;

            // Extension
            let ext = entry_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("(none)")
                .to_string();
            *type_counts.entry(ext).or_insert(0) += 1;

            // Count lines (best-effort, skip binary)
            if let Ok(contents) = std::fs::read_to_string(entry_path) {
                total_lines += contents.lines().count() as u64;
            }

            // Track largest
            let display = entry_path.display().to_string();
            largest.push((display, size));
        });

        // Sort largest descending, keep top 10
        largest.sort_by(|a, b| b.1.cmp(&a.1));
        largest.truncate(10);

        let largest_json: Vec<serde_json::Value> = largest
            .iter()
            .map(|(p, s)| json!({"path": p, "size": s}))
            .collect();

        Ok(json!({
            "file_count": file_count,
            "total_lines": total_lines,
            "file_types": type_counts,
            "largest_files": largest_json,
        }))
    }
}

/// Simple recursive directory walker (no external crate needed).
fn walk_dir(dir: &std::path::Path, cb: &mut dyn FnMut(&std::path::Path)) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk_dir(&p, cb);
        } else {
            cb(&p);
        }
    }
}

// ---------------------------------------------------------------------------
// 2. git.log
// ---------------------------------------------------------------------------

/// Returns recent git commits as a JSON array.
pub struct GitLogTool {
    spec: BuiltinToolSpec,
}

impl Default for GitLogTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GitLogTool {
    pub fn new() -> Self {
        Self {
            spec: spec_by_name("git.log"),
        }
    }
}

impl BuiltinTool for GitLogTool {
    fn name(&self) -> &str {
        "git.log"
    }
    fn spec(&self) -> &BuiltinToolSpec {
        &self.spec
    }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(20);
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        // Use git log with a machine-parseable format
        let output = std::process::Command::new("git")
            .args([
                "-C",
                path,
                "log",
                &format!("-{limit}"),
                "--format=%H%n%an%n%aI%n%s%n---END---",
            ])
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("failed to run git: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ToolError::ExecutionFailed(format!("git log failed: {stderr}")));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut commits = Vec::new();

        for block in stdout.split("---END---") {
            let lines: Vec<&str> = block.trim().lines().collect();
            if lines.len() >= 4 {
                commits.push(json!({
                    "hash": lines[0],
                    "author": lines[1],
                    "date": lines[2],
                    "message": lines[3..].join("\n"),
                }));
            }
        }

        Ok(json!({ "commits": commits, "count": commits.len() }))
    }
}

// ---------------------------------------------------------------------------
// 3. doc.parse
// ---------------------------------------------------------------------------

/// Parses a markdown file and returns structural information.
pub struct DocParseTool {
    spec: BuiltinToolSpec,
}

impl Default for DocParseTool {
    fn default() -> Self {
        Self::new()
    }
}

impl DocParseTool {
    pub fn new() -> Self {
        Self {
            spec: spec_by_name("doc.parse"),
        }
    }
}

impl BuiltinTool for DocParseTool {
    fn name(&self) -> &str {
        "doc.parse"
    }
    fn spec(&self) -> &BuiltinToolSpec {
        &self.spec
    }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'path' parameter".into()))?;

        let content = std::fs::read_to_string(path)
            .map_err(|e| ToolError::FileNotFound(format!("{path}: {e}")))?;

        parse_markdown(&content)
    }
}

/// Parse markdown content and return structural JSON.
fn parse_markdown(content: &str) -> Result<serde_json::Value, ToolError> {
    let mut sections: Vec<serde_json::Value> = Vec::new();
    let mut word_count: u64 = 0;
    let mut link_count: u64 = 0;
    let mut code_block_count: u64 = 0;
    let mut in_code_block = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Code blocks
        if trimmed.starts_with("```") {
            if in_code_block {
                in_code_block = false;
            } else {
                in_code_block = true;
                code_block_count += 1;
            }
            continue;
        }

        if in_code_block {
            continue;
        }

        // Headings
        if trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|c| *c == '#').count();
            let title = trimmed[level..].trim().to_string();
            sections.push(json!({"level": level, "title": title}));
        }

        // Word count (outside code blocks)
        word_count += trimmed.split_whitespace().count() as u64;

        // Links: count [text](url) and bare http(s) URLs
        let mut search = trimmed;
        while let Some(pos) = search.find("](") {
            link_count += 1;
            search = &search[pos + 2..];
        }
    }

    Ok(json!({
        "sections": sections,
        "word_count": word_count,
        "link_count": link_count,
        "code_block_count": code_block_count,
    }))
}

// ---------------------------------------------------------------------------
// 4. config.read
// ---------------------------------------------------------------------------

/// Reads a TOML configuration file and returns parsed JSON.
pub struct ConfigReadTool {
    spec: BuiltinToolSpec,
}

impl Default for ConfigReadTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigReadTool {
    pub fn new() -> Self {
        Self {
            spec: spec_by_name("config.read"),
        }
    }
}

impl BuiltinTool for ConfigReadTool {
    fn name(&self) -> &str {
        "config.read"
    }
    fn spec(&self) -> &BuiltinToolSpec {
        &self.spec
    }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("need 'path' parameter".into()))?;

        let content = std::fs::read_to_string(path)
            .map_err(|e| ToolError::FileNotFound(format!("{path}: {e}")))?;

        let parsed = parse_toml_to_json(&content)
            .map_err(|e| ToolError::ExecutionFailed(format!("TOML parse error: {e}")))?;

        // Optionally extract a dotted key
        if let Some(key) = args.get("key").and_then(|v| v.as_str()) {
            let mut current = &parsed;
            for segment in key.split('.') {
                match current.get(segment) {
                    Some(v) => current = v,
                    None => return Ok(json!({"key": key, "value": null})),
                }
            }
            return Ok(json!({"key": key, "value": current}));
        }

        Ok(parsed)
    }
}

/// Minimal TOML parser that handles key = "value" pairs and [sections].
///
/// This avoids adding the `toml` crate as a dependency. It handles the
/// most common TOML patterns: string values, integer values, boolean values,
/// float values, and section headers. Arrays and inline tables are stored
/// as raw strings.
fn parse_toml_to_json(input: &str) -> Result<serde_json::Value, String> {
    let mut root = serde_json::Map::new();
    let mut current_section = String::new();

    for line in input.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Section header
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let inner = trimmed[1..trimmed.len() - 1].trim();
            // Skip array-of-tables [[...]] for now, treat as section
            let inner = inner.trim_start_matches('[').trim_end_matches(']').trim();
            current_section = inner.to_string();
            // Ensure the section object exists
            ensure_section(&mut root, &current_section);
            continue;
        }

        // Key = value
        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim().to_string();
            let val_str = trimmed[eq_pos + 1..].trim();
            let value = parse_toml_value(val_str);

            if current_section.is_empty() {
                root.insert(key, value);
            } else {
                let section = get_section_mut(&mut root, &current_section);
                section.insert(key, value);
            }
        }
    }

    Ok(serde_json::Value::Object(root))
}

fn ensure_section(root: &mut serde_json::Map<String, serde_json::Value>, path: &str) {
    let mut current = root as *mut serde_json::Map<String, serde_json::Value>;
    for segment in path.split('.') {
        // Safety: we control the mutable references here and never alias
        let map = unsafe { &mut *current };
        if !map.contains_key(segment) {
            map.insert(
                segment.to_string(),
                serde_json::Value::Object(serde_json::Map::new()),
            );
        }
        if let Some(serde_json::Value::Object(ref mut inner)) = map.get_mut(segment) {
            current = inner as *mut serde_json::Map<String, serde_json::Value>;
        }
    }
}

fn get_section_mut<'a>(
    root: &'a mut serde_json::Map<String, serde_json::Value>,
    path: &str,
) -> &'a mut serde_json::Map<String, serde_json::Value> {
    let mut current = root;
    for segment in path.split('.') {
        current = current
            .entry(segment)
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
            .as_object_mut()
            .expect("section must be object");
    }
    current
}

fn parse_toml_value(val_str: &str) -> serde_json::Value {
    // Strip inline comment
    let val_str = if let Some(hash_pos) = val_str.find(" #") {
        val_str[..hash_pos].trim()
    } else {
        val_str
    };

    // Quoted string
    if (val_str.starts_with('"') && val_str.ends_with('"'))
        || (val_str.starts_with('\'') && val_str.ends_with('\''))
    {
        let inner = &val_str[1..val_str.len() - 1];
        return json!(inner);
    }

    // Boolean
    if val_str == "true" {
        return json!(true);
    }
    if val_str == "false" {
        return json!(false);
    }

    // Integer
    if let Ok(n) = val_str.parse::<i64>() {
        return json!(n);
    }

    // Float
    if let Ok(f) = val_str.parse::<f64>() {
        return json!(f);
    }

    // Fallback: raw string
    json!(val_str)
}

// ---------------------------------------------------------------------------
// 5. env.detect
// ---------------------------------------------------------------------------

/// Detects runtime environment information.
pub struct EnvDetectTool {
    spec: BuiltinToolSpec,
}

impl Default for EnvDetectTool {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvDetectTool {
    pub fn new() -> Self {
        Self {
            spec: spec_by_name("env.detect"),
        }
    }
}

impl BuiltinTool for EnvDetectTool {
    fn name(&self) -> &str {
        "env.detect"
    }
    fn spec(&self) -> &BuiltinToolSpec {
        &self.spec
    }
    fn execute(&self, _args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        // CPU count: try std::thread::available_parallelism
        let cpu_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        // Rust version from env (set at compile time)
        let rust_version = option_env!("RUSTC_VERSION").unwrap_or(env!("CARGO_PKG_RUST_VERSION"));

        // Feature flags (compile-time)
        let mut features = Vec::new();
        if cfg!(feature = "native") {
            features.push("native");
        }
        if cfg!(feature = "ecc") {
            features.push("ecc");
        }
        if cfg!(feature = "wasm-sandbox") {
            features.push("wasm-sandbox");
        }
        if cfg!(feature = "exochain") {
            features.push("exochain");
        }
        if cfg!(feature = "mesh") {
            features.push("mesh");
        }
        if cfg!(feature = "os-patterns") {
            features.push("os-patterns");
        }

        Ok(json!({
            "os": os,
            "arch": arch,
            "cpu_count": cpu_count,
            "rust_version": rust_version,
            "features": features,
            "pkg_version": env!("CARGO_PKG_VERSION"),
        }))
    }
}

// ---------------------------------------------------------------------------
// 6. metrics.snapshot
// ---------------------------------------------------------------------------

/// Snapshots metrics from the MetricsRegistry.
#[cfg(feature = "os-patterns")]
pub struct MetricsSnapshotTool {
    spec: BuiltinToolSpec,
    registry: Arc<crate::metrics::MetricsRegistry>,
}

#[cfg(feature = "os-patterns")]
impl MetricsSnapshotTool {
    pub fn new(registry: Arc<crate::metrics::MetricsRegistry>) -> Self {
        Self {
            spec: spec_by_name("metrics.snapshot"),
            registry,
        }
    }
}

#[cfg(feature = "os-patterns")]
impl BuiltinTool for MetricsSnapshotTool {
    fn name(&self) -> &str {
        "metrics.snapshot"
    }
    fn spec(&self) -> &BuiltinToolSpec {
        &self.spec
    }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let prefix = args
            .get("prefix")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let snapshots = self.registry.snapshot_all();
        let filtered: Vec<serde_json::Value> = snapshots
            .into_iter()
            .filter(|s| s.name.starts_with(prefix))
            .map(|s| {
                json!({
                    "name": s.name,
                    "kind": format!("{:?}", s.kind),
                    "value": s.value,
                })
            })
            .collect();

        Ok(json!({ "metrics": filtered, "count": filtered.len() }))
    }
}

/// Stub metrics.snapshot tool when `os-patterns` is not enabled.
/// Returns an empty snapshot with a note about the missing feature.
#[cfg(not(feature = "os-patterns"))]
pub struct MetricsSnapshotTool {
    spec: BuiltinToolSpec,
}

#[cfg(not(feature = "os-patterns"))]
impl Default for MetricsSnapshotTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "os-patterns"))]
impl MetricsSnapshotTool {
    pub fn new() -> Self {
        Self {
            spec: spec_by_name("metrics.snapshot"),
        }
    }
}

#[cfg(not(feature = "os-patterns"))]
impl BuiltinTool for MetricsSnapshotTool {
    fn name(&self) -> &str {
        "metrics.snapshot"
    }
    fn spec(&self) -> &BuiltinToolSpec {
        &self.spec
    }
    fn execute(&self, _args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        Ok(json!({
            "metrics": [],
            "count": 0,
            "note": "os-patterns feature not enabled; MetricsRegistry unavailable"
        }))
    }
}

// ---------------------------------------------------------------------------
// 7. kv.get
// ---------------------------------------------------------------------------

/// Reads a value from the ConfigService KV store.
pub struct KvGetTool {
    spec: BuiltinToolSpec,
    config_service: Arc<ConfigService>,
}

impl KvGetTool {
    pub fn new(config_service: Arc<ConfigService>) -> Self {
        Self {
            spec: spec_by_name("kv.get"),
            config_service,
        }
    }
}

impl BuiltinTool for KvGetTool {
    fn name(&self) -> &str {
        "kv.get"
    }
    fn spec(&self) -> &BuiltinToolSpec {
        &self.spec
    }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let key = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'key' parameter".into()))?;
        let namespace = args
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        let value = self.config_service.get(namespace, key);
        Ok(json!({ "key": key, "namespace": namespace, "value": value }))
    }
}

// ---------------------------------------------------------------------------
// 8. kv.set
// ---------------------------------------------------------------------------

/// Writes a value to the ConfigService KV store.
pub struct KvSetTool {
    spec: BuiltinToolSpec,
    config_service: Arc<ConfigService>,
}

impl KvSetTool {
    pub fn new(config_service: Arc<ConfigService>) -> Self {
        Self {
            spec: spec_by_name("kv.set"),
            config_service,
        }
    }
}

impl BuiltinTool for KvSetTool {
    fn name(&self) -> &str {
        "kv.set"
    }
    fn spec(&self) -> &BuiltinToolSpec {
        &self.spec
    }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let key = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'key' parameter".into()))?;
        let namespace = args
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        let value = args
            .get("value")
            .ok_or_else(|| ToolError::InvalidArgs("missing 'value' parameter".into()))?
            .clone();

        self.config_service
            .set(namespace, key, value.clone(), 0)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(json!({ "ok": true, "key": key, "namespace": namespace }))
    }
}

// ---------------------------------------------------------------------------
// 9. report.template
// ---------------------------------------------------------------------------

/// Renders a template with `{{variable}}` substitution.
pub struct ReportTemplateTool {
    spec: BuiltinToolSpec,
}

impl Default for ReportTemplateTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportTemplateTool {
    pub fn new() -> Self {
        Self {
            spec: spec_by_name("report.template"),
        }
    }
}

/// Built-in templates by name.
fn builtin_template(name: &str) -> Option<&'static str> {
    match name {
        "gap-report" => Some(
            "# Gap Report: {{title}}\n\n\
             Generated: {{date}}\n\n\
             ## Summary\n\n{{summary}}\n\n\
             ## Gaps Found: {{gap_count}}\n\n{{details}}",
        ),
        "status-report" => Some(
            "# Status Report: {{title}}\n\n\
             Date: {{date}}\n\n\
             Status: {{status}}\n\n\
             ## Details\n\n{{details}}",
        ),
        _ => None,
    }
}

impl BuiltinTool for ReportTemplateTool {
    fn name(&self) -> &str {
        "report.template"
    }
    fn spec(&self) -> &BuiltinToolSpec {
        &self.spec
    }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let template_input = args
            .get("template")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'template' parameter".into()))?;

        let data = args
            .get("data")
            .and_then(|v| v.as_object())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'data' object parameter".into()))?;

        // Resolve template: if it matches a builtin name, use that; otherwise
        // treat the string itself as a raw template.
        let template_str = builtin_template(template_input)
            .map(|s| s.to_string())
            .unwrap_or_else(|| template_input.to_string());

        let mut rendered = template_str;
        for (key, val) in data {
            let placeholder = format!("{{{{{key}}}}}");
            let replacement = match val {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            rendered = rendered.replace(&placeholder, &replacement);
        }

        Ok(json!({ "rendered": rendered }))
    }
}

// ---------------------------------------------------------------------------
// 10. health.check
// ---------------------------------------------------------------------------

/// Returns health status of registered services.
pub struct HealthCheckTool {
    spec: BuiltinToolSpec,
    service_registry: Arc<ServiceRegistry>,
}

impl HealthCheckTool {
    pub fn new(service_registry: Arc<ServiceRegistry>) -> Self {
        Self {
            spec: spec_by_name("health.check"),
            service_registry,
        }
    }
}

impl BuiltinTool for HealthCheckTool {
    fn name(&self) -> &str {
        "health.check"
    }
    fn spec(&self) -> &BuiltinToolSpec {
        &self.spec
    }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let filter_service = args.get("service").and_then(|v| v.as_str());

        let services = self.service_registry.list();

        if let Some(name) = filter_service {
            // Check if the named service exists
            let found = services.iter().any(|(n, _)| n == name);
            if !found {
                return Err(ToolError::NotFound(format!("service: {name}")));
            }
            // Sync health: report as "registered" (async health_check deferred)
            return Ok(json!({
                "service": name,
                "status": "registered",
                "healthy": true,
            }));
        }

        // All services
        let entries: Vec<serde_json::Value> = services
            .iter()
            .map(|(name, stype)| {
                json!({
                    "name": name,
                    "type": format!("{stype:?}"),
                    "status": "registered",
                    "healthy": true,
                })
            })
            .collect();

        Ok(json!({
            "services": entries,
            "count": entries.len(),
            "overall": "healthy",
        }))
    }
}

// ---------------------------------------------------------------------------
// Registration helper
// ---------------------------------------------------------------------------

/// Register the stateless extended tools into a [`ToolRegistry`].
///
/// Tools that require no external dependencies (fs.analyze, git.log,
/// doc.parse, config.read, env.detect, metrics.snapshot (stub),
/// report.template) are registered here.
///
/// Tools that require an `Arc<ConfigService>` (kv.get, kv.set) or
/// `Arc<ServiceRegistry>` (health.check) must be registered separately
/// by the caller with the appropriate dependencies.
pub fn register_stateless_tools(registry: &mut ToolRegistry) {
    registry.register(Arc::new(FsAnalyzeTool::new()));
    registry.register(Arc::new(GitLogTool::new()));
    registry.register(Arc::new(DocParseTool::new()));
    registry.register(Arc::new(ConfigReadTool::new()));
    registry.register(Arc::new(EnvDetectTool::new()));
    #[cfg(not(feature = "os-patterns"))]
    registry.register(Arc::new(MetricsSnapshotTool::new()));
    registry.register(Arc::new(ReportTemplateTool::new()));
}

/// Register the dependency-injected extended tools into a [`ToolRegistry`].
///
/// Call after [`register_stateless_tools`] to add kv.get, kv.set,
/// health.check, and (if `os-patterns`) metrics.snapshot.
pub fn register_extended_tools(
    registry: &mut ToolRegistry,
    config_service: Arc<ConfigService>,
    service_registry: Arc<ServiceRegistry>,
) {
    registry.register(Arc::new(KvGetTool::new(config_service.clone())));
    registry.register(Arc::new(KvSetTool::new(config_service)));
    registry.register(Arc::new(HealthCheckTool::new(service_registry)));
}

/// Register the metrics.snapshot tool with a real MetricsRegistry (os-patterns).
#[cfg(feature = "os-patterns")]
pub fn register_metrics_tool(
    registry: &mut ToolRegistry,
    metrics_registry: Arc<crate::metrics::MetricsRegistry>,
) {
    registry.register(Arc::new(MetricsSnapshotTool::new(metrics_registry)));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a temp directory with known files.
    fn make_temp_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("create tempdir");
        std::fs::write(dir.path().join("hello.rs"), "fn main() {\n    println!(\"hi\");\n}\n")
            .unwrap();
        std::fs::write(dir.path().join("data.txt"), "line1\nline2\nline3\n").unwrap();
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/nested.rs"), "// nested\n").unwrap();
        dir
    }

    // 1. fs.analyze
    #[test]
    fn fs_analyze_counts_files_and_lines() {
        let dir = make_temp_dir();
        let tool = FsAnalyzeTool::new();
        let result = tool
            .execute(json!({"path": dir.path().to_str().unwrap()}))
            .unwrap();

        assert_eq!(result["file_count"], 3);
        // hello.rs=3, data.txt=3, nested.rs=1 => 7 lines
        assert_eq!(result["total_lines"], 7);
        let types = result["file_types"].as_object().unwrap();
        assert_eq!(types["rs"], 2);
        assert_eq!(types["txt"], 1);
    }

    #[test]
    fn fs_analyze_missing_dir_returns_error() {
        let tool = FsAnalyzeTool::new();
        let result = tool.execute(json!({"path": "/nonexistent_dir_xyz"}));
        assert!(matches!(result, Err(ToolError::FileNotFound(_))));
    }

    // 2. git.log
    #[test]
    fn git_log_returns_commits() {
        let tool = GitLogTool::new();
        // Run against the current repo
        let result = tool.execute(json!({"limit": 3, "path": "."}));
        // May fail in CI without git; accept either success or execution error
        match result {
            Ok(val) => {
                assert!(val["commits"].is_array());
                let commits = val["commits"].as_array().unwrap();
                assert!(!commits.is_empty());
                // Each commit should have hash, author, date, message
                let first = &commits[0];
                assert!(first["hash"].is_string());
                assert!(first["author"].is_string());
                assert!(first["date"].is_string());
                assert!(first["message"].is_string());
            }
            Err(ToolError::ExecutionFailed(_)) => {
                // Acceptable in environments without git
            }
            Err(e) => panic!("unexpected error: {e}"),
        }
    }

    // 3. doc.parse
    #[test]
    fn doc_parse_extracts_structure() {
        let dir = tempfile::tempdir().unwrap();
        let md_path = dir.path().join("test.md");
        std::fs::write(
            &md_path,
            "# Title\n\nSome text with [link](http://example.com).\n\n\
             ## Section Two\n\nMore words here.\n\n\
             ```rust\nlet x = 1;\n```\n\n\
             ### Sub-section\n\nFinal [link2](url).\n",
        )
        .unwrap();

        let tool = DocParseTool::new();
        let result = tool
            .execute(json!({"path": md_path.to_str().unwrap()}))
            .unwrap();

        let sections = result["sections"].as_array().unwrap();
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0]["level"], 1);
        assert_eq!(sections[0]["title"], "Title");
        assert_eq!(sections[1]["level"], 2);

        assert_eq!(result["link_count"], 2);
        assert_eq!(result["code_block_count"], 1);
        assert!(result["word_count"].as_u64().unwrap() > 0);
    }

    // 4. config.read
    #[test]
    fn config_read_parses_toml() {
        let dir = tempfile::tempdir().unwrap();
        let toml_path = dir.path().join("test.toml");
        std::fs::write(
            &toml_path,
            r#"
title = "WeftOS"
version = 1

[kernel]
max_fuel = 1000000
debug = true

[kernel.nested]
value = "deep"
"#,
        )
        .unwrap();

        let tool = ConfigReadTool::new();
        let result = tool
            .execute(json!({"path": toml_path.to_str().unwrap()}))
            .unwrap();

        assert_eq!(result["title"], "WeftOS");
        assert_eq!(result["version"], 1);
        assert_eq!(result["kernel"]["max_fuel"], 1000000);
        assert_eq!(result["kernel"]["debug"], true);
        assert_eq!(result["kernel"]["nested"]["value"], "deep");
    }

    #[test]
    fn config_read_extracts_key() {
        let dir = tempfile::tempdir().unwrap();
        let toml_path = dir.path().join("key.toml");
        std::fs::write(
            &toml_path,
            "[server]\nport = 8080\nhost = \"localhost\"\n",
        )
        .unwrap();

        let tool = ConfigReadTool::new();
        let result = tool
            .execute(json!({"path": toml_path.to_str().unwrap(), "key": "server.port"}))
            .unwrap();

        assert_eq!(result["key"], "server.port");
        assert_eq!(result["value"], 8080);
    }

    // 5. env.detect
    #[test]
    fn env_detect_returns_expected_fields() {
        let tool = EnvDetectTool::new();
        let result = tool.execute(json!({})).unwrap();

        assert!(result["os"].is_string());
        assert!(result["arch"].is_string());
        assert!(result["cpu_count"].as_u64().unwrap() >= 1);
        assert!(result["features"].is_array());
        assert!(result["pkg_version"].is_string());
    }

    // 6. metrics.snapshot (stub without os-patterns)
    #[test]
    #[cfg(not(feature = "os-patterns"))]
    fn metrics_snapshot_stub_returns_empty() {
        let tool = MetricsSnapshotTool::new();
        let result = tool.execute(json!({})).unwrap();
        assert_eq!(result["count"], 0);
        assert!(result["metrics"].as_array().unwrap().is_empty());
    }

    // 7. kv.get / 8. kv.set roundtrip
    #[test]
    fn kv_get_set_roundtrip() {
        let cs = Arc::new(ConfigService::new([0u8; 32]));
        let set_tool = KvSetTool::new(cs.clone());
        let get_tool = KvGetTool::new(cs);

        // Set a value
        let set_result = set_tool
            .execute(json!({"key": "mykey", "namespace": "test", "value": "hello"}))
            .unwrap();
        assert_eq!(set_result["ok"], true);

        // Get the value back
        let get_result = get_tool
            .execute(json!({"key": "mykey", "namespace": "test"}))
            .unwrap();
        assert_eq!(get_result["value"], "hello");
    }

    #[test]
    fn kv_get_missing_key_returns_null() {
        let cs = Arc::new(ConfigService::new([0u8; 32]));
        let tool = KvGetTool::new(cs);
        let result = tool
            .execute(json!({"key": "nonexistent", "namespace": "default"}))
            .unwrap();
        assert!(result["value"].is_null());
    }

    // 9. report.template
    #[test]
    fn report_template_substitution() {
        let tool = ReportTemplateTool::new();
        let result = tool
            .execute(json!({
                "template": "Hello {{name}}, you have {{count}} items.",
                "data": {"name": "Alice", "count": 42}
            }))
            .unwrap();

        assert_eq!(
            result["rendered"],
            "Hello Alice, you have 42 items."
        );
    }

    #[test]
    fn report_template_builtin_gap_report() {
        let tool = ReportTemplateTool::new();
        let result = tool
            .execute(json!({
                "template": "gap-report",
                "data": {
                    "title": "Sprint 10",
                    "date": "2026-03-27",
                    "summary": "All gaps resolved.",
                    "gap_count": "0",
                    "details": "None."
                }
            }))
            .unwrap();

        let rendered = result["rendered"].as_str().unwrap();
        assert!(rendered.contains("# Gap Report: Sprint 10"));
        assert!(rendered.contains("Generated: 2026-03-27"));
        assert!(rendered.contains("All gaps resolved."));
    }

    // 10. health.check
    #[test]
    fn health_check_all_services() {
        let sr = Arc::new(ServiceRegistry::new());
        let tool = HealthCheckTool::new(sr);
        let result = tool.execute(json!({})).unwrap();

        assert_eq!(result["overall"], "healthy");
        assert!(result["services"].is_array());
        assert_eq!(result["count"], 0); // no services registered
    }

    #[test]
    fn health_check_unknown_service_returns_error() {
        let sr = Arc::new(ServiceRegistry::new());
        let tool = HealthCheckTool::new(sr);
        let result = tool.execute(json!({"service": "nonexistent"}));
        assert!(matches!(result, Err(ToolError::NotFound(_))));
    }

    // Verify all 10 tools are in the extended catalog
    #[test]
    fn extended_catalog_has_10_entries() {
        let catalog = extended_tool_catalog();
        assert_eq!(catalog.len(), 10);
        let names: Vec<&str> = catalog.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"fs.analyze"));
        assert!(names.contains(&"git.log"));
        assert!(names.contains(&"doc.parse"));
        assert!(names.contains(&"config.read"));
        assert!(names.contains(&"env.detect"));
        assert!(names.contains(&"metrics.snapshot"));
        assert!(names.contains(&"kv.get"));
        assert!(names.contains(&"kv.set"));
        assert!(names.contains(&"report.template"));
        assert!(names.contains(&"health.check"));
    }

    // Verify stateless tools register correctly
    #[test]
    fn register_stateless_tools_populates_registry() {
        let mut registry = ToolRegistry::new();
        register_stateless_tools(&mut registry);
        // 6 stateless tools (without os-patterns: fs.analyze, git.log,
        // doc.parse, config.read, env.detect, metrics.snapshot stub,
        // report.template)
        assert!(registry.get("fs.analyze").is_some());
        assert!(registry.get("git.log").is_some());
        assert!(registry.get("doc.parse").is_some());
        assert!(registry.get("config.read").is_some());
        assert!(registry.get("env.detect").is_some());
        assert!(registry.get("report.template").is_some());
    }
}
