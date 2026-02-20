# Architecture Review: SPARC Elements 07, 08, 09 (Iteration 1)

**Reviewer**: SPARC Architecture Agent
**Date**: 2026-02-19
**Scope**: Elements 07 (Dev Tools & Apps), 08 (Memory & Workspace), 09 (Multi-Agent Routing)
**Input Documents**: `07-dev-tools-apps/00-orchestrator.md`, `08-memory-workspace/00-orchestrator.md`, `09-multi-agent-routing/00-orchestrator.md`, `02-improvements-overview/00-orchestrator.md`, draft requirements (`01-biz-features-deploy.md`, `02-tech-pipeline.md`, `03-dev-phases.md`, `03-dev-contracts.md`), `improvements.md`

---

## 1. Element 07: Dev Tools & Applications (Workstream F)

### 1.1 Architectural Assessment: 6/10

**Strengths**:
- Clean decomposition into independent plugin crates per tool (git, cargo, tree-sitter, browser, calendar, oauth2, docker). Each can compile and test in isolation.
- All tools implement the `Tool` plugin trait from C1, enforcing a consistent interface. This means new tools do not touch `clawft-core`.
- OAuth2 helper (F6) factored as a shared plugin consumed by calendar (F5), email (E2), and others. Good reuse pattern.
- Feature-gating via Cargo features keeps the base binary small (target: <10 MB).

**Weaknesses and Gaps**:

1. **Crate structure not formally specified.** The business requirements reference individual plugin crate names (`clawft-plugin-git`, `clawft-plugin-cargo`, etc.) but the orchestrator document does not specify whether these are:
   - Separate workspace member crates (one per tool), or
   - Modules within a single `clawft-plugins` crate, or
   - External crates outside the workspace.
   This matters for CI build times, dependency graphs, and release versioning. **Recommendation**: Separate workspace crates (`crates/clawft-plugin-git`, etc.) gated behind feature flags in the CLI crate. This keeps compilation parallel and avoids monolithic plugin crates.

2. **F9 (MCP Client) depends on D9 (MCP transport concurrency), but M4 (Dynamic MCP Discovery) depends on F9.** The orchestrator lists M4 at Week 5 and F9 at Week 10. This is the **M4/F9 timeline conflict** flagged in the known issues list. M4 cannot function without F9's client-side MCP connection capability. **Recommendation**: Split F9 into two sub-deliverables:
   - **F9a (Week 5-6)**: Core MCP client library -- connect to a single external MCP server, list tools, invoke tools. Lives in `clawft-services/src/mcp/client.rs`.
   - **F9b (Week 9-10)**: Full MCP client features -- auto-discovery, connection pooling, schema caching, health checks. This is the "1000+ servers" story.
   M4 depends on F9a only. The F9b work can proceed later without blocking Claude Flow integration.

3. **No tool permission model specified.** The `Tool` trait includes `permissions() -> ToolPermissions`, but the orchestrator does not describe what `ToolPermissions` contains for dev tools. Git can write to arbitrary paths. Browser CDP can access any URL. Docker can execute arbitrary containers. The security surface is large. **Recommendation**: Each dev tool plugin must declare its permission requirements in the plugin manifest (`permissions.filesystem`, `permissions.network`, `permissions.shell`). The sandbox (K3) enforces these at runtime. This must be specified before implementation begins, not retrofitted.

4. **Browser CDP sandboxing architecture missing.** F4 uses `chromiumoxide` to control headless Chrome. The risk register identifies "Browser CDP plugin security exposure" as Medium/High, but the architectural response (separate process, domain allowlist) is not formalized as a contract. **Recommendation**: F4 must define a `BrowserSandboxConfig` struct specifying: allowed domains, max concurrent pages, max memory, session lifetime, cookie/storage policy. This should be part of the C1 plugin manifest extension.

5. **tree-sitter language grammar loading not addressed.** F3 depends on tree-sitter language grammars, which are compiled C shared objects. This creates a native dependency problem for WASM builds and cross-compilation. **Recommendation**: F3 should be native-only (no WASM variant) with explicit documentation that AST analysis requires the native build. Language grammars should be bundled as optional features (`tree-sitter-rust`, `tree-sitter-typescript`, etc.) to avoid pulling in all languages.

