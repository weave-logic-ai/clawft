# Sprint 09c: Weaver Runtime

**Document ID**: 09c
**Workstream**: W-KERNEL
**Duration**: 5 days
**Goal**: Make the Weaver a live, running service that processes data within the cognitive tick
**Depends on**: Sprint 09a (TestKernel infrastructure), K0-K6 complete
**Orchestrator**: `09-orchestrator.md`
**Priority**: P1 (High) -- required for confidence target and ongoing codebase monitoring

---

## S -- Specification

### Problem Statement

The Weaver is currently a structural analysis tool -- it can ingest data and
produce reports, but it does not run continuously. It uses mock embeddings
(SHA-256 hash-based, not semantic), has no CognitiveTick integration, cannot
detect changes incrementally, and cannot export models for edge deployment.
The Weaver confidence is 0.78 (structural only), limited by the absence of
real embeddings and runtime behavior data.

**Source data**: `.weftos/weaver_todo.md` (21 done, 20 remaining),
`docs/weftos/09-symposium/00-symposium-overview.md` section 4,
`docs/weftos/09-symposium/01-graph-findings.md` section 6.

### Current Capabilities vs Target

| Capability | Current | Target | Gap |
|-----------|---------|--------|-----|
| Embedding backend | Mock (SHA-256 hash) | Real semantic (LLM API + ONNX) | No semantic similarity |
| CognitiveTick | Not integrated | Tick consumer with budget | No live monitoring |
| Git polling | One-shot analysis | Incremental per-tick | No change detection |
| File watching | None | inotify/poll-based | No source change detection |
| Confidence scoring | Manual | Edge-coverage based | No automated scoring |
| Gap detection | Manual report | Automated per-tick | No continuous monitoring |
| Model export | Type defined | CLI command operational | No export pipeline |
| Meta-Loom | Types defined | Persistence operational | No strategy learning |

### Weaver TODO Items Addressed

From `.weftos/weaver_todo.md`:

**EmbeddingProvider Backends** (2 of 5 items):
- [ ] LLM API backend (call clawft-llm provider for embeddings) -- P0
- [ ] ONNX backend (all-MiniLM-L6-v2, 384d) behind `onnx-embeddings` -- P1

**Cognitive Tick Integration** (4 of 4 items):
- [ ] Register Weaver with CognitiveTick as a tick consumer
- [ ] Budget-aware tick processing (respect tick_budget_ratio)
- [ ] Incremental git polling (detect new commits since last tick)
- [ ] File watcher integration (detect source file changes)

**Confidence Improvement** (2 of 4 items):
- [ ] Confidence scoring based on edge coverage
- [ ] Gap detection: modules with no incoming/outgoing causal edges

**Export / Import** (1 of 4 items):
- [ ] weave-model.json export CLI command

**Self-Improvement** (1 of 3 items):
- [ ] Track which analysis strategies improved confidence (meta-Loom)

**Total**: 10 items addressed out of 20 remaining (50%).

### Feature Gate

```toml
[features]
ecc = ["blake3", "clawft-core/vector-memory"]
onnx-embeddings = ["ort"]  # optional, behind separate feature flag
```

The LLM API backend uses the existing `clawft-llm` provider trait and adds no
new dependencies. The ONNX backend adds `ort` (Rust ONNX Runtime bindings)
behind the `onnx-embeddings` feature flag.

---

## P -- Pseudocode

### LLM API Embedding Backend

```
struct LlmApiEmbeddingProvider {
    provider: Arc<dyn LlmProvider>,
    model: String,           // e.g., "text-embedding-3-small"
    dimensions: usize,       // e.g., 384 or 1536
    batch_size: usize,       // max texts per API call
}

impl EmbeddingProvider for LlmApiEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let response = self.provider.embed(EmbedRequest {
            model: self.model.clone(),
            input: vec![text.to_string()],
            dimensions: Some(self.dimensions),
        }).await?
        Ok(response.embeddings[0].clone())
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // Process in chunks of batch_size
        let mut results = Vec::new()
        for chunk in texts.chunks(self.batch_size) {
            let response = self.provider.embed(EmbedRequest {
                model: self.model.clone(),
                input: chunk.iter().map(|s| s.to_string()).collect(),
                dimensions: Some(self.dimensions),
            }).await?
            results.extend(response.embeddings)
        }
        Ok(results)
    }

    fn dimensions(&self) -> usize { self.dimensions }
}
```

