# AgentDB Extension API v2.0

## Overview

This document defines the Extension API for creating custom simulation scenarios, components, and integrations with AgentDB v2.0.

---

## Table of Contents

1. [Creating Custom Scenarios](#creating-custom-scenarios)
2. [Component Interfaces](#component-interfaces)
3. [Plugin System](#plugin-system)
4. [Event System](#event-system)
5. [Code Examples](#code-examples)

---

## Creating Custom Scenarios

### Scenario Interface

All simulation scenarios must implement the `SimulationScenario` interface:

```typescript
interface SimulationScenario {
  metadata: SimulationMetadata;
  execute(config: AgentDBConfig): Promise<SimulationResult>;
  validate?(config: AgentDBConfig): ValidationResult;
  cleanup?(): Promise<void>;
}

interface SimulationMetadata {
  id: string;
  name: string;
  version: string;
  category: 'core' | 'experimental' | 'plugin';
  description: string;
  author?: string;
  agentdbVersion: string; // Semver range (e.g., "^2.0.0")
  tags?: string[];
  estimatedDuration?: number; // milliseconds
  requiredMemoryMB?: number;
}

interface AgentDBConfig {
  profile: 'production' | 'memory' | 'latency' | 'recall' | 'custom';
  hnsw: { M: number; efConstruction: number; efSearch: number };
  attention: { heads: number; dimension: number };
  traversal: { beamWidth: number; strategy: 'greedy' | 'beam' | 'dynamic' };
  clustering: { algorithm: 'louvain' | 'leiden' | 'spectral'; resolution: number };
  neural: { mode: 'none' | 'gnn-only' | 'full'; reinforcementLearning: boolean };
  hypergraph: { enabled: boolean; maxEdgeSize: number };
  storage: { reportPath: string; autoBackup: boolean };
  monitoring: { enabled: boolean; alertThresholds: { memoryMB: number; latencyMs: number } };
}

interface SimulationResult {
  scenario: string;
  timestamp: Date;
  config: AgentDBConfig;
  metrics: {
    recall: number;
    latency: number;
    throughput: number;
    memoryUsage: number;
    [key: string]: any;
  };
  insights: string[];
  recommendations: string[];
  iterations?: number;
  duration?: number;
}

interface ValidationResult {
  valid: boolean;
  errors?: string[];
  warnings?: string[];
}
```

### Minimal Example

```typescript
// ~/.agentdb/plugins/my-scenario/index.ts
import { SimulationScenario, SimulationResult, AgentDBConfig } from 'agentdb';

export const myScenario: SimulationScenario = {
  metadata: {
    id: 'my-custom-scenario',
    name: 'My Custom Scenario',
    version: '1.0.0',
    category: 'plugin',
    description: 'Custom simulation for specific use case',
    author: 'Your Name',
    agentdbVersion: '^2.0.0',
    tags: ['custom', 'experimental'],
    estimatedDuration: 30000, // 30 seconds
    requiredMemoryMB: 512
  },

  async execute(config: AgentDBConfig): Promise<SimulationResult> {
    // Your simulation logic here
    console.log('Running custom scenario...');

    // Example: Simulate HNSW search
    const recall = 0.95;
    const latency = 120;
    const throughput = 1000;
    const memoryUsage = 512;

    return {
      scenario: this.metadata.id,
      timestamp: new Date(),
      config,
      metrics: {
        recall,
        latency,
        throughput,
        memoryUsage
      },
      insights: [
        'Custom insight 1: Performance is optimal',
        'Custom insight 2: Memory usage is within bounds'
      ],
      recommendations: [
        'Try increasing M parameter for better recall',
        'Consider enabling neural augmentation'
      ],
      iterations: 1,
      duration: 25000
    };
  },

  validate(config: AgentDBConfig): ValidationResult {
    const errors: string[] = [];
    const warnings: string[] = [];

    // Custom validation logic
    if (config.hnsw.M < 16) {
      errors.push('M must be at least 16 for this scenario');
    }

    if (config.neural.mode === 'none') {
      warnings.push('Neural mode recommended for best results');
    }

    return {
      valid: errors.length === 0,
      errors: errors.length > 0 ? errors : undefined,
      warnings: warnings.length > 0 ? warnings : undefined
    };
  },

  async cleanup(): Promise<void> {
    // Optional cleanup logic
    console.log('Cleaning up custom scenario...');
  }
};

export default myScenario;
```

### Advanced Example with Progress Tracking

```typescript
import { SimulationScenario, SimulationResult, AgentDBConfig } from 'agentdb';
import { EventEmitter } from 'events';

export class AdvancedScenario extends EventEmitter implements SimulationScenario {
  metadata = {
    id: 'advanced-scenario',
    name: 'Advanced Scenario',
    version: '1.0.0',
    category: 'plugin' as const,
    description: 'Advanced simulation with progress tracking',
    agentdbVersion: '^2.0.0',
    estimatedDuration: 60000,
    requiredMemoryMB: 1024
  };

  async execute(config: AgentDBConfig): Promise<SimulationResult> {
    const totalIterations = 100;
    const metrics = {
      recall: 0,
      latency: 0,
      throughput: 0,
      memoryUsage: 0
    };

    for (let i = 0; i < totalIterations; i++) {
      // Simulate work
      await this.simulateIteration(i, config);

      // Emit progress
      this.emit('progress', {
        iteration: i + 1,
        total: totalIterations,
        percent: ((i + 1) / totalIterations) * 100
      });

      // Update metrics
      metrics.recall += Math.random() * 0.01;
      metrics.latency += Math.random() * 5;
    }

    metrics.recall /= totalIterations;
    metrics.latency /= totalIterations;
    metrics.throughput = 1000 / (metrics.latency / 1000);
    metrics.memoryUsage = process.memoryUsage().heapUsed / 1024 / 1024;

    return {
      scenario: this.metadata.id,
      timestamp: new Date(),
      config,
      metrics,
      insights: [
        `Achieved ${(metrics.recall * 100).toFixed(1)}% recall`,
        `Average latency: ${metrics.latency.toFixed(2)}ms`
      ],
      recommendations: this.generateRecommendations(metrics, config),
      iterations: totalIterations
    };
  }

  private async simulateIteration(iteration: number, config: AgentDBConfig): Promise<void> {
    // Simulate work with delay
    await new Promise(resolve => setTimeout(resolve, 100));
  }

  private generateRecommendations(metrics: any, config: AgentDBConfig): string[] {
    const recommendations: string[] = [];

    if (metrics.recall < 0.95) {
      recommendations.push('Increase beam width to improve recall');
    }

    if (metrics.latency > 200) {
      recommendations.push('Reduce efSearch to lower latency');
    }

    if (metrics.memoryUsage > config.monitoring.alertThresholds.memoryMB) {
      recommendations.push('Enable memory-constrained profile');
    }

    return recommendations;
  }
}

export default new AdvancedScenario();
```

---

## Component Interfaces

### SearchStrategy Interface

```typescript
interface SearchStrategy {
  name: string;

  // Build index from vectors
  build(vectors: Vector[]): Promise<void>;

  // Search for k nearest neighbors
  search(query: Vector, k: number): Promise<SearchResult[]>;

  // Get index statistics
  getStats(): SearchStats;

  // Optional: Incremental updates
  insert?(vector: Vector): Promise<void>;
  delete?(id: string): Promise<void>;
}

interface Vector {
  id: string;
  data: number[];
  metadata?: Record<string, any>;
}

interface SearchResult {
  id: string;
  distance: number;
  vector: Vector;
}

interface SearchStats {
  totalVectors: number;
  dimensions: number;
  indexSize: number; // bytes
  buildTime: number; // ms
  avgSearchTime: number; // ms
}
```

**Example Implementation**:

```typescript
class CustomSearchStrategy implements SearchStrategy {
  name = 'custom-hnsw';
  private index: Map<string, Vector> = new Map();

  async build(vectors: Vector[]): Promise<void> {
    const startTime = Date.now();

    for (const vector of vectors) {
      this.index.set(vector.id, vector);
    }

    console.log(`Built index in ${Date.now() - startTime}ms`);
  }

  async search(query: Vector, k: number): Promise<SearchResult[]> {
    const results: SearchResult[] = [];

    for (const [id, vector] of this.index.entries()) {
      const distance = this.calculateDistance(query.data, vector.data);
      results.push({ id, distance, vector });
    }

    results.sort((a, b) => a.distance - b.distance);
    return results.slice(0, k);
  }

  getStats(): SearchStats {
    return {
      totalVectors: this.index.size,
      dimensions: this.index.values().next().value?.data.length || 0,
      indexSize: 0, // Calculate actual size
      buildTime: 0,
      avgSearchTime: 0
    };
  }

  private calculateDistance(a: number[], b: number[]): number {
    // Euclidean distance
    return Math.sqrt(a.reduce((sum, val, i) => sum + Math.pow(val - b[i], 2), 0));
  }
}
```

### ClusteringAlgorithm Interface

```typescript
interface ClusteringAlgorithm {
  name: string;

  // Cluster graph into communities
  cluster(graph: Graph): Promise<Community[]>;

  // Get modularity score
  getModularity(): number;

  // Refine clustering
  refine(): Promise<void>;
}

interface Graph {
  nodes: Node[];
  edges: Edge[];
}

interface Node {
  id: string;
  data?: any;
}

interface Edge {
  source: string;
  target: string;
  weight?: number;
}

interface Community {
  id: string;
  nodes: string[];
  modularity: number;
}
```

**Example Implementation**:

```typescript
class CustomClusteringAlgorithm implements ClusteringAlgorithm {
  name = 'custom-louvain';
  private communities: Community[] = [];
  private modularity: number = 0;

  async cluster(graph: Graph): Promise<Community[]> {
    // Implement Louvain algorithm
    // (simplified example)

    const communityMap = new Map<string, string[]>();

    for (const node of graph.nodes) {
      const communityId = `community-${Math.floor(Math.random() * 5)}`;

      if (!communityMap.has(communityId)) {
        communityMap.set(communityId, []);
      }

      communityMap.get(communityId)!.push(node.id);
    }

    this.communities = Array.from(communityMap.entries()).map(([id, nodes]) => ({
      id,
      nodes,
      modularity: this.calculateModularity(graph, nodes)
    }));

    this.modularity = this.communities.reduce((sum, c) => sum + c.modularity, 0);

    return this.communities;
  }

  getModularity(): number {
    return this.modularity;
  }

  async refine(): Promise<void> {
    // Implement refinement logic
  }

  private calculateModularity(graph: Graph, nodes: string[]): number {
    // Simplified modularity calculation
    return Math.random() * 0.5;
  }
}
```

### NeuralAugmentation Interface

```typescript
interface NeuralAugmentation {
  name: string;

  // Augment features with neural network
  augment(features: Tensor): Promise<Tensor>;

  // Train neural network
  train(samples: TrainingSample[]): Promise<void>;

  // Evaluate performance
  evaluate(): Promise<EvaluationMetrics>;
}

interface Tensor {
  shape: number[];
  data: number[];
}

interface TrainingSample {
  input: Tensor;
  target: Tensor;
}

interface EvaluationMetrics {
  accuracy: number;
  loss: number;
  f1Score: number;
}
```

---

## Plugin System

### Plugin Structure

```
~/.agentdb/plugins/my-plugin/
├── index.ts               # Main entry point
├── metadata.json          # Plugin metadata
├── package.json           # npm package (optional)
├── README.md              # Documentation
└── tests/                 # Tests (optional)
    └── index.test.ts
```

### metadata.json

```json
{
  "id": "my-custom-plugin",
  "name": "My Custom Plugin",
  "version": "1.0.0",
  "category": "plugin",
  "description": "Custom plugin for AgentDB",
  "author": "Your Name <your.email@example.com>",
  "agentdbVersion": "^2.0.0",
  "tags": ["custom", "experimental"],
  "estimatedDuration": 30000,
  "requiredMemoryMB": 512,
  "license": "MIT"
}
```

### Installing Plugins

```bash
# Install from directory
agentdb plugin install ~/.agentdb/plugins/my-plugin

# Install from npm
agentdb plugin install my-agentdb-plugin

# Install from git
agentdb plugin install https://github.com/user/my-plugin.git
```

### Listing Plugins

```bash
agentdb plugin list
```

**Output**:
```
📦 Installed Plugins:

✓ my-custom-plugin (v1.0.0)
  Category: plugin
  Author: Your Name
  Description: Custom plugin for AgentDB

✓ advanced-scenario (v1.0.0)
  Category: experimental
  Author: AgentDB Team
  Description: Advanced simulation with progress tracking
```

### Uninstalling Plugins

```bash
agentdb plugin uninstall my-custom-plugin
```

---

## Event System

### Available Events

```typescript
// Simulation lifecycle
runner.on('start', (scenario: string, config: AgentDBConfig) => {
  console.log(`Starting ${scenario}...`);
});

runner.on('progress', (progress: ProgressUpdate) => {
  console.log(`Progress: ${progress.percent.toFixed(1)}%`);
});

runner.on('complete', (result: SimulationResult) => {
  console.log(`Completed with recall: ${result.metrics.recall}`);
});

runner.on('error', (error: Error) => {
  console.error(`Error: ${error.message}`);
});

runner.on('cancelled', () => {
  console.log('Simulation cancelled');
});

// Health monitoring
monitor.on('alert', (alert: Alert) => {
  console.warn(`Alert: ${alert.message}`);
});

monitor.on('metrics', (metrics: HealthMetrics) => {
  console.log(`Memory: ${metrics.memory.heapUsed.toFixed(0)}MB`);
});

monitor.on('healing', (action: HealingAction) => {
  console.log(`Self-healing: ${action.type}`);
});

// Registry events
registry.on('scenario-discovered', (scenario: SimulationScenario) => {
  console.log(`Discovered: ${scenario.metadata.name}`);
});

registry.on('plugin-registered', (plugin: SimulationScenario) => {
  console.log(`Registered plugin: ${plugin.metadata.name}`);
});
```

### Custom Event Emitters

```typescript
class CustomScenario extends EventEmitter implements SimulationScenario {
  async execute(config: AgentDBConfig): Promise<SimulationResult> {
    // Emit custom events
    this.emit('custom-event', { data: 'example' });

    // Emit progress
    for (let i = 0; i < 100; i++) {
      this.emit('progress', {
        iteration: i,
        total: 100,
        percent: (i / 100) * 100
      });

      await this.simulateWork();
    }

    return {
      scenario: this.metadata.id,
      timestamp: new Date(),
      config,
      metrics: { recall: 0.95, latency: 100, throughput: 1000, memoryUsage: 512 },
      insights: [],
      recommendations: []
    };
  }

  private async simulateWork(): Promise<void> {
    await new Promise(resolve => setTimeout(resolve, 10));
  }
}
```

---

## Code Examples

### Example 1: HNSW Optimization Plugin

```typescript
// ~/.agentdb/plugins/hnsw-optimizer/index.ts
import { SimulationScenario, SimulationResult, AgentDBConfig } from 'agentdb';

export const hnswOptimizer: SimulationScenario = {
  metadata: {
    id: 'hnsw-optimizer',
    name: 'HNSW Optimizer',
    version: '1.0.0',
    category: 'plugin',
    description: 'Find optimal HNSW parameters for your dataset',
    agentdbVersion: '^2.0.0',
    tags: ['hnsw', 'optimization'],
    estimatedDuration: 120000,
    requiredMemoryMB: 2048
  },

  async execute(config: AgentDBConfig): Promise<SimulationResult> {
    const results: Array<{ M: number; recall: number; latency: number }> = [];

    // Test different M values
    for (let M = 8; M <= 64; M += 8) {
      const recall = await this.testHNSW(M, config.hnsw.efConstruction, config.hnsw.efSearch);
      const latency = await this.measureLatency(M, config.hnsw.efSearch);

      results.push({ M, recall, latency });
    }

    // Find optimal M (best recall with latency < threshold)
    const optimal = results
      .filter(r => r.latency < config.monitoring.alertThresholds.latencyMs)
      .sort((a, b) => b.recall - a.recall)[0];

    return {
      scenario: this.metadata.id,
      timestamp: new Date(),
      config,
      metrics: {
        recall: optimal.recall,
        latency: optimal.latency,
        throughput: 1000 / (optimal.latency / 1000),
        memoryUsage: process.memoryUsage().heapUsed / 1024 / 1024,
        optimalM: optimal.M
      },
      insights: [
        `Optimal M found: ${optimal.M}`,
        `Achieves ${(optimal.recall * 100).toFixed(1)}% recall`,
        `Latency: ${optimal.latency.toFixed(2)}ms`
      ],
      recommendations: [
        `Set hnsw.M to ${optimal.M} for best results`,
        `This provides ${((optimal.recall - config.hnsw.M / 100) * 100).toFixed(1)}% better recall`
      ]
    };
  },

  private async testHNSW(M: number, efConstruction: number, efSearch: number): Promise<number> {
    // Implement HNSW testing logic
    return 0.9 + (M / 100) * 0.08; // Simplified
  },

  private async measureLatency(M: number, efSearch: number): Promise<number> {
    // Implement latency measurement
    return 100 + M * 2; // Simplified
  }
};

export default hnswOptimizer;
```

### Example 2: A/B Testing Plugin

```typescript
// ~/.agentdb/plugins/ab-testing/index.ts
import { SimulationScenario, SimulationResult, AgentDBConfig } from 'agentdb';
import { ReportStore } from 'agentdb/cli/lib/report-store';

export class ABTestingScenario implements SimulationScenario {
  metadata = {
    id: 'ab-testing',
    name: 'A/B Testing',
    version: '1.0.0',
    category: 'plugin' as const,
    description: 'Compare two configurations automatically',
    agentdbVersion: '^2.0.0',
    estimatedDuration: 60000
  };

  async execute(config: AgentDBConfig): Promise<SimulationResult> {
    // Configuration A (current)
    const resultA = await this.runConfiguration(config, 'A');

    // Configuration B (alternative with higher M)
    const configB = { ...config, hnsw: { ...config.hnsw, M: config.hnsw.M + 16 } };
    const resultB = await this.runConfiguration(configB, 'B');

    // Statistical significance test
    const pValue = this.calculatePValue(resultA.recall, resultB.recall);
    const significant = pValue < 0.05;

    const winner = resultB.recall > resultA.recall ? 'B' : 'A';

    return {
      scenario: this.metadata.id,
      timestamp: new Date(),
      config,
      metrics: {
        recall: Math.max(resultA.recall, resultB.recall),
        latency: winner === 'A' ? resultA.latency : resultB.latency,
        throughput: winner === 'A' ? resultA.throughput : resultB.throughput,
        memoryUsage: process.memoryUsage().heapUsed / 1024 / 1024,
        pValue,
        winner: winner === 'A' ? 0 : 1
      },
      insights: [
        `Configuration ${winner} wins with ${significant ? 'significant' : 'insignificant'} improvement`,
        `p-value: ${pValue.toFixed(4)}`,
        `Recall improvement: ${((resultB.recall - resultA.recall) * 100).toFixed(2)}%`
      ],
      recommendations: winner === 'B' ? [
        `Switch to configuration B (M=${configB.hnsw.M})`,
        `Expected improvement: ${((resultB.recall - resultA.recall) * 100).toFixed(1)}%`
      ] : [
        'Current configuration is optimal'
      ]
    };
  }

  private async runConfiguration(config: AgentDBConfig, variant: string): Promise<any> {
    // Run simulation with configuration
    // (simplified example)
    return {
      recall: 0.92 + Math.random() * 0.05,
      latency: 100 + Math.random() * 50,
      throughput: 1000
    };
  }

  private calculatePValue(recallA: number, recallB: number): number {
    // Simplified p-value calculation
    const diff = Math.abs(recallB - recallA);
    return Math.max(0.01, 0.5 - diff * 2);
  }
}

export default new ABTestingScenario();
```

---

## Testing Plugins

### Unit Tests

```typescript
// ~/.agentdb/plugins/my-plugin/tests/index.test.ts
import { describe, it, expect } from 'vitest';
import myPlugin from '../index';

describe('My Plugin', () => {
  it('should have valid metadata', () => {
    expect(myPlugin.metadata.id).toBe('my-plugin');
    expect(myPlugin.metadata.version).toMatch(/^\d+\.\d+\.\d+$/);
  });

  it('should execute successfully', async () => {
    const config = createMockConfig();
    const result = await myPlugin.execute(config);

    expect(result.metrics.recall).toBeGreaterThan(0);
    expect(result.metrics.latency).toBeGreaterThan(0);
  });

  it('should validate configuration', () => {
    const config = createMockConfig();
    const validation = myPlugin.validate!(config);

    expect(validation.valid).toBe(true);
  });
});

function createMockConfig(): any {
  return {
    profile: 'production',
    hnsw: { M: 32, efConstruction: 200, efSearch: 100 },
    // ... other fields
  };
}
```

---

## Publishing Plugins

### npm Package

```json
// package.json
{
  "name": "agentdb-plugin-myname",
  "version": "1.0.0",
  "description": "My custom AgentDB plugin",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "keywords": ["agentdb", "plugin", "simulation"],
  "peerDependencies": {
    "agentdb": "^2.0.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0",
    "vitest": "^1.0.0"
  },
  "scripts": {
    "build": "tsc",
    "test": "vitest"
  }
}
```

### Publishing

```bash
npm publish
```

### Installing Published Plugin

```bash
npm install -g agentdb-plugin-myname
agentdb plugin install agentdb-plugin-myname
```

---

**Document Version**: 1.0
**Last Updated**: 2025-11-30
**Maintainer**: AgentDB Team
