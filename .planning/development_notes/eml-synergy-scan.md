# EML Synergy Scan: Complete Hardcoded Heuristic Map

> Scanned 2026-04-04. Every magic number, fixed threshold, linear combination,
> and hand-tuned formula across graphify, kernel, LLM, assessment, and bench
> subsystems. Each row is a candidate for replacement with an
> `EmlModel::new(depth, inputs, heads)` learned function.

---

## Module: graphify/analyze.rs

| Line | Current | What It Does | EML Replacement | Training Data | Impact |
|------|---------|-------------|----------------|---------------|--------|
| 209-213 | `Ambiguous=>3, Inferred=>2, Extracted=>1` | Confidence bonus in surprise scoring (linear integer mapping) | `EmlModel::new(2, 1, 1)` -- learn confidence->surprise_contribution mapping | User feedback: "was this edge actually surprising?" | Non-linear surprise detection; ambiguous may deserve 5x not 3x |
| 226 | `score += 2` | Cross file-type bonus (code <-> paper) | Part of composite EML model | Same user feedback corpus | Learns optimal weight for cross-type edges per domain |
| 232 | `score += 2` | Cross-repo/directory bonus | Part of composite EML model | Same | Some repos are tightly coupled; EML learns which |
| 242 | `score += 1` | Cross-community bonus | Part of composite EML model | Same | |
| 248 | `score * 1.5` | Semantic similarity multiplier (truncated via `as i32`) | Part of composite EML model | Same | Continuous multiplier, not truncated |
| 254 | `deg_u.min(deg_v) <= 2 && deg_u.max(deg_v) >= 5` | Peripheral-to-hub detection threshold | `EmlModel::new(2, 2, 1)` with inputs (min_deg, max_deg) | Validated surprising-edge annotations | Learns optimal degree gap for surprise |
| 255 | `score += 1` | Peripheral-hub bonus | Part of composite | Same | |
| 526 | `score = neighbor_comms.len() / kg.node_count()` | Bridge node betweenness approximation | `EmlModel::new(3, 3, 1)` with (neighbor_comm_count, degree, graph_size) | Expert bridge-node labeling | Better betweenness proxy |
| 534 | `.take(3)` | Top-3 bridge nodes for questions | Could be dynamic based on graph structure | Question quality ratings | Adaptive question count |
| 582 | `god_nodes(kg, 5)` | Top-5 god nodes for inferred-edge questions | Could be dynamic | Same | |
| 590 | `inferred.len() >= 2` | Threshold for "enough inferred edges to generate a question" | `EmlModel::new(2, 2, 1)` with (inferred_count, total_degree) | Question relevance ratings | |
| 661 | `score < 0.15 && nodes.len() >= 5` | Low-cohesion community threshold for question generation | `EmlModel::new(2, 2, 1)` with (cohesion, community_size) | Validated community quality | Per-domain optimal cohesion threshold |
| 373-374 | `1.0 / (deg_a * deg_b)` | Inverse degree product as betweenness heuristic (no-community fallback) | `EmlModel::new(2, 2, 1)` with (deg_src, deg_tgt) | Expert edge importance annotations | |
| 79 | `label.ends_with("()") && kg.degree(id) <= 1` | File node classification: degree <= 1 as stub | `EmlModel::new(2, 2, 1)` with (degree, label_pattern_features) | Labeled file-vs-real-node dataset | Fewer false classifications |

**Composite surprise scorer:** Lines 204-269 could be a single `EmlModel::new(3, 7, 1)` with inputs: `[confidence_ordinal, same_file_type, same_repo, same_community, is_semantic, min_degree, max_degree]` -> single surprise score. Training on validated "actually surprising" labels.

---

## Module: graphify/cluster.rs

| Line | Current | What It Does | EML Replacement | Training Data | Impact |
|------|---------|-------------|----------------|---------------|--------|
| 13 | `MAX_COMMUNITY_FRACTION = 0.25` | Split communities > 25% of graph | `EmlModel::new(2, 2, 1)` with (graph_size, edge_density) -> optimal fraction | Validated community quality ratings | Per-graph optimal split point |
| 15 | `MIN_SPLIT_SIZE = 10` | Only split if >= 10 nodes | Part of above model | Same | |
| 119 | `for _ in 0..50` | Max label propagation iterations (safety valve) | Could learn convergence predictor, but low priority | N/A | Minor |
| 236 | `(score * 100.0).round() / 100.0` | Cohesion rounding to 2 decimals | Not a heuristic | N/A | N/A |

