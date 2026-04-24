# AgentDB Simulation Quick Start Guide

**Reading Time**: 5 minutes
**Prerequisites**: Node.js 18+, npm or yarn
**Target Audience**: New users

Get up and running with AgentDB simulations in 5 minutes. This guide covers installation, running your first simulation, and understanding the results.

---

## 🚀 Installation

### Option 1: Global Installation (Recommended)
```bash
npm install -g agentdb
agentdb --version
```

### Option 2: Local Development
```bash
git clone https://github.com/ruvnet/agentic-flow.git
cd agentic-flow/packages/agentdb
npm install
npm run build
npm link
```

### Verify Installation
```bash
agentdb simulate --help
```

You should see the simulation command help with available scenarios.

---

## 🎯 Run Your First Simulation (3 Methods)

### Method 1: Interactive Wizard (Easiest) ⭐

The wizard guides you through simulation creation step-by-step:

```bash
agentdb simulate --wizard
```

**What you'll see**:
```
🧙 AgentDB Simulation Wizard

? What would you like to do?
  ❯ 🎯 Run validated scenario (recommended)
    🔧 Build custom simulation
    📊 View past reports

? Choose a simulation scenario:
  ❯ ⚡ HNSW Exploration (8.2x speedup)
    🧠 Attention Analysis (12.4% improvement)
    🎯 Traversal Optimization (96.8% recall)
    🔄 Self-Organizing (97.9% uptime)
    ...

? Number of nodes: 100000
? Vector dimensions: 384
? Number of runs (for coherence): 3
? Use optimal validated configuration? Yes

📋 Simulation Configuration:
   Scenario: hnsw
   Nodes: 100,000
   Dimensions: 384
   Iterations: 3
   ✅ Using optimal validated parameters

? Start simulation? Yes

🚀 Running simulation...
```

### Method 2: Quick Command (Fastest)

Run a validated scenario with optimal defaults:

```bash
agentdb simulate hnsw --iterations 3
```

**What happens**:
- Executes HNSW graph topology simulation
- Runs 3 iterations for coherence validation
- Uses optimal configuration (M=32, ef=200)
- Generates markdown report in `./reports/`

### Method 3: Custom Configuration (Advanced)

Build your own simulation from components:

```bash
agentdb simulate --custom \
  --backend ruvector \
  --attention-heads 8 \
  --search beam 5 \
  --cluster louvain \
  --self-healing mpc \
  --iterations 3
```

**👉 [See Custom Simulations Guide for all options →](CUSTOM-SIMULATIONS.md)**

---

## 📊 Understanding the Output

### Console Output

During execution, you'll see real-time progress:

```
🚀 AgentDB Latent Space Simulation
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📋 Scenario: HNSW Graph Topology Exploration
⚙️  Configuration: M=32, efConstruction=200, efSearch=100

🔄 Iteration 1/3
  ├─ Building graph... [████████████] 100% (2.3s)
  ├─ Running queries... [████████████] 100% (1.8s)
  ├─ Analyzing topology... [████████████] 100% (0.4s)
  └─ ✅ Complete: 61.2μs latency, 96.8% recall

🔄 Iteration 2/3
  └─ ✅ Complete: 60.8μs latency, 96.9% recall

🔄 Iteration 3/3
  └─ ✅ Complete: 61.4μs latency, 96.7% recall

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ Simulation Complete!

📊 Summary:
   Average Latency: 61.2μs (8.2x vs baseline)
   Recall@10: 96.8%
   Coherence: 98.4% (highly consistent)
   Memory: 151 MB

📄 Report saved: ./reports/hnsw-exploration-2025-11-30.md
```

### Report File Structure

The generated markdown report contains:

```markdown
# HNSW Graph Topology Exploration - Results

## Executive Summary
- Speedup: 8.2x vs hnswlib
- Latency: 61.2μs average
- Recall@10: 96.8%

## Configuration
[Details of M, ef parameters]

## Performance Metrics
[Latency distribution, QPS, memory]

## Graph Properties
- Small-world index (σ): 2.84 ✅
- Clustering coefficient: 0.39
- Average path length: 5.1 hops

## Coherence Analysis
[Variance across 3 runs]

## Recommendations
[Production deployment suggestions]
```

---

## 🎓 Understanding Key Metrics

### Latency
**What it means**: How long one search query takes
**Good value**: <100μs for real-time applications
**Your result**: 61.2μs ✅ Excellent

### Recall@10
**What it means**: % of correct results in top 10
**Good value**: >95%
**Your result**: 96.8% ✅ High accuracy

