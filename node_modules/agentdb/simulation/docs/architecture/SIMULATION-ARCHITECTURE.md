# AgentDB Simulation Architecture

**Version**: 2.0.0
**Last Updated**: 2025-11-30
**Target Audience**: Developers extending the simulation system

This document describes the TypeScript architecture of AgentDB's latent space simulation system, including design patterns, extension points, and how to add custom scenarios or components.

---

## 🏗️ Architecture Overview

```
packages/agentdb/simulation/
├── scenarios/                    # Simulation implementations
│   └── latent-space/
│       ├── attention-analysis.ts
│       ├── hnsw-exploration.ts
│       ├── clustering-analysis.ts
│       ├── traversal-optimization.ts
│       ├── hypergraph-exploration.ts
│       ├── self-organizing-hnsw.ts
│       ├── neural-augmentation.ts
│       ├── quantum-hybrid.ts
│       ├── types.ts              # Shared TypeScript interfaces
│       └── index.ts              # Scenario registry
├── src/
│   └── cli/
│       ├── commands/
│       │   ├── simulate.ts       # Main CLI command
│       │   ├── simulate-wizard.ts
│       │   ├── simulate-custom.ts
│       │   └── simulate-report.ts
│       └── lib/
│           ├── simulation-runner.ts
│           ├── config-validator.ts
│           ├── report-generator.ts
│           └── help-formatter.ts
├── tests/
│   └── latent-space/
│       └── [test files].test.ts
└── docs/
    ├── guides/
    ├── architecture/ (this file)
    └── reports/
```

---

## 🎯 Core Concepts

### 1. Simulation Scenario

A **scenario** is a complete benchmark test that:
1. Configures a vector database setup
2. Executes operations (inserts, queries, updates)
3. Measures performance metrics
4. Generates a comprehensive report

**Example**: `hnsw-exploration.ts` tests HNSW graph topology and small-world properties.

---

### 2. Component

A **component** is a reusable building block like:
- Backend (RuVector, hnswlib, FAISS)
- Attention mechanism (4/8/16-head GNN)
- Search strategy (greedy, beam, A*)
- Clustering algorithm (Louvain, spectral)
- Self-healing policy (MPC, reactive)
- Neural feature (GNN edges, RL navigation)

**Components are composable** via the custom builder.

---

### 3. Report

A **report** is a structured document (Markdown, JSON, or HTML) containing:
- Executive summary
- Configuration details
- Performance metrics
- Coherence analysis
- Recommendations

---

## 📦 TypeScript Type System

### Core Interfaces (`scenarios/latent-space/types.ts`)

```typescript
/**
 * Base interface for all simulation scenarios
 */
export interface SimulationScenario {
  /** Unique scenario identifier */
  id: string;

  /** Human-readable name */
  name: string;

  /** Category for organization */
  category: string;

  /** Brief description */
  description: string;

  /** Expected duration in seconds */
  expectedDuration: number;

  /** Run the simulation */
  run(config: SimulationConfig): Promise<SimulationReport>;

  /** Validate configuration before execution */
  validate(config: SimulationConfig): ValidationResult;
}

/**
 * Simulation configuration
 */
export interface SimulationConfig {
  /** Number of vectors */
  nodes: number;

  /** Vector dimensions */
  dimensions: number;

  /** Number of iterations for coherence */
  iterations: number;

  /** Backend selection */
  backend: 'ruvector' | 'hnswlib' | 'faiss';

  /** HNSW parameters */
  hnsw?: HNSWConfig;

  /** Attention configuration */
  attention?: AttentionConfig;

  /** Search strategy */
  search?: SearchConfig;

  /** Clustering algorithm */
  clustering?: ClusteringConfig;

  /** Self-healing policy */
  selfHealing?: SelfHealingConfig;

  /** Neural augmentation features */
  neural?: NeuralConfig;

  /** Output options */
  output?: OutputConfig;
}

/**
 * Simulation results
 */
export interface SimulationReport {
  /** Scenario metadata */
  scenario: {
    id: string;
    name: string;
    timestamp: Date;
  };

  /** Configuration used */
  config: SimulationConfig;

  /** Performance metrics */
  metrics: PerformanceMetrics;

  /** Coherence analysis */
  coherence: CoherenceAnalysis;

  /** Recommendations */
  recommendations: string[];

  /** Raw iteration data */
  iterations: IterationResult[];
}

/**
 * Performance metrics
 */
export interface PerformanceMetrics {
  /** Latency statistics */
  latency: {
    p50: number;  // microseconds
    p95: number;
    p99: number;
    mean: number;
    stddev: number;
  };

  /** Recall at different k values */
  recall: {
    k10: number;  // Recall@10
    k50: number;
    k100: number;
  };

  /** Queries per second */
  qps: number;

  /** Memory usage in MB */
  memory: number;

  /** Graph properties (for HNSW) */
  graph?: GraphProperties;
}

/**
 * HNSW graph properties
 */
export interface GraphProperties {
  /** Small-world index */
  smallWorldIndex: number;  // σ value

  /** Clustering coefficient */
  clusteringCoefficient: number;

  /** Average path length */
  avgPathLength: number;

  /** Modularity */
  modularity: number;

  /** Layer distribution */
  layerDistribution: number[];
}

/**
 * Coherence analysis across iterations
 */
export interface CoherenceAnalysis {
  /** Overall coherence score (0-1) */
  score: number;

  /** Variance in latency */
  latencyVariance: number;

  /** Variance in recall */
  recallVariance: number;

  /** Statistical significance */
  pValue: number;
}
```