---

## Module: graphify/export/html.rs

| Line | Current | What It Does | EML Replacement | Training Data | Impact |
|------|---------|-------------|----------------|---------------|--------|
| 18 | `MAX_NODES_FOR_VIZ = 5_000` | Max nodes for HTML visualization | Not suited for EML (hardware constraint) | N/A | N/A |
| 52 | `size = 10.0 + 30.0 * (deg / max_deg)` | Node size: linear mapping degree -> visual size [10, 40] | `EmlModel::new(2, 2, 1)` with (deg_ratio, community_size) | User preference for readability | Non-linear sizing (sqrt or log may be better) |
| 53-56 | `font_size = if deg >= max_deg * 0.15 { 12 } else { 0 }` | Font visibility threshold: top 15% degree nodes get labels | `EmlModel::new(2, 2, 1)` with (deg_ratio, total_nodes) | User readability feedback | Adaptive label density |
| 94-95 | `width: if is_extracted { 2 } else { 1 }` | Edge width by confidence | Could be 3-level via EML | User preference | Minor |
| 95 | `opacity: if is_extracted { 0.7 } else { 0.35 }` | Edge opacity by confidence | Same | Same | Minor |
| 246 | `gravitationalConstant: -60` | vis.js ForceAtlas2 gravitational constant | `EmlModel::new(2, 2, 6)` with (node_count, edge_density) -> 6 physics params | User layout quality ratings | Auto-tuned physics per graph topology |
| 247 | `centralGravity: 0.005` | ForceAtlas2 central gravity | Part of above | Same | |
| 248 | `springLength: 120` | ForceAtlas2 spring length | Part of above | Same | |
| 249 | `springConstant: 0.08` | ForceAtlas2 spring constant | Part of above | Same | |
| 250 | `damping: 0.4` | ForceAtlas2 damping factor | Part of above | Same | |
| 251 | `avoidOverlap: 0.8` | ForceAtlas2 overlap avoidance | Part of above | Same | |
| 253 | `stabilization: { iterations: 200 }` | Stabilization iteration count | Part of above or separate | Same | |
| 257 | `tooltipDelay: 100` | Tooltip delay in ms | UI preference, not EML | N/A | N/A |
| 263 | `roundness: 0.2` | Edge curvature | Low priority | N/A | Minor |
| 389 | `1.15` | Hyperedge hull expansion factor | `EmlModel::new(2, 1, 1)` with (node_count_in_hyperedge) | Visual clarity ratings | |
| 376 | `ctx.globalAlpha = 0.12` | Hyperedge fill opacity | Low priority | Same | Minor |
| 394 | `ctx.globalAlpha = 0.4` | Hyperedge stroke opacity | Same | Same | Minor |

**Composite physics tuner:** A single `EmlModel::new(3, 2, 6)` mapping (node_count, edge_density) to the 6 ForceAtlas2 params. Training data: user ratings of layout quality on diverse graph topologies.

---

## Module: graphify/pipeline.rs

| Line | Current | What It Does | EML Replacement | Training Data | Impact |
|------|---------|-------------|----------------|---------------|--------|
| 46 | `god_nodes_top_n: 10` | Default top-N god nodes | Could be adaptive per graph size | Graph analysis quality | Minor |
| 48 | `surprises_top_n: 5` | Default top-N surprises | Could be adaptive | Same | Minor |
| 49 | `questions_top_n: 7` | Default top-N questions | Could be adaptive | Same | Minor |

---

## Module: graphify/report.rs

| Line | Current | What It Does | EML Replacement | Training Data | Impact |
|------|---------|-------------|----------------|---------------|--------|
| 178 | `.take(8)` | Show at most 8 node labels per community in report | Not a heuristic | N/A | N/A |
| 215 | `kg.degree(id) <= 1` | Isolated node threshold for knowledge gap detection | `EmlModel::new(2, 2, 1)` with (degree, graph_avg_degree) | Validated "is this actually a gap?" | Adaptive to graph density |
| 222 | `nodes.len() < 3` | "Thin community" threshold | Part of above | Same | |
| 225 | `amb_pct > 20` | High ambiguity warning threshold (>20%) | `EmlModel::new(2, 1, 1)` with (ambiguity_pct) | Report quality ratings | |