### Speedup
**What it means**: How many times faster than baseline (hnswlib)
**Good value**: >2x
**Your result**: 8.2x ✅ Industry-leading

### Coherence
**What it means**: Consistency across multiple runs
**Good value**: >95%
**Your result**: 98.4% ✅ Highly reproducible

### Small-World Index (σ)
**What it means**: Graph has "small-world" properties (fast navigation)
**Good value**: 2.5-3.5
**Your result**: 2.84 ✅ Optimal range

---

## 🏆 What You Accomplished

You just:
1. ✅ Installed AgentDB simulation CLI
2. ✅ Ran a production-grade vector database benchmark
3. ✅ Validated that RuVector is **8.2x faster** than industry baseline
4. ✅ Generated a comprehensive performance report

**Total time**: ~5 minutes (including 4.5s simulation execution)

---

## 📈 Next Steps

### Explore Other Scenarios

Try the other 7 validated scenarios:

```bash
# Multi-head attention analysis (12.4% improvement)
agentdb simulate attention

# Search strategy optimization (96.8% recall)
agentdb simulate traversal

# 30-day self-healing simulation (97.9% uptime)
agentdb simulate self-organizing

# Full neural augmentation (29.4% boost)
agentdb simulate neural
```

### Build Custom Configurations

Learn to compose optimal configurations:

```bash
# Memory-constrained setup
agentdb simulate --custom \
  --backend ruvector \
  --attention-heads 8 \
  --neural-edges \
  --cluster louvain

# Latency-critical setup
agentdb simulate --custom \
  --backend ruvector \
  --search beam 5 \
  --search dynamic-k \
  --neural-navigation
```

**👉 [See Custom Simulations Guide →](CUSTOM-SIMULATIONS.md)**

### Deep Dive into Results

Understand the research behind the numbers:

- **[Master Synthesis Report](../reports/latent-space/MASTER-SYNTHESIS.md)** - Cross-simulation analysis
- **[Individual Reports](../reports/latent-space/)** - Detailed findings for each scenario
- **[Optimization Strategy](../architecture/OPTIMIZATION-STRATEGY.md)** - How to tune for your use case

---

## 🛠️ Common Options

### Change Dataset Size
```bash
agentdb simulate hnsw --nodes 1000000 --dimensions 768
```

### Run More Iterations (Better Coherence)
```bash
agentdb simulate hnsw --iterations 10
```

### Custom Report Path
```bash
agentdb simulate hnsw --output ./my-reports/
```

### JSON Output
```bash
agentdb simulate hnsw --format json
```

### Verbose Logging
```bash
agentdb simulate hnsw --verbose
```

**👉 [See Complete CLI Reference →](CLI-REFERENCE.md)**

---

## ❓ Troubleshooting

### "Command not found: agentdb"
```bash
# Verify installation
npm list -g agentdb

# Reinstall if needed
npm install -g agentdb --force
```

### Simulation Runs Too Slowly
```bash
# Reduce dataset size for faster testing
agentdb simulate hnsw --nodes 10000 --iterations 1
```

### Out of Memory Errors
```bash
# Use smaller dimensions or fewer nodes
agentdb simulate hnsw --nodes 50000 --dimensions 128
```

**👉 [See Full Troubleshooting Guide →](TROUBLESHOOTING.md)**

---

## 📚 Learn More

### User Guides
- **[Wizard Guide](WIZARD-GUIDE.md)** - Interactive simulation builder
- **[Custom Simulations](CUSTOM-SIMULATIONS.md)** - Component reference
- **[CLI Reference](CLI-REFERENCE.md)** - All commands and options

### Technical Docs
- **[Simulation Architecture](../architecture/SIMULATION-ARCHITECTURE.md)** - TypeScript implementation
- **[Optimization Strategy](../architecture/OPTIMIZATION-STRATEGY.md)** - Performance tuning

### Research
- **[Latent Space Reports](../reports/latent-space/README.md)** - Executive summary
- **[Master Synthesis](../reports/latent-space/MASTER-SYNTHESIS.md)** - Complete analysis

---

## 🎉 You're Ready!

You now have the tools to:
- ✅ Run production-grade vector database benchmarks
- ✅ Validate performance optimizations
- ✅ Compare configurations
- ✅ Generate comprehensive reports

**Start exploring**: Try different scenarios and configurations to find the optimal setup for your use case.

---

**Questions?** Check the **[Troubleshooting Guide →](TROUBLESHOOTING.md)** or open an issue on GitHub.