6. **MCP IDE integration (F8) depends on C6 (MCP Skill Exposure) but the interface contract between them is not defined.** How does C6 expose dynamically loaded skills to the MCP server? Is it a tool schema registry that F8 reads, or an event-driven push? **Recommendation**: Define a `ToolSchemaRegistry` trait in `clawft-plugin` that the MCP server queries. C6 populates it; F8 consumes it.

### 1.2 Data Flow Assessment

```
User -> Channel -> Bus -> AgentLoop -> Router -> ToolRegistry -> Tool Plugin (F1-F7)
                                                                    |
                                                              ToolPermissions check
                                                                    |
                                                              Plugin executes (git2/CDP/etc)
                                                                    |
                                                              Result -> AgentLoop -> Bus -> Channel -> User
```

The data flow is clean for individual tool calls. The gap is the **MCP client data flow** (F9):

```
AgentLoop -> Router identifies MCP tool -> MCP Client -> External MCP Server
                                              |
                                        (Missing: How does the Router know about MCP tools?
                                         Answer: F9 must register external tools in ToolRegistry
                                         at connection time, refreshing on reconnect.)
```

**Gap**: The mechanism for MCP client tools to appear in the Router's tool selection is not specified. External MCP server tools must be dynamically registered in the `ToolRegistry` with a namespace prefix (e.g., `mcp:server-name:tool-name`) to avoid name collisions with local tools.

---

## 2. Element 08: Memory & Workspace (Workstream H)

### 2.1 Architectural Assessment: 5/10

**Strengths**:
- Per-agent workspace isolation (`~/.clawft/agents/<agentId>/`) is a clean model with filesystem-based boundaries. Simple to implement and reason about.
- RVF Phase 3 builds on existing `rvf-runtime` and `rvf-types` workspace dependencies.
- Timestamp unification (H3) is a well-scoped, low-risk refactor with clear deliverables.

**Weaknesses and Gaps**:

1. **HNSW crate selection not finalized.** H2.1 says "replace brute-force cosine scan" and suggests `instant-distance` or `hnsw` crate. These have very different APIs, performance characteristics, and WASM compatibility:
   - `instant-distance`: Pure Rust, no unsafe, WASM-compatible. Immutable after build (no incremental insert). Good for batch indexing.
   - `hnsw` / `hnswlib-rs`: C++ binding, faster, supports incremental insert. Not WASM-compatible.
   - Rolling your own via `rvf-index`: Possible but unproven.
   **Recommendation**: Use `instant-distance` for the production HNSW implementation (H2.1) since it covers both native and WASM targets. For the WASM micro-HNSW (H2.8), use a stripped-down version of the same algorithm to share code. Incremental insert limitation can be mitigated by periodic full re-index (acceptable for <100K vectors at the stated performance target of <10ms).

2. **Production embedder integration architecture is incomplete.** H2.2 says "LLM embedding API (OpenAI / local ONNX)" but does not specify:
   - Which trait the embedder implements (the existing code has `hash_embedder.rs` and `api_embedder.rs` but no shared `Embedder` trait in the public API).
   - How embedding model selection is configured (per-agent? global?).
   - Fallback behavior when the embedding API is unavailable (fall back to hash? queue for later?).
   - Embedding dimensionality normalization (OpenAI `text-embedding-3-small` = 1536 dims, local ONNX models vary).
   **Recommendation**: Define an `Embedder` trait in `clawft-core/src/embeddings/mod.rs`:
   ```rust
   #[async_trait]
   pub trait Embedder: Send + Sync {
       fn dimensions(&self) -> usize;
       async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
       fn name(&self) -> &str;
   }
   ```
   This should be defined as part of C1 (or H2 pre-work in Week 4) and implemented by both `HashEmbedder` (for testing/offline) and `ApiEmbedder` (for production). The `MemoryBackend` plugin trait should accept an `Arc<dyn Embedder>` at construction time.

