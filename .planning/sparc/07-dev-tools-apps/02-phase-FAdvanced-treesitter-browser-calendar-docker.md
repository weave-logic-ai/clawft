# Phase F-Advanced: Tree-sitter, Browser CDP, Calendar, Docker/Podman

| Field | Value |
|-------|-------|
| **Element** | 07 -- Dev Tools & Applications |
| **Phase** | F-Advanced (F3, F4, F5, F7) |
| **Timeline** | Week 6-9 |
| **Priority** | P2 |
| **Crates** | `clawft-plugin-treesitter`, `clawft-plugin-browser`, `clawft-plugin-calendar`, `clawft-plugin-containers` |
| **Dependencies IN** | 04/C1 (plugin traits -- `Tool` trait), 07/F6 (OAuth2 helper -- F5 blocked until available) |
| **Blocks** | None directly (all are leaf tools) |
| **Status** | Planning |

---

## 1. Overview

Phase F-Advanced implements four advanced developer tool plugins as separate workspace crates. These are specialized integrations that extend the platform's capabilities beyond core dev tooling (F1/F2):

1. **F3 -- Code Analysis via Tree-sitter**: Native AST parsing, structural queries, and cyclomatic complexity calculation for multiple programming languages. Uses C FFI via tree-sitter grammars, native-only (no WASM variant).
2. **F4 -- Browser CDP Automation** (SECURITY-CRITICAL): Headless Chrome automation for screenshots, form filling, scraping, and JavaScript evaluation. Requires comprehensive sandboxing via `BrowserSandboxConfig` to prevent security exposure.
3. **F5 -- Calendar Integration**: Google Calendar, Microsoft Outlook, and local iCal support. Depends on F6 (OAuth2 helper) for provider authentication.
4. **F7 -- Docker/Podman Orchestration**: Container lifecycle management via subprocess execution. Supports both `docker` and `podman` runtimes with validated command construction.

All four tools implement the `Tool` trait from `clawft-plugin` (C1), are feature-gated behind individual Cargo feature flags, and declare permissions via the plugin manifest for K3 sandbox enforcement.

---

## 2. Crate Structure

Each tool is a separate workspace crate, one per tool, to keep compilation parallel and avoid a monolithic plugin crate.

| Crate | Tool | Feature Flag | Key Dependency | Transport |
|-------|------|-------------|----------------|-----------|
| `crates/clawft-plugin-treesitter` | F3: Code analysis | `plugin-treesitter` | `tree-sitter` + grammar crates | Native FFI |
| `crates/clawft-plugin-browser` | F4: Browser CDP | `plugin-browser` | `chromiumoxide` | CDP WebSocket |
| `crates/clawft-plugin-calendar` | F5: Calendar | `plugin-calendar` | `reqwest`, `icalendar` | REST + OAuth2 |
| `crates/clawft-plugin-containers` | F7: Docker/Podman | `plugin-containers` | `tokio::process` | Subprocess |

### 2.1 Directory Layout (Per Crate)

```text
crates/clawft-plugin-<name>/
  Cargo.toml
  src/
    lib.rs          -- Tool trait implementation, tool registration
    operations.rs   -- Individual tool operation implementations
    types.rs        -- Config, request/response types, serde models
    validation.rs   -- Input sanitization and security checks (F4, F7)
    tests.rs        -- Unit tests
```

---

## 3. F3: Code Analysis via Tree-sitter

### 3.1 Crate: `crates/clawft-plugin-treesitter/`

**Feature Flag**: `plugin-treesitter`
**Native-only, NO WASM variant** -- tree-sitter uses C FFI bindings that are incompatible with the WASM plugin host.

### 3.2 Cargo.toml

```toml
[package]
name = "clawft-plugin-treesitter"
version = "0.1.0"
edition = "2021"

[dependencies]
clawft-plugin = { workspace = true }
clawft-types = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tree-sitter = "0.24"
thiserror = { workspace = true }
tracing = { workspace = true }

# Language grammars -- each is an optional dependency behind a sub-feature
tree-sitter-rust = { version = "0.23", optional = true }
tree-sitter-typescript = { version = "0.23", optional = true }
tree-sitter-python = { version = "0.23", optional = true }
tree-sitter-javascript = { version = "0.23", optional = true }
tree-sitter-go = { version = "0.23", optional = true }
tree-sitter-json = { version = "0.23", optional = true }
tree-sitter-toml = { version = "0.23", optional = true }

[features]
default = []
rust = ["dep:tree-sitter-rust"]
typescript = ["dep:tree-sitter-typescript"]
python = ["dep:tree-sitter-python"]
javascript = ["dep:tree-sitter-javascript"]
go = ["dep:tree-sitter-go"]
json = ["dep:tree-sitter-json"]
toml = ["dep:tree-sitter-toml"]
all-languages = ["rust", "typescript", "python", "javascript", "go", "json", "toml"]
```

### 3.3 Operations

| Tool Name | Description | Input Parameters | Output |
|-----------|-------------|------------------|--------|
| `ts_parse` | Parse file and return AST | `file_path: String`, `language: String` | JSON AST tree |
| `ts_query` | Run tree-sitter query on parsed AST | `file_path: String`, `language: String`, `query: String` | Matched nodes with ranges |
| `ts_complexity` | Calculate cyclomatic complexity | `file_path: String`, `language: String` | Per-function complexity scores |
| `ts_symbols` | List functions, structs, classes | `file_path: String`, `language: String`, `kind: Option<String>` | Symbol list with name, kind, range |
| `ts_references` | Find references to a symbol | `file_path: String`, `language: String`, `symbol: String` | Reference locations |

### 3.4 Implementation Tasks

#### Task F3.1: Language registry and grammar loading

Create `src/lib.rs` with a `LanguageRegistry` that maps language names to tree-sitter `Language` objects:

```rust
use std::collections::HashMap;
use tree_sitter::Language;

pub struct LanguageRegistry {
    languages: HashMap<String, Language>,
}

impl LanguageRegistry {
    pub fn new() -> Self {
        let mut languages = HashMap::new();

        #[cfg(feature = "rust")]
        languages.insert("rust".into(), tree_sitter_rust::LANGUAGE.into());

        #[cfg(feature = "typescript")]
        {
            languages.insert("typescript".into(), tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into());
            languages.insert("tsx".into(), tree_sitter_typescript::LANGUAGE_TSX.into());
        }

        #[cfg(feature = "python")]
        languages.insert("python".into(), tree_sitter_python::LANGUAGE.into());

        #[cfg(feature = "javascript")]
        languages.insert("javascript".into(), tree_sitter_javascript::LANGUAGE.into());

        #[cfg(feature = "go")]
        languages.insert("go".into(), tree_sitter_go::LANGUAGE.into());

        #[cfg(feature = "json")]
        languages.insert("json".into(), tree_sitter_json::LANGUAGE.into());

        #[cfg(feature = "toml")]
        languages.insert("toml".into(), tree_sitter_toml::LANGUAGE.into());

        Self { languages }
    }

    pub fn get(&self, name: &str) -> Option<&Language> {
        self.languages.get(name)
    }

    pub fn supported_languages(&self) -> Vec<String> {
        self.languages.keys().cloned().collect()
    }
}
```

Also implement file extension to language name mapping:

```rust
pub fn detect_language(file_path: &str) -> Option<&'static str> {
    match file_path.rsplit('.').next()? {
        "rs" => Some("rust"),
        "ts" => Some("typescript"),
        "tsx" => Some("tsx"),
        "py" => Some("python"),
        "js" | "mjs" | "cjs" => Some("javascript"),
        "go" => Some("go"),
        "json" => Some("json"),
        "toml" => Some("toml"),
        _ => None,
    }
}
```

#### Task F3.2: `ts_parse` -- Parse file and return AST

