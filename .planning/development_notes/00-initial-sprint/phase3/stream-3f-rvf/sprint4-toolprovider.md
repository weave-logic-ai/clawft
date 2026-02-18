# Sprint 4: RvfToolProvider + MCP Registration

## Deliverables

**File created**: `crates/clawft-services/src/rvf_tools.rs`

**Module registered in**: `crates/clawft-services/src/lib.rs` (feature-gated: `rvf`)

**Feature added to**: `crates/clawft-services/Cargo.toml` (`rvf = []`)

## Types Introduced

| Type | Purpose |
|------|---------|
| `StubVectorStore` | In-memory stub vector store for development/testing |
| `StubEntry` | A single vector entry with id, embedding, metadata |
| `RvfToolProvider` | `ToolProvider` implementation exposing 11 RVF tools via MCP |

## Tools Exposed (11)

| Tool | Description |
|------|-------------|
| `create_store` | Create a new vector store with given dimension |
| `open_store` | Open/check an existing vector store |
| `ingest` | Add a vector entry with optional metadata |
| `query` | Search by embedding vector (brute-force cosine similarity) |
| `status` | Get store metrics (entries, dimension, backend) |
| `delete` | Remove a vector by ID |
| `compact` | Reclaim space (no-op in stub) |
| `delete_filter` | Delete vectors matching a metadata key-value filter |
| `list_stores` | List all open stores with entry counts |
| `route` | Route a task to optimal model tier by complexity |
| `witness` | Record an audit trail entry with timestamp and sequence |

## Design Decisions

1. **Feature-gated**: The entire module is behind `#[cfg(feature = "rvf")]` so it adds zero overhead when not enabled. No new dependencies are required -- it uses `chrono` which is already a workspace dependency.

2. **Stub backends**: All vector operations use in-memory `HashMap<String, StubVectorStore>` and brute-force cosine similarity. This matches the same argument parsing and validation that the real RVF backend will require. Replacing the stub with real `rvf-runtime` calls is a drop-in swap per handler.

3. **Mutex-based thread safety**: `RvfToolProvider` uses `Mutex<HashMap<String, StubVectorStore>>` rather than `Arc<Mutex<...>>` since the `ToolProvider` trait requires `Send + Sync` and the provider itself is typically held behind an `Arc` by the composite provider or server shell.

4. **Handler returns, not async**: Each handler is synchronous (returns `Result<CallToolResult, ToolError>`) since the stub operations are all in-memory. The `async fn call_tool` dispatches to these synchronous handlers. When real RVF I/O is wired, the handlers can be made async.

5. **Overwrite semantics**: `ingest` with a duplicate ID overwrites the existing entry (matching typical vector store behavior).

6. **Argument validation**: Every handler uses `require_str`, `require_u64`, `require_f32_array` helpers that return `ToolError::ExecutionFailed` with descriptive messages on missing or malformed arguments.

7. **Route tool**: Uses a stub 3-tier routing model based on complexity thresholds (< 0.3 = haiku, < 0.7 = sonnet, >= 0.7 = opus). Includes a keyword-based auto-complexity estimator when `complexity_hint` is omitted.

8. **Witness tool**: Maintains an append-only `Vec<Value>` log with monotonically increasing sequence numbers and RFC 3339 timestamps. This will be replaced by the WITNESS segment hash chain when `rvf-crypto` is integrated.

## Registration Wiring

In `McpServerShell` or any service that builds a `CompositeToolProvider`:

```rust
#[cfg(feature = "rvf")]
{
    let rvf_provider = clawft_services::rvf_tools::RvfToolProvider::new();
    composite.register(Box::new(rvf_provider));
}
```

Tools appear as `rvf__create_store`, `rvf__query`, etc. via the composite namespace prefixing.

## Tests (33 total)

### Trait tests
- `namespace_returns_rvf` -- namespace is "rvf"
- `list_tools_returns_11_tools` -- all 11 tools listed
- `unknown_tool_returns_not_found` -- unknown name returns ToolError::NotFound
- `debug_format` -- Debug impl shows store count

### create_store
- `create_store_succeeds` -- happy path
- `create_store_duplicate_returns_error` -- duplicate name rejected
- `create_store_zero_dimension_returns_error` -- zero dimension rejected
- `create_store_missing_name_returns_error` -- missing required arg

### ingest + query
- `ingest_and_query_round_trip` -- create, ingest 2, query returns correct order
- `ingest_dimension_mismatch_returns_error` -- wrong dimension rejected
- `ingest_nonexistent_store_returns_error` -- missing store
- `ingest_overwrites_same_id` -- duplicate ID replaced, count stays at 1

### delete
- `delete_removes_entry` -- entry removed, count decremented
- `delete_nonexistent_entry_returns_false` -- deleting ghost returns deleted=false

### delete_filter
- `delete_filter_removes_matching` -- metadata filter deletes matching entries

### status
- `status_returns_metrics` -- returns entries, dimension, backend info

### compact
- `compact_succeeds` -- no-op compact returns success

### list_stores
- `list_stores_empty` -- empty provider returns count=0
- `list_stores_after_create` -- two stores listed after creation

### open_store
- `open_store_found` -- existing store returns open status
- `open_store_not_found` -- missing store returns error

### route
- `route_simple_task_routes_to_haiku` -- low complexity -> tier 1
- `route_complex_task_routes_to_opus` -- high complexity -> tier 3
- `route_auto_complexity_estimation` -- auto-estimates from description

### witness
- `witness_records_entry` -- records with timestamp and sequence=1
- `witness_increments_sequence` -- second entry gets sequence=2

### Composite integration
- `composite_routes_rvf_query_correctly` -- full round-trip through CompositeToolProvider with rvf__ prefix

### Helper unit tests
- `cosine_similarity_identical` -- identical vectors -> 1.0
- `cosine_similarity_orthogonal` -- orthogonal vectors -> 0.0
- `cosine_similarity_zero_vector` -- zero vector -> 0.0
- `estimate_complexity_short_simple` -- short description -> low
- `estimate_complexity_with_keyword` -- "refactor" keyword -> high

## Verification

```
cargo test -p clawft-services --features rvf   # 176 passed (33 new)
cargo clippy -p clawft-services --features rvf -- -D warnings  # 0 warnings
cargo test -p clawft-services                   # existing tests still pass (no rvf)
```