---

## Module: graphify/domain/forensic.rs

| Line | Current | What It Does | EML Replacement | Training Data | Impact |
|------|---------|-------------|----------------|---------------|--------|
| 141 | `deg <= 1` | Unlinked evidence threshold (degree 0-1) | `EmlModel::new(2, 2, 1)` with (degree, graph_density) | Expert forensic analyst labeling | Domain-adaptive gap detection |
| 227 | `density * avg_confidence` | Coherence score formula: linear combination | `EmlModel::new(3, 2, 1)` with (density, avg_confidence) | Expert coherence ratings | Discovers non-linear interaction |
| 260-273 | `new_density * new_avg - current` | Counterfactual delta: assumes linear coherence model | Same EML model as coherence score | Same | Better counterfactual predictions |

---

## Module: kernel/governance.rs

| Line | Current | What It Does | EML Replacement | Training Data | Impact |
|------|---------|-------------|----------------|---------------|--------|
| 141-148 | `magnitude() = sqrt(sum of squares)` | EffectVector L2 norm for governance threshold | `EmlModel::new(3, 5, 1)` with (risk, fairness, privacy, novelty, security) -> composite score | Historical governance decisions with human review | Non-L2 importance weighting per dimension |
| 150-156 | `any_exceeds(threshold)` | Per-dimension threshold check (uniform threshold) | Part of above; learns per-dimension thresholds | Same | Different dimensions may need different thresholds |
| 359 | `magnitude > self.risk_threshold` | Global threshold comparison | Part of above | Same | |
| 448 | `env.governance.risk_threshold * 0.5` | Production halves the threshold | `EmlModel::new(2, 2, 1)` with (base_threshold, environment_ordinal) | Production incident data | Learn optimal per-environment scaling |

---

## Module: kernel/supervisor.rs

| Line | Current | What It Does | EML Replacement | Training Data | Impact |
|------|---------|-------------|----------------|---------------|--------|
| 65-66 | `max_restarts: 5, within_secs: 60` | Default restart budget (5 in 60s) | `EmlModel::new(2, 2, 2)` with (crash_frequency, uptime_history) -> (max_restarts, window) | Historical agent crash/restart data | Adaptive restart budgets |
| 102 | `base: 100` ms | Backoff base delay | `EmlModel::new(2, 2, 1)` with (restart_count, error_type_ordinal) -> delay_ms | Restart success/failure outcomes | |
| 104 | `delay.min(30_000)` | Max backoff cap (30s) | Part of above | Same | |
| 219 | `ratio >= 0.8` | Resource warning threshold (80% utilization) | `EmlModel::new(2, 2, 1)` with (resource_type_ordinal, current_ratio) | OOM/timeout correlation data | Per-resource optimal warning point |

---

## Module: kernel/health.rs

| Line | Current | What It Does | EML Replacement | Training Data | Impact |
|------|---------|-------------|----------------|---------------|--------|
| 158 | `liveness_interval_secs: 10` | Default liveness probe interval | `EmlModel::new(2, 2, 2)` with (service_type, historical_failure_rate) -> (liveness_interval, readiness_interval) | Service uptime data | Adaptive probe frequencies |
| 160 | `readiness_interval_secs: 5` | Default readiness probe interval | Part of above | Same | |
| 162 | `failure_threshold: 3` | Consecutive failures before marking failed | `EmlModel::new(2, 2, 1)` with (service_type, failure_pattern) | False positive/negative data on service health | Fewer false alarms |
| 164 | `success_threshold: 1` | Consecutive successes for recovery | Part of above | Same | |

---

## Module: kernel/cluster.rs

| Line | Current | What It Does | EML Replacement | Training Data | Impact |
|------|---------|-------------|----------------|---------------|--------|
| 134 | `heartbeat_interval_secs: 5` | Default heartbeat interval | `EmlModel::new(2, 3, 1)` with (node_count, network_latency_estimate, platform_type) | Cluster partition/detection latency data | Adaptive heartbeat per cluster topology |
| 139 | `suspect_threshold: 3` | Missed heartbeats -> suspect (3) | Part of above | Same | |
| 143 | `unreachable_threshold: 10` | Missed heartbeats -> unreachable (10) | Part of above | Same | |