---

## 🔌 Extension Points

### Adding a New Simulation Scenario

**Step 1**: Create TypeScript file

Create `packages/agentdb/simulation/scenarios/my-category/my-simulation.ts`:

```typescript
import { SimulationScenario, SimulationConfig, SimulationReport } from '../latent-space/types';

export class MySimulation implements SimulationScenario {
  id = 'my-simulation';
  name = 'My Custom Simulation';
  category = 'my-category';
  description = 'Tests my custom feature';
  expectedDuration = 5.2; // seconds

  async run(config: SimulationConfig): Promise<SimulationReport> {
    // 1. Initialize database
    const db = await this.initializeDatabase(config);

    // 2. Insert vectors
    const vectors = this.generateVectors(config.nodes, config.dimensions);
    await db.insertBatch(vectors);

    // 3. Run queries
    const queries = this.generateQueries(1000);
    const results = await this.runQueries(db, queries);

    // 4. Measure performance
    const metrics = this.calculateMetrics(results);

    // 5. Generate report
    return {
      scenario: {
        id: this.id,
        name: this.name,
        timestamp: new Date(),
      },
      config,
      metrics,
      coherence: this.calculateCoherence(results),
      recommendations: this.generateRecommendations(metrics),
      iterations: results,
    };
  }

  validate(config: SimulationConfig): ValidationResult {
    if (config.nodes < 1000) {
      return {
        valid: false,
        errors: ['Minimum 1000 nodes required for my-simulation'],
      };
    }
    return { valid: true };
  }

  private async initializeDatabase(config: SimulationConfig) {
    // Implementation
  }

  private generateVectors(nodes: number, dimensions: number): Float32Array[] {
    // Implementation
  }

  // ... other helper methods
}
```

---

**Step 2**: Register in index

Edit `packages/agentdb/simulation/scenarios/latent-space/index.ts`:

```typescript
import { HNSWExploration } from './hnsw-exploration';
import { AttentionAnalysis } from './attention-analysis';
// ... other imports
import { MySimulation } from '../my-category/my-simulation';

export const SCENARIOS = {
  'hnsw': new HNSWExploration(),
  'attention': new AttentionAnalysis(),
  // ... other scenarios
  'my-simulation': new MySimulation(),
};

export function getScenario(id: string): SimulationScenario | undefined {
  return SCENARIOS[id];
}

export function listScenarios(): SimulationScenario[] {
  return Object.values(SCENARIOS);
}
```

---

**Step 3**: Add CLI integration

Edit `packages/agentdb/src/cli/commands/simulate.ts`:

```typescript
program
  .command('my-simulation')
  .description('My custom simulation')
  .option('--custom-option <value>', 'Custom option')
  .action(async (options) => {
    const scenario = getScenario('my-simulation');
    const config = buildConfig(options);
    const report = await scenario.run(config);
    await saveReport(report);
  });
```

---

**Step 4**: Add tests

Create `packages/agentdb/simulation/tests/my-category/my-simulation.test.ts`:

```typescript
import { MySimulation } from '../../scenarios/my-category/my-simulation';
import { SimulationConfig } from '../../scenarios/latent-space/types';

describe('MySimulation', () => {
  let simulation: MySimulation;

  beforeEach(() => {
    simulation = new MySimulation();
  });

  test('should validate config', () => {
    const config: SimulationConfig = {
      nodes: 10000,
      dimensions: 384,
      iterations: 3,
      backend: 'ruvector',
    };

    const result = simulation.validate(config);
    expect(result.valid).toBe(true);
  });

  test('should run simulation', async () => {
    const config: SimulationConfig = {
      nodes: 1000, // Small for tests
      dimensions: 128,
      iterations: 1,
      backend: 'ruvector',
    };

    const report = await simulation.run(config);

    expect(report.metrics.latency.mean).toBeLessThan(200); // μs
    expect(report.metrics.recall.k10).toBeGreaterThan(0.90);
  });
});
```

---

### Adding a New Component

**Example**: Adding a new search strategy

**Step 1**: Define interface

Edit `scenarios/latent-space/types.ts`:

```typescript
export interface SearchStrategy {
  name: string;
  search(query: Float32Array, graph: HNSWGraph, k: number): Promise<Neighbor[]>;
}
```

---

**Step 2**: Implement strategy

Create `scenarios/latent-space/components/my-search.ts`:

```typescript
import { SearchStrategy, HNSWGraph, Neighbor } from '../types';

export class MySearchStrategy implements SearchStrategy {
  name = 'my-search';

  async search(
    query: Float32Array,
    graph: HNSWGraph,
    k: number
  ): Promise<Neighbor[]> {
    // 1. Start from entry point
    let current = graph.entryPoint;
    const visited = new Set<number>();
    const candidates: Neighbor[] = [];

    // 2. Navigate graph using your algorithm
    while (candidates.length < k) {
      // Your search logic here
      // Example: Use custom heuristic
      const neighbors = graph.getNeighbors(current);
      for (const neighbor of neighbors) {
        if (!visited.has(neighbor.id)) {
          const distance = this.computeDistance(query, graph.vectors[neighbor.id]);
          candidates.push({ id: neighbor.id, distance });
          visited.add(neighbor.id);
        }
      }

      // Select next node to visit
      current = this.selectNext(candidates);
    }

    // 3. Return top-k results
    return candidates
      .sort((a, b) => a.distance - b.distance)
      .slice(0, k);
  }

  private computeDistance(a: Float32Array, b: Float32Array): number {
    // Cosine, Euclidean, or custom distance
  }

  private selectNext(candidates: Neighbor[]): number {
    // Your selection heuristic
  }
}
```

---

**Step 3**: Register component

Edit `scenarios/latent-space/components/index.ts`:

```typescript
import { GreedySearch } from './greedy-search';
import { BeamSearch } from './beam-search';
import { MySearchStrategy } from './my-search';

export const SEARCH_STRATEGIES = {
  'greedy': new GreedySearch(),
  'beam': new BeamSearch(),
  'my-search': new MySearchStrategy(),
};

export function getSearchStrategy(name: string): SearchStrategy {
  return SEARCH_STRATEGIES[name];
}
```

---

**Step 4**: Add CLI option

Edit `src/cli/commands/simulate-custom.ts`:

```typescript
program
  .command('custom')
  .option('--search [strategy]', 'Search strategy: greedy|beam|astar|my-search', 'beam')
  .action(async (options) => {
    const strategy = getSearchStrategy(options.search);
    // Use strategy in simulation
  });
```

---

## 🧪 Testing Architecture

### Test Structure

```
tests/
├── unit/                         # Unit tests for components
│   ├── search-strategies.test.ts
│   ├── clustering.test.ts
│   └── neural-components.test.ts
├── integration/                  # Integration tests
│   ├── hnsw-integration.test.ts
│   └── cli-integration.test.ts
└── e2e/                         # End-to-end tests
    ├── full-simulation.test.ts
    └── wizard.test.ts
```

---

### Example Unit Test

