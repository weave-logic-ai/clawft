# Graphify Extraction, Build, Detect & Ingest Modules -- Complete Function Map

Source: `/tmp/graphify-source/graphify/` (Python)
Target: WeftOS Rust port (`crates/clawft-graphify/`)
Date: 2026-04-04

---

## 1. extract.py -- Deterministic Structural Extraction (1,734 lines)

The core module. Implements a generic, config-driven AST extraction framework using tree-sitter, supporting 15 languages through a shared `LanguageConfig` dataclass plus custom extractors for Go and Rust.

### Architecture: Two-Pass Extraction

**Pass 1 -- Structural Walk (per-file):**
Walks the tree-sitter AST top-down. For each node, dispatches based on type:
- Import nodes -> language-specific import handler
- Class nodes -> create entity + "contains" edge from file, recurse into body
- Function nodes -> create entity + "contains"/"method" edge, record body for Pass 2
- Language-specific extras (JS arrow functions, C# namespaces, Swift enum cases)
- Default -> recurse into children

**Pass 2 -- Call Graph Inference (per-file):**
After Pass 1 completes, builds a `label_to_nid` lookup map (normalized label -> node ID). Then walks each recorded function body looking for call expressions. When a callee name resolves to a known node, emits an INFERRED "calls" edge (weight 0.8). Deduplicates by (caller, callee) pair.

**Pass 3 -- Cross-File Import Resolution (Python only, multi-file):**
After all files are extracted, `_resolve_cross_file_imports()` performs:
1. Global map: `stem -> {ClassName: node_id}` across all files
2. For each file with `from .module import Name`, resolves Name to the target node ID
3. Emits INFERRED "uses" edges from each local class to each resolved imported entity

### Core Data Structures

#### `LanguageConfig` (dataclass)

| Field | Type | Purpose |
|-------|------|---------|
| `ts_module` | `str` | Python import path for tree-sitter grammar (e.g., `"tree_sitter_python"`) |
| `ts_language_fn` | `str` | Attribute name to call for language instance (default: `"language"`) |
| `class_types` | `frozenset` | AST node types that represent class/struct/interface declarations |
| `function_types` | `frozenset` | AST node types that represent function/method declarations |
| `import_types` | `frozenset` | AST node types that represent import/use statements |
| `call_types` | `frozenset` | AST node types that represent call expressions |
| `name_field` | `str` | Field name to extract entity name (default: `"name"`) |
| `name_fallback_child_types` | `tuple` | Child node types to try if `name_field` is absent |
| `body_field` | `str` | Field name for body block (default: `"body"`) |
| `body_fallback_child_types` | `tuple` | Child types to try for body (e.g., `"declaration_list"`) |
| `call_function_field` | `str` | Field on call node for the callee (default: `"function"`) |
| `call_accessor_node_types` | `frozenset` | Member/attribute accessor node types |
| `call_accessor_field` | `str` | Field on accessor for method name (default: `"attribute"`) |
| `function_boundary_types` | `frozenset` | Stop recursion in `walk_calls` at these types |
| `import_handler` | `Callable or None` | Language-specific import edge generator |
| `resolve_function_name_fn` | `Callable or None` | Custom name resolver for C/C++ declarator unwrapping |
| `function_label_parens` | `bool` | If True, function labels get `"name()"` format |
| `extra_walk_fn` | `Callable or None` | Extra dispatch hook (JS arrow functions, C# namespaces) |

**Rust mapping:** This becomes a `LanguageConfig` struct in `extract/ast.rs`. The `frozenset` fields become `&'static [&'static str]` or `HashSet<&'static str>`. The callback fields become trait objects or enum dispatch.

### Helper Functions

#### `_make_id(*parts: str) -> str`
- Concatenates parts with `_`, strips leading/trailing `_.`, replaces non-alphanumeric with `_`, lowercases
- Produces stable, deterministic node IDs
- **Rust mapping:** `fn make_id(parts: &[&str]) -> String` -- same algorithm. NOTE: The architecture doc specifies BLAKE3 hashing for EntityId. The Python uses simple string concatenation. The Rust port must support BOTH: the Python-compatible string ID (for JSON export compatibility) and the BLAKE3 EntityId (for CausalGraph integration). Store both on Entity.

#### `_read_text(node, source: bytes) -> str`
- Extracts UTF-8 text from source bytes using node byte range
- **Rust mapping:** `fn read_text(node: &Node, source: &[u8]) -> String` using `std::str::from_utf8_lossy`

#### `_resolve_name(node, source, config) -> str | None`
- Gets name from `config.name_field`, falls back to `config.name_fallback_child_types`
- Short-circuits if `config.resolve_function_name_fn` is set (C/C++)

#### `_find_body(node, config) -> Node | None`
- Gets body from `config.body_field`, falls back to `config.body_fallback_child_types`

### Import Handlers (one per language family)

Each handler signature: `(node, source, file_nid, stem, edges, str_path) -> None`

All handlers follow the same pattern: walk child nodes to find the module/path string, extract the final component, create a `_make_id()` target, emit an `imports` or `imports_from` edge with `confidence: "EXTRACTED"`, `weight: 1.0`.

| Handler | Language(s) | Import Pattern | Target Extraction |
|---------|-------------|----------------|-------------------|
| `_import_python` | Python | `import X` / `from X import Y` | `dotted_name` or `module_name` field, strip leading dots |
| `_import_js` | JS, TS, TSX | `import ... from "path"` | String child, strip `./`, take last path segment |
| `_import_java` | Java | `import com.foo.Bar` | `scoped_identifier` recursive walk, take last segment |
| `_import_c` | C, C++, H, HPP | `#include <header.h>` / `#include "header.h"` | `string_literal` or `system_lib_string`, strip path and extension |
| `_import_csharp` | C# | `using Namespace.Type` | `qualified_name` or `identifier`, take last dot-segment |
| `_import_kotlin` | Kotlin | `import com.foo.Bar` | `path` field, take last dot-segment; fallback to `identifier` child |
| `_import_scala` | Scala | `import com.foo.Bar` | `stable_id` or `identifier`, take last dot-segment, skip `_` wildcard |
| `_import_php` | PHP | `use Namespace\Type` | `qualified_name` or `name`, take last backslash-segment |
| `_import_lua` | Lua | `require('module')` | Regex on full node text: `require\s*['"]module['"]`, take last dot-segment |
| `_import_swift` | Swift | `import Module` | First `identifier` child |

### C/C++ Function Name Helpers

#### `_get_c_func_name(node, source) -> str | None`
- Recursively unwraps `declarator` field to find innermost `identifier`
- Handles nested declarators like `int (*func_ptr)(int)`

#### `_get_cpp_func_name(node, source) -> str | None`
- Same as C but also handles `qualified_identifier` (e.g., `MyClass::method`)
- Extracts `name` field from `qualified_identifier`

### Extra Walk Hooks

#### `_js_extra_walk(...) -> bool`
- Handles `lexical_declaration` nodes containing `variable_declarator` with `arrow_function` value
- Extracts arrow function name from declarator `name` field
- Creates function entity + "contains" edge + records body for call graph pass

#### `_csharp_extra_walk(...) -> bool`
- Handles `namespace_declaration` nodes
- Creates namespace entity + "contains" edge from file
- Recurses into namespace body with `walk_fn`

#### `_swift_extra_walk(...) -> bool`
- Handles `enum_entry` nodes within a parent class
- Creates case entity + "case_of" edge

### Language Configurations (15 languages)

| Config | Languages | Extensions | Class Types | Function Types | Import Types | Call Types | Special Handling |
|--------|-----------|------------|-------------|----------------|--------------|------------|------------------|
| `_PYTHON_CONFIG` | Python | `.py` | `class_definition` | `function_definition` | `import_statement`, `import_from_statement` | `call` | Inheritance via `superclasses`, rationale extraction post-pass |
| `_JS_CONFIG` | JavaScript | `.js` | `class_declaration` | `function_declaration`, `method_definition` | `import_statement` | `call_expression` | Arrow function extra walk |
| `_TS_CONFIG` | TypeScript | `.ts`, `.tsx` | `class_declaration` | `function_declaration`, `method_definition` | `import_statement` | `call_expression` | Arrow function extra walk, `language_typescript` function name |
| `_JAVA_CONFIG` | Java | `.java` | `class_declaration`, `interface_declaration` | `method_declaration`, `constructor_declaration` | `import_declaration` | `method_invocation` | Java-specific import handler (scoped_identifier walk) |
| `_C_CONFIG` | C | `.c`, `.h` | (none) | `function_definition` | `preproc_include` | `call_expression` | `_get_c_func_name` declarator unwrapping |
| `_CPP_CONFIG` | C++ | `.cpp`, `.cc`, `.cxx`, `.hpp` | `class_specifier` | `function_definition` | `preproc_include` | `call_expression` | `_get_cpp_func_name` declarator unwrapping |
| `_RUBY_CONFIG` | Ruby | `.rb` | `class` | `method`, `singleton_method` | (none) | `call` | `body_statement` body fallback, `constant`/`scope_resolution` name fallback |
| `_CSHARP_CONFIG` | C# | `.cs` | `class_declaration`, `interface_declaration` | `method_declaration` | `using_directive` | `invocation_expression` | Namespace extra walk, `declaration_list` body fallback |
| `_KOTLIN_CONFIG` | Kotlin | `.kt`, `.kts` | `class_declaration`, `object_declaration` | `function_declaration` | `import_header` | `call_expression` | `simple_identifier` name fallback, `function_body`/`class_body` body fallback |
| `_SCALA_CONFIG` | Scala | `.scala` | `class_definition`, `object_definition` | `function_definition` | `import_declaration` | `call_expression` | `identifier` name fallback, `template_body` body fallback |
| `_PHP_CONFIG` | PHP | `.php` | `class_declaration` | `function_definition`, `method_declaration` | `namespace_use_clause` | `function_call_expression`, `member_call_expression` | `language_php` function name, dual call types |
| `_LUA_CONFIG` | Lua | `.lua`, `.toc` | (none) | `function_declaration` | `variable_declaration` | `function_call` | Regex-based `require()` import extraction |
| `_SWIFT_CONFIG` | Swift | `.swift` | `class_declaration`, `protocol_declaration` | `function_declaration`, `init_declaration`, `deinit_declaration`, `subscript_declaration` | `import_declaration` | `call_expression` | Enum case extra walk, conformance/inheritance via `inheritance_specifier` |
| (Go) | Go | `.go` | (custom walk) | (custom walk) | (custom walk) | (custom walk) | Fully custom extractor -- see `extract_go` |
| (Rust) | Rust | `.rs` | (custom walk) | (custom walk) | (custom walk) | (custom walk) | Fully custom extractor -- see `extract_rust` |

### Generic Extractor: `_extract_generic(path, config) -> dict`

The main extraction engine. Steps:
1. Dynamically import the tree-sitter grammar module (`config.ts_module`)
2. Create `Language` and `Parser` instances
3. Parse source file into AST
4. Create file-level node (`_make_id(stem)`, label = filename)
5. Define `walk(node, parent_class_nid)` recursive walker dispatching on node type
6. Execute `walk(root)`
7. **Call graph pass:** Build `label_to_nid` map, walk each recorded function body with `walk_calls`. Emit INFERRED "calls" edges.
8. **Clean edges:** Filter out edges where source is not in `seen_ids` (but allow import targets to external modules)
9. Return `{nodes, edges}`

**Rust mapping:** `pub fn extract_ast(path: &Path, lang: LanguageId, config: &LanguageConfig) -> ExtractionResult` in `extract/ast.rs`. The walk logic becomes iterative (stack-based) rather than recursive to avoid stack overflow on deeply nested ASTs.

### Python Rationale Extraction: `_extract_python_rationale(path, result) -> None`

Post-pass that mutates the extraction result in-place. Extracts:
1. **Module-level docstrings:** First string expression in root body, if > 20 chars
2. **Class/function docstrings:** First string expression in class/function body
3. **Rationale comments:** Lines matching prefixes: `# NOTE:`, `# IMPORTANT:`, `# HACK:`, `# WHY:`, `# RATIONALE:`, `# TODO:`, `# FIXME:`

Creates `rationale` file_type nodes with `rationale_for` edges to the parent entity.

### Custom Extractors

#### `extract_go(path) -> dict`
- Custom walker (not using `_extract_generic`)
- Handles: `function_declaration`, `method_declaration` (with receiver type), `type_declaration` (struct/enum), `import_declaration` (spec list + single spec)
- Go method declarations extract receiver type from `parameter_declaration.type`, stripping leading `*`
- Import paths: strip quotes, take last `/` segment
- Call graph: handles `identifier` and `selector_expression.field` callee resolution

#### `extract_rust(path) -> dict`
- Custom walker (not using `_extract_generic`)
- Handles: `function_item`, `struct_item`, `enum_item`, `trait_item`, `impl_item`, `use_declaration`
- `impl_item`: extracts type name, recurses into body with `parent_impl_nid` context
- `use_declaration`: parses `argument` field, splits on `{`, strips `::*`, takes last `::` segment
- Call graph: handles `identifier`, `field_expression.field`, `scoped_identifier.name`

### Top-Level Entry Points

#### `extract(paths: list[Path]) -> dict`
- Iterates paths, dispatches to per-language extractor by extension
- Checks cache (`load_cached`) before extraction; saves to cache on success
- Aggregates all nodes and edges across files
- Runs `_resolve_cross_file_imports` for Python files
- Extension dispatch table (22 entries): `.py`, `.js`, `.ts`, `.tsx`, `.go`, `.rs`, `.java`, `.c`, `.h`, `.cpp`, `.cc`, `.cxx`, `.hpp`, `.rb`, `.cs`, `.kt`, `.kts`, `.scala`, `.php`, `.swift`, `.lua`, `.toc`

#### `collect_files(target: Path) -> list[Path]`
- If target is a file, returns `[target]`
- If directory, globs for all 22 supported extensions (sorted)
- Filters out hidden directories (any path part starting with `.`)

### Entity Types Extracted

| Entity Type | Label Format | Source |
|-------------|-------------|--------|
| File | `filename.ext` | Always created as first node |
| Class/Struct/Interface/Protocol | `ClassName` | class_types AST nodes |
| Function (top-level) | `func_name()` | function_types outside classes |
| Method (class member) | `.method_name()` | function_types inside classes |
| Type (Go) | `TypeName` | `type_declaration` -> `type_spec` |
| Struct/Enum/Trait (Rust) | `ItemName` | `struct_item`, `enum_item`, `trait_item` |
| Namespace (C#) | `NamespaceName` | `namespace_declaration` |
| Enum Case (Swift) | `CaseName` | `enum_entry` |
| Rationale (Python) | First 80 chars of comment | Docstrings and `# NOTE:` comments |
| Base Class (phantom) | `BaseName` | Inherited classes not in current file |

### Edge Types Extracted

| Relation | Confidence | Weight | Source |
|----------|-----------|--------|--------|
| `contains` | EXTRACTED | 1.0 | File -> class/function |
| `method` | EXTRACTED | 1.0 | Class/struct -> method |
| `imports` | EXTRACTED | 1.0 | File -> imported module |
| `imports_from` | EXTRACTED | 1.0 | File -> module in `from X import Y` |
| `inherits` | EXTRACTED | 1.0 | Class -> base class (Python, Swift) |
| `case_of` | EXTRACTED | 1.0 | Enum -> case (Swift) |
| `calls` | INFERRED | 0.8 | Function -> function (call graph pass) |
| `uses` | INFERRED | 0.8 | Class -> imported class (cross-file, Python) |
| `rationale_for` | EXTRACTED | 1.0 | Rationale -> parent entity (Python) |

### Confidence Assignment Logic

- **EXTRACTED (weight 1.0):** Edges derived directly from AST structure
- **INFERRED (weight 0.8):** Edges derived from heuristic matching -- call graph and cross-file resolution
- No AMBIGUOUS edges from AST extractor. AMBIGUOUS is reserved for the semantic (LLM) extractor.

### Node ID Generation: `_make_id(*parts)`

Algorithm: join with `_`, strip `_.`, replace non-alphanumeric with `_`, strip `_`, lowercase.

The Rust port must maintain this scheme for backward compatibility. The BLAKE3 `EntityId` is a SEPARATE identifier for CausalGraph integration.

---

## 2. build.py -- Graph Assembly (71 lines)

### Node Deduplication Strategy

Three layers:
1. **Within a file (AST):** `seen_ids` set prevents duplicates
2. **Between files (build):** NetworkX `add_node()` idempotent, last attributes win
3. **Semantic merge (skill):** explicit `seen` set before `build()`

### Functions

#### `build_from_json(extraction: dict) -> nx.Graph`
- Validates via `validate_extraction()`, filters expected dangling-edge warnings
- Creates undirected `nx.Graph`, adds nodes (id as key), adds edges (skip missing src/tgt)
- Preserves direction as `_src`/`_tgt` attributes
- Stores hyperedges in `G.graph["hyperedges"]`

#### `build(extractions: list[dict]) -> nx.Graph`
- Concatenates nodes, edges, hyperedges; accumulates tokens
- Delegates to `build_from_json(combined)`
- Last extraction's attributes win for duplicate IDs

---

## 3. detect.py -- File Discovery & Classification (280 lines)

### Functions

#### `classify_file(path) -> FileType | None`
Extension-based. For `.md`/`.txt`/`.rst`: checks `_looks_like_paper()` (13 regex signals, threshold 3).

#### `detect(root) -> dict`
Walks directory, prunes 37+ noise dirs, classifies files, counts words, skips sensitive files. Returns `{files, total_files, total_words, needs_graph, warning, skipped_sensitive}`.

#### `detect_incremental(root, manifest_path) -> dict`
Compares mtime against manifest. Returns `{new_files, unchanged_files, deleted_files, new_total}`.

#### `save_manifest(files, manifest_path)` / `load_manifest(manifest_path)`
JSON serialization of `{path: mtime}` for incremental detection.

### Constants
- 22 CODE_EXTENSIONS, 3 DOC_EXTENSIONS, 1 PAPER_EXTENSION, 6 IMAGE_EXTENSIONS
- 6 sensitive file regex patterns
- 13 paper signal regexes (threshold 3)
- Corpus warnings: 50K words (too small), 500K words / 200 files (too large)

---

## 4. ingest.py -- URL Fetching & Query Storage (292 lines)

### Functions

#### `ingest(url, target_dir, author, contributor) -> Path`
Validates URL, dispatches by type (tweet/arxiv/pdf/image/webpage), saves as markdown with YAML frontmatter. Filename deduplication via counter.

#### `save_query_result(question, answer, memory_dir, ...) -> Path`
Saves Q&A as markdown in `graphify-out/memory/`. Feedback loop: detected on next `--update`.

#### URL type detection
`tweet` (twitter/x.com), `arxiv`, `github`, `youtube`, `pdf` (by extension), `image` (by extension), `webpage` (default).

#### Fetchers
- Tweet: oEmbed API, HTML strip, markdown frontmatter
- Webpage: fetch HTML, extract title, html2text conversion, 12K char cap
- arXiv: regex for ID, abstract page fetch, extract title/abstract/authors
- Binary: direct download for PDF/image

---

## Rust Type Mapping Summary

| Python Type | Rust Type | Location |
|-------------|-----------|----------|
| Node dict | `Entity` struct | `model.rs` |
| Edge dict | `Relationship` struct | `model.rs` |
| `LanguageConfig` dataclass | `LanguageConfig` struct | `extract/ast.rs` |
| `FileType` enum | `FileType` enum | `model.rs` |
| Confidence string | `Confidence` enum | `relationship.rs` |
| Relation string | `RelationType` enum | `relationship.rs` |
| `_make_id()` string ID | `String` (legacy) + `EntityId` (BLAKE3) | `entity.rs` |
| `nx.Graph` | `KnowledgeGraph` (petgraph) | `model.rs` |
| Detection result dict | `DetectionResult` struct | `extract/detect.rs` |
| Extraction result dict | `ExtractionResult` struct | `extract/mod.rs` |
