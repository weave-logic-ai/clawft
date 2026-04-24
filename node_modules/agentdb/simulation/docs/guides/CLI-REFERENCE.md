# AgentDB Simulation CLI Reference

**Version**: 2.0.0
**Last Updated**: 2025-11-30

Complete command-line reference for the AgentDB latent space simulation system. Covers all commands, options, and examples.

---

## 📖 Table of Contents

- [Command Overview](#command-overview)
- [Scenario Commands](#scenario-commands)
- [Interactive Modes](#interactive-modes)
- [Global Options](#global-options)
- [Configuration Management](#configuration-management)
- [Report Management](#report-management)
- [Advanced Usage](#advanced-usage)
- [Examples](#examples)

---

## 🎯 Command Overview

```bash
agentdb simulate [scenario] [options]
agentdb simulate --wizard
agentdb simulate --custom [component-options]
agentdb simulate --list
agentdb simulate --report [id]
```

### Quick Reference

| Command | Description | Example |
|---------|-------------|---------|
| `simulate [scenario]` | Run validated scenario | `agentdb simulate hnsw` |
| `simulate --wizard` | Interactive builder | `agentdb simulate --wizard` |
| `simulate --custom` | Custom configuration | `agentdb simulate --custom --backend ruvector` |
| `simulate --list` | List all scenarios | `agentdb simulate --list` |
| `simulate --report` | View past results | `agentdb simulate --report latest` |

---

## 🎬 Scenario Commands

### HNSW Graph Topology Exploration

```bash
agentdb simulate hnsw [options]
```

**Description**: Validates HNSW small-world properties, layer connectivity, and search performance. Discovered 8.2x speedup vs hnswlib.

**Validated Configuration**:
- M: 32 (8.2x speedup)
- efConstruction: 200 (small-world σ=2.84)
- efSearch: 100 (96.8% recall@10)

**Options**:
```bash
--nodes N                  # Node count (default: 100000)
--dimensions D             # Vector dimensions (default: 384)
--m [8,16,32,64]          # HNSW M parameter (default: 32)
--ef-construction N        # Build-time ef (default: 200)
--ef-search N              # Query-time ef (default: 100)
--validate-smallworld      # Measure σ, clustering (default: true)
--benchmark-baseline       # Compare vs hnswlib (default: false)
```

**Example**:
```bash
agentdb simulate hnsw \
  --nodes 1000000 \
  --dimensions 768 \
  --benchmark-baseline
```

**Expected Output**:
- Small-world index (σ): 2.84
- Clustering coefficient: 0.39
- Average path length: 5.1 hops
- Search latency (p50/p95/p99): 61/68/74μs
- QPS: 16,358
- Speedup vs baseline: 8.2x

---

### Multi-Head Attention Analysis

```bash
agentdb simulate attention [options]
```

**Description**: Tests GNN multi-head attention mechanisms for query enhancement. Validated +12.4% recall improvement.

**Validated Configuration**:
- Attention heads: 8 (optimal)
- Forward pass target: 5ms (achieved 3.8ms)
- Convergence: 35 epochs

**Options**:
```bash
--nodes N                  # Node count (default: 100000)
--dimensions D             # Vector dimensions (default: 384)
--heads [4,8,16,32]       # Number of attention heads (default: 8)
--train-epochs N           # Training epochs (default: 50)
--learning-rate F          # Learning rate (default: 0.001)
--validate-transfer        # Test transfer to unseen data (default: true)
```

**Example**:
```bash
agentdb simulate attention \
  --heads 8 \
  --train-epochs 100 \
  --validate-transfer
```

**Expected Output**:
- Query enhancement: +12.4%
- Forward pass latency: 3.8ms
- Convergence: 35 epochs
- Transfer accuracy: 91%
- Attention entropy: 0.72 (balanced)
- Concentration: 67% on top 20% edges

---

### Clustering Analysis

```bash
agentdb simulate clustering [options]
```

**Description**: Community detection algorithms comparison. Louvain validated as optimal with Q=0.758 modularity.

**Validated Configuration**:
- Algorithm: Louvain
- Modularity target: >0.75
- Semantic purity target: >85%

**Options**:
```bash
--nodes N                  # Node count (default: 100000)
--dimensions D             # Vector dimensions (default: 384)
--algorithm [louvain,spectral,hierarchical]  # Algorithm (default: louvain)
--min-modularity F         # Minimum Q (default: 0.75)
--analyze-hierarchy        # Detect hierarchical levels (default: true)
```

**Example**:
```bash
agentdb simulate clustering \
  --algorithm louvain \
  --analyze-hierarchy
```

**Expected Output**:
- Modularity (Q): 0.758
- Semantic purity: 87.2%
- Hierarchical levels: 3-4
- Cluster stability: 97%
- Coverage: 99.8% of nodes

---

### Traversal Optimization

```bash
agentdb simulate traversal [options]
```

**Description**: Search strategy comparison (greedy, beam, A*). Beam-5 + Dynamic-k validated as Pareto optimal.

**Validated Configuration**:
- Strategy: Beam search
- Beam width: 5
- Dynamic-k: 5-20 range

**Options**:
```bash
--nodes N                  # Node count (default: 100000)
--dimensions D             # Vector dimensions (default: 384)
--strategy [greedy,beam,astar,best-first]  # Search strategy
--beam-width N             # Beam width for beam search (default: 5)
--dynamic-k                # Enable adaptive k selection (default: false)
--dynamic-k-min N          # Min k value (default: 5)
--dynamic-k-max N          # Max k value (default: 20)
--pareto-analysis          # Find Pareto frontier (default: true)
```

**Example**:
```bash
agentdb simulate traversal \
  --strategy beam \
  --beam-width 5 \
  --dynamic-k \
  --pareto-analysis
```

**Expected Output**:
- Beam-5 latency: 87.3μs
- Beam-5 recall: 96.8%
- Dynamic-k improvement: -18.4% latency
- Pareto optimal: 3-5 configurations
- Trade-off analysis

---

### Hypergraph Exploration

```bash
agentdb simulate hypergraph [options]
```

**Description**: Multi-agent collaboration patterns using hypergraphs. Validated 73% edge compression.

**Validated Configuration**:
- Max hyperedge size: 3-7 nodes
- Compression target: >70%
- Query latency target: <15ms

**Options**:
```bash
--nodes N                  # Node count (default: 100000)
--dimensions D             # Vector dimensions (default: 384)
--max-hyperedge-size N     # Max nodes per hyperedge (default: 5)
--collaboration-patterns   # Test hierarchical/peer patterns (default: true)
--neo4j-export             # Export Cypher queries (default: false)
```

**Example**:
```bash
agentdb simulate hypergraph \
  --max-hyperedge-size 7 \
  --collaboration-patterns \
  --neo4j-export
```

**Expected Output**:
- Edge compression: 73% reduction
- Hyperedge size distribution: 3-7 nodes
- Query latency (3-node): 12.4ms
- Collaboration coverage: 96.2%
- Cypher query examples

---

### Self-Organizing HNSW

```bash
agentdb simulate self-organizing [options]
```

**Description**: 30-day performance stability simulation. MPC adaptation validated at 97.9% degradation prevention.

**Validated Configuration**:
- Adaptation: MPC (Model Predictive Control)
- Monitoring interval: 100ms
- Deletion rate: 10%/day

**Options**:
```bash
--nodes N                  # Node count (default: 100000)
--dimensions D             # Vector dimensions (default: 384)
--days N                   # Simulation duration (default: 30)
--deletion-rate F          # Daily deletion % (default: 0.1)
--adaptation [mpc,reactive,online,evolutionary,none]  # Strategy
--monitoring-interval-ms N # Adaptation interval (default: 100)
```

**Example**:
```bash
agentdb simulate self-organizing \
  --days 30 \
  --deletion-rate 0.1 \
  --adaptation mpc
```

**Expected Output**:
- Day 1 latency: 94.2μs
- Day 30 latency: 96.2μs (+2.1%)
- Degradation prevented: 97.9%
- Self-healing events: 124
- Reconnected edges: 6,184

---

### Neural Augmentation

```bash
agentdb simulate neural [options]
```

**Description**: Full neural pipeline testing (GNN + RL + Joint Opt). Validated +29.4% improvement.

**Validated Configuration**:
- GNN edges: Enabled (-18% memory)
- RL navigation: Enabled (-26% hops)
- Joint optimization: Enabled (+9.1%)

**Options**:
```bash
--nodes N                  # Node count (default: 100000)
--dimensions D             # Vector dimensions (default: 384)
--gnn-edges                # Enable GNN edge selection (default: true)
--rl-navigation            # Enable RL navigation (default: true)
--joint-optimization       # Enable joint embedding-topology (default: true)
--attention-routing        # Enable attention-based layer routing (default: false)
--train-rl-episodes N      # RL training episodes (default: 1000)
--train-joint-iters N      # Joint opt iterations (default: 10)
```

**Example**:
```bash
agentdb simulate neural \
  --gnn-edges \
  --rl-navigation \
  --joint-optimization \
  --train-rl-episodes 2000
```

**Expected Output**:
- Full pipeline latency: 82.1μs
- Full pipeline recall: 94.7%
- Overall improvement: +29.4%
- GNN edge savings: -18% memory
- RL hop reduction: -26%
- Joint opt improvement: +9.1%

---

### Quantum-Hybrid (Theoretical)

```bash
agentdb simulate quantum [options]
```

**Description**: Theoretical quantum computing integration analysis. Timeline: 2040+ viability.

**Validated Configuration**:
- Grover's algorithm: √N speedup
- Qubit requirement: 1000+ (2040+)
- Current viability: False

**Options**:
```bash
--nodes N                  # Node count (default: 100000)
--dimensions D             # Vector dimensions (default: 384)
--analyze-timeline         # Project viability timeline (default: true)
--qubit-requirements       # Calculate qubit needs (default: true)
```

**Example**:
```bash
agentdb simulate quantum \
  --analyze-timeline \
  --qubit-requirements
```

**Expected Output**:
- Current viability (2025): FALSE
- Near-term viability (2030): 38.2%
- Long-term viability (2040): 84.7%
- Qubit requirements: 1000+
- Theoretical speedup: √N (Grover's)

---

## 🧙 Interactive Modes

### Wizard Mode

```bash
agentdb simulate --wizard
```

**Description**: Interactive step-by-step simulation builder with guided prompts.

**Features**:
- Scenario selection with descriptions
- Parameter validation
- Real-time configuration preview
- Save/load configurations
- Inline help system

**Keyboard Shortcuts**:
- `↑/↓`: Navigate options
- `Enter`: Confirm
- `Space`: Toggle (checkboxes)
- `?`: Show help
- `i`: Show info panel
- `Ctrl+C`: Exit

**Example**:
```bash
agentdb simulate --wizard

# Or with pre-selected mode
agentdb simulate --wizard --mode custom
```

---

### Custom Builder

```bash
agentdb simulate --custom [component-options]
```

**Description**: Build simulations by composing validated components.

**Component Options**:

#### Backend Selection
```bash
--backend [ruvector|hnswlib|faiss]  # Default: ruvector
```

#### Attention Configuration
```bash
--attention-heads [4|8|16|32]       # Default: 8
--attention-gnn                     # Enable GNN attention
--attention-none                    # Disable attention
```

#### Search Strategy
```bash
--search [greedy|beam|astar]        # Strategy type
--search-beam-width N               # Beam width (default: 5)
--search-dynamic-k                  # Enable adaptive k
```

#### Clustering
```bash
--cluster [louvain|spectral|hierarchical|none]  # Default: louvain
```

#### Self-Healing
```bash
--self-healing [mpc|reactive|online|none]  # Default: mpc
```

#### Neural Features
```bash
--neural-edges                      # GNN edge selection
--neural-navigation                 # RL navigation
--neural-joint                      # Joint optimization
--neural-attention-routing          # Attention-based routing
--neural-full                       # All neural features
```

**Example**:
```bash
agentdb simulate --custom \
  --backend ruvector \
  --attention-heads 8 \
  --search beam \
  --search-beam-width 5 \
  --search-dynamic-k \
  --cluster louvain \
  --self-healing mpc \
  --neural-full
```

---

## ⚙️ Global Options

### Dataset Configuration

```bash
--nodes N                  # Number of vectors (default: 100000)
--dimensions D             # Vector dimensions (default: 384)
--distance [cosine|euclidean|dot]  # Distance metric (default: cosine)
```

**Common Dimension Values**:
- 128: Lightweight embeddings
- 384: BERT-base, sentence transformers
- 768: BERT-large, OpenAI ada-002
- 1536: OpenAI text-embedding-3

---

### Execution Configuration

```bash
--iterations N             # Number of runs (default: 3)
--seed N                   # Random seed for reproducibility
--parallel                 # Enable parallel execution (default: true)
--threads N                # Thread count (default: CPU cores)
```

---

### Output Configuration

```bash
--output PATH              # Report output directory (default: ./reports/)
--format [md|json|html]    # Report format (default: md)
--quiet                    # Suppress console output
--verbose                  # Detailed logging
--no-spinner               # Disable progress spinners
--simple                   # Simple text output (no colors)
```

---

### Report Options

```bash
--report-title TEXT        # Custom report title
--report-author TEXT       # Report author name
--report-timestamp         # Include timestamp in filename (default: true)
--report-compare PATH      # Compare with existing report
```

---

## 📁 Configuration Management

### Save Configuration

```bash
agentdb simulate [scenario] --save-config NAME
```

**Example**:
```bash
agentdb simulate hnsw \
  --nodes 1000000 \
  --dimensions 768 \
  --save-config large-hnsw
```

**Saved to**: `~/.agentdb/configs/large-hnsw.json`

---

### Load Configuration

```bash
agentdb simulate --config NAME
```

**Example**:
```bash
agentdb simulate --config large-hnsw
```

---

### List Configurations

```bash
agentdb simulate --list-configs
```

**Output**:
```
Saved Configurations:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✓ large-hnsw         (hnsw, 1M nodes, 768d)
✓ production-neural  (neural, full pipeline)
✓ latency-critical   (custom, beam-2 + rl)
```

---

### Export/Import Configurations

```bash
# Export to file
agentdb simulate --config NAME --export config.json

# Import from file
agentdb simulate --import config.json
```

---

## 📊 Report Management

### View Latest Report

```bash
agentdb simulate --report latest
```

---

### View Specific Report

```bash
agentdb simulate --report [id|filename]
```

**Examples**:
```bash
agentdb simulate --report hnsw-exploration-2025-11-30
agentdb simulate --report ./reports/custom-config.md
```

---

### List All Reports

```bash
agentdb simulate --list-reports
```

**Output**:
```
Recent Simulation Reports:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
⭐ hnsw-exploration-2025-11-30-143522.md      (4.5s ago)
  neural-augmentation-2025-11-30-142134.md   (15m ago)
  custom-config-2025-11-30-135842.md         (48m ago)
  traversal-optimization-2025-11-29-182341.md (Yesterday)

Total: 24 reports
```

---

### Compare Reports

```bash
agentdb simulate --compare REPORT1 REPORT2
```

**Example**:
```bash
agentdb simulate --compare \
  baseline-hnsw.md \
  optimized-hnsw.md
```

**Output**:
```
Report Comparison:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Metric          │ Baseline   │ Optimized  │ Δ
────────────────┼────────────┼────────────┼──────
Latency         │ 498.3μs    │ 61.2μs     │ -87.7%
Recall@10       │ 95.6%      │ 96.8%      │ +1.2%
Memory          │ 184 MB     │ 151 MB     │ -17.9%
QPS             │ 2,007      │ 16,358     │ +715%
```

---

### Delete Reports

```bash
agentdb simulate --delete-report [id|all]
```

**Example**:
```bash
# Delete specific report
agentdb simulate --delete-report hnsw-exploration-2025-11-30

# Delete all reports older than 30 days
agentdb simulate --delete-reports --older-than 30d
```

---

## 🚀 Advanced Usage

### Benchmark Mode

```bash
agentdb simulate [scenario] --benchmark
```

**Features**:
- Runs 10 iterations for high confidence
- Compares against all baselines (hnswlib, FAISS)
- Generates comprehensive performance report
- Includes statistical analysis

**Example**:
```bash
agentdb simulate hnsw --benchmark
```

---

### Stress Test Mode

```bash
agentdb simulate [scenario] --stress-test
```

**Features**:
- Tests with increasing dataset sizes
- Identifies performance cliffs
- Validates scaling predictions
- Generates scaling charts

**Example**:
```bash
agentdb simulate hnsw \
  --stress-test \
  --stress-test-sizes "10k,100k,1M,10M"
```

---

### CI/CD Integration

```bash
# Non-interactive mode
agentdb simulate [scenario] \
  --ci-mode \
  --fail-threshold "latency>100us,recall<95%"
```

**Features**:
- No prompts (fully automated)
- Exit code 1 if thresholds exceeded
- JSON output for parsing

**Example**:
```bash
agentdb simulate hnsw \
  --ci-mode \
  --fail-threshold "latency>100us,recall<95%" \
  --format json \
  --output ./ci-reports/
```

---

### Environment Variables

```bash
# Default configuration
export AGENTDB_DEFAULT_NODES=100000
export AGENTDB_DEFAULT_DIMENSIONS=384
export AGENTDB_DEFAULT_ITERATIONS=3

# Output configuration
export AGENTDB_REPORT_DIR=./my-reports/
export AGENTDB_REPORT_FORMAT=json

# Behavior
export AGENTDB_VERBOSE=1
export AGENTDB_NO_SPINNER=1

agentdb simulate hnsw
```

---

## 📝 Examples

### Quick Validation

```bash
# Run HNSW with defaults
agentdb simulate hnsw
```

---

### Production Benchmarking

```bash
# High-confidence benchmark
agentdb simulate hnsw \
  --nodes 1000000 \
  --dimensions 768 \
  --iterations 10 \
  --benchmark \
  --output ./production-reports/ \
  --report-title "Production HNSW Benchmark"
```

---

### Custom Optimal Config

```bash
# Build optimal configuration
agentdb simulate --custom \
  --backend ruvector \
  --attention-heads 8 \
  --search beam 5 \
  --search-dynamic-k \
  --cluster louvain \
  --self-healing mpc \
  --neural-edges \
  --nodes 1000000 \
  --iterations 5 \
  --save-config production-optimal
```

---

### Compare Configurations

```bash
# Baseline
agentdb simulate hnsw \
  --output ./compare/baseline.md

# Optimized
agentdb simulate --config production-optimal \
  --output ./compare/optimized.md

# Compare
agentdb simulate --compare \
  ./compare/baseline.md \
  ./compare/optimized.md
```

---

### CI Pipeline

```bash
# .github/workflows/benchmark.yml
agentdb simulate hnsw \
  --ci-mode \
  --iterations 10 \
  --fail-threshold "latency>100us,recall<95%,coherence<95%" \
  --format json \
  --output ./ci-reports/hnsw-${CI_COMMIT_SHA}.json
```

---

## 🔍 Help System

### General Help

```bash
agentdb simulate --help
```

---

### Scenario-Specific Help

```bash
agentdb simulate [scenario] --help
```

**Example**:
```bash
agentdb simulate hnsw --help
```

---

### Component Help

```bash
agentdb simulate --custom --help
```

**Shows**:
- All component options
- Validated optimal values
- Performance impact of each component

---

## 📚 See Also

- **[Quick Start Guide](QUICK-START.md)** - Get started in 5 minutes
- **[Custom Simulations](CUSTOM-SIMULATIONS.md)** - Component reference
- **[Wizard Guide](WIZARD-GUIDE.md)** - Interactive builder
- **[Troubleshooting](TROUBLESHOOTING.md)** - Common issues

---

## 📜 Version History

### v2.0.0 (2025-11-30)
- Added 8 validated scenarios
- Interactive wizard mode
- Custom simulation builder
- Report management system
- Configuration save/load
- CI/CD integration
- Comprehensive documentation

---

**Need help?** Check **[Troubleshooting Guide →](TROUBLESHOOTING.md)** or open an issue on GitHub.
