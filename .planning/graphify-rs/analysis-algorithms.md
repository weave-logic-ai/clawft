# Graphify Analysis, Clustering, Export & Utility Modules -- Complete Feature Map

Source: `/tmp/graphify-source/graphify/` (Python)
Target: WeftOS Rust port
Date: 2026-04-04

---

## 1. analyze.py -- Graph Analysis Algorithms

### Functions

#### `_node_community_map(communities: dict[int, list[str]]) -> dict[str, int]`
- Inverts `{community_id: [node_ids]}` to `{node_id: community_id}`
- Pure utility, used throughout analyze/export/report

#### `_is_file_node(G, node_id) -> bool`
- Filters synthetic AST nodes from analysis results
- Checks 3 conditions:
  1. Label matches source filename (file-level hub)
  2. Label starts with `.` and ends with `()` (method stub, e.g. `.auth_flow()`)
  3. Label ends with `()` and degree <= 1 (isolated function stub)
- Used in: god_nodes, surprising_connections, suggest_questions, report

#### `_is_concept_node(G, node_id) -> bool`
- Returns True for manually-injected semantic nodes (not from source code)
- Signals: empty `source_file`, or source_file has no file extension
- Used to exclude concept nodes from surprising connections

#### `god_nodes(G, top_n=10) -> list[dict]`
- **Algorithm**: Degree centrality (simple degree sort, descending)
- Excludes file nodes and concept nodes
- Returns: `[{id, label, edges}]`
- WeftOS equivalent: Partial -- `causal.rs` has centrality but not this filtering logic

#### `surprising_connections(G, communities, top_n=5) -> list[dict]`
- **Strategy selector**: Multi-file vs single-file corpora
  - Multi-file: calls `_cross_file_surprises()`
  - Single-file: calls `_cross_community_surprises()`

#### `_surprise_score(G, u, v, data, node_community, u_source, v_source) -> (int, list[str])`
- **Composite surprise scoring algorithm** with 5 factors:
  1. **Confidence weight**: AMBIGUOUS=3, INFERRED=2, EXTRACTED=1
  2. **Cross file-type bonus** (+2): code<->paper, code<->image
  3. **Cross-repo bonus** (+2): different top-level directory
  4. **Cross-community bonus** (+1): Leiden says structurally distant
  5. **Peripheral->hub bonus** (+1): low-degree node reaching high-degree node
  6. **Semantic similarity multiplier** (1.5x): `semantically_similar_to` relation
- Returns score + human-readable reasons list

#### `_cross_file_surprises(G, communities, top_n) -> list[dict]`
- Finds cross-file edges between real entities
- Excludes: structural edges (imports, contains, method), concept nodes, file nodes, same-file edges
- Scores each candidate with `_surprise_score()`, sorts descending
- Falls back to `_cross_community_surprises()` if no candidates
- Returns: `[{source, target, source_files, confidence, relation, why}]`

#### `_cross_community_surprises(G, communities, top_n) -> list[dict]`
- For single-source corpora: cross-community edges
- **Fallback when no communities**: Uses `nx.edge_betweenness_centrality(G)` (Floyd-Warshall based)
- With communities: finds edges crossing community boundaries
- Sorts by confidence (AMBIGUOUS first)
- **Deduplicates** by community pair to prevent god-node domination
- Returns: `[{source, target, source_files, confidence, relation, note}]`

#### `suggest_questions(G, communities, community_labels, top_n=7) -> list[dict]`
- **5 question generation strategies**:
  1. **AMBIGUOUS edges**: "What is the exact relationship between X and Y?"
  2. **Bridge nodes**: Uses `nx.betweenness_centrality(G)`, top-3 non-file nodes. "Why does X connect A to B?"
  3. **God nodes with INFERRED edges**: "Are the N inferred relationships involving X correct?"
  4. **Isolated/weakly-connected nodes** (degree <= 1): "What connects X, Y, Z to the rest?"
  5. **Low-cohesion communities** (score < 0.15, size >= 5): "Should X be split?"
- Falls back to `{type: "no_signal"}` if no questions generated
- Returns: `[{type, question, why}]`

