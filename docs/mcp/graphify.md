# Graphify MCP server (WEFT-369)

stdio MCP server that exposes **clawft-graphify** knowledge-graph tools to
MCP clients (Claude Desktop, Cursor, Grok, etc.). Implements Phase 6 of the
graphify Rust port (`.planning/graphify-rs/`).

## Transport

| Property | Value |
|----------|--------|
| Protocol | MCP JSON-RPC 2.0 over **newline-delimited stdio** |
| Protocol version | `2025-06-18` |
| Server name | `clawft-graphify` |
| Feature flag | `clawft-graphify` feature `mcp` |

HTTP transport is out of scope for this phase; the product MCP surface for
control/workspace tools remains `weft mcp-server` (ADR-076). Graphify is a
**specialized** server you attach as an additional MCP process.

## Run

```bash
# From a project that has (or will have) graphify-out/
weaver graphify serve

# Custom work dir / default graph path
weaver graphify serve --work-dir /path/to/repo --graph graphify-out/graph.json
```

Banners go to **stderr** only — stdout is the MCP wire.

### Example client config (Claude Desktop / Cursor style)

```json
{
  "mcpServers": {
    "graphify": {
      "command": "weaver",
      "args": ["graphify", "serve", "--work-dir", "/path/to/repo"]
    }
  }
}
```

Or via WeftOS tools registry (`tools.mcp_servers` / `weft mcp add`):

```toml
[tools.mcp_servers.graphify]
command = "weaver"
args = ["graphify", "serve"]
```

## Tools

| Tool | Purpose |
|------|---------|
| `graphify_query` | Keyword search + optional BFS/DFS neighbourhood |
| `graphify_ingest` | Ingest a local directory (file-level entities) or URL (`http-client`) |
| `graphify_export` | Export graph JSON → json / obsidian / wiki / … |
| `graphify_diff` | Diff two graph JSON snapshots |

### `graphify_query`

| Arg | Type | Default | Notes |
|-----|------|---------|-------|
| `question` | string | *required* | Keywords / natural language |
| `graph` | string | `graphify-out/graph.json` | Relative to work dir |
| `mode` | `bfs` \| `dfs` \| `none` | `bfs` | Neighbourhood expansion |
| `depth` | int 1–6 | `3` | Traversal depth |
| `limit` | int | `10` | Max ranked hits |

### `graphify_ingest`

| Arg | Type | Default | Notes |
|-----|------|---------|-------|
| `target` | string | *required* | Directory path or `http(s)://` URL |
| `output` | string | `<target>/graphify-out` | Artifact root |
| `contributor` | string | — | URL ingest metadata only |

Local directory ingest uses the same file-level pipeline as
`weaver graphify rebuild` (no tree-sitter required). URL ingest needs the
`http-client` feature (enabled on the weaver binary).

### `graphify_export`

| Arg | Type | Default |
|-----|------|---------|
| `format` | string | *required* (`json`, `obsidian`, `wiki`, …) |
| `graph` | string | `graphify-out/graph.json` |
| `output` | string | format-specific under `graphify-out/` |

### `graphify_diff`

| Arg | Type | Default |
|-----|------|---------|
| `old` | string | `graphify-out/graph.json.bak` |
| `current` | string | `graphify-out/graph.json` |

## Library API

```rust
use clawft_graphify::mcp::{McpConfig, serve_stdio, tool_definitions, dispatch_tool};

// Tool schemas
let tools = tool_definitions();

// Direct handler (no transport)
let cfg = McpConfig::default();
let result = dispatch_tool(&cfg, "graphify_query", &serde_json::json!({
    "question": "auth module"
}));

// stdio server
// serve_stdio(cfg).await?;
```

## Tests

```bash
# Unit + in-memory MCP protocol tests
scripts/build.sh test -p clawft-graphify --features mcp

# Or via cargo when debugging
cargo test -p clawft-graphify --features mcp
```

## Related

- Plane: **WEFT-369** (ws12 Phase 6)
- CLI siblings: `weaver graphify {ingest,query,export,diff,rebuild}`
- Product MCP server: `docs/guides/mcp.md`, ADR-076
- Graphify master plan: `.planning/graphify-rs/MASTER_PLAN.md`