```rust
pub fn parse_file(
    registry: &LanguageRegistry,
    file_path: &str,
    language: &str,
) -> Result<serde_json::Value, TreeSitterError> {
    let lang = registry.get(language)
        .ok_or_else(|| TreeSitterError::UnsupportedLanguage(language.into()))?;

    let source = std::fs::read_to_string(file_path)
        .map_err(|e| TreeSitterError::FileRead(file_path.into(), e))?;

    // Reject files larger than 4 MB to prevent excessive parsing time
    if source.len() > 4 * 1024 * 1024 {
        return Err(TreeSitterError::FileTooLarge(file_path.into()));
    }

    let mut parser = tree_sitter::Parser::new();
    parser.set_language(lang)?;

    let tree = parser.parse(&source, None)
        .ok_or(TreeSitterError::ParseFailed)?;

    Ok(ast_to_json(tree.root_node(), &source))
}

fn ast_to_json(node: tree_sitter::Node, source: &str) -> serde_json::Value {
    let mut obj = serde_json::json!({
        "type": node.kind(),
        "start": { "row": node.start_position().row, "col": node.start_position().column },
        "end": { "row": node.end_position().row, "col": node.end_position().column },
    });

    // Include text for leaf nodes (identifiers, literals, operators)
    if node.child_count() == 0 {
        if let Ok(text) = node.utf8_text(source.as_bytes()) {
            obj["text"] = serde_json::Value::String(text.to_string());
        }
    }

    if node.child_count() > 0 {
        let children: Vec<serde_json::Value> = (0..node.child_count())
            .filter_map(|i| node.child(i))
            .filter(|c| c.is_named()) // Skip anonymous nodes (punctuation)
            .map(|c| ast_to_json(c, source))
            .collect();
        if !children.is_empty() {
            obj["children"] = serde_json::Value::Array(children);
        }
    }

    obj
}
```

#### Task F3.3: `ts_query` -- Run tree-sitter query

```rust
pub fn query_file(
    registry: &LanguageRegistry,
    file_path: &str,
    language: &str,
    query_str: &str,
) -> Result<Vec<QueryMatch>, TreeSitterError> {
    let lang = registry.get(language)
        .ok_or_else(|| TreeSitterError::UnsupportedLanguage(language.into()))?;

    let source = std::fs::read_to_string(file_path)
        .map_err(|e| TreeSitterError::FileRead(file_path.into(), e))?;

    let mut parser = tree_sitter::Parser::new();
    parser.set_language(lang)?;

    let tree = parser.parse(&source, None)
        .ok_or(TreeSitterError::ParseFailed)?;

    let query = tree_sitter::Query::new(lang, query_str)
        .map_err(|e| TreeSitterError::InvalidQuery(e.to_string()))?;

    let mut cursor = tree_sitter::QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    let mut results = Vec::new();
    for m in matches {
        for capture in m.captures {
            let node = capture.node;
            results.push(QueryMatch {
                capture_name: query.capture_names()[capture.index as usize].to_string(),
                text: node.utf8_text(source.as_bytes()).unwrap_or("").to_string(),
                start_row: node.start_position().row,
                start_col: node.start_position().column,
                end_row: node.end_position().row,
                end_col: node.end_position().column,
                node_kind: node.kind().to_string(),
            });
        }
    }

    Ok(results)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct QueryMatch {
    pub capture_name: String,
    pub text: String,
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
    pub node_kind: String,
}
```

#### Task F3.4: `ts_complexity` -- Cyclomatic complexity

Calculate cyclomatic complexity by counting decision points within each function body:

```rust
/// Decision-point node kinds per language.
/// Cyclomatic complexity = 1 + count of decision nodes in the function body.
fn decision_node_kinds(language: &str) -> &'static [&'static str] {
    match language {
        "rust" => &["if_expression", "match_expression", "while_expression",
                     "for_expression", "loop_expression", "binary_expression"],
        "typescript" | "javascript" => &["if_statement", "switch_case", "while_statement",
                     "for_statement", "for_in_statement", "catch_clause",
                     "ternary_expression", "binary_expression"],
        "python" => &["if_statement", "elif_clause", "while_statement",
                      "for_statement", "except_clause", "conditional_expression",
                      "boolean_operator"],
        _ => &[],
    }
}

/// Function definition node kinds per language.
fn function_node_kinds(language: &str) -> &'static [&'static str] {
    match language {
        "rust" => &["function_item", "closure_expression"],
        "typescript" | "javascript" => &["function_declaration", "arrow_function",
                     "method_definition", "function_expression"],
        "python" => &["function_definition"],
        _ => &[],
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ComplexityResult {
    pub function_name: String,
    pub start_row: usize,
    pub end_row: usize,
    pub complexity: u32,
}
```

Walk the AST: for each function node, count its descendant decision nodes. Complexity = 1 + count.

For `binary_expression` (Rust) or `boolean_operator` (Python), only count `&&` and `||` operators as decision points.

#### Task F3.5: `ts_symbols` -- List symbols

Query for function definitions, struct/class declarations, and other symbol types. Return a list:

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct Symbol {
    pub name: String,
    pub kind: String, // "function", "struct", "class", "method", "interface", etc.
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
}
```

Use pre-defined tree-sitter queries per language to find symbol definitions. Support filtering by `kind` parameter.

#### Task F3.6: `ts_references` -- Find references

Simple text-matching approach: parse the file, then search for all identifier nodes whose text matches the given symbol name. This is a syntactic search (not semantic), suitable for single-file analysis:

```rust
pub fn find_references(
    registry: &LanguageRegistry,
    file_path: &str,
    language: &str,
    symbol: &str,
) -> Result<Vec<ReferenceLocation>, TreeSitterError> { ... }

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReferenceLocation {
    pub text: String,
    pub row: usize,
    pub col: usize,
    pub context_kind: String, // Parent node kind for context
}
```

#### Task F3.7: `Tool` trait implementation

Register all five operations as tools with MCP-compatible JSON Schema `input_schema`:

```rust
impl Tool for TreeSitterPlugin {
    fn name(&self) -> &str { "treesitter" }

    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "ts_parse".into(),
                description: "Parse a source file and return its AST".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "Path to the source file" },
                        "language": { "type": "string", "description": "Language name (rust, typescript, python, etc.)" }
                    },
                    "required": ["file_path"]
                }),
            },
            // ... similar for ts_query, ts_complexity, ts_symbols, ts_references
        ]
    }

    async fn execute(&self, tool_name: &str, params: Value) -> Result<Value, ToolError> {
        match tool_name {
            "ts_parse" => { /* dispatch to parse_file */ },
            "ts_query" => { /* dispatch to query_file */ },
            "ts_complexity" => { /* dispatch to calculate_complexity */ },
            "ts_symbols" => { /* dispatch to list_symbols */ },
            "ts_references" => { /* dispatch to find_references */ },
            _ => Err(ToolError::UnknownTool(tool_name.into())),
        }
    }
}
```

### 3.5 Permission Manifest (F3)

```json
{
  "permissions": {
    "filesystem": { "read": true, "write": false, "paths": ["$WORKSPACE"] },
    "network": { "allowed": [] },
    "shell": false
  }
}
```

Tree-sitter is a read-only, local-only tool. No network access, no shell execution, no file writes.

---

## 4. F4: Browser CDP Automation (SECURITY-CRITICAL)

### 4.1 Crate: `crates/clawft-plugin-browser/`

**Feature Flag**: `plugin-browser`

### 4.2 Cargo.toml

```toml
[package]
name = "clawft-plugin-browser"
version = "0.1.0"
edition = "2021"

[dependencies]
clawft-plugin = { workspace = true }
clawft-types = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chromiumoxide = { version = "0.7", features = ["tokio-runtime"] }
tokio = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
url = "2"
base64 = "0.22"
```

### 4.3 Operations

| Tool Name | Description | Input Parameters | Output |
|-----------|-------------|------------------|--------|
| `browser_navigate` | Navigate to URL | `url: String` | Page title, final URL |
| `browser_screenshot` | Capture page screenshot | `selector: Option<String>`, `full_page: bool` | Base64-encoded PNG |
| `browser_fill` | Fill a form field | `selector: String`, `value: String` | Success/failure |
| `browser_click` | Click an element | `selector: String` | Success/failure |
| `browser_get_text` | Extract text from element | `selector: String` | Text content |
| `browser_evaluate` | Run JavaScript expression | `expression: String` | Evaluation result as JSON |

### 4.4 BrowserSandboxConfig (CRITICAL security boundary)

This config is part of the C1 plugin manifest extension for the browser tool. The K3 sandbox enforces it at runtime.

```rust
use std::time::Duration;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct BrowserSandboxConfig {
    /// Domains the browser is allowed to navigate to. Empty = block all.
    #[serde(default)]
    pub allowed_domains: Vec<String>,

    /// Maximum number of concurrent browser pages per agent.
    #[serde(default = "default_max_concurrent_pages")]
    pub max_concurrent_pages: u32,

    /// Maximum browser session lifetime before forced termination.
    #[serde(default = "default_session_lifetime_secs")]
    pub session_lifetime_secs: u64,

    /// Maximum memory for the browser process in megabytes.
    #[serde(default = "default_max_memory_mb")]
    pub max_memory_mb: u64,

    /// Cookie/storage policy: clear between sessions.
    #[serde(default = "default_clear_state")]
    pub clear_state_between_sessions: bool,
}