3. **Cross-agent shared memory namespace (L2 overlap) is underspecified.** The orchestrator says "explicit opt-in for shared memory namespace" but does not define the sharing protocol:
   - How does Agent A declare a memory namespace as sharable?
   - How does Agent B discover and subscribe to it?
   - What consistency model applies (eventual? read-your-writes?)?
   - Is shared memory read-only or read-write for subscribers?
   **Recommendation**: Shared memory should be read-only by default. Agent A exports a namespace via config (`shared_namespaces = ["project-context"]`). Agent B imports it via config (`import_namespaces = [{ agent = "agent-a", namespace = "project-context" }]`). The implementation uses symlinks or file references to Agent A's memory directory. Write access requires explicit `read_write = true` flag with filesystem-level locking.

4. **WASM micro-HNSW (H2.8) size budget unclear relative to main WASM build.** The main `clawft-wasm` crate has a 300KB budget. If H2.8 is bundled into it, the vector search code must fit within that budget alongside the agent loop. If H2.8 is a separate WASM module, the loading and communication protocol needs specification. **Recommendation**: H2.8 should be a separate WASM module (`micro-hnsw-wasm`) with its own 8KB budget as stated in MW-7. It communicates with the main WASM agent via shared memory or message passing (not embedded). This keeps the base agent WASM small and makes vector search optional.

5. **Temperature-based quantization (H2.7) interaction with HNSW index is unspecified.** HNSW indexes store vectors at fixed dimensionality. Quantization (fp16/PQ/binary) changes the representation. Does the HNSW index get rebuilt when vectors move between tiers? Or are there separate indexes per tier? **Recommendation**: Use a single HNSW index with full-precision vectors for search. Quantization applies only to the *storage* layer (cold vectors stored as PQ on disk, loaded and decompressed on access). The index itself uses full-precision pointers. This avoids index rebuild on tier transitions.

6. **RVF file I/O (H2.3) dependency on `rvf-runtime` 0.2.** The workspace pins `rvf-runtime = "0.2"`. The SPARC plan does not verify whether RVF 0.2 actually supports the segment read/write operations needed. If `rvf-runtime` 0.2 only provides stub segment types, H2.3 may require contributing upstream or forking. **Recommendation**: Audit `rvf-runtime` 0.2 API surface in Week 4 before committing to H2.3 implementation. If segment I/O is missing, plan a local implementation that serializes to the RVF format directly using `rvf-types`.

7. **WITNESS segments (H2.6) cryptographic requirements not specified.** "Tamper-evident audit trail" implies hash chaining or Merkle trees, but the SPARC plan does not specify the cryptographic algorithm, chain structure, or verification protocol. **Recommendation**: Use SHA-256 hash chaining (each segment includes hash of previous segment). Verification is sequential scan from root. The `sha2` crate is already a workspace dependency. Define the segment structure early so that all memory writes from H2.1-H2.5 can optionally include WITNESS metadata.

### 2.2 Data Flow Assessment

```
Agent writes memory:
  AgentLoop -> MemoryBackend.store(key, value)
    -> Embedder.embed(value)           [H2.2]
    -> VectorStore.insert(embedding)   [H2.1]
    -> RVF.write_segment(...)          [H2.3]
    -> WITNESS.append(hash_chain)      [H2.6]

Agent reads memory:
  AgentLoop -> MemoryBackend.search(query, limit)
    -> Embedder.embed(query)
    -> VectorStore.search(embedding, k) [H2.1 HNSW]
    -> RVF.read_segments(ids)           [H2.3]
    -> Return ranked results

Export/Import:
  CLI -> MemoryBackend.export() -> RVF file + WITNESS chain [H2.4]
  CLI -> MemoryBackend.import(file) -> Validate WITNESS -> Rebuild HNSW index [H2.4]
```

**Gap in data flow**: The path from `MemoryBackend.store()` to `VectorStore.insert()` requires embedding computation, which is async and potentially slow (API call). The orchestrator does not specify whether embedding is synchronous (blocking the agent loop) or asynchronous (queued, with eventual consistency). **Recommendation**: Embedding should be asynchronous. `store()` writes the raw data immediately (for keyword search), then spawns a background task for embedding. The vector index is eventually consistent. A `pending_embeddings` queue tracks items awaiting embedding.

---

## 3. Element 09: Multi-Agent Routing & Claude Flow Integration (Workstreams L, M)

### 3.1 Architectural Assessment: 5/10