```typescript
import { BeamSearch } from '../../scenarios/latent-space/components/beam-search';

describe('BeamSearch', () => {
  test('should find k nearest neighbors', async () => {
    const search = new BeamSearch(5); // beam width 5
    const graph = createMockGraph();
    const query = new Float32Array([0.1, 0.2, 0.3, ...]);

    const results = await search.search(query, graph, 10);

    expect(results.length).toBe(10);
    expect(results[0].distance).toBeLessThanOrEqual(results[1].distance); // sorted
  });

  test('should achieve >95% recall', async () => {
    const search = new BeamSearch(5);
    const graph = createMockGraph();

    const recall = await measureRecall(search, graph, 1000);

    expect(recall).toBeGreaterThan(0.95);
  });
});
```

---

### Example Integration Test

```typescript
import { HNSWExploration } from '../../scenarios/latent-space/hnsw-exploration';

describe('HNSW Integration', () => {
  test('should complete simulation in <10s', async () => {
    const simulation = new HNSWExploration();
    const config = {
      nodes: 10000,
      dimensions: 384,
      iterations: 3,
      backend: 'ruvector',
    };

    const start = Date.now();
    const report = await simulation.run(config);
    const duration = (Date.now() - start) / 1000;

    expect(duration).toBeLessThan(10);
    expect(report.metrics.latency.mean).toBeLessThan(100);
  });
});
```

---

## 📊 Report Generation Architecture

### Report Generator Interface

```typescript
export interface ReportGenerator {
  format: 'md' | 'json' | 'html' | 'pdf';

  generate(report: SimulationReport): Promise<string>;
}
```

---

### Markdown Report Generator

```typescript
import { ReportGenerator, SimulationReport } from '../types';

export class MarkdownReportGenerator implements ReportGenerator {
  format = 'md' as const;

  async generate(report: SimulationReport): Promise<string> {
    let markdown = '';

    // Header
    markdown += `# ${report.scenario.name} - Results\n\n`;
    markdown += `**Date**: ${report.scenario.timestamp.toISOString()}\n\n`;

    // Executive Summary
    markdown += '## Executive Summary\n\n';
    markdown += `- **Latency**: ${report.metrics.latency.mean.toFixed(1)}μs\n`;
    markdown += `- **Recall@10**: ${(report.metrics.recall.k10 * 100).toFixed(1)}%\n`;
    markdown += `- **QPS**: ${report.metrics.qps.toLocaleString()}\n`;
    markdown += `- **Coherence**: ${(report.coherence.score * 100).toFixed(1)}%\n\n`;

    // Configuration
    markdown += '## Configuration\n\n';
    markdown += '```json\n';
    markdown += JSON.stringify(report.config, null, 2);
    markdown += '\n```\n\n';

    // Performance Metrics
    markdown += '## Performance Metrics\n\n';
    markdown += this.generateMetricsTable(report.metrics);

    // Coherence Analysis
    markdown += '## Coherence Analysis\n\n';
    markdown += this.generateCoherenceSection(report.coherence);

    // Recommendations
    markdown += '## Recommendations\n\n';
    for (const rec of report.recommendations) {
      markdown += `- ${rec}\n`;
    }

    return markdown;
  }

  private generateMetricsTable(metrics: PerformanceMetrics): string {
    return `| Metric | Value |
|--------|-------|
| Latency (p50) | ${metrics.latency.p50.toFixed(1)}μs |
| Latency (p95) | ${metrics.latency.p95.toFixed(1)}μs |
| Latency (p99) | ${metrics.latency.p99.toFixed(1)}μs |
| Recall@10 | ${(metrics.recall.k10 * 100).toFixed(1)}% |
| QPS | ${metrics.qps.toLocaleString()} |
| Memory | ${metrics.memory.toFixed(1)} MB |
\n\n`;
  }

  private generateCoherenceSection(coherence: CoherenceAnalysis): string {
    return `- **Score**: ${(coherence.score * 100).toFixed(1)}% (${this.coherenceLabel(coherence.score)})
- **Latency Variance**: ${coherence.latencyVariance.toFixed(2)}%
- **Recall Variance**: ${coherence.recallVariance.toFixed(2)}%
- **Statistical Significance**: p=${coherence.pValue.toFixed(4)}
\n\n`;
  }

  private coherenceLabel(score: number): string {
    if (score >= 0.98) return 'Excellent';
    if (score >= 0.95) return 'Good';
    if (score >= 0.90) return 'Acceptable';
    return 'Needs Improvement';
  }
}
```

---

## 🔄 Simulation Runner Architecture

### Runner Interface

```typescript
export class SimulationRunner {
  private scenario: SimulationScenario;
  private config: SimulationConfig;
  private progress: ProgressReporter;