### ONNX Embedding Backend

```
#[cfg(feature = "onnx-embeddings")]
struct OnnxEmbeddingProvider {
    session: ort::Session,
    tokenizer: Tokenizer,     // from tokenizers crate
    dimensions: usize,        // 384 for all-MiniLM-L6-v2
    max_sequence_length: usize,  // 512 tokens
}

#[cfg(feature = "onnx-embeddings")]
impl EmbeddingProvider for OnnxEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let encoding = self.tokenizer.encode(text, true)?
        let input_ids = encoding.get_ids()
        let attention_mask = encoding.get_attention_mask()

        // Truncate to max_sequence_length
        let len = min(input_ids.len(), self.max_sequence_length)

        let outputs = self.session.run(ort::inputs! {
            "input_ids" => ndarray::Array2::from_shape_vec((1, len), input_ids[..len].to_vec())?,
            "attention_mask" => ndarray::Array2::from_shape_vec((1, len), attention_mask[..len].to_vec())?,
        }?)?

        // Mean pooling over token embeddings
        let embeddings = outputs["last_hidden_state"].extract_tensor::<f32>()?
        let pooled = mean_pool(embeddings, attention_mask)
        Ok(normalize(pooled))
    }

    fn dimensions(&self) -> usize { self.dimensions }
}
```

### CognitiveTick Integration

```
impl TickConsumer for WeaverEngine {
    fn name(&self) -> &str { "weaver" }

    fn budget_ratio(&self) -> f32 { 0.15 }  // 15% of tick budget

    async fn on_tick(&self, tick: &TickContext) -> TickResult {
        let start = Instant::now()
        let budget = tick.budget * self.budget_ratio()

        // Phase 1: Detect changes (fast, <10% of budget)
        let changes = self.detect_changes().await
        if changes.is_empty() && !tick.force_full {
            return TickResult::NoWork
        }

        // Phase 2: Process changes (main work, 70% of budget)
        for change in changes {
            if start.elapsed() > budget * 0.8 {
                break  // respect budget
            }
            match change {
                Change::NewCommit(hash) => {
                    self.ingest_commit(hash).await?
                }
                Change::FileModified(path) => {
                    self.re_analyze_file(path).await?
                }
                Change::FileCreated(path) => {
                    self.analyze_new_file(path).await?
                }
            }
        }

        // Phase 3: Update confidence (fast, <20% of budget)
        let confidence = self.compute_confidence().await
        if confidence.changed_significantly() {
            self.emit_impulse(ImpulseType::BeliefUpdate)
        }

        TickResult::Processed {
            changes_processed: changes.len(),
            new_confidence: confidence.overall,
            elapsed: start.elapsed(),
        }
    }
}
```

### Incremental Git Polling

```
struct GitPoller {
    repo_path: PathBuf,
    last_known_head: Option<String>,  // commit hash
}

impl GitPoller {
    async fn detect_new_commits(&mut self) -> Vec<CommitInfo> {
        let current_head = git_rev_parse("HEAD", &self.repo_path)?
        if self.last_known_head == Some(current_head.clone()) {
            return vec![]
        }

        let new_commits = if let Some(ref prev) = self.last_known_head {
            git_log_range(prev, &current_head, &self.repo_path)?
        } else {
            // First run: ingest last 10 commits only (avoid full history)
            git_log_n(10, &self.repo_path)?
        }

        self.last_known_head = Some(current_head)
        new_commits
    }
}
```

### Confidence Scoring