**Strengths**:
- Agent routing table design is simple and config-driven. First-match-wins semantics are easy to reason about.
- FlowDelegator fallback chain (Flow -> Claude -> Local) provides graceful degradation.
- M2 and M3 are standalone fixes with zero dependencies -- can land immediately.
- Bidirectional MCP bridge (M5) leverages existing MCP server and client infrastructure.

**Weaknesses and Gaps**:

1. **Cross-agent communication protocol not specified.** L3 (Multi-agent swarming) says "agents can delegate subtasks to other agents, share results through the message bus." But the message bus is currently a simple inbound/outbound channel pair for a single agent loop. Multi-agent communication requires:
   - **Addressing**: How does Agent A address a message to Agent B? By agent ID? By role?
   - **Message format**: What is the inter-agent message schema? Is it the same as `InboundMessage`/`OutboundMessage` or a new type?
   - **Routing**: Who routes inter-agent messages? The bus? A coordinator?
   - **Acknowledgment**: Does the sender get confirmation that the message was received? Processed?
   - **Timeout**: What happens if Agent B does not respond?
   **Recommendation**: Define an `InterAgentMessage` type:
   ```rust
   pub struct InterAgentMessage {
       pub id: Uuid,
       pub from_agent: String,
       pub to_agent: String,
       pub task: String,
       pub payload: Value,
       pub reply_to: Option<Uuid>,  // for responses
       pub ttl: Duration,
   }
   ```
   Route through a new `AgentBus` (separate from the channel message bus) that maintains per-agent inboxes. The coordinator pattern uses this bus. This is a **major architectural gap** that must be resolved before L3 implementation begins.

2. **FlowDelegator error handling contract incomplete.** The draft contracts (contract 17) describe the happy path (spawn, stream, return) but not:
   - What happens when the `claude` subprocess exits with non-zero status?
   - What happens when stdout produces invalid JSON?
   - What happens when the subprocess exceeds memory limits?
   - What happens when the MCP callback from Claude Code targets a tool that no longer exists (hot-reload race)?
   - What is the maximum delegation timeout?
   **Recommendation**: Define an `enum DelegationError`:
   ```rust
   pub enum DelegationError {
       SubprocessFailed { exit_code: i32, stderr: String },
       OutputParseFailed { raw_output: String, parse_error: String },
       Timeout { elapsed: Duration },
       Cancelled,
       FallbackExhausted { attempts: Vec<(DelegationTarget, String)> },
   }
   ```
   Add this to the contract specification before M1 implementation.

3. **Hot-reload protocol not formally specified for MCP servers.** M4 says "hot-reload: watch `clawft.toml` for `mcp_servers` changes." But the protocol for updating the live MCP server list is not defined:
   - Are existing connections preserved when a new server is added?
   - Are connections drained gracefully when a server is removed?
   - What happens to in-flight tool calls when a server is removed?
   - How does the tool registry update atomically?
   **Recommendation**: Use a "drain-and-swap" protocol:
   1. File watcher detects change, debounce 500ms.
   2. Diff old and new server lists.
   3. New servers: connect immediately, add tools to registry.
   4. Removed servers: mark as "draining", complete in-flight calls (up to 30s timeout), then disconnect and remove tools from registry.
   5. Changed servers: treat as remove + add.
   This must be documented as a contract.

4. **Phase 4/5 ownership transition for shared crates.** The file ownership table in `03-dev-contracts.md` shows that `clawft-services/src/mcp/` is touched by C6, F8, F9, M4, and M5 -- five different streams. The mitigation ("each stream owns distinct files within the directory") is reasonable but the specific file assignments are not enumerated:
   - C6: Which file exposes skills via MCP?
   - F8: Which file handles IDE integration?
   - F9: Which file implements MCP client connections?
   - M4: Which file handles dynamic discovery?
   - M5: Which file implements the bidirectional bridge?
   **Recommendation**: Formalize the file ownership within `clawft-services/src/mcp/`:
   ```
   server.rs       -- Existing MCP server (owned by C6 for skill exposure)
   client.rs       -- NEW: MCP client connections (owned by F9)
   discovery.rs    -- NEW: Dynamic server management (owned by M4)
   bridge.rs       -- NEW: Bidirectional bridge orchestration (owned by M5)
   ide.rs          -- NEW: IDE-specific MCP extensions (owned by F8)
   transport.rs    -- Existing transport layer (shared, changes require cross-stream review)
   types.rs        -- Existing types (shared)
   middleware.rs   -- Existing middleware (shared)
   ```