fn default_max_concurrent_pages() -> u32 { 2 }
fn default_session_lifetime_secs() -> u64 { 300 }
fn default_max_memory_mb() -> u64 { 512 }
fn default_clear_state() -> bool { true }

impl BrowserSandboxConfig {
    pub fn session_lifetime(&self) -> Duration {
        Duration::from_secs(self.session_lifetime_secs)
    }
}

impl Default for BrowserSandboxConfig {
    fn default() -> Self {
        Self {
            allowed_domains: Vec::new(),
            max_concurrent_pages: 2,
            session_lifetime_secs: 300,
            max_memory_mb: 512,
            clear_state_between_sessions: true,
        }
    }
}
```

### 4.5 Implementation Tasks

#### Task F4.1: URL validation and domain enforcement

Create `src/validation.rs`:

```rust
use url::Url;

/// URL schemes that are always blocked.
const BLOCKED_SCHEMES: &[&str] = &["file", "data", "javascript", "ftp", "gopher", "blob"];

#[derive(Debug, thiserror::Error)]
pub enum BrowserValidationError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("blocked URL scheme: {0}")]
    BlockedScheme(String),
    #[error("domain not in allowed list: {0}")]
    DomainNotAllowed(String),
    #[error("no allowed domains configured -- all navigation blocked")]
    NoDomainsConfigured,
    #[error("concurrent page limit reached: {current}/{max}")]
    PageLimitReached { current: u32, max: u32 },
}

pub fn validate_navigation_url(
    url_str: &str,
    config: &BrowserSandboxConfig,
) -> Result<Url, BrowserValidationError> {
    let url = Url::parse(url_str)
        .map_err(|e| BrowserValidationError::InvalidUrl(e.to_string()))?;

    // Block dangerous URL schemes
    let scheme = url.scheme().to_lowercase();
    if BLOCKED_SCHEMES.contains(&scheme.as_str()) {
        return Err(BrowserValidationError::BlockedScheme(scheme));
    }

    // Only allow http and https
    if scheme != "http" && scheme != "https" {
        return Err(BrowserValidationError::BlockedScheme(scheme));
    }

    // Enforce allowed_domains
    if config.allowed_domains.is_empty() {
        return Err(BrowserValidationError::NoDomainsConfigured);
    }

    let host = url.host_str()
        .ok_or_else(|| BrowserValidationError::InvalidUrl("no host in URL".into()))?;

    let host_lower = host.to_lowercase();
    let domain_allowed = config.allowed_domains.iter().any(|d| {
        let d_lower = d.to_lowercase();
        if let Some(suffix) = d_lower.strip_prefix("*.") {
            // Wildcard: *.example.com matches sub.example.com but NOT example.com
            host_lower.ends_with(&format!(".{}", suffix))
        } else {
            host_lower == d_lower
        }
    });

    if !domain_allowed {
        return Err(BrowserValidationError::DomainNotAllowed(host.to_string()));
    }

    Ok(url)
}
```

#### Task F4.2: Browser session manager

Create `src/operations.rs` with session lifecycle management:

```rust
use chromiumoxide::Browser;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::Mutex;

pub struct BrowserSessionManager {
    config: BrowserSandboxConfig,
    browser: Mutex<Option<Browser>>,
    active_pages: AtomicU32,
    session_start: Mutex<Option<tokio::time::Instant>>,
}

impl BrowserSessionManager {
    pub fn new(config: BrowserSandboxConfig) -> Self {
        Self {
            config,
            browser: Mutex::new(None),
            active_pages: AtomicU32::new(0),
            session_start: Mutex::new(None),
        }
    }

    /// Lazily launch browser with sandbox args.
    /// Returns error if session_lifetime has expired.
    async fn ensure_browser(&self) -> Result<(), BrowserError> {
        let mut browser = self.browser.lock().await;
        if browser.is_some() {
            // Check session lifetime
            let start = self.session_start.lock().await;
            if let Some(start_time) = *start {
                if start_time.elapsed() >= self.config.session_lifetime() {
                    // Force-kill and restart
                    *browser = None;
                    drop(start);
                    return Err(BrowserError::SessionExpired);
                }
            }
            return Ok(());
        }

        // Launch headless Chrome with restrictive flags
        let launch_options = chromiumoxide::BrowserConfig::builder()
            .no_sandbox()          // Running inside container sandbox already
            .disable_gpu()
            .arg("--disable-dev-shm-usage")
            .arg("--disable-extensions")
            .arg("--disable-plugins")
            .arg("--disable-popup-blocking")
            .arg("--disable-translate")
            .arg("--no-first-run")
            .arg(format!("--js-flags=--max-old-space-size={}", self.config.max_memory_mb))
            .build()
            .map_err(|e| BrowserError::LaunchFailed(e.to_string()))?;

        let (new_browser, _handler) = Browser::launch(launch_options)
            .await
            .map_err(|e| BrowserError::LaunchFailed(e.to_string()))?;

        *browser = Some(new_browser);
        *self.session_start.lock().await = Some(tokio::time::Instant::now());

        Ok(())
    }

    /// Clear all browser state (cookies, storage, sessions).
    /// Called between logical sessions when clear_state_between_sessions is true.
    pub async fn clear_state(&self) -> Result<(), BrowserError> {
        if !self.config.clear_state_between_sessions {
            return Ok(());
        }
        let browser = self.browser.lock().await;
        if let Some(ref b) = *browser {
            // Clear via CDP commands:
            // - Network.clearBrowserCookies
            // - Storage.clearDataForOrigin
            // - CacheStorage.deleteCache
            // Implementation uses chromiumoxide's execute_cdp methods
            tracing::info!("clearing browser state between sessions");
        }
        Ok(())
    }

    /// Force-kill the browser process.
    /// Called when session_lifetime expires or on shutdown.
    pub async fn force_kill(&self) {
        let mut browser = self.browser.lock().await;
        if browser.is_some() {
            tracing::warn!("force-killing browser process");
            *browser = None; // Drop triggers process kill
        }
        self.active_pages.store(0, Ordering::Relaxed);
        *self.session_start.lock().await = None;
    }

    /// Check and enforce page limit before opening a new page.
    fn check_page_limit(&self) -> Result<(), BrowserValidationError> {
        let current = self.active_pages.load(Ordering::Relaxed);
        if current >= self.config.max_concurrent_pages {
            return Err(BrowserValidationError::PageLimitReached {
                current,
                max: self.config.max_concurrent_pages,
            });
        }
        Ok(())
    }
}
```

#### Task F4.3: Implement `browser_navigate`

```rust
pub async fn navigate(
    manager: &BrowserSessionManager,
    url_str: &str,
) -> Result<NavigateResult, BrowserError> {
    // 1. Validate URL against sandbox config
    let url = validate_navigation_url(url_str, &manager.config)?;

    // 2. Ensure browser is running and within session lifetime
    manager.ensure_browser().await?;

    // 3. Check page limit
    manager.check_page_limit()?;

    // 4. Open page and navigate
    let browser = manager.browser.lock().await;
    let b = browser.as_ref().ok_or(BrowserError::NotRunning)?;

    let page = b.new_page(&url).await
        .map_err(|e| BrowserError::NavigationFailed(e.to_string()))?;

    manager.active_pages.fetch_add(1, Ordering::Relaxed);

    // 5. Wait for page load
    page.wait_for_navigation().await
        .map_err(|e| BrowserError::NavigationFailed(e.to_string()))?;

    let title = page.get_title().await
        .map_err(|e| BrowserError::OperationFailed(e.to_string()))?;

    let final_url = page.url().await
        .map_err(|e| BrowserError::OperationFailed(e.to_string()))?;

    // 6. Validate final URL (after redirects) against allowed_domains
    if let Some(ref final_url_str) = final_url {
        validate_navigation_url(final_url_str, &manager.config)?;
    }

    Ok(NavigateResult {
        title: title.unwrap_or_default(),
        url: final_url.unwrap_or_default(),
    })
}