```
fn compute_confidence(&self) -> ConfidenceReport {
    let total_modules = self.module_count()
    let modules_with_edges = self.modules_with_causal_edges()
    let edge_coverage = modules_with_edges as f32 / total_modules as f32

    let areas = vec![
        ("codebase_structure", self.structural_confidence()),
        ("module_relationships", edge_coverage),
        ("decision_chain", self.decision_coverage()),
        ("test_coverage_map", self.test_mapping_confidence()),
        ("temporal_patterns", self.temporal_confidence()),
        ("embedding_quality", self.embedding_confidence()),
    ]

    let overall = areas.iter().map(|(_, c)| c).sum::<f32>() / areas.len() as f32

    ConfidenceReport {
        overall,
        areas,
        gaps: self.identify_gaps(),
        suggestions: self.suggest_improvements(),
    }
}

fn identify_gaps(&self) -> Vec<ConfidenceGap> {
    let mut gaps = vec![]

    // Modules with no causal edges
    for module in self.all_modules() {
        if self.edges_for(module).is_empty() {
            gaps.push(ConfidenceGap::OrphanModule {
                module: module.name.clone(),
                lines: module.lines,
                suggestion: format!("Add data source for {}", module.name),
            })
        }
    }

    // Areas below threshold
    for (area, confidence) in self.area_confidences() {
        if confidence < 0.7 {
            gaps.push(ConfidenceGap::WeakArea {
                area: area.clone(),
                confidence,
                suggestion: self.suggest_for_area(area),
            })
        }
    }

    gaps
}
```

### Model Export

```
fn export_model(&self, domain: &str, min_confidence: f32) -> ExportedModel {
    let graph = self.causal_graph.snapshot()
    let nodes = graph.nodes()
        .filter(|n| n.confidence >= min_confidence)
        .collect()
    let edges = graph.edges()
        .filter(|e| nodes.contains(&e.source) && nodes.contains(&e.target))
        .collect()

    ExportedModel {
        version: "1.0",
        domain: domain.to_string(),
        generated_at: Utc::now(),
        confidence: self.overall_confidence(),
        nodes,
        edges,
        metadata: ModelMetadata {
            total_modules: self.module_count(),
            total_commits: self.commit_count(),
            embedding_dimensions: self.embedding_provider.dimensions(),
            graph_stats: GraphStats {
                node_count: nodes.len(),
                edge_count: edges.len(),
                avg_degree: edges.len() as f32 / nodes.len() as f32,
            },
        },
    }
}
```

---

## A -- Architecture

### Component Integration

```
+----------------------------------------------------------------+
|                    EXISTING KERNEL (K0-K6)                       |
|                                                                  |
|  +---------------+  +------------+  +-----------+  +---------+  |
|  | CognitiveTick |  | CausalGraph|  | HnswIndex |  | Impulse |  |
|  | (K3c)         |  | (K3c)      |  | (K3c)     |  | Queue   |  |
|  +-------+-------+  +------+-----+  +-----+-----+  +----+----+  |
|          |                 |               |              |       |
+----------|-----------------|---------------|--------------|-------+
           |                 |               |              |
+----------|-----------------|---------------|--------------|-------+
| 09c      |                 |               |              |       |
| WEAVER   |                 |               |              |       |
|  +-------v-------+  +-----v-----+  +------v----+  +-----v----+  |
|  | TickConsumer   |  | Model     |  | Embedding |  | Impulse  |  |
|  | (on_tick)      |  | Builder   |  | Provider  |  | Emitter  |  |
|  +-------+-------+  +-----+-----+  +-----+-----+  +----------+  |
|          |                 |               |                      |
|  +-------v-------+  +-----v-----+  +------v----------+          |
|  | GitPoller      |  | Confidence|  | LlmApiBackend   |          |
|  | FileWatcher    |  | Scorer    |  | (P0, default)   |          |
|  +---------------+  +-----------+  +---------+-------+          |
|                                              |                   |
|                                    +---------v-------+           |
|                                    | OnnxBackend     |           |
|                                    | (P1, optional)  |           |
|                                    +-----------------+           |
|                                                                  |
|  +---------------+  +-----------+                                |
|  | ModelExporter  |  | MetaLoom  |                               |
|  | (CLI: export)  |  | (persist) |                               |
|  +---------------+  +-----------+                                |
+------------------------------------------------------------------+
```

### File Map