5. **Agent routing table does not specify behavior for unrecognized channels or identifiers.** L1 describes first-match-wins with a wildcard catch-all, but what happens when:
   - A message arrives from a channel type not listed in any route?
   - A message arrives with no identifier (anonymous)?
   - The matched agent's workspace does not exist yet?
   **Recommendation**: Define explicit fallback behavior:
   - No matching route + no catch-all: reject message with "no agent configured for this channel/user" error, logged at `warn` level.
   - Matched agent workspace does not exist: auto-create from default template (`~/.clawft/agents/default/` if it exists, otherwise bare minimum with empty SOUL.md).
   - Anonymous messages: route to a dedicated "anonymous" agent or the catch-all agent, with reduced permissions.

6. **ReAct/Plan-and-Execute (L4) infinite loop prevention is mentioned in the risk register but not in the architecture.** The risk register says "configurable max planning depth" and "cost budget as hard stop" but L4's technical specification does not include these guards. **Recommendation**: L4 must define:
   - `max_planning_depth: u32` (default: 10) in Router config.
   - `max_planning_cost_usd: f64` (default: 1.0) as a hard budget.
   - `planning_step_timeout: Duration` (default: 60s) per step.
   - Circuit breaker: if 3 consecutive planning steps produce no actionable output, abort and return partial results to user with explanation.

7. **UI forward-compat hooks need tech specs for bus payload changes.** The overview document says "agent loop and bus support structured/binary payloads." Currently the bus uses `InboundMessage` and `OutboundMessage` which are text-oriented structs. The forward-compat requirement is that these support binary data (for future canvas rendering and voice). But the specific changes to the message types are not specified. **Recommendation**: Add a `payload: MessagePayload` enum to the message types:
   ```rust
   pub enum MessagePayload {
       Text(String),
       Structured(Value),
       Binary { mime_type: String, data: Vec<u8> },
   }
   ```
   This should be implemented in stream 5D (D8 bounded bus) or 5B (B1/B2 type unification) since it is a shared type change that all downstream consumers must handle.

### 3.2 Data Flow Assessment

**Agent Routing Flow**:
```
Channel -> InboundMessage(sender_id, channel_type)
  -> RoutingTable.match(channel_type, sender_id)
    -> AgentId resolved
      -> AgentLoop[agent_id] receives message
        -> Agent-specific workspace, skills, memory
          -> Pipeline processes, generates response
            -> OutboundMessage routed back to original channel
```

This flow is sound but has one gap: **how does the `RoutingTable` interact with the existing `MessageBus`?** Currently there is one bus per gateway. Multi-agent routing requires either:
- One bus per agent (each agent has its own inbound/outbound channels), or
- A single bus with agent-addressed messages (messages carry agent_id and are demultiplexed).

**Recommendation**: One bus per agent is simpler and provides better isolation. The gateway holds a `HashMap<AgentId, MessageBus>`. Channel adapters send messages to the gateway, which routes to the correct bus. This avoids cross-agent message leakage by construction.

**Claude Flow Delegation Flow**:
```
AgentLoop -> DelegationEngine.decide(task)
  -> DelegationTarget::Flow
    -> FlowDelegator.delegate(task)
      -> spawn "claude --print <task>"
      -> Register clawft as MCP server for callback
      -> Stream stdout -> parse JSON lines
      -> Return result to AgentLoop
```

**Gap**: The MCP callback registration step is not detailed. When `FlowDelegator` spawns `claude`, how does it tell Claude Code about the MCP server? Options:
- Pass `--mcp-server` flag to `claude` CLI (if supported).
- Rely on Claude Code's `~/.claude/mcp_servers.json` config (requires pre-setup by user).
- Use environment variable injection.

**Recommendation**: Document that users must pre-register clawft as an MCP server in Claude Code's config (M6 documentation task). The FlowDelegator should verify the MCP server is accessible before spawning Claude Code and log a warning if it cannot.

---

## 4. Cross-Element Interface Gaps

### 4.1 Element 07 <-> Element 08 Interface