#[derive(Debug, serde::Serialize)]
pub struct NavigateResult {
    pub title: String,
    pub url: String,
}
```

#### Task F4.4: Implement `browser_screenshot`

```rust
pub async fn screenshot(
    manager: &BrowserSessionManager,
    selector: Option<&str>,
    full_page: bool,
) -> Result<String, BrowserError> {
    // Returns base64-encoded PNG
    // If selector is Some, screenshot only that element
    // If full_page is true, capture entire scrollable page
    // Limit screenshot size to 10 MB to prevent memory exhaustion
    ...
}
```

#### Task F4.5: Implement `browser_fill`, `browser_click`, `browser_get_text`

Standard DOM interaction operations using CSS selectors. Each validates the selector string (no script injection through selector values).

#### Task F4.6: Implement `browser_evaluate` (sandboxed)

```rust
pub async fn evaluate(
    manager: &BrowserSessionManager,
    expression: &str,
) -> Result<serde_json::Value, BrowserError> {
    // Expression length limit: 10 KB
    if expression.len() > 10 * 1024 {
        return Err(BrowserError::ExpressionTooLarge);
    }

    // Execute with timeout (5 seconds)
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        page.evaluate(expression),
    ).await
    .map_err(|_| BrowserError::EvaluationTimeout)?
    .map_err(|e| BrowserError::EvaluationFailed(e.to_string()))?;

    Ok(result.into_value()?)
}
```

#### Task F4.7: Session lifetime watchdog

Spawn a background task that monitors session age and force-kills the browser when `session_lifetime` expires:

```rust
pub fn spawn_session_watchdog(
    manager: Arc<BrowserSessionManager>,
    cancel: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let check_interval = Duration::from_secs(10);
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = tokio::time::sleep(check_interval) => {
                    let start = manager.session_start.lock().await;
                    if let Some(start_time) = *start {
                        if start_time.elapsed() >= manager.config.session_lifetime() {
                            drop(start);
                            tracing::warn!("browser session lifetime expired, force-killing");
                            manager.force_kill().await;
                        }
                    }
                }
            }
        }
    })
}
```

#### Task F4.8: `Tool` trait implementation

Register all six operations with MCP-compatible JSON Schema. Each operation validates inputs through the sandbox config before execution.

### 4.6 Security Requirements (F4) -- CRITICAL

| Requirement | Implementation | Enforcement Point |
|-------------|----------------|-------------------|
| Block `file://` URL scheme | `validate_navigation_url()` rejects `file` scheme | Before every navigation |
| Block `data://` URL scheme | `validate_navigation_url()` rejects `data` scheme | Before every navigation |
| Block `javascript://` URL scheme | `validate_navigation_url()` rejects `javascript` scheme | Before every navigation |
| Enforce `allowed_domains` | `validate_navigation_url()` checks host against config | Before every navigation AND after redirects |
| Clear state between sessions | `clear_state()` clears cookies, storage, cache | Between logical sessions |
| Max concurrent pages | `check_page_limit()` tracks active pages | Before opening new page |
| Session lifetime | `spawn_session_watchdog()` force-kills browser | Background watchdog task |
| Memory limit | `--js-flags=--max-old-space-size` Chrome flag | Browser launch |
| Post-redirect validation | Final URL checked after navigation completes | After `wait_for_navigation()` |
| JS evaluation limits | Expression length limit (10 KB), execution timeout (5s) | Before `evaluate()` |

### 4.7 Permission Manifest (F4)

```json
{
  "permissions": {
    "filesystem": { "read": false, "write": false, "paths": [] },
    "network": { "allowed": ["$ALLOWED_DOMAINS"] },
    "shell": true
  },
  "browser_sandbox": {
    "allowed_domains": [],
    "max_concurrent_pages": 2,
    "session_lifetime_secs": 300,
    "max_memory_mb": 512,
    "clear_state_between_sessions": true
  }
}
```

### 4.8 Resource Limits (F4)

| Resource | Default | Rationale |
|----------|---------|-----------|
| Browser sessions per agent | 1 | Prevent resource exhaustion |
| Concurrent pages per session | 2 | Limit memory usage |
| Session lifetime | 300s (5 min) | Prevent abandoned sessions |
| Browser memory | 512 MB | Chrome V8 heap limit |
| JS evaluation timeout | 5s | Prevent infinite loops |
| JS expression length | 10 KB | Prevent code injection |
| Screenshot max size | 10 MB | Prevent memory exhaustion |

---

## 5. F5: Calendar Integration

### 5.1 Crate: `crates/clawft-plugin-calendar/`

**Feature Flag**: `plugin-calendar`
**Blocked on**: F6 (OAuth2 helper) for Google Calendar and Outlook providers

### 5.2 Cargo.toml

```toml
[package]
name = "clawft-plugin-calendar"
version = "0.1.0"
edition = "2021"

[dependencies]
clawft-plugin = { workspace = true }
clawft-types = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
reqwest = { workspace = true }
tokio = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
chrono = { workspace = true }
icalendar = { version = "0.16", optional = true }

[features]
default = []
google = []           # Google Calendar API v3 (requires clawft-plugin-oauth2 at runtime)
outlook = []          # Microsoft Graph API (requires clawft-plugin-oauth2 at runtime)
ical = ["dep:icalendar"]  # Local .ics file parsing
```

### 5.3 Operations

| Tool Name | Description | Input Parameters | Output |
|-----------|-------------|------------------|--------|
| `cal_list_events` | List upcoming events | `provider: String`, `start: DateTime`, `end: DateTime`, `calendar_id: Option<String>` | List of events |
| `cal_create_event` | Create a new event | `provider: String`, `summary: String`, `start: DateTime`, `end: DateTime`, `description: Option<String>`, `attendees: Option<Vec<String>>` | Created event with ID |
| `cal_update_event` | Modify an existing event | `provider: String`, `event_id: String`, fields to update | Updated event |
| `cal_delete_event` | Remove an event | `provider: String`, `event_id: String` | Success/failure |
| `cal_check_availability` | Check free/busy status | `provider: String`, `start: DateTime`, `end: DateTime`, `attendees: Vec<String>` | Busy intervals |

### 5.4 Implementation Tasks

#### Task F5.1: Provider trait

```rust
#[async_trait::async_trait]
pub trait CalendarProvider: Send + Sync {
    fn name(&self) -> &str;

    async fn list_events(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
        calendar_id: Option<&str>,
    ) -> Result<Vec<CalendarEvent>, CalendarError>;

    async fn create_event(
        &self,
        event: NewEvent,
    ) -> Result<CalendarEvent, CalendarError>;

    async fn update_event(
        &self,
        event_id: &str,
        updates: EventUpdate,
    ) -> Result<CalendarEvent, CalendarError>;

    async fn delete_event(
        &self,
        event_id: &str,
    ) -> Result<(), CalendarError>;

    async fn check_availability(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
        attendees: &[String],
    ) -> Result<Vec<BusyInterval>, CalendarError>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub summary: String,
    pub description: Option<String>,
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
    pub attendees: Vec<String>,
    pub location: Option<String>,
    pub status: String,
    pub html_link: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct NewEvent {
    pub summary: String,
    pub description: Option<String>,
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
    pub attendees: Option<Vec<String>>,
    pub location: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BusyInterval {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
}
```

#### Task F5.2: Google Calendar provider

```rust
/// Google Calendar API v3 provider.
/// Base URL: https://www.googleapis.com/calendar/v3
///
/// Requires F6 OAuth2 helper for authentication.
/// Scopes: https://www.googleapis.com/auth/calendar
pub struct GoogleCalendarProvider {
    http: reqwest::Client,
    base_url: String,
    // oauth2_token_provider: from F6
}

impl GoogleCalendarProvider {
    /// List events: GET /calendars/{calendarId}/events
    /// Parameters: timeMin, timeMax, singleEvents=true, orderBy=startTime
    async fn list_events(...) -> Result<Vec<CalendarEvent>, CalendarError> { ... }

    /// Create event: POST /calendars/{calendarId}/events
    async fn create_event(...) -> Result<CalendarEvent, CalendarError> { ... }

    /// Update event: PATCH /calendars/{calendarId}/events/{eventId}
    async fn update_event(...) -> Result<CalendarEvent, CalendarError> { ... }

    /// Delete event: DELETE /calendars/{calendarId}/events/{eventId}
    async fn delete_event(...) -> Result<(), CalendarError> { ... }

    /// Free/busy: POST /freeBusy
    async fn check_availability(...) -> Result<Vec<BusyInterval>, CalendarError> { ... }
}
```