  constructor(scenario: SimulationScenario, config: SimulationConfig) {
    this.scenario = scenario;
    this.config = config;
    this.progress = new ProgressReporter();
  }

  async run(): Promise<SimulationReport> {
    // 1. Validate config
    const validation = this.scenario.validate(this.config);
    if (!validation.valid) {
      throw new Error(`Invalid config: ${validation.errors.join(', ')}`);
    }

    // 2. Initialize progress reporting
    this.progress.start(this.scenario.name);

    // 3. Run simulation
    try {
      const report = await this.scenario.run(this.config);
      this.progress.complete();
      return report;
    } catch (error) {
      this.progress.fail(error.message);
      throw error;
    }
  }
}
```

---

## 🎨 Design Patterns Used

### 1. Strategy Pattern

**Used for**: Search strategies, clustering algorithms, self-healing policies

```typescript
interface SearchStrategy {
  search(query: Float32Array, graph: HNSWGraph, k: number): Promise<Neighbor[]>;
}

class BeamSearch implements SearchStrategy { ... }
class GreedySearch implements SearchStrategy { ... }
```

---

### 2. Factory Pattern

**Used for**: Creating scenarios, components, report generators

```typescript
class ScenarioFactory {
  static create(id: string): SimulationScenario {
    switch (id) {
      case 'hnsw': return new HNSWExploration();
      case 'attention': return new AttentionAnalysis();
      default: throw new Error(`Unknown scenario: ${id}`);
    }
  }
}
```

---

### 3. Builder Pattern

**Used for**: Configuration building, custom simulation composition

```typescript
class ConfigBuilder {
  private config: Partial<SimulationConfig> = {};

  nodes(n: number): this {
    this.config.nodes = n;
    return this;
  }

  dimensions(d: number): this {
    this.config.dimensions = d;
    return this;
  }

  backend(b: Backend): this {
    this.config.backend = b;
    return this;
  }

  build(): SimulationConfig {
    // Validate and return
    return this.config as SimulationConfig;
  }
}
```

---

### 4. Observer Pattern

**Used for**: Progress reporting, event monitoring

```typescript
interface ProgressObserver {
  onStart(scenario: string): void;
  onProgress(percent: number): void;
  onComplete(report: SimulationReport): void;
  onError(error: Error): void;
}

class ProgressReporter {
  private observers: ProgressObserver[] = [];

  subscribe(observer: ProgressObserver): void {
    this.observers.push(observer);
  }

  notifyProgress(percent: number): void {
    for (const observer of this.observers) {
      observer.onProgress(percent);
    }
  }
}
```

---

## 🚀 Performance Optimization

### 1. Lazy Loading

```typescript
// Load scenarios only when needed
const SCENARIOS = {
  get hnsw() { return require('./hnsw-exploration').HNSWExploration; },
  get attention() { return require('./attention-analysis').AttentionAnalysis; },
};
```

---

### 2. Worker Threads (for parallel iterations)

```typescript
import { Worker } from 'worker_threads';

async function runParallelIterations(config: SimulationConfig): Promise<IterationResult[]> {
  const workers = [];
  for (let i = 0; i < config.iterations; i++) {
    workers.push(new Worker('./iteration-worker.js', { workerData: config }));
  }

  return Promise.all(workers.map(w => new Promise((resolve) => {
    w.on('message', resolve);
  })));
}
```

---

### 3. Memory Pooling

```typescript
class VectorPool {
  private pool: Float32Array[] = [];

  acquire(size: number): Float32Array {
    return this.pool.pop() || new Float32Array(size);
  }

  release(vector: Float32Array): void {
    this.pool.push(vector);
  }
}
```

---

## 📚 Further Reading

- **[Optimization Strategy](OPTIMIZATION-STRATEGY.md)** - Performance tuning guide
- **[Custom Simulations Guide](../guides/CUSTOM-SIMULATIONS.md)** - Component reference
- **[CLI Reference](../guides/CLI-REFERENCE.md)** - Command-line usage

---

**Ready to extend?** Check the **[Component Reference →](../guides/CUSTOM-SIMULATIONS.md#complete-component-reference)**