**Gap**: Dev tools (F1-F7) need memory to persist state (e.g., git repo locations, OAuth tokens, browser bookmarks). The interface between tool plugins and the memory system is not specified. Do tool plugins use the `MemoryBackend` trait directly? Or do they use a simpler key-value store?

**Recommendation**: Tool plugins should NOT depend on `MemoryBackend` directly (that would create a circular dependency between `clawft-plugin` traits). Instead, provide a `ToolContext` struct passed to `Tool::execute()` that includes a `dyn KeyValueStore` for plugin-local state. The `KeyValueStore` is a simpler interface than `MemoryBackend` (just get/set/delete, no vector search). The agent loop provides the implementation backed by the agent's workspace directory.

### 4.2 Element 07 <-> Element 09 Interface

**Gap**: MCP client (F9) tools must be available to the delegation system (M5) and the agent routing system (L1). When Agent A has access to MCP server X, and Agent B does not, the per-agent tool registry must respect this. The current plan does not specify per-agent MCP server configuration.

**Recommendation**: MCP server connections should be configurable per-agent in the agent workspace config (`~/.clawft/agents/<agentId>/config.toml`). The `mcp_servers` section in the agent config overrides or extends the global config. The routing table entry can optionally specify `mcp_servers` to connect for that agent.

### 4.3 Element 08 <-> Element 09 Interface

**Gap**: Per-agent workspace isolation (H1/L2) is described in both elements but the ownership is unclear. H1 defines the workspace structure. L2 defines the routing-driven agent creation. Who creates the workspace directory? Who manages the lifecycle (creation, deletion, backup)?

**Recommendation**: H1 owns the workspace structure and provides a `WorkspaceManager` that can create, delete, and list agent workspaces. L2 (agent routing) calls `WorkspaceManager::ensure_workspace(agent_id)` when routing a message to an agent for the first time. This keeps workspace management in H1 and routing logic in L2.

### 4.4 Element 09 Internal: Workstream L <-> Workstream M Interface

**Gap**: The delegation system (M) and the multi-agent routing system (L) both deal with agent orchestration but through different mechanisms. Delegation routes tasks to external agents (Claude Code). Multi-agent routing routes messages to internal agents. There is no specification for how these interact. Can an internal agent delegate to Claude Code? Can Claude Code route back to a specific internal agent?

**Recommendation**: Yes to both. The delegation system should be accessible from any agent's pipeline (not just the default agent). The `FlowDelegator` should include the requesting agent's ID in the MCP callback context so that Claude Code's responses route back to the correct agent. This requires threading `agent_id` through the delegation path.

---

## 5. Crate Structure Recommendations

### Current (9 crates):
```
clawft-types, clawft-platform, clawft-core, clawft-llm,
clawft-tools, clawft-channels, clawft-services, clawft-cli, clawft-wasm
```

### Recommended additions for Elements 07/08/09:

| New Crate | Purpose | Owner Element |
|-----------|---------|---------------|
| `clawft-plugin` | Plugin trait definitions, manifest schema, SKILL.md types | 04 (C1) -- prerequisite for all |
| `clawft-plugin-git` | Git tool plugin (F1) | 07 |
| `clawft-plugin-cargo` | Cargo/build tool plugin (F2) | 07 |
| `clawft-plugin-treesitter` | tree-sitter code analysis (F3) | 07 |
| `clawft-plugin-browser` | Browser CDP automation (F4) | 07 |
| `clawft-plugin-calendar` | Calendar integration (F5) | 07 |
| `clawft-plugin-oauth2` | OAuth2 helper (F6) | 07 |
| `clawft-plugin-containers` | Docker/Podman orchestration (F7) | 07 |

**Note**: F8 and F9 are NOT separate crates. They extend `clawft-services/src/mcp/` (MCP layer). MCP client (F9) and IDE integration (F8) are modules within the existing services crate because they are infrastructure, not plugins.

**Note**: H1, H2, L1-L4, and M1-M5 do NOT require new crates. They extend existing crates:
- H1/H2: Extend `clawft-core` (memory/embeddings modules) and `clawft-types` (workspace config).
- L1-L4: Extend `clawft-core` (agent routing) and `clawft-types` (route config).
- M1-M5: Extend `clawft-services` (delegation, MCP).