#### Task F5.3: Microsoft Outlook provider

```rust
/// Microsoft Graph API provider for Outlook Calendar.
/// Base URL: https://graph.microsoft.com/v1.0
///
/// Requires F6 OAuth2 helper for authentication.
/// Scopes: Calendars.ReadWrite
pub struct OutlookCalendarProvider {
    http: reqwest::Client,
    base_url: String,
    // oauth2_token_provider: from F6
}

impl OutlookCalendarProvider {
    /// List events: GET /me/calendarView?startDateTime=...&endDateTime=...
    async fn list_events(...) -> Result<Vec<CalendarEvent>, CalendarError> { ... }

    /// Create event: POST /me/events
    async fn create_event(...) -> Result<CalendarEvent, CalendarError> { ... }

    /// Update event: PATCH /me/events/{id}
    async fn update_event(...) -> Result<CalendarEvent, CalendarError> { ... }

    /// Delete event: DELETE /me/events/{id}
    async fn delete_event(...) -> Result<(), CalendarError> { ... }

    /// Free/busy: POST /me/calendar/getSchedule
    async fn check_availability(...) -> Result<Vec<BusyInterval>, CalendarError> { ... }
}
```

#### Task F5.4: iCal local provider

```rust
/// Local .ics file parser.
/// Read-only for list_events. Write support creates/modifies .ics files on disk.
/// No network access required.
#[cfg(feature = "ical")]
pub struct ICalProvider {
    file_path: std::path::PathBuf,
}

impl ICalProvider {
    /// Parse .ics file and filter events by date range.
    async fn list_events(...) -> Result<Vec<CalendarEvent>, CalendarError> {
        let contents = tokio::fs::read_to_string(&self.file_path).await?;
        let calendar = icalendar::parser::read_calendar(&contents)?;
        // Filter VEVENT components by DTSTART/DTEND within range
        ...
    }

    /// Append a VEVENT to the .ics file.
    async fn create_event(...) -> Result<CalendarEvent, CalendarError> { ... }

    // update_event and delete_event: parse file, find event by UID, modify, rewrite
}
```

#### Task F5.5: `Tool` trait implementation

Route tool calls through the appropriate provider based on `provider` parameter. Validate date ranges (start < end, max range 1 year).

### 5.5 Permission Manifest (F5)

```json
{
  "permissions": {
    "filesystem": { "read": true, "write": true, "paths": ["$WORKSPACE/.ics"] },
    "network": {
      "allowed": [
        "www.googleapis.com",
        "graph.microsoft.com",
        "login.microsoftonline.com",
        "oauth2.googleapis.com"
      ]
    },
    "shell": false
  }
}
```

---

## 6. F7: Docker/Podman Orchestration

### 6.1 Crate: `crates/clawft-plugin-containers/`

**Feature Flag**: `plugin-containers`
**Transport**: subprocess (`docker` / `podman` CLI)

### 6.2 Cargo.toml

```toml
[package]
name = "clawft-plugin-containers"
version = "0.1.0"
edition = "2021"

[dependencies]
clawft-plugin = { workspace = true }
clawft-types = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true, features = ["process"] }
thiserror = { workspace = true }
tracing = { workspace = true }
regex = "1"
```

### 6.3 Operations

| Tool Name | Description | Input Parameters | Output |
|-----------|-------------|------------------|--------|
| `container_build` | Build an image | `context_path: String`, `dockerfile: Option<String>`, `tag: String`, `args: Option<HashMap<String,String>>` | Build output, image ID |
| `container_run` | Run a container | `image: String`, `name: Option<String>`, `ports: Option<Vec<String>>`, `env: Option<HashMap<String,String>>`, `volumes: Option<Vec<String>>`, `detach: bool` | Container ID |
| `container_stop` | Stop a running container | `container: String`, `timeout: Option<u32>` | Success/failure |
| `container_logs` | Get container logs | `container: String`, `tail: Option<u32>`, `follow: bool` | Log output |
| `container_list` | List running containers | `all: bool`, `filter: Option<String>` | Container list |
| `container_exec` | Execute command in container | `container: String`, `command: Vec<String>`, `workdir: Option<String>` | Command output |

### 6.4 Implementation Tasks

#### Task F7.1: Runtime detection

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContainerRuntime {
    Docker,
    Podman,
}

impl ContainerRuntime {
    /// Detect available runtime. Prefer docker, fall back to podman.
    pub async fn detect() -> Result<Self, ContainerError> {
        if is_available("docker").await {
            Ok(ContainerRuntime::Docker)
        } else if is_available("podman").await {
            Ok(ContainerRuntime::Podman)
        } else {
            Err(ContainerError::NoRuntimeFound)
        }
    }

    pub fn binary_name(&self) -> &str {
        match self {
            ContainerRuntime::Docker => "docker",
            ContainerRuntime::Podman => "podman",
        }
    }
}

async fn is_available(binary: &str) -> bool {
    tokio::process::Command::new(binary)
        .arg("--version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}
```

#### Task F7.2: Command builder with input validation

**CRITICAL**: Never construct commands via string interpolation. Use a builder pattern that validates each argument.

```rust
use std::process::Stdio;

/// Validated container name pattern: alphanumeric, hyphens, underscores, dots.
/// Max 128 characters.
const CONTAINER_NAME_PATTERN: &str = r"^[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}$";

/// Validated image name pattern: registry/repo:tag format.
const IMAGE_NAME_PATTERN: &str = r"^[a-zA-Z0-9][a-zA-Z0-9._/-]{0,255}(:[a-zA-Z0-9._-]{1,128})?$";

/// Validated env var key pattern: alphanumeric and underscores.
const ENV_KEY_PATTERN: &str = r"^[a-zA-Z_][a-zA-Z0-9_]{0,255}$";

pub struct ContainerCommand {
    runtime: ContainerRuntime,
    args: Vec<String>,
}

impl ContainerCommand {
    pub fn new(runtime: ContainerRuntime) -> Self {
        Self {
            runtime,
            args: Vec::new(),
        }
    }

    pub fn arg(&mut self, arg: &str) -> &mut Self {
        self.args.push(arg.to_string());
        self
    }

    /// Add a container name argument after validation.
    pub fn container_name(&mut self, name: &str) -> Result<&mut Self, ContainerError> {
        let re = regex::Regex::new(CONTAINER_NAME_PATTERN).unwrap();
        if !re.is_match(name) {
            return Err(ContainerError::InvalidContainerName(name.into()));
        }
        self.args.push(name.to_string());
        Ok(self)
    }

    /// Add an image name argument after validation.
    pub fn image_name(&mut self, name: &str) -> Result<&mut Self, ContainerError> {
        let re = regex::Regex::new(IMAGE_NAME_PATTERN).unwrap();
        if !re.is_match(name) {
            return Err(ContainerError::InvalidImageName(name.into()));
        }
        self.args.push(name.to_string());
        Ok(self)
    }

    /// Add an environment variable after key validation.
    /// Value is passed as-is (it's inside the container).
    pub fn env_var(&mut self, key: &str, value: &str) -> Result<&mut Self, ContainerError> {
        let re = regex::Regex::new(ENV_KEY_PATTERN).unwrap();
        if !re.is_match(key) {
            return Err(ContainerError::InvalidEnvKey(key.into()));
        }
        self.args.push("-e".to_string());
        self.args.push(format!("{}={}", key, value));
        Ok(self)
    }

    /// Execute the command and return stdout/stderr.
    pub async fn execute(&self) -> Result<CommandOutput, ContainerError> {
        let output = tokio::process::Command::new(self.runtime.binary_name())
            .args(&self.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| ContainerError::ExecutionFailed(e.to_string()))?;

        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            success: output.status.success(),
        })
    }
}

#[derive(Debug, serde::Serialize)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub success: bool,
}
```

#### Task F7.3: Global concurrency limiter

```rust
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Global concurrency limiter for container operations.
/// Default: 3 concurrent operations.
pub struct ContainerConcurrencyLimiter {
    semaphore: Arc<Semaphore>,
}