#### `graph_diff(G_old, G_new) -> dict`
- **Graph snapshot comparison algorithm**
- Computes: new_nodes, removed_nodes, new_edges, removed_edges
- Edge identity: `(min(u,v), max(u,v), relation)` -- undirected edge key
- Returns: `{new_nodes, removed_nodes, new_edges, removed_edges, summary}`

### Data Structures (analyze.py)
- `_CODE_EXTENSIONS`: set of 14 code file extensions
- `_DOC_EXTENSIONS`: {md, txt, rst}
- `_PAPER_EXTENSIONS`: {pdf}
- `_IMAGE_EXTENSIONS`: {png, jpg, jpeg, webp, gif, svg}

### NetworkX Algorithms Used
- `G.degree()` -- degree centrality
- `nx.betweenness_centrality(G)` -- node betweenness
- `nx.edge_betweenness_centrality(G)` -- edge betweenness
- `nx.shortest_path(G, src, tgt)` -- in serve.py

---

## 2. cluster.py -- Community Detection

### Functions

#### `_partition(G) -> dict[str, int]`
- **Primary**: `graspologic.partition.leiden(G)` (Leiden algorithm)
- **Fallback**: `nx.community.louvain_communities(G, seed=42)` (Louvain)
- Returns `{node_id: community_id}`

#### `build_graph(nodes, edges) -> nx.Graph`
- Constructs undirected NetworkX graph from graphify node/edge dicts
- Preserves original direction as `_src`/`_tgt` edge attributes
- Node attributes: all fields except `id` (which becomes the node key)
- Edge attributes: all fields except `source`/`target` plus `_src`/`_tgt`

#### `cluster(G) -> dict[int, list[str]]`
- **Full community detection pipeline**:
  1. Handle empty graph (return `{}`) and edgeless graph (each node = own community)
  2. Separate isolates from connected nodes
  3. Run `_partition()` on connected subgraph
  4. Assign each isolate its own community
  5. **Split oversized communities**: if community > 25% of graph AND >= 10 nodes, run second Leiden pass on subgraph
  6. Re-index communities by size descending (0 = largest)
  7. Sort node lists within each community
- Constants: `_MAX_COMMUNITY_FRACTION = 0.25`, `_MIN_SPLIT_SIZE = 10`

#### `_split_community(G, nodes) -> list[list[str]]`
- Runs second Leiden pass on community subgraph
- Falls back to individual nodes if edgeless, or single community if Leiden fails

#### `cohesion_score(G, community_nodes) -> float`
- **Density metric**: `actual_edges / max_possible_edges`
- Complete subgraph = 1.0, disconnected = 0.0
- Single node = 1.0 by convention

#### `score_all(G, communities) -> dict[int, float]`
- Batch cohesion scoring for all communities

---

## 3. export.py -- Export Formats

### JSON Export: `to_json(G, communities, output_path)`
- Uses `networkx.readwrite.json_graph.node_link_data(G, edges="links")`
- **Schema** (output):
  ```json
  {
    "directed": false,
    "multigraph": false,
    "graph": {},
    "nodes": [
      {"id": "...", "label": "...", "file_type": "...", "source_file": "...",
       "source_location": "...", "community": int}
    ],
    "links": [
      {"source": "...", "target": "...", "relation": "...",
       "confidence": "EXTRACTED|INFERRED|AMBIGUOUS",
       "confidence_score": float, "weight": float,
       "_src": "...", "_tgt": "..."}
    ],
    "hyperedges": [{"id": "...", "label": "...", "nodes": [...], "confidence": "...", "confidence_score": float}]
  }
  ```
- Adds `community` attribute to each node
- Adds default `confidence_score` (EXTRACTED=1.0, INFERRED=0.5, AMBIGUOUS=0.2) if missing

### Neo4j Cypher Export: `to_cypher(G, output_path)`
- Generates `.cypher` text file with MERGE statements
- Node labels derived from `file_type` (capitalized, alphanumeric only, fallback "Entity")
- Edge relation types: uppercase, non-alphanumeric replaced with `_`
- Cypher injection protection: `_cypher_escape()` escapes backslash and single quote