**Rationale for keeping H/L/M in existing crates**: These features are core infrastructure, not optional plugins. Vector memory and agent routing are fundamental to the agent loop. Moving them to separate crates would create excessive inter-crate dependency and complicate the build. They should be feature-gated within their host crates instead.

---

## 6. WASM Boundary Analysis

| Component | Runs in WASM? | Rationale |
|-----------|--------------|-----------|
| F1 (Git) | No | `git2` has C dependencies, libgit2 not WASM-compatible |
| F2 (Cargo) | No | Subprocess-based, requires OS process spawning |
| F3 (tree-sitter) | No | C grammar bindings, not WASM-compatible |
| F4 (Browser CDP) | No | Network + process spawning for headless Chrome |
| F5 (Calendar) | Possible | Pure HTTP + JSON, could work in WASM with WASI HTTP |
| F6 (OAuth2) | Possible | HTTP-based flow, but requires redirect URI handling |
| F7 (Docker) | No | Subprocess-based, requires Docker socket access |
| F8 (MCP IDE) | No | Server-side MCP, requires socket listening |
| F9 (MCP Client) | Possible | Client-side HTTP/stdio, partial WASM with WASI |
| H2.1 (HNSW) | Yes | `instant-distance` is pure Rust, WASM-compatible |
| H2.2 (Embedder) | Partial | API embedder needs HTTP; hash embedder is WASM-safe |
| H2.8 (micro-HNSW) | Yes | Explicitly designed for WASM |
| L1 (Routing Table) | Yes | Pure config lookup, no OS deps |
| L3 (Swarming) | No | Requires multi-process/multi-thread coordination |
| M1 (FlowDelegator) | No | Subprocess spawning |

**Recommendation**: The WASM build should include: agent loop, HNSW vector search (H2.8), routing table (L1), and basic tool execution. All dev tools (F1-F7), subprocess-based delegation (M1), and multi-agent swarming (L3) are native-only. This keeps the WASM binary within the 300KB budget.

---

## 7. Scalability Assessment

### Element 07 (Dev Tools)
- **Concern**: Browser CDP (F4) and Docker (F7) are resource-heavy. Multiple agents using browser automation concurrently could exhaust system resources.
- **Recommendation**: Add per-tool concurrency limits in the plugin manifest. Default: 1 concurrent browser session per agent, 3 concurrent Docker operations globally.

### Element 08 (Memory)
- **Concern**: HNSW index with 100K vectors must deliver <10ms search. `instant-distance` with 1536-dim embeddings (OpenAI) at 100K vectors requires ~600MB RAM for the index alone.
- **Recommendation**: Document memory requirements. For constrained environments, support dimensionality reduction (e.g., 256-dim via PCA or Matryoshka embeddings). Add a `max_vectors` config option with automatic eviction of cold vectors when exceeded.

### Element 09 (Multi-Agent)
- **Concern**: Agent routing with many agents (>10) on a single gateway. Each agent has its own pipeline, tools, and memory. Resource consumption scales linearly.
- **Recommendation**: Add `max_agents` config option (default: 10). Agents that have been idle for a configurable period should be "parked" (memory freed, workspace on disk, reactivated on next message). The `AgentPool` pattern: keep N active agents, park the rest.

---

## 8. Known Architectural Issues Assessment

| Issue | Status | Severity | Recommendation |
|-------|--------|----------|----------------|
| M4/F9 dependency timeline conflict | CONFIRMED | High | Split F9 into F9a (Week 5-6, core client) and F9b (Week 9-10, full features). M4 depends on F9a only. |
| Phase 4/5 ownership transition for shared crates | CONFIRMED | Medium | Formalize per-file ownership within `clawft-services/src/mcp/` as specified in Section 3.1 point 4 above. |
| Cross-agent communication protocol not specified | CONFIRMED | High | Define `InterAgentMessage` type and `AgentBus` as specified in Section 3.1 point 1. Must be resolved before L3 implementation. |
| Hot-reload protocol not formally specified | CONFIRMED | Medium | Define drain-and-swap protocol for MCP server hot-reload as specified in Section 3.1 point 3. |
| FlowDelegator error handling contract incomplete | CONFIRMED | High | Define `DelegationError` enum as specified in Section 3.1 point 2. Must be resolved before M1 implementation. |
| UI forward-compat hooks need tech specs for bus payload changes | CONFIRMED | Medium | Define `MessagePayload` enum as specified in Section 3.1 point 7. Should land with D8 (bounded bus) changes. |