impl ContainerConcurrencyLimiter {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// Acquire a permit before executing a container operation.
    /// Returns a guard that releases the permit on drop.
    pub async fn acquire(&self) -> Result<tokio::sync::SemaphorePermit<'_>, ContainerError> {
        self.semaphore.acquire().await
            .map_err(|_| ContainerError::ConcurrencyLimitClosed)
    }
}
```

Default limit: 3 concurrent Docker/Podman operations globally across all agents.

#### Task F7.4: Implement `container_build`

```rust
pub async fn build(
    runtime: ContainerRuntime,
    limiter: &ContainerConcurrencyLimiter,
    context_path: &str,
    dockerfile: Option<&str>,
    tag: &str,
    build_args: Option<&HashMap<String, String>>,
) -> Result<CommandOutput, ContainerError> {
    let _permit = limiter.acquire().await?;

    let mut cmd = ContainerCommand::new(runtime);
    cmd.arg("build");

    if let Some(df) = dockerfile {
        // Validate dockerfile path -- no directory traversal
        if df.contains("..") {
            return Err(ContainerError::InvalidPath(df.into()));
        }
        cmd.arg("-f").arg(df);
    }

    cmd.arg("-t");
    cmd.image_name(tag)?;

    if let Some(args) = build_args {
        for (key, value) in args {
            let re = regex::Regex::new(ENV_KEY_PATTERN).unwrap();
            if !re.is_match(key) {
                return Err(ContainerError::InvalidBuildArg(key.clone()));
            }
            cmd.arg("--build-arg");
            cmd.arg(&format!("{}={}", key, value));
        }
    }

    cmd.arg(context_path);
    cmd.execute().await
}
```

#### Task F7.5: Implement `container_run`

```rust
pub async fn run(
    runtime: ContainerRuntime,
    limiter: &ContainerConcurrencyLimiter,
    image: &str,
    name: Option<&str>,
    ports: Option<&[String]>,
    env: Option<&HashMap<String, String>>,
    volumes: Option<&[String]>,
    detach: bool,
) -> Result<CommandOutput, ContainerError> {
    let _permit = limiter.acquire().await?;

    let mut cmd = ContainerCommand::new(runtime);
    cmd.arg("run");

    if detach {
        cmd.arg("-d");
    }

    if let Some(n) = name {
        cmd.arg("--name");
        cmd.container_name(n)?;
    }

    if let Some(ports) = ports {
        for port in ports {
            // Validate port mapping format: host:container or host:container/protocol
            validate_port_mapping(port)?;
            cmd.arg("-p").arg(port);
        }
    }

    if let Some(env) = env {
        for (key, value) in env {
            cmd.env_var(key, value)?;
        }
    }

    if let Some(volumes) = volumes {
        for vol in volumes {
            // Validate volume mount: no absolute host paths pointing outside workspace
            validate_volume_mount(vol)?;
            cmd.arg("-v").arg(vol);
        }
    }

    cmd.image_name(image)?;
    cmd.execute().await
}
```

#### Task F7.6: Implement `container_stop`, `container_logs`, `container_list`, `container_exec`

Each follows the same pattern: acquire concurrency permit, build validated command, execute, return output.

- `container_stop`: `docker stop [--time <timeout>] <container>`
- `container_logs`: `docker logs [--tail <n>] <container>` (truncate output to 1 MB)
- `container_list`: `docker ps [--all] [--filter <filter>] --format json`
- `container_exec`: `docker exec [-w <workdir>] <container> <command...>` (validate command args)

#### Task F7.7: `Tool` trait implementation

Register all six operations with MCP-compatible JSON Schema.

### 6.5 Security Requirements (F7)

| Requirement | Implementation |
|-------------|----------------|
| No command injection | Builder pattern with regex validation on all names/args |
| Container name validation | `^[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}$` |
| Image name validation | `^[a-zA-Z0-9][a-zA-Z0-9._/-]{0,255}(:[a-zA-Z0-9._-]{1,128})?$` |
| Env var key validation | `^[a-zA-Z_][a-zA-Z0-9_]{0,255}$` |
| Volume mount validation | No absolute host paths outside `$WORKSPACE`; reject `..` traversal |
| Port mapping validation | Must match `\d+:\d+(/\w+)?` format |
| Dockerfile path validation | No `..` traversal |
| Log output truncation | Max 1 MB stdout/stderr per operation |
| Concurrency limiting | Global semaphore (default 3) |

### 6.6 Permission Manifest (F7)

```json
{
  "permissions": {
    "filesystem": { "read": true, "write": false, "paths": ["$WORKSPACE"] },
    "network": { "allowed": [] },
    "shell": true
  }
}
```

### 6.7 Resource Limits (F7)

| Resource | Default | Rationale |
|----------|---------|-----------|
| Concurrent operations | 3 (global) | Prevent resource exhaustion under multi-agent use |
| Log output per operation | 1 MB | Prevent memory exhaustion from verbose logs |
| Command timeout | 600s (10 min) | Prevent hung builds from blocking the semaphore |

---

## 7. Tool Permission Model (Cross-Cutting)

Each plugin declares permissions in its manifest. The K3 sandbox enforces at runtime. Permissions must be specified before implementation begins.

| Permission | F3 Tree-sitter | F4 Browser | F5 Calendar | F7 Docker |
|-----------|----------------|------------|-------------|-----------|
| `permissions.filesystem.read` | Yes (`$WORKSPACE`) | No | Yes (`.ics` files) | Yes (`$WORKSPACE`) |
| `permissions.filesystem.write` | No | No | Yes (`.ics` files) | No |
| `permissions.network` | None | `$ALLOWED_DOMAINS` | Google/MS APIs | None |
| `permissions.shell` | No | Yes (Chrome) | No | Yes (docker/podman) |

---

## 8. Tests Required

### 8.1 F3: Tree-sitter Tests

| # | Test | Type | Description | Expected Result |
|---|------|------|-------------|-----------------|
| T01 | `parse_rust_file` | Unit | Parse a Rust source file | Valid AST JSON with function_item nodes |
| T02 | `parse_typescript_file` | Unit | Parse a TypeScript file | Valid AST JSON |
| T03 | `parse_python_file` | Unit | Parse a Python file | Valid AST JSON |
| T04 | `parse_unsupported_language` | Unit | Request parsing for unsupported language | `Err(UnsupportedLanguage)` |
| T05 | `parse_file_not_found` | Unit | Parse non-existent file | `Err(FileRead)` |
| T06 | `parse_file_too_large` | Unit | Parse file > 4 MB | `Err(FileTooLarge)` |
| T07 | `query_function_definitions` | Unit | Query for function defs in Rust file | List of function names and ranges |
| T08 | `query_invalid_pattern` | Unit | Run malformed tree-sitter query | `Err(InvalidQuery)` |
| T09 | `complexity_simple_function` | Unit | Calculate complexity of linear function | Complexity = 1 |
| T10 | `complexity_branching_function` | Unit | Function with 3 if/match branches | Complexity = 4 |
| T11 | `complexity_nested_loops` | Unit | Nested loops with conditions | Correct composite score |
| T12 | `symbols_list_all` | Unit | List all symbols in a Rust file | Functions, structs, enums found |
| T13 | `symbols_filter_by_kind` | Unit | List only functions | Only function symbols returned |
| T14 | `references_find_symbol` | Unit | Find references to a function name | All call sites returned |
| T15 | `detect_language_by_extension` | Unit | `.rs` -> rust, `.ts` -> typescript | Correct mapping |
| T16 | `supported_languages_matches_features` | Unit | Registry reports only enabled grammars | Matches compiled features |
| T17 | `fuzz_rust_grammar` | Fuzz | Random bytes to Rust parser | No panics, no memory corruption |
| T18 | `fuzz_typescript_grammar` | Fuzz | Random bytes to TypeScript parser | No panics, no memory corruption |

### 8.2 F4: Browser CDP Tests

| # | Test | Type | Description | Expected Result |
|---|------|------|-------------|-----------------|
| T19 | `validate_url_http_allowed` | Unit | HTTP URL with domain in allowed list | Ok(Url) |
| T20 | `validate_url_https_allowed` | Unit | HTTPS URL with domain in allowed list | Ok(Url) |
| T21 | `validate_url_file_scheme_blocked` | Unit | `file:///etc/passwd` | `Err(BlockedScheme("file"))` |
| T22 | `validate_url_data_scheme_blocked` | Unit | `data:text/html,...` | `Err(BlockedScheme("data"))` |
| T23 | `validate_url_javascript_scheme_blocked` | Unit | `javascript:alert(1)` | `Err(BlockedScheme("javascript"))` |
| T24 | `validate_url_blob_scheme_blocked` | Unit | `blob:...` | `Err(BlockedScheme("blob"))` |
| T25 | `validate_url_ftp_scheme_blocked` | Unit | `ftp://...` | `Err(BlockedScheme("ftp"))` |
| T26 | `validate_url_domain_not_allowed` | Unit | URL with domain not in allowed list | `Err(DomainNotAllowed)` |
| T27 | `validate_url_no_domains_configured` | Unit | Empty allowed_domains | `Err(NoDomainsConfigured)` |
| T28 | `validate_url_wildcard_domain` | Unit | `*.example.com` matches `sub.example.com` | Ok(Url) |
| T29 | `validate_url_wildcard_no_bare` | Unit | `*.example.com` does NOT match `example.com` | `Err(DomainNotAllowed)` |
| T30 | `validate_url_redirect_checked` | Unit | Navigate, final URL on different domain | `Err(DomainNotAllowed)` |
| T31 | `page_limit_enforced` | Unit | Open pages beyond max_concurrent_pages | `Err(PageLimitReached)` |
| T32 | `session_lifetime_expiry` | Unit | Session exceeds configured lifetime | `Err(SessionExpired)` or force-kill |
| T33 | `clear_state_between_sessions` | Unit | State cleared when flag is true | Cookies/storage cleared |
| T34 | `evaluate_expression_too_large` | Unit | JS expression > 10 KB | `Err(ExpressionTooLarge)` |
| T35 | `evaluate_timeout` | Unit | Infinite loop JS expression | `Err(EvaluationTimeout)` |
| T36 | `default_sandbox_config` | Unit | Default BrowserSandboxConfig values | Matches documented defaults |
| T37 | `screenshot_max_size` | Unit | Very large page screenshot | Truncated or error at 10 MB |