| Component | File | Lines (est.) | Feature |
|-----------|------|:------------:|---------|
| LlmApiEmbeddingProvider | `crates/clawft-kernel/src/embedding.rs` | ~120 new | `ecc` |
| OnnxEmbeddingProvider | `crates/clawft-kernel/src/embedding_onnx.rs` | ~180 new | `onnx-embeddings` |
| TickConsumer impl | `crates/clawft-kernel/src/ecc_weaver.rs` | ~150 new | `ecc` |
| GitPoller | `crates/clawft-kernel/src/weaver_poller.rs` | ~120 new | `ecc` |
| FileWatcher | `crates/clawft-kernel/src/weaver_poller.rs` | ~80 new | `ecc` |
| ConfidenceScorer | `crates/clawft-kernel/src/weaver_confidence.rs` | ~150 new | `ecc` |
| ModelExporter | `crates/clawft-kernel/src/weaver_export.rs` | ~100 new | `ecc` |
| MetaLoom persistence | `crates/clawft-kernel/src/ecc_weaver.rs` | ~80 new | `ecc` |
| CLI wiring | `crates/clawft-weave/src/commands/ecc.rs` | ~60 changed | -- |
| **Total** | | **~1,040 new** | |

### Embedding Provider Selection Logic

```rust
fn select_embedding_provider(config: &WeaverConfig) -> Box<dyn EmbeddingProvider> {
    // Priority order:
    // 1. ONNX if onnx-embeddings feature enabled and model file exists
    // 2. LLM API if clawft-llm provider is configured
    // 3. Mock (fallback, for testing or when no backend available)

    #[cfg(feature = "onnx-embeddings")]
    if let Some(model_path) = &config.onnx_model_path {
        if Path::new(model_path).exists() {
            return Box::new(OnnxEmbeddingProvider::load(model_path).unwrap());
        }
    }

    if let Some(llm_config) = &config.llm_embedding {
        return Box::new(LlmApiEmbeddingProvider::new(
            llm_config.provider.clone(),
            llm_config.model.clone(),
            llm_config.dimensions,
        ));
    }

    Box::new(MockEmbeddingProvider::new())
}
```

---

## R -- Refinement

### Embedding Backend Decision (Weaver Agent Review)

**weaver's concern**: "ONNX may be overkill for Sprint 09. The all-MiniLM-L6-v2
model is 22MB and requires the `ort` crate, which brings in a C++ dependency
(ONNX Runtime). An alternative is to use the existing clawft-llm provider trait
to call an external embedding API."

**Resolution**: Both backends are implemented:
- **LLM API backend** (P0): Uses existing `clawft-llm` provider infrastructure.
  No new dependencies. Works with any LLM provider that supports embedding
  endpoints. This is the default and primary backend.
- **ONNX backend** (P1): Behind `onnx-embeddings` feature flag. Adds `ort`
  dependency only when explicitly enabled. Provides offline, low-latency
  embeddings for edge deployment scenarios.

### CognitiveTick Budget (ECC Analyst Review)

**ecc-analyst's concern**: "Default tick budget for consumers is 20%. The Weaver
should use 15% to leave headroom for other tick consumers (causal graph
maintenance, HNSW index updates)."

**Resolution**: Weaver budget set to 15% (`budget_ratio: 0.15`). This gives
approximately 54ms per tick at the default 360ms tick period. Sufficient for:
- Git poll check: ~5ms (stat comparison)
- File change detection: ~10ms (inotify read)
- One file re-analysis: ~20ms (parse + embed)
- Confidence update: ~5ms (edge count computation)

### Performance Considerations

- Git polling is stat-based (compare HEAD hash), not git-log-based. Takes <5ms.
- File watching uses `notify` crate (already in workspace) for inotify on Linux.
- Embedding via LLM API has network latency (~100ms). Budget-aware processing
  limits to 1-2 embeddings per tick. Batch embedding on idle ticks.
- ONNX embedding is local (~5ms per text). Allows 10+ embeddings per tick.
- Confidence computation is O(n) over modules, cached between ticks.

### Security Considerations

- LLM API backend sends source code to external API. The `WeaverConfig` must
  specify which files are embeddable (default: none). Sensitive files excluded.
- ONNX backend runs locally. No data leaves the machine.
- Model export (`weave-model.json`) may contain file paths and module names.
  Not sensitive but should be noted in export output.