---

## 9. Development Readiness

### Element 07: Dev Tools & Applications -- NEEDS MINOR FIXES

**Rationale**: The plugin-per-tool architecture is sound. Individual tools (F1-F7) can proceed once C1 lands. The two blockers are:
1. F9 must be split (F9a/F9b) to resolve the M4 timeline conflict. Without this, Claude Flow dynamic MCP discovery is blocked until Week 10.
2. Tool permission model must be specified before implementation begins -- security cannot be retrofitted.
3. MCP tool namespace convention must be defined for F9 tools appearing in the registry.

**What must happen before implementation**:
- Finalize crate-per-plugin structure decision (recommend yes).
- Define tool permission schema in plugin manifest.
- Split F9 deliverable and update orchestrator timeline.
- Specify MCP tool namespace convention.

### Element 08: Memory & Workspace -- NEEDS MAJOR WORK

**Rationale**: The element has significant architectural gaps that could lead to rework if not resolved upfront:
1. No `Embedder` trait defined -- the interface between vector store and embedding providers is implicit.
2. HNSW crate not selected -- affects API design, WASM compatibility, and performance characteristics.
3. Cross-agent shared memory protocol not specified -- could require redesign of workspace structure.
4. RVF 0.2 API surface not audited -- H2.3 may be infeasible without upstream work.
5. WITNESS cryptographic structure not defined -- affects all memory write paths.
6. Async embedding pipeline not designed -- could cause agent loop latency issues.

**What must happen before implementation**:
- Define `Embedder` trait and wire into existing code.
- Select HNSW crate with WASM compatibility proof-of-concept.
- Audit `rvf-runtime` 0.2 API for segment I/O capabilities.
- Define WITNESS segment hash-chain structure.
- Design async embedding pipeline with eventual consistency model.
- Specify cross-agent shared memory protocol.

### Element 09: Multi-Agent Routing & Claude Flow -- NEEDS MAJOR WORK

**Rationale**: The element combines two architecturally distinct subsystems (agent routing and Claude Flow delegation) with significant interface gaps:
1. Cross-agent communication protocol is entirely missing -- L3 cannot be implemented without it.
2. FlowDelegator error handling contract is incomplete -- M1 will produce fragile code without it.
3. Multi-agent bus architecture not decided (per-agent bus vs shared bus with demux).
4. MCP callback registration mechanism for Claude Code not specified.
5. Inter-element interfaces (L <-> M, L <-> H, routing -> delegation) are not formally defined.

**What must happen before implementation**:
- Define `InterAgentMessage` type and `AgentBus` architecture.
- Complete `DelegationError` enum and fallback contract.
- Decide on per-agent bus architecture (recommend per-agent bus).
- Specify MCP callback registration for FlowDelegator.
- Formally define L <-> M interface (how delegation interacts with routing).
- Define auto-workspace-creation behavior for first-time agent routing.
- Specify ReAct/Plan-and-Execute guard rails (max depth, cost budget, timeout).

---

## 10. Summary Scores

| Element | Architecture Score | Readiness Rating | Key Blocker |
|---------|-------------------|-----------------|-------------|
| 07 (Dev Tools) | 6/10 | NEEDS MINOR FIXES | F9 split, tool permissions, MCP namespacing |
| 08 (Memory) | 5/10 | NEEDS MAJOR WORK | Embedder trait, HNSW selection, RVF audit, WITNESS spec |
| 09 (Multi-Agent) | 5/10 | NEEDS MAJOR WORK | Inter-agent protocol, DelegationError contract, bus architecture |

**Overall Sprint Risk**: The sprint plan assumes these elements can start in Week 3-5. Given the architectural gaps identified, Elements 08 and 09 need 1-2 weeks of design work before implementation can begin safely. Element 07 can proceed with minor pre-work. Recommend dedicating Week 3-4 to architectural specification for all three elements, pushing feature implementation to Week 5+.