---

## Module: kernel/dead_letter.rs

| Line | Current | What It Does | EML Replacement | Training Data | Impact |
|------|---------|-------------|----------------|---------------|--------|
| 17 | `DEFAULT_DLQ_CAPACITY = 10_000` | Max dead letters retained | Not a heuristic (memory constraint) | N/A | N/A |

---

## Module: kernel/assessment/analyzers/complexity.rs

| Line | Current | What It Does | EML Replacement | Training Data | Impact |
|------|---------|-------------|----------------|---------------|--------|
| 39 | `line_count > 500` | Large file warning threshold | `EmlModel::new(2, 3, 1)` with (line_count, language_type, function_count) | Bug density correlation data | Language-adaptive complexity thresholds |

---

## Module: llm/retry.rs

| Line | Current | What It Does | EML Replacement | Training Data | Impact |
|------|---------|-------------|----------------|---------------|--------|
| 23 | `max_retries: 3` | Default max retry attempts | `EmlModel::new(2, 2, 1)` with (error_type, provider_type) -> optimal_retries | Historical retry success/failure data | Fewer wasted retries on permanent errors |
| 25 | `base_delay: Duration::from_secs(1)` | Base retry delay (1s) | `EmlModel::new(2, 3, 1)` with (error_type, attempt, provider_load) -> delay_ms | Retry success timing data | Smarter backoff |
| 27 | `max_delay: Duration::from_secs(30)` | Max retry delay (30s) | Part of above | Same | |
| 28 | `jitter_fraction: 0.25` | Jitter as fraction of delay (25%) | Low priority for EML | N/A | Minor |
| 64 | `2^n` exponential backoff | Exponential backoff formula | EML could learn optimal backoff curve | Same | |
| 186 | `mpsc::channel(256)` | Streaming retry buffer size | Not a heuristic (capacity tuning) | N/A | N/A |

---

## Module: weave/bench_cmd.rs

| Line | Current | What It Does | EML Replacement | Training Data | Impact |
|------|---------|-------------|----------------|---------------|--------|
| 12 | `Throughput 25%, Latency 25%, Scalability 20%, Stability 15%, Endurance 15%` | Dimension weights for composite score | `EmlModel::new(2, 5, 1)` with 5 dimension scores -> composite | Expert benchmark quality ratings | Discovers optimal weight balance |
| 577 | `efficiency < 80.0` | Scalability knee point threshold | Part of scalability scorer | Validated knee point annotations | |
| 917-930 | Throughput breakpoints: `(1K,0), (5K,20), (10K,40), (20K,60), (50K,80), (100K,100)` | Piecewise linear throughput scoring | `EmlModel::new(3, 1, 1)` input=throughput | Expert benchmark grade labels | Smooth non-linear scoring curve |
| 934-947 | Latency breakpoints: `(50us,100), (100us,80), (500us,60), (1ms,40), (5ms,20), (10ms,0)` | Piecewise linear latency scoring | Same architecture | Same | |
| 951-961 | Scalability breakpoints: `(0.3,0), (0.5,50), (0.7,70), (0.9,100)` | Piecewise linear scalability scoring | Same | Same | |
| 965-977 | Stability breakpoints: `(1.5,100), (2.0,80), (3.0,60), (5.0,40), (10.0,20), (20.0,0)` | Piecewise linear stability scoring | Same | Same | |
| 981-994 | Endurance breakpoints: `(1%,100), (5%,80), (10%,60), (25%,40), (50%,20), (100%,0)` | Piecewise linear endurance scoring | Same | Same | |
| 1014-1028 | Grade thresholds: `95+=A+, 90-94=A, 85-89=A-, ...` | Score-to-grade mapping | Not EML (discrete label) | N/A | N/A |
| 1139-1147 | Scalability quality labels: `>=0.9=excellent, >=0.7=good, >=0.5=fair` | Scalability quality classification | Part of scalability scorer | Same | |
| 1159-1165 | Endurance quality: `<5%=stable, <15%=minor drift` | Drift classification thresholds | Part of endurance scorer | Same | |