### Neo4j Direct Push: `push_to_neo4j(G, uri, user, password, communities)`
- Requires `pip install neo4j`
- Uses Python `neo4j.GraphDatabase` driver
- MERGE-based upsert (safe to re-run)
- Sets all scalar node/edge properties
- Sanitizes node labels and relation types for Cypher safety
- Returns `{nodes: int, edges: int}`

### HTML/vis.js Visualization: `to_html(G, communities, output_path, community_labels)`
- **Max nodes**: 5,000 (raises ValueError above this)
- **Layout engine**: vis.js `forceAtlas2Based` physics
  - `gravitationalConstant: -60`, `centralGravity: 0.005`
  - `springLength: 120`, `springConstant: 0.08`, `damping: 0.4`
  - `avoidOverlap: 0.8`
  - Stabilization: 200 iterations then physics disabled
- **Node sizing**: `10 + 30 * (degree / max_degree)`
- **Node labels**: Only shown for nodes with degree >= 15% of max_degree; others show on hover
- **Edge styling**: EXTRACTED = solid width-2 opacity-0.7; non-EXTRACTED = dashed width-1 opacity-0.35
- **10 community colors**: `#4E79A7, #F28E2B, #E15759, #76B7B2, #59A14F, #EDC948, #B07AA1, #FF9DA7, #9C755F, #BAB0AC`
- **Interactive features**:
  - Click-to-inspect sidebar panel (node info, neighbors list)
  - Search box with autocomplete (top 20 matches)
  - Community legend with click-to-toggle visibility
  - Neighbor navigation links
  - Stats footer
- **Hyperedge rendering**: Convex hull polygons drawn on canvas `afterDrawing` event, with labels
- All labels sanitized via `sanitize_label()` (XSS prevention)

### GraphML Export: `to_graphml(G, communities, output_path)`
- Copies graph, adds `community` attribute to nodes
- Uses `nx.write_graphml(H, path)`
- Opens in Gephi, yEd