### 8.3 F5: Calendar Tests

| # | Test | Type | Description | Expected Result |
|---|------|------|-------------|-----------------|
| T38 | `google_list_events_response` | Unit | Parse Google Calendar API response | Vec of CalendarEvent |
| T39 | `google_create_event_request` | Unit | NewEvent -> API request body | Correct JSON structure |
| T40 | `google_update_event_request` | Unit | EventUpdate -> PATCH body | Only changed fields |
| T41 | `google_delete_event_request` | Unit | Delete -> DELETE request | Correct URL and method |
| T42 | `google_freebusy_response` | Unit | Parse freeBusy API response | Vec of BusyInterval |
| T43 | `outlook_list_events_response` | Unit | Parse Graph API calendarView response | Vec of CalendarEvent |
| T44 | `outlook_create_event_request` | Unit | NewEvent -> Graph API body | Correct JSON structure |
| T45 | `outlook_freebusy_response` | Unit | Parse getSchedule response | Vec of BusyInterval |
| T46 | `ical_parse_events` | Unit | Parse .ics file with multiple VEVENTs | Correct event list |
| T47 | `ical_filter_by_date_range` | Unit | Events outside range excluded | Only in-range events |
| T48 | `ical_create_event_appends` | Unit | New VEVENT appended to .ics | File contains new event |
| T49 | `invalid_date_range` | Unit | start > end | `Err(InvalidDateRange)` |
| T50 | `excessive_date_range` | Unit | Range > 1 year | `Err(DateRangeTooLarge)` |
| T51 | `provider_not_configured` | Unit | Request to unconfigured provider | `Err(ProviderNotConfigured)` |
| T52 | `tool_trait_schema_valid` | Unit | All tool input schemas valid JSON Schema | Schema validates |

### 8.4 F7: Docker/Podman Tests

| # | Test | Type | Description | Expected Result |
|---|------|------|-------------|-----------------|
| T53 | `detect_runtime_docker` | Unit | `docker --version` succeeds | `ContainerRuntime::Docker` |
| T54 | `detect_runtime_podman_fallback` | Unit | Docker unavailable, podman available | `ContainerRuntime::Podman` |
| T55 | `detect_runtime_none` | Unit | Neither available | `Err(NoRuntimeFound)` |
| T56 | `validate_container_name_valid` | Unit | `my-container-1` | Ok |
| T57 | `validate_container_name_invalid` | Unit | `; rm -rf /` | `Err(InvalidContainerName)` |
| T58 | `validate_container_name_too_long` | Unit | 200-char name | `Err(InvalidContainerName)` |
| T59 | `validate_image_name_valid` | Unit | `nginx:latest` | Ok |
| T60 | `validate_image_name_registry` | Unit | `ghcr.io/org/image:v1` | Ok |
| T61 | `validate_image_name_invalid` | Unit | `; cat /etc/passwd` | `Err(InvalidImageName)` |
| T62 | `validate_env_key_valid` | Unit | `MY_VAR` | Ok |
| T63 | `validate_env_key_invalid` | Unit | `MY;VAR` | `Err(InvalidEnvKey)` |
| T64 | `validate_port_mapping_valid` | Unit | `8080:80` | Ok |
| T65 | `validate_port_mapping_protocol` | Unit | `8080:80/tcp` | Ok |
| T66 | `validate_port_mapping_invalid` | Unit | `abc:xyz` | `Err(InvalidPortMapping)` |
| T67 | `validate_volume_traversal` | Unit | `../../etc:/data` | `Err(InvalidVolume)` |
| T68 | `validate_dockerfile_traversal` | Unit | `../../Dockerfile` | `Err(InvalidPath)` |
| T69 | `concurrency_limiter_permits` | Unit | 3 concurrent ops succeed, 4th blocks | Semaphore behavior correct |
| T70 | `concurrency_limiter_release` | Unit | Completed op releases permit | Next op proceeds |
| T71 | `build_command_construction` | Unit | build() produces correct args | `["build", "-t", "tag", "."]` |
| T72 | `run_command_with_env` | Unit | run() with env vars | `-e KEY=VALUE` args present |
| T73 | `stop_command_with_timeout` | Unit | stop() with timeout | `["stop", "--time", "30", "name"]` |
| T74 | `exec_command_construction` | Unit | exec() produces correct args | `["exec", "container", "cmd", "args"]` |
| T75 | `list_command_json_format` | Unit | list() requests JSON output | `--format json` present |
| T76 | `logs_output_truncated` | Unit | Logs > 1 MB | Output truncated to 1 MB |
| T77 | `build_arg_key_validation` | Unit | Invalid build-arg key rejected | `Err(InvalidBuildArg)` |

### 8.5 Cross-Cutting Tests

| # | Test | Type | Description | Expected Result |
|---|------|------|-------------|-----------------|
| T78 | `all_tools_implement_trait` | Unit | Each plugin returns valid ToolDefinitions | Non-empty tool list with schemas |
| T79 | `tool_schemas_valid_json_schema` | Unit | All input_schema values are valid JSON Schema | Validation passes |
| T80 | `unknown_tool_name_errors` | Unit | Execute with unknown tool name | `Err(UnknownTool)` |

---

## 9. Acceptance Criteria

### F3: Tree-sitter Code Analysis
- [ ] `crates/clawft-plugin-treesitter` compiles as workspace crate
- [ ] Feature-gated behind `plugin-treesitter` with per-language sub-features
- [ ] `ts_parse` returns AST for Rust, TypeScript, and Python source files
- [ ] `ts_query` executes tree-sitter queries and returns matched nodes with ranges
- [ ] `ts_complexity` calculates cyclomatic complexity per function
- [ ] `ts_symbols` lists functions, structs, classes with name, kind, and range
- [ ] `ts_references` finds syntactic references to a named symbol
- [ ] Language detection from file extension works for all supported languages
- [ ] File size limit (4 MB) enforced on parse input
- [ ] Implements `Tool` plugin trait with MCP-compatible JSON Schema
- [ ] Fuzz tests pass for Rust and TypeScript grammars (no panics, no memory corruption)
- [ ] All unit tests T01-T18 pass
- [ ] `cargo clippy -p clawft-plugin-treesitter --features all-languages -- -D warnings` is clean