---

## C -- Completion

### Work Packages

#### WP-1: LLM API Embedding Backend (Day 1)

**Owner**: weaver
**Reviewer**: ecc-analyst

- Implement `LlmApiEmbeddingProvider` in `crates/clawft-kernel/src/embedding.rs`
- Uses existing `clawft-llm` provider trait for embed calls
- Support single and batch embedding
- Configurable model name and dimensions
- Add 5+ tests: embed single text, embed batch, handle API error, verify dimensions
- File: `crates/clawft-kernel/src/embedding.rs`
- Estimated: ~120 lines new

#### WP-2: ONNX Embedding Backend (Day 1-2)

**Owner**: weaver
**Reviewer**: kernel-architect

- Implement `OnnxEmbeddingProvider` in new `crates/clawft-kernel/src/embedding_onnx.rs`
- Behind `onnx-embeddings` feature flag
- Load all-MiniLM-L6-v2 ONNX model
- Tokenize with `tokenizers` crate, run inference with `ort`
- Mean pooling + L2 normalization
- Add 3+ tests (behind feature flag): embed text, verify dimensions, batch embedding
- File: `crates/clawft-kernel/src/embedding_onnx.rs` (new)
- Estimated: ~180 lines new
- Note: May skip if `ort` dependency proves problematic on ARM. LLM API is sufficient.

#### WP-3: CognitiveTick Integration (Day 2-3)

**Owner**: weaver
**Reviewer**: ecc-analyst

- Implement `TickConsumer for WeaverEngine` in `crates/clawft-kernel/src/ecc_weaver.rs`
- Register Weaver with CognitiveTick at boot (when `ecc` feature enabled)
- Budget-aware processing: respect 15% tick budget
- Three-phase tick: detect changes, process changes, update confidence
- Add 5+ tests: tick with no changes, tick with new commit, tick budget respected,
  tick produces confidence update, tick emits impulse on significant change
- File: `crates/clawft-kernel/src/ecc_weaver.rs`
- Estimated: ~150 lines new

#### WP-4: Git Polling and File Watching (Day 3)

**Owner**: weaver
**Reviewer**: kernel-architect

- Implement `GitPoller` and `FileWatcher` in new `crates/clawft-kernel/src/weaver_poller.rs`
- GitPoller: stat-based HEAD comparison, incremental commit ingestion
- FileWatcher: `notify` crate integration for inotify (Linux) / kqueue (macOS)
- Both produce `Change` events consumed by the tick handler
- Add 5+ tests: detect new commit, detect file modify, detect file create,
  no false positives on unchanged repo, handle git repo not available
- File: `crates/clawft-kernel/src/weaver_poller.rs` (new)
- Estimated: ~200 lines new

#### WP-5: Confidence Scoring (Day 4)

**Owner**: weaver
**Reviewer**: ecc-analyst

- Implement `ConfidenceScorer` in new `crates/clawft-kernel/src/weaver_confidence.rs`
- Edge coverage: % of modules with at least one causal edge
- Gap detection: list modules with zero edges (orphans)
- Area-based scoring: codebase structure, module relationships, decision chain,
  test coverage mapping, temporal patterns, embedding quality
- Suggestion engine: recommend data sources for weak areas
- Add 4+ tests: scoring with full graph, scoring with orphans, gap detection
  accuracy, suggestion generation
- File: `crates/clawft-kernel/src/weaver_confidence.rs` (new)
- Estimated: ~150 lines new

#### WP-6: Model Export CLI (Day 4-5)

**Owner**: weaver
**Reviewer**: kernel-architect

- Implement `ModelExporter` in new `crates/clawft-kernel/src/weaver_export.rs`
- `weaver ecc export` CLI command produces `weave-model.json`
- Filters nodes by minimum confidence threshold
- Includes metadata: module count, commit count, embedding dimensions, graph stats
- Wire through daemon socket IPC to CLI
- Add 3+ tests: export produces valid JSON, confidence filter works,
  roundtrip (export then re-import node count matches)
- Files: `crates/clawft-kernel/src/weaver_export.rs` (new),
  `crates/clawft-weave/src/commands/ecc.rs` (changed)