### SVG Export: `to_svg(G, communities, output_path, community_labels, figsize=(20,14))`
- Requires matplotlib
- Layout: `nx.spring_layout(G, seed=42, k=2.0/(sqrt(n)+1))`
- Node sizing: `300 + 1200 * (degree / max_degree)`
- Edge styling: solid for EXTRACTED, dashed otherwise
- Dark background (#1a1a2e)
- Community color legend
- Output: SVG format

### Obsidian Vault Export: `to_obsidian(G, communities, output_dir, community_labels, cohesion)`
- **Per-node .md files** with:
  - YAML frontmatter: `source_file`, `type`, `community`, `location`, `tags`
  - Tags: `graphify/{file_type}`, `graphify/{confidence}`, `community/{name}`
  - `## Connections` section with `[[wikilinks]]` per neighbor
  - Inline tags at bottom
- **Per-community `_COMMUNITY_{name}.md` overview notes** with:
  - YAML frontmatter: `type: community`, `cohesion`, `members`
  - Cohesion description (tightly/moderately/loosely connected)
  - Members list with wikilinks
  - Dataview live query block
  - Cross-community edge counts
  - Top 5 bridge nodes (sorted by community reach, then degree)
- **`.obsidian/graph.json`**: Community color groups for Obsidian graph view
- Filename deduplication: appends numeric suffix for collisions
- Returns: total notes written (nodes + community notes)

### Obsidian Canvas Export: `to_canvas(G, communities, output_path, community_labels, node_filenames)`
- Generates `.canvas` JSON file
- **Layout**: Communities in grid (sqrt(n) columns), nodes in rows of 3 within each community group
- Community groups: colored (6 canvas colors cycling)
- Node cards: file-type referencing obsidian vault notes
- Edges: capped at 200 highest-weight, labeled with relation + confidence
- Returns: `.canvas` JSON with `{nodes: [...], edges: [...]}`

### Utility: `attach_hyperedges(G, hyperedges)`
- Stores hyperedge list in `G.graph["hyperedges"]`, deduplicating by ID

---

## 4. cache.py -- Incremental Processing Cache

### Functions

#### `file_hash(path) -> str`
- **SHA256** of: file contents + null byte + resolved absolute path
- Path included to prevent collisions between identical-content files at different paths

#### `cache_dir(root) -> Path`
- Returns `{root}/graphify-out/cache/`, creating if needed

#### `load_cached(path, root) -> dict | None`
- Cache key: SHA256 hash of file
- Cache location: `graphify-out/cache/{hash}.json`
- Returns None on: missing cache, hash mismatch, JSON decode error, OS error

#### `save_cached(path, result, root)`
- Atomic write: writes to `.tmp` then `os.replace()` to final path
- Result: `{nodes: [...], edges: [...], hyperedges: [...]}`

#### `cached_files(root) -> set[str]`
- Returns set of cached hash stems from cache directory

#### `clear_cache(root)`
- Deletes all `*.json` files from cache directory

#### `check_semantic_cache(files, root) -> (cached_nodes, cached_edges, cached_hyperedges, uncached_files)`
- Batch cache check for file list
- Returns accumulated nodes/edges/hyperedges from cached files + list of uncached paths

#### `save_semantic_cache(nodes, edges, hyperedges, root) -> int`
- Groups nodes/edges/hyperedges by `source_file`
- Saves one cache entry per source file
- Returns count of files cached

### Cache Invalidation
- Content-based: any byte change to file invalidates its cache entry (SHA256 hash changes)
- No TTL -- cache is permanent until content changes or explicit `clear_cache()`
- Cache is per-file, not per-project

---

## 5. validate.py -- Schema Validation

### Constants
- `VALID_FILE_TYPES`: `{code, document, paper, image}`
- `VALID_CONFIDENCES`: `{EXTRACTED, INFERRED, AMBIGUOUS}`
- `REQUIRED_NODE_FIELDS`: `{id, label, file_type, source_file}`
- `REQUIRED_EDGE_FIELDS`: `{source, target, relation, confidence, source_file}`

### Functions

#### `validate_extraction(data) -> list[str]`
- Validates extraction JSON dict
- Checks:
  1. Top-level is dict
  2. `nodes` key exists and is list
  3. Each node has required fields
  4. `file_type` is valid enum
  5. `edges` key exists and is list
  6. Each edge has required fields
  7. `confidence` is valid enum
  8. Edge `source`/`target` reference existing node IDs

#### `assert_valid(data)`
- Wrapper that raises `ValueError` with formatted error list

---

## 6. report.py -- GRAPH_REPORT.md Generation

### Function: `generate(...) -> str`
- **Sections generated**:
  1. **Header**: `# Graph Report - {root} ({date})`
  2. **Corpus Check**: file count, word count, or warning
  3. **Summary**: nodes, edges, communities, confidence breakdown (% EXTRACTED/INFERRED/AMBIGUOUS), INFERRED avg confidence score, token cost
  4. **God Nodes**: numbered list with label + edge count
  5. **Surprising Connections**: source->target with relation, confidence, semantic tag, source files, notes
  6. **Hyperedges**: group relationships with node labels and confidence
  7. **Communities**: per-community: label, cohesion score, top 8 real nodes (excluding file nodes), overflow count
  8. **Ambiguous Edges**: review section listing all AMBIGUOUS edges
  9. **Knowledge Gaps**: isolated nodes (degree<=1), thin communities (<3 nodes), high ambiguity warning (>20%)
  10. **Suggested Questions**: from `suggest_questions()` output

---

## 7. security.py -- Security Measures

### URL Validation: `validate_url(url) -> str`
- **Allowed schemes**: http, https only
- **SSRF protection**:
  - Blocks known cloud metadata hostnames (`metadata.google.internal`, `metadata.google.com`)
  - DNS resolution + IP range check: blocks private, reserved, loopback, link-local addresses
  - Custom redirect handler (`_NoFileRedirectHandler`) re-validates every redirect target

### Safe Fetch: `safe_fetch(url, max_bytes, timeout) -> bytes`
- URL validation before fetch
- **DoS protection**: streaming read with hard cap (`_MAX_FETCH_BYTES = 50MB`, `_MAX_TEXT_BYTES = 10MB`)
- Custom User-Agent: `Mozilla/5.0 graphify/1.0`
- Non-2xx status raises HTTPError

### Safe Fetch Text: `safe_fetch_text(url, max_bytes, timeout) -> str`
- Wraps `safe_fetch` with UTF-8 decode (replace bad bytes)

### Path Validation: `validate_graph_path(path, base) -> Path`
- **Directory traversal prevention**: resolves path, verifies it stays inside `graphify-out/`
- Requires base directory to exist (prevents reading before graph is built)

### Label Sanitization: `sanitize_label(text) -> str`
- **XSS prevention**: strips control chars (`\x00-\x1f`, `\x7f`), caps at 256 chars, HTML-escapes
- Applied to all labels in HTML output and MCP server responses

---

## 8. manifest.py -- Project Manifest

- Re-export shim: imports `save_manifest`, `load_manifest`, `detect_incremental` from `graphify.detect`
- Provides backwards compatibility for import paths

---

## 9. serve.py -- MCP Server Mode

### Architecture
- stdio-based MCP server using `mcp.server.Server`
- Loads graph from `graphify-out/graph.json` at startup
- 7 tools exposed to Claude/agents

### MCP Tools

#### `query_graph(question, mode=bfs, depth=3, token_budget=2000)`
- **Search**: keyword matching against node labels (1 point) and source_file (0.5 point)
- **Traversal**: BFS or DFS from top-3 matching nodes
- **Output**: text rendering of subgraph, truncated to token budget (3 chars/token)
- Max depth: 6

#### `get_node(label)`
- Case-insensitive label/ID search
- Returns: label, ID, source, type, community, degree

#### `get_neighbors(label, relation_filter)`
- All direct neighbors with optional relation filter
- Shows relation type and confidence per edge

#### `get_community(community_id)`
- All nodes in a community by ID

#### `god_nodes(top_n=10)`
- Delegates to `analyze.god_nodes()`

#### `graph_stats()`
- Node/edge count, community count, confidence percentage breakdown

#### `shortest_path(source, target, max_hops=8)`
- **Algorithm**: `nx.shortest_path(G, src, tgt)` (BFS for unweighted)
- Keyword matching to find source/target nodes
- Renders path as chain of labeled edges

### Helper Functions
- `_load_graph(path)`: loads + validates graph JSON via `validate_graph_path`
- `_communities_from_graph(G)`: reconstructs community dict from node attributes
- `_score_nodes(G, terms)`: keyword scoring for node search
- `_bfs(G, start, depth)`: breadth-first traversal returning (nodes, edges)
- `_dfs(G, start, depth)`: depth-first traversal returning (nodes, edges)
- `_subgraph_to_text(G, nodes, edges, token_budget)`: text renderer with truncation
- `_find_node(G, label)`: case-insensitive node lookup

---

## 10. watch.py -- File Watcher

### Architecture
- Uses `watchdog` library (Observer + FileSystemEventHandler)
- Debounced event handling (default 3s)

### Watched Extensions
- Code: `.py, .ts, .js, .go, .rs, .java, .cpp, .c, .rb, .swift, .kt, .cs, .scala, .php, .cc, .cxx, .hpp, .h, .kts`
- Docs: `.md, .txt, .rst, .pdf`
- Images: `.png, .jpg, .jpeg, .webp, .gif, .svg`

### Behavior
- **Code-only changes**: Runs full pipeline locally (AST extract -> build -> cluster -> report -> JSON export). No LLM needed.
- **Non-code changes**: Writes `graphify-out/needs_update` flag file, prints notification. Requires `/graphify --update` for LLM re-extraction.
- Filters out: hidden directories (`.`), `graphify-out/`, `__pycache__/`

### Functions
- `watch(watch_path, debounce=3.0)`: main loop
- `_rebuild_code(watch_path) -> bool`: full pipeline rebuild for code files
- `_notify_only(watch_path)`: flag file + notification for non-code changes
- `_has_non_code(changed_paths) -> bool`: checks if any changed file is non-code

---

## 11. hooks.py -- Git Hook Integration

### Hooks Installed
1. **post-commit**: Detects changed code files, runs `_rebuild_code()` on code changes
2. **post-checkout**: Rebuilds graph on branch switch (only if `graphify-out/` exists)

### Functions
- `install(path) -> str`: Installs both hooks, appends to existing hooks if present
- `uninstall(path) -> str`: Removes graphify sections using start/end markers, preserves other hook content
- `status(path) -> str`: Reports installation status of both hooks
- `_git_root(path) -> Path`: Walks up to find `.git` directory
- `_install_hook(hooks_dir, name, script, marker) -> str`: Single hook installer
- `_uninstall_hook(hooks_dir, name, marker, marker_end) -> str`: Single hook uninstaller

### Safety
- Idempotent: checks for marker before installing
- Appends to existing hooks rather than overwriting
- Marker-delimited sections for clean removal
- Preserves non-graphify hook content on uninstall

---

## 12. wiki.py -- Wikipedia-Style Wiki Export

### Functions

#### `to_wiki(G, communities, output_dir, community_labels, cohesion, god_nodes_data) -> int`
- Generates 3 types of articles:
  1. **`index.md`**: entry point with community list (sorted by size) and god node list
  2. **`{CommunityName}.md`**: per-community article
  3. **`{GodNodeLabel}.md`**: per-god-node article
- Returns article count (excluding index)

#### `_community_article(G, cid, nodes, label, labels, cohesion) -> str`
- Sections: Key Concepts (top 25 by degree), Relationships (cross-community links, top 12), Source Files (top 20), Audit Trail (confidence breakdown)
- Uses `[[wikilinks]]` for Obsidian compatibility
- Navigation footer linking to `[[index]]`

#### `_god_node_article(G, nid, labels) -> str`
- Sections: metadata line, Community link, Connections by Relation (grouped by relation type, top 20 per relation, all wikilinked)

#### `_index_md(communities, labels, god_nodes_data, total_nodes, total_edges) -> str`
- Header with stats, communities sorted by size, god nodes section

#### Helper: `_cross_community_links(G, nodes, own_cid, labels) -> list[(label, count)]`
- Counts edges to other communities, sorted descending

---

## 13. benchmark.py -- Performance Benchmarking

### Token Reduction Measurement
- Compares: naive full-corpus token count vs graph-query token count
- `_CHARS_PER_TOKEN = 4` (standard approximation)
- Corpus token estimate: `words * 100 / 75`

### Functions

#### `run_benchmark(graph_path, corpus_words, questions) -> dict`
- Loads graph, estimates corpus tokens
- Runs BFS queries for each question (default: 5 sample questions)
- Returns: `{corpus_tokens, corpus_words, nodes, edges, avg_query_tokens, reduction_ratio, per_question}`

#### `_query_subgraph_tokens(G, question, depth=3) -> int`
- Keyword match -> BFS from top 3 nodes -> estimate tokens in subgraph text
- Same BFS logic as serve.py

#### `print_benchmark(result)`
- Human-readable report with reduction ratios per question

### Sample Questions
```
"how does authentication work"
"what is the main entry point"
"how are errors handled"
"what connects the data layer to the api"
"what are the core abstractions"
```

---

## WeftOS Mapping & Priority

### Already Exists in WeftOS (causal.rs / graph infrastructure)

| Graphify Feature | WeftOS Equivalent | Notes |
|---|---|---|
| Leiden/Louvain community detection | `detect_communities()` in causal.rs | Verify algorithm parity; WeftOS may use different impl |
| Degree centrality (god_nodes) | `spectral_analysis()` | Basic degree sort exists |
| Betweenness centrality | `spectral_analysis()` | May need to expose separately |
| Graph construction from nodes/edges | petgraph `Graph` builder | Different API but same concept |
| Cohesion score (edge density) | Partial -- density metrics exist | Need exact ratio metric |
| Graph diff | Not implemented | New feature |
| Schema validation | Serde derive + custom validation | Different approach but covers same ground |
| SHA256 file hashing | `ring` or `sha2` crate | Standard |

### New -- Must Implement for MVP (P0)

| Feature | Rust Crate Deps | Complexity | Notes |
|---|---|---|---|
| Surprise scoring algorithm | None (pure logic) | Medium | 5-factor composite score, file-type detection |
| Question generation | None (pure logic) | Medium | 5 strategies, needs community + centrality data |
| JSON export (node_link_data format) | `serde_json` | Low | Schema must match Python output exactly |
| Neo4j Cypher text export | None (string formatting) | Low | Cypher injection escaping |
| GraphML export | `quick-xml` or `xml-rs` | Low | Or use petgraph's GraphML writer |
| HTML/vis.js visualization | `askama` (templates) | High | 400+ lines of embedded HTML/CSS/JS |
| Label sanitization (XSS) | `html-escape` crate | Low | |
| Path traversal guard | `std::path::Path` | Low | |
| Graph report markdown generation | None (string formatting) | Medium | 10 sections, many conditional blocks |
| Cache system (SHA256 + JSON files) | `sha2`, `serde_json` | Medium | Atomic writes, per-file keying |
| Extraction schema validation | `serde`, custom validators | Low | Enum validation + referential integrity |

### New -- Nice-to-Have for MVP (P1)

| Feature | Rust Crate Deps | Complexity | Notes |
|---|---|---|---|
| SVG export | `svg` crate or `plotters` | High | Spring layout + matplotlib-style rendering |
| Obsidian vault export | None (file I/O) | Medium | YAML frontmatter + wikilinks + Dataview queries |
| Obsidian canvas export | `serde_json` | Medium | Grid layout algorithm |
| Wiki article generation | None (string formatting) | Medium | Community + god node articles |
| MCP stdio server | `mcp-rs` or custom | High | 7 tools, BFS/DFS traversal |
| File watcher | `notify` crate | Medium | Debounced, code-vs-doc discrimination |
| Git hooks | Shell script generation | Low | Template-based, marker-delimited |
| Neo4j direct push | `neo4rs` crate | Medium | Driver-based, MERGE upsert |
| Token reduction benchmark | None (pure logic) | Low | BFS + token estimation |

### New -- Post-MVP (P2)

| Feature | Rust Crate Deps | Complexity | Notes |
|---|---|---|---|
| SSRF-safe HTTP fetch | `reqwest` + IP validation | Medium | Cloud metadata blocking, redirect validation |
| Hyperedge rendering in HTML | Embedded JS | Medium | Convex hull drawing on canvas |
| Dataview query blocks in Obsidian | N/A | Low | Just markdown template strings |
| `.obsidian/graph.json` color config | `serde_json` | Low | |

### Rust Crate Dependencies Summary

| Crate | Purpose | Required For |
|---|---|---|
| `petgraph` | Graph data structure | Core (already in WeftOS) |
| `serde` + `serde_json` | JSON serialization | JSON/Canvas export, cache, validation |
| `sha2` | SHA256 hashing | Cache system |
| `html-escape` | XSS prevention | HTML export, MCP server |
| `quick-xml` or `xml-rs` | XML generation | GraphML export |
| `askama` | Template engine | HTML visualization |
| `notify` | File system events | File watcher |
| `neo4rs` | Neo4j async driver | Neo4j push (P1) |
| `reqwest` | HTTP client | Safe fetch (P2) |
| `plotters` or `svg` | SVG rendering | SVG export (P1) |

---

## Test Coverage Analysis

### Test Files (25 total in /tmp/graphify-source/tests/)

| Test File | Module | Tests | Coverage Focus |
|---|---|---|---|
| `test_analyze.py` | analyze.py | 15 | god_nodes return shape/sorting/keys; surprising_connections multi-file, single-file, concept exclusion, scoring comparisons, why field, required keys; graph_diff add/remove nodes/edges, empty diff; file_category; is_concept_node |
| `test_cluster.py` | cluster.py | 7 | cluster returns dict, covers all nodes; cohesion_score for complete/single/disconnected/range graphs; score_all key matching |
| `test_export.py` | export.py | 11 | to_json file creation, valid JSON, community attribute; to_cypher file creation, MERGE statements; to_graphml file creation, valid XML, community attribute; to_html file creation, vis.js, search, legend, nodes/edges data |
| `test_cache.py` | cache.py | 6 | file_hash consistency/change detection; cache roundtrip; cache miss on content change; cached_files set; clear_cache |
| `test_validate.py` | validate.py | 10 | valid passes; missing nodes/edges keys; not-a-dict; invalid file_type; invalid confidence; dangling edge source/target; missing node field; assert_valid raises/passes |
| `test_report.py` | report.py | 8 | Contains header, corpus check, god nodes, surprising connections, communities, ambiguous section, token cost, raw cohesion scores |
| `test_security.py` | security.py | 13 | validate_url accepts http/https, rejects file/ftp/data/empty; safe_fetch rejects bad URLs, returns bytes, raises on 4xx, raises on size exceeded; safe_fetch_text UTF-8 decode + bad bytes; validate_graph_path inside/traversal/missing-base/missing-file; sanitize_label HTML escape, control chars, length cap, safe passthrough |
| `test_serve.py` | serve.py | 15 | communities_from_graph basic/no-attr/isolated; score_nodes exact/no-match/partial; bfs depth-1/2/disconnected/edges; dfs depth-1/full-chain; subgraph_to_text labels/truncation/edges; load_graph roundtrip/missing |
| `test_watch.py` | watch.py | 7 | notify_only creates flag/dir/idempotent; watched extensions include code/docs/images, exclude noise; watch raises without watchdog |
| `test_hooks.py` | hooks.py | 12 | install creates hook/executable/idempotent/appends; uninstall removes/no-hook; status installed/not-installed; no-git-repo raises; post-checkout install/executable/uninstall; status shows both hooks |
| `test_wiki.py` | wiki.py | 13 | to_wiki writes index, returns count, creates community/god-node articles; index links communities/god-nodes; community article cross-links/cohesion/audit-trail; god node connections/community-link; skips missing IDs; fallback labels; navigation footer; truncation notice |
| `test_benchmark.py` | benchmark.py | 9 | query returns positive/zero/expands-with-depth; run_benchmark reduction/proportional/per-question/estimates/empty-error/node-edge-counts; print_benchmark no-crash/error-message |
| `test_detect.py` | detect.py | -- | File detection (not in scope of this analysis) |
| `test_pipeline.py` | pipeline | -- | End-to-end (not in scope) |
| `test_install.py` | install | -- | Package install (not in scope) |
| `test_multilang.py` | multi-lang | -- | Multi-language (not in scope) |
| `test_confidence.py` | confidence | -- | Confidence scoring (not in scope) |
| `test_semantic_similarity.py` | similarity | -- | Embedding-based (not in scope) |
| `test_extract.py` | extract | -- | Extraction (not in scope) |
| `test_hypergraph.py` | hypergraph | -- | Hyperedge (not in scope) |
| `test_build.py` | build | -- | Graph build (not in scope) |
| `test_ingest.py` | ingest | -- | File ingestion (not in scope) |
| `test_languages.py` | languages | -- | Language support (not in scope) |
| `test_claude_md.py` | claude_md | -- | CLAUDE.md (not in scope) |
| `test_rationale.py` | rationale | -- | Rationale (not in scope) |

### Testing Patterns Observed
- **Fixtures**: `tests/fixtures/extraction.json` used as canonical test graph
- **pytest fixtures**: `tmp_path`, `tmp_file`, `cache_root`, `monkeypatch`
- **Mocking**: `unittest.mock.patch` for network calls, `MagicMock` for HTTP responses, `monkeypatch` for import blocking
- **Edge cases tested**: empty graphs, single-node graphs, disconnected graphs, missing files, invalid JSON, size limits, idempotent operations, path traversal, XSS payloads, control characters
- **Property-based patterns**: score ordering tests (AMBIGUOUS > EXTRACTED, cross-type > same-type), proportionality tests (corpus_tokens scales linearly)

### Rust Test Plan Implications
- Need `fixtures/extraction.json` equivalent as test data
- Use `tempfile::TempDir` for `tmp_path` equivalent
- Use `mockall` or `wiremock` for network mocking
- Community detection tests need deterministic seeding (already have `seed=42` in Louvain)
- Export tests should verify file creation + content structure
- Security tests need SSRF test vectors (private IPs, redirects, scheme attacks)
- Benchmark tests should verify reduction ratio > 1.0 as sanity check