**Composite benchmark scorer:** All 5 scoring functions + dimension weights could be a single `EmlModel::new(4, 5, 1)` mapping raw (throughput, p95, coefficient, ratio, drift) to a single score. Training on expert-labeled benchmark quality ratings.

---

## Module: graphify/build.rs

| Line | Current | What It Does | EML Replacement | Training Data | Impact |
|------|---------|-------------|----------------|---------------|--------|
| 127 | `weight: 1.0` (default edge weight in JSON parsing) | Default relationship weight | Not a heuristic (schema default) | N/A | N/A |

No EML candidates in build.rs -- it is purely structural graph assembly.

---

## Module: graphify/domain/code.rs

No hardcoded thresholds or heuristics. Pure domain type configuration.

---

## Module: llm/router.rs

No hardcoded thresholds. Prefix-based routing is deterministic string matching.

---

## Module: llm/failover.rs

No hardcoded thresholds. Chain tries providers in order.

---

## Module: core/agent/skills.rs, skills_v2.rs, context.rs

No scoring heuristics found. Skill loading is YAML parsing; context building is template assembly.

---

## Module: kernel/mesh_assess.rs

No hardcoded thresholds. Transport layer for assessment sync (serialize/deserialize).

---

## Priority Tiers

### Tier 1: High Impact, Clear Training Data (implement first)

| # | Location | Description | EML Spec |
|---|----------|-------------|----------|
| 1 | `analyze.rs:204-269` | Composite surprise scorer (7 features, 6 magic numbers) | `EmlModel::new(3, 7, 1)` |
| 2 | `analyze.rs:661` | Low-cohesion question threshold | `EmlModel::new(2, 2, 1)` |
| 3 | `forensic.rs:227` | Coherence score formula (density * confidence) | `EmlModel::new(3, 2, 1)` |
| 4 | `governance.rs:141` | EffectVector magnitude (L2 norm may not be optimal) | `EmlModel::new(3, 5, 1)` |
| 5 | `bench_cmd.rs:811-913` | All 5 benchmark scoring functions + weights | `EmlModel::new(4, 5, 1)` |

### Tier 2: Medium Impact, Feasible Training Data

| # | Location | Description | EML Spec |
|---|----------|-------------|----------|
| 6 | `html.rs:246-253` | ForceAtlas2 physics params per graph topology | `EmlModel::new(3, 2, 6)` |
| 7 | `html.rs:52-56` | Node sizing and label visibility thresholds | `EmlModel::new(2, 2, 2)` |
| 8 | `cluster.rs:13-15` | Community split thresholds (fraction + min size) | `EmlModel::new(2, 2, 1)` |
| 9 | `supervisor.rs:65-104` | Restart budget + backoff parameters | `EmlModel::new(2, 2, 2)` |
| 10 | `health.rs:158-164` | Probe intervals and failure/success thresholds | `EmlModel::new(2, 2, 2)` |
| 11 | `cluster.rs:134-143` | Heartbeat/suspect/unreachable thresholds | `EmlModel::new(2, 3, 1)` |
| 12 | `retry.rs:23-28` | Retry config (max retries, base delay, max delay) | `EmlModel::new(2, 2, 1)` |

### Tier 3: Low Impact or Hard to Train

| # | Location | Description | EML Spec |
|---|----------|-------------|----------|
| 13 | `report.rs:215,222,225` | Report display thresholds | `EmlModel::new(2, 3, 1)` |
| 14 | `complexity.rs:39` | 500-line complexity threshold | `EmlModel::new(2, 3, 1)` |
| 15 | `governance.rs:448` | Production 0.5x risk multiplier | `EmlModel::new(2, 2, 1)` |
| 16 | `supervisor.rs:219` | 80% resource warning threshold | `EmlModel::new(2, 2, 1)` |

---

## Total Count

- **Files scanned:** 18
- **Hardcoded values found:** 73
- **EML-replaceable candidates:** 56
- **Distinct EmlModel instances needed:** ~16 (many values collapse into composite models)
- **Estimated training data sources:**
  - User feedback on graph quality (surprise, cohesion, layout)
  - Expert forensic analyst ratings
  - Historical governance decisions
  - Agent crash/restart telemetry
  - Service health false-positive rates
  - Benchmark grade expert labels
  - LLM retry success/failure logs