### F4: Browser CDP Automation
- [ ] `crates/clawft-plugin-browser` compiles as workspace crate
- [ ] Feature-gated behind `plugin-browser`
- [ ] `BrowserSandboxConfig` struct with documented defaults (2 pages, 300s lifetime, 512 MB, clear state)
- [ ] `validate_navigation_url()` blocks `file://`, `data://`, `javascript://`, `blob://`, `ftp://` schemes
- [ ] `validate_navigation_url()` enforces `allowed_domains` with exact and wildcard matching
- [ ] Post-redirect URL validation (final URL checked against allowed_domains)
- [ ] `browser_navigate` navigates and returns title + final URL
- [ ] `browser_screenshot` captures PNG (element or full-page)
- [ ] `browser_fill` fills form fields by CSS selector
- [ ] `browser_click` clicks elements by CSS selector
- [ ] `browser_get_text` extracts text by CSS selector
- [ ] `browser_evaluate` executes JS with expression size limit (10 KB) and timeout (5s)
- [ ] Session lifetime watchdog force-kills browser after expiry
- [ ] State clearing (cookies, storage) between sessions when configured
- [ ] Concurrent page limit enforced per `max_concurrent_pages`
- [ ] Implements `Tool` plugin trait with MCP-compatible JSON Schema
- [ ] All unit tests T19-T37 pass
- [ ] `cargo clippy -p clawft-plugin-browser -- -D warnings` is clean

### F5: Calendar Integration
- [ ] `crates/clawft-plugin-calendar` compiles as workspace crate
- [ ] Feature-gated behind `plugin-calendar` with `google`, `outlook`, `ical` sub-features
- [ ] `CalendarProvider` trait with list, create, update, delete, check_availability operations
- [ ] Google Calendar provider implements all 5 operations via Calendar API v3
- [ ] Outlook Calendar provider implements all 5 operations via Graph API
- [ ] iCal provider parses `.ics` files and supports local CRUD
- [ ] Date range validation (start < end, max 1 year)
- [ ] Uses F6 OAuth2 helper for Google and Microsoft authentication
- [ ] Implements `Tool` plugin trait with MCP-compatible JSON Schema
- [ ] All unit tests T38-T52 pass
- [ ] `cargo clippy -p clawft-plugin-calendar -- -D warnings` is clean

### F7: Docker/Podman Orchestration
- [ ] `crates/clawft-plugin-containers` compiles as workspace crate
- [ ] Feature-gated behind `plugin-containers`
- [ ] Runtime detection: prefers docker, falls back to podman
- [ ] `container_build`, `container_run`, `container_stop`, `container_logs`, `container_list`, `container_exec` all functional
- [ ] Builder pattern for command construction -- no string interpolation
- [ ] Container name validation: `^[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}$`
- [ ] Image name validation: `^[a-zA-Z0-9][a-zA-Z0-9._/-]{0,255}(:[a-zA-Z0-9._-]{1,128})?$`
- [ ] Environment variable key validation: `^[a-zA-Z_][a-zA-Z0-9_]{0,255}$`
- [ ] Port mapping validation: `\d+:\d+(/\w+)?` format
- [ ] Volume mount validation: no `..` traversal, no absolute host paths outside `$WORKSPACE`
- [ ] Dockerfile path validation: no `..` traversal
- [ ] Global concurrency limiter (default: 3 concurrent operations)
- [ ] Log output truncation (max 1 MB per operation)
- [ ] Implements `Tool` plugin trait with MCP-compatible JSON Schema
- [ ] All unit tests T53-T77 pass
- [ ] `cargo clippy -p clawft-plugin-containers -- -D warnings` is clean

### Cross-Cutting
- [ ] All four crates are registered in workspace `Cargo.toml`
- [ ] All four crates implement the `Tool` trait from `clawft-plugin` (C1)
- [ ] All tools have MCP-compatible JSON Schema for `input_schema`
- [ ] All permission manifests defined before implementation begins
- [ ] `cargo test --workspace` passes with all plugin features enabled
- [ ] `cargo test --workspace` passes with no optional plugin features
- [ ] All existing tests pass (regression gate)

---

## 10. Risk Notes

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| Browser CDP security exposure (local file access, JS eval exfiltration) | Medium | High | **6** | `BrowserSandboxConfig` with `allowed_domains`; block file/data/javascript/blob URL schemes; post-redirect validation; clear state between sessions; session lifetime watchdog |
| Browser + Docker resource exhaustion under multi-agent use | Medium | Medium | **4** | Per-tool concurrency limits: 1 browser session/agent, 2 pages/session, 3 Docker ops global. Session lifetime watchdog force-kills expired browsers |
| tree-sitter C grammar FFI memory safety | Low | Medium | **3** | Native-only (no WASM); fuzz tests on C grammar bindings; optional features per language; file size limit (4 MB) |
| F5 blocked on F6 OAuth2 helper delivery | Medium | Medium | **4** | F6 is P0 priority in Phase F-Core (Week 5-7). If delayed, F5 Google/Outlook providers defer; iCal provider has no dependency and can ship independently |
| Docker command injection via user-provided names | Medium | High | **6** | Builder pattern with regex validation on all user inputs; never use string interpolation for commands; reject inputs that don't match strict patterns |
| Volume mount escape allowing host filesystem access | Medium | High | **6** | Reject `..` in paths; reject absolute host paths outside `$WORKSPACE`; validate all volume mount strings before passing to CLI |
| chromiumoxide crate version churn | Low | Medium | **3** | Pin to `0.7.x`; Chrome DevTools Protocol is stable; wrap all chromiumoxide calls in adapter layer for easy migration |
| Plugin crate proliferation increases CI build time | Low | Low | **2** | Feature-gated; CI builds only changed crates; parallel workspace compilation |

---

## 11. Concurrency Plan

### Parallel Tasks (can execute simultaneously)

F3 (tree-sitter) and F7 (Docker) have **no mutual dependencies** and can be developed in parallel from Week 6.

F4 (browser) and F5 (calendar) can start in parallel, but F5 Google/Outlook providers are blocked on F6.

### Suggested Execution Order

```text
Week 6:
  ├── F3: Language registry, ts_parse, ts_query (T01-T08)
  ├── F4: BrowserSandboxConfig, URL validation (T19-T29)
  └── F7: Runtime detection, command builder, validation (T53-T68)

Week 7:
  ├── F3: ts_complexity, ts_symbols, ts_references, Tool trait (T09-T16)
  ├── F4: Session manager, navigate, screenshot (T30-T33)
  ├── F5: Provider trait, iCal provider (T46-T48) -- no F6 dependency
  └── F7: container_build, container_run, concurrency limiter (T69-T72)

Week 8:
  ├── F3: Fuzz tests (T17-T18)
  ├── F4: fill, click, get_text, evaluate, watchdog (T34-T37)
  ├── F5: Google Calendar provider (T38-T42) -- requires F6
  └── F7: container_stop, logs, list, exec, Tool trait (T73-T77)

Week 9:
  ├── F5: Outlook provider, Tool trait (T43-T45, T49-T52)
  └── Integration testing across all four tools (T78-T80)
```

### Dependency Graph

```text
C1 (plugin traits) ──────────┬── F3 (tree-sitter)  ── no further deps
                              ├── F4 (browser CDP)  ── no further deps
                              ├── F7 (docker/podman) ── no further deps
                              └── F5 (calendar)
                                    │
                                    └── F6 (OAuth2) ── Google/Outlook providers blocked
                                          │
                                          └── (iCal provider is NOT blocked)
```

---

## 12. References

- Orchestrator: `00-orchestrator.md` (this directory)
- Plugin Traits (C1): `../04-plugin-skill-system/01-phase-C1-plugin-traits.md`
- WASM Security Spec: `../04-plugin-skill-system/01-wasm-security-spec.md`
- Dev Assignment: `../02-improvements-overview/dev-assignment-07-dev-tools-apps.md`
- Cross-Element Integration: `../02-improvements-overview/01-cross-element-integration.md`
- tree-sitter docs: https://tree-sitter.github.io/tree-sitter/
- chromiumoxide docs: https://docs.rs/chromiumoxide/latest/chromiumoxide/
- Google Calendar API v3: https://developers.google.com/calendar/api/v3/reference
- Microsoft Graph Calendar API: https://learn.microsoft.com/en-us/graph/api/resources/calendar
- icalendar crate: https://docs.rs/icalendar/latest/icalendar/