- Estimated: ~160 lines new

#### WP-7: Meta-Loom Persistence (Day 5)

**Owner**: weaver
**Reviewer**: ecc-analyst

- Implement strategy tracking in `crates/clawft-kernel/src/ecc_weaver.rs`
- Each modeling decision recorded as `MetaLoomEvent`
- Track which strategies improved confidence (before/after comparison)
- Persist events under `meta-loom/{domain}` namespace in tree manager
- Add 3+ tests: record event, retrieve events for domain, strategy
  effectiveness calculation
- File: `crates/clawft-kernel/src/ecc_weaver.rs`
- Estimated: ~80 lines new

### Exit Criteria

- [ ] LLM API embedding backend operational (produces real semantic vectors)
- [ ] ONNX embedding backend operational behind `onnx-embeddings` feature (or documented as deferred if `ort` dependency issues)
- [ ] Weaver registered with CognitiveTick as tick consumer
- [ ] Budget-aware tick processing respects 15% tick_budget_ratio
- [ ] Incremental git polling detects new commits since last tick
- [ ] File watcher integration detects source file changes
- [ ] Confidence scoring based on edge coverage implemented
- [ ] Gap detection for modules with no causal edges implemented
- [ ] `weaver ecc export` CLI command produces valid weave-model.json
- [ ] Meta-Loom persistence tracks strategy effectiveness
- [ ] Weaver confidence reaches 0.85+ (from 0.78)
- [ ] 25+ new Weaver tests pass
- [ ] `scripts/build.sh test` passes with `ecc` feature enabled
- [ ] No new mandatory dependencies (ONNX is optional feature)

### Agent Assignment

| Agent | Role | Work Packages |
|-------|------|---------------|
| **weaver** | Primary implementer | WP-1 through WP-7 |
| **ecc-analyst** | Reviewer + domain expert | WP-1, WP-3, WP-5, WP-7 |
| **kernel-architect** | Reviewer | WP-2, WP-4, WP-6 |

### Expert Review Notes

**weaver**: "The LLM API backend is the pragmatic choice for Sprint 09. It
uses existing infrastructure (clawft-llm provider trait) and requires zero new
dependencies. ONNX is a nice-to-have for edge deployment but can be deferred
if the `ort` crate causes build issues on ARM."

**ecc-analyst**: "CognitiveTick budget of 15% is appropriate. The Weaver should
not be the dominant tick consumer -- the causal graph maintenance and HNSW
index updates also need tick time. The three-phase tick design (detect, process,
update) ensures graceful degradation: if the budget is tight, confidence updates
still happen even if not all changes are processed."

**kernel-architect**: "The new files (weaver_poller.rs, weaver_confidence.rs,
weaver_export.rs) should be feature-gated behind `ecc` in `lib.rs`. The
embedding_onnx.rs file should be gated behind `onnx-embeddings`. Both follow
the established pattern in the kernel crate."

### Testing Verification Commands

```bash
# Build with ECC feature
scripts/build.sh native --features ecc

# Run Weaver tests
cargo test -p clawft-kernel --features ecc -- weaver

# Run embedding tests
cargo test -p clawft-kernel --features ecc -- embedding

# Run with ONNX (if available)
cargo test -p clawft-kernel --features ecc,onnx-embeddings -- onnx

# Check confidence
weave ecc confidence --domain clawft 2>/dev/null || echo "Kernel not running"

# Clippy
scripts/build.sh clippy
```

### Implementation Order

```
Day 1:
  WP-1: LLM API embedding backend + tests
  WP-2: Start ONNX backend (may defer if ort problematic)

Day 2:
  WP-2: Complete ONNX backend (or document deferral)
  WP-3: CognitiveTick integration (start)

Day 3:
  WP-3: CognitiveTick integration (complete + tests)
  WP-4: Git polling + file watching

Day 4:
  WP-5: Confidence scoring + gap detection
  WP-6: Model export CLI (start)

Day 5:
  WP-6: Model export CLI (complete)
  WP-7: Meta-Loom persistence
  Integration test: full tick cycle (detect -> process -> score -> export)
```
