# AgentDB v2.0 Integration Architecture

## Executive Summary

This document defines the integration architecture for AgentDB v2.0, bringing together:
- 8 optimized simulation scenarios (from Swarm 1)
- CLI infrastructure with wizard/custom modes (from Swarm 2)
- Comprehensive documentation (from Swarm 3)
- Full test coverage (from Swarm 4)

**Key Design Principles**:
1. **Plugin Architecture**: Dynamic scenario loading via registry pattern
2. **Configuration Profiles**: Preset configurations for common use cases
3. **Embedded Persistence**: SQLite for zero-dependency report storage
4. **Event-Driven Progress**: Real-time feedback and monitoring
5. **Self-Healing**: Automatic recovery using discovered MPC algorithms

---

## 1. System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     AgentDB CLI (Entry Point)               │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Commander.js Framework                               │  │
│  │    ├─ agentdb simulate [scenario]                     │  │
│  │    ├─ agentdb simulate --wizard                       │  │
│  │    ├─ agentdb simulate --custom                       │  │
│  │    ├─ agentdb simulate --compare <ids>                │  │
│  │    └─ agentdb simulate --history                      │  │
│  └───────────────────────────────────────────────────────┘  │
│                          │                                   │
│         ┌────────────────┼────────────────┐                 │
│         ▼                ▼                ▼                 │
│  ┌──────────┐   ┌──────────────┐  ┌────────────┐          │
│  │  Wizard  │   │ Custom       │  │ Direct     │          │
│  │  Flow    │   │ Builder      │  │ Execution  │          │
│  │  (Inquirer) │ (Interactive) │  │ (Flags)    │          │
│  └──────────┘   └──────────────┘  └────────────┘          │
│         │                │                │                 │
│         └────────────────┴────────────────┘                 │
│                          ▼                                   │
│         ┌──────────────────────────────────┐                │
│         │  Configuration Manager           │                │
│         │  ┌────────────────────────────┐  │                │
│         │  │ Profiles:                  │  │                │
│         │  │ - production (optimal)     │  │                │
│         │  │ - memory-constrained       │  │                │
│         │  │ - latency-critical         │  │                │
│         │  │ - high-recall              │  │                │
│         │  └────────────────────────────┘  │                │
│         │  - Validation & Defaults         │                │
│         │  - .agentdb.json support         │                │
│         │  - Environment variables         │                │
│         └──────────────────────────────────┘                │
│                          ▼                                   │
│         ┌──────────────────────────────────┐                │
│         │  Simulation Registry             │                │
│         │  - Auto-discovery of scenarios   │                │
│         │  - Metadata extraction           │                │
│         │  - Version compatibility         │                │
│         │  - Plugin validation             │                │
│         └──────────────────────────────────┘                │
│                          ▼                                   │
│         ┌──────────────────────────────────┐                │
│         │  Simulation Runner               │                │
│         │  - Orchestration engine          │                │
│         │  - Multi-iteration support       │                │
│         │  - Progress events (EventEmitter)│                │
│         │  - Cancellation support          │                │
│         └──────────────────────────────────┘                │
│                          ▼                                   │
│    ┌────────────────────────────────────────────┐          │
│    │  Simulation Scenarios (8 core + plugins)   │          │
│    │  ┌──────────────────────────────────────┐  │          │
│    │  │ Core Scenarios:                      │  │          │
│    │  │ 1. hnsw-exploration (M=32, 8.2x)     │  │          │
│    │  │ 2. attention-analysis (8-head, 12.4%)│  │          │
│    │  │ 3. traversal-optimization (beam-5)   │  │          │
│    │  │ 4. clustering-analysis (Louvain)     │  │          │
│    │  │ 5. self-organizing-hnsw (MPC)        │  │          │
│    │  │ 6. neural-augmentation (full)        │  │          │
│    │  │ 7. hypergraph-exploration (3.7x)     │  │          │
│    │  │ 8. quantum-hybrid (theoretical)      │  │          │
│    │  └──────────────────────────────────────┘  │          │
│    │  ┌──────────────────────────────────────┐  │          │
│    │  │ Plugin Scenarios:                    │  │          │
│    │  │ - Custom implementations             │  │          │
│    │  │ - Third-party extensions             │  │          │
│    │  └──────────────────────────────────────┘  │          │
│    └────────────────────────────────────────────┘          │
│                          ▼                                   │
│         ┌──────────────────────────────────┐                │
│         │  Health Monitor                  │                │
│         │  - Resource tracking             │                │
│         │  - Memory leak detection         │                │
│         │  - Performance alerts            │                │
│         │  - Self-healing (MPC)            │                │
│         └──────────────────────────────────┘                │
│                          ▼                                   │
│         ┌──────────────────────────────────┐                │
│         │  Report Generator                │                │
│         │  - Markdown (detailed analysis)  │                │
│         │  - JSON (machine-readable)       │                │
│         │  - HTML (interactive charts)     │                │
│         └──────────────────────────────────┘                │
│                          ▼                                   │
│         ┌──────────────────────────────────┐                │
│         │  Report Store (SQLite)           │                │
│         │  - Embedded database             │                │
│         │  - Simulation history            │                │
│         │  - Trend analysis                │                │
│         │  - Comparison queries            │                │
│         │  - Export/import                 │                │
│         └──────────────────────────────────┘                │
│                          ▼                                   │
│         ┌──────────────────────────────────┐                │
│         │  History Tracker                 │                │
│         │  - Performance trends            │                │
│         │  - Regression detection          │                │
│         │  - Visualization data            │                │
│         └──────────────────────────────────┘                │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Core Components

### 2.1 Configuration Manager

**Purpose**: Centralize configuration with validation and profiles.

**Key Features**:
- **Profile System**: Production, memory-constrained, latency-critical, high-recall
- **Validation**: Schema-based validation of all parameters
- **Defaults**: Optimal defaults based on simulation discoveries
- **File Support**: `.agentdb.json` for project-level configuration
- **Environment Variables**: Override with `AGENTDB_*` env vars

**Configuration Schema**:
```typescript
interface AgentDBConfig {
  profile: 'production' | 'memory' | 'latency' | 'recall' | 'custom';
  hnsw: {
    M: number;              // Connections per layer (default: 32)
    efConstruction: number; // Construction quality (default: 200)
    efSearch: number;       // Search quality (default: 100)
  };
  attention: {
    heads: number;          // Multi-head count (default: 8)
    dimension: number;      // Attention dim (default: 64)
  };
  traversal: {
    beamWidth: number;      // Beam search width (default: 5)
    strategy: 'greedy' | 'beam' | 'dynamic';
  };
  clustering: {
    algorithm: 'louvain' | 'leiden' | 'spectral';
    resolution: number;     // Modularity resolution
  };
  neural: {
    mode: 'none' | 'gnn-only' | 'full';
    reinforcementLearning: boolean;
  };
  hypergraph: {
    enabled: boolean;
    maxEdgeSize: number;
  };
  storage: {
    reportPath: string;     // SQLite database path
    autoBackup: boolean;
  };
  monitoring: {
    enabled: boolean;
    alertThresholds: {
      memoryMB: number;
      latencyMs: number;
    };
  };
}
```

**Preset Profiles**:

1. **Production (Optimal)**:
   - M=32 (8.2x speedup from HNSW exploration)
   - 8-head attention (12.4% accuracy boost)
   - Beam-5 traversal (96.8% recall)
   - Louvain clustering (Q=0.758)
   - Full neural augmentation (29.4% gain)
   - Self-healing enabled (MPC)

2. **Memory-Constrained**:
   - M=16 (reduced memory footprint)
   - 4-head attention
   - Greedy traversal
   - GNN edges only (no full neural)
   - Disabled hypergraph

3. **Latency-Critical**:
   - M=32 (fast search)
   - RL-based navigation (dynamic-k)
   - Beam-3 (speed vs. recall tradeoff)
   - Louvain clustering (fast)
   - GNN only

4. **High-Recall**:
   - M=64 (maximum connectivity)
   - Beam-10 (exhaustive search)
   - Full neural augmentation
   - Hypergraph enabled
   - efSearch=200

**File Location**: `packages/agentdb/src/cli/lib/config-manager.ts`

---

### 2.2 Simulation Registry

**Purpose**: Auto-discover and manage simulation scenarios.

**Key Features**:
- **Auto-Discovery**: Scan `simulation/scenarios/` directory
- **Metadata Extraction**: Read scenario manifests (metadata.json)
- **Validation**: Ensure scenarios implement required interface
- **Version Compatibility**: Check AgentDB version requirements
- **Plugin Support**: Load third-party scenarios

**Scenario Interface**:
```typescript
interface SimulationScenario {
  metadata: {
    id: string;
    name: string;
    version: string;
    category: 'core' | 'experimental' | 'plugin';
    description: string;
    author?: string;
    agentdbVersion: string; // Semver range
  };

  // Main execution entry point
  execute(config: AgentDBConfig): Promise<SimulationResult>;

  // Validation (optional)
  validate?(config: AgentDBConfig): ValidationResult;

  // Cleanup (optional)
  cleanup?(): Promise<void>;
}

interface ValidationResult {
  valid: boolean;
  errors?: string[];
  warnings?: string[];
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
}
```

**Registry API**:
```typescript
class SimulationRegistry {
  // Discover all scenarios
  async discover(): Promise<SimulationScenario[]>;

  // Get scenario by ID
  get(id: string): SimulationScenario | undefined;

  // List all scenarios
  list(): SimulationScenario[];

  // Register a plugin scenario
  register(scenario: SimulationScenario): void;

  // Validate scenario implementation
  validate(scenario: SimulationScenario): ValidationResult;

  // Check version compatibility
  isCompatible(scenario: SimulationScenario): boolean;
}
```

**File Location**: `packages/agentdb/src/cli/lib/simulation-registry.ts`

---

### 2.3 Report Store (SQLite)

**Purpose**: Persist simulation results with queryable history.

**Why SQLite?**
- ✅ **Zero Dependencies**: Embedded, no external database server
- ✅ **SQL Power**: Complex queries for comparisons and trends
- ✅ **Portable**: Single file, easy backup/restore
- ✅ **Upgrade Path**: Can migrate to PostgreSQL for production scale

**Schema Design**:
```sql
-- Simulation runs
CREATE TABLE simulations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  scenario_id TEXT NOT NULL,
  scenario_name TEXT NOT NULL,
  timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
  config_json TEXT NOT NULL,  -- Full config as JSON
  profile TEXT,               -- Profile name used
  agentdb_version TEXT,
  duration_ms INTEGER,
  status TEXT CHECK(status IN ('running', 'completed', 'failed', 'cancelled'))
);

-- Metrics (normalized for efficient queries)
CREATE TABLE metrics (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  simulation_id INTEGER REFERENCES simulations(id) ON DELETE CASCADE,
  metric_name TEXT NOT NULL,
  metric_value REAL NOT NULL,
  iteration INTEGER,          -- For multi-iteration runs
  UNIQUE(simulation_id, metric_name, iteration)
);

-- Insights and recommendations
CREATE TABLE insights (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  simulation_id INTEGER REFERENCES simulations(id) ON DELETE CASCADE,
  type TEXT CHECK(type IN ('insight', 'recommendation', 'warning')),
  content TEXT NOT NULL,
  category TEXT               -- e.g., 'performance', 'accuracy', 'memory'
);

-- Comparison groups (for A/B testing)
CREATE TABLE comparison_groups (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  description TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE comparison_members (
  group_id INTEGER REFERENCES comparison_groups(id) ON DELETE CASCADE,
  simulation_id INTEGER REFERENCES simulations(id) ON DELETE CASCADE,
  PRIMARY KEY(group_id, simulation_id)
);

-- Indexes for performance
CREATE INDEX idx_simulations_scenario ON simulations(scenario_id);
CREATE INDEX idx_simulations_timestamp ON simulations(timestamp);
CREATE INDEX idx_metrics_simulation ON metrics(simulation_id);
CREATE INDEX idx_metrics_name ON metrics(metric_name);
```

**Store API**:
```typescript
class ReportStore {
  // Save a simulation run
  async save(result: SimulationResult): Promise<number>;

  // Get simulation by ID
  async get(id: number): Promise<SimulationResult | null>;

  // List recent simulations
  async list(limit?: number): Promise<SimulationResult[]>;

  // Search by scenario
  async findByScenario(scenarioId: string): Promise<SimulationResult[]>;

  // Compare multiple runs
  async compare(ids: number[]): Promise<ComparisonReport>;

  // Get performance trends
  async getTrends(scenarioId: string, metric: string): Promise<TrendData>;

  // Detect regressions
  async detectRegressions(scenarioId: string, threshold: number): Promise<Regression[]>;

  // Export to JSON
  async export(ids: number[]): Promise<string>;

  // Import from JSON
  async import(json: string): Promise<number[]>;

  // Backup database
  async backup(path: string): Promise<void>;
}
```

**File Location**: `packages/agentdb/src/cli/lib/report-store.ts`

---

### 2.4 History Tracker

**Purpose**: Track performance trends and detect regressions.

**Key Features**:
- **Trend Analysis**: Plot metric changes over time
- **Regression Detection**: Alert when performance degrades
- **Baseline Comparison**: Compare against known-good runs
- **Visualization Data**: Prepare data for charts (Chart.js, D3.js)

**Regression Detection Algorithm**:
```typescript
interface Regression {
  metric: string;
  baseline: number;
  current: number;
  degradation: number;  // Percentage drop
  severity: 'minor' | 'major' | 'critical';
  firstDetected: Date;
  affectedRuns: number[];
}

// Detect regressions using moving average
async detectRegressions(
  scenarioId: string,
  windowSize: number = 5,
  threshold: number = 0.1  // 10% degradation
): Promise<Regression[]> {
  // 1. Get recent runs
  const runs = await this.store.findByScenario(scenarioId);

  // 2. Calculate moving averages for each metric
  const averages = this.calculateMovingAverages(runs, windowSize);

  // 3. Compare current run to baseline
  const regressions: Regression[] = [];
  for (const metric of Object.keys(averages)) {
    const baseline = averages[metric].baseline;
    const current = averages[metric].current;
    const degradation = (baseline - current) / baseline;

    if (degradation > threshold) {
      regressions.push({
        metric,
        baseline,
        current,
        degradation,
        severity: this.getSeverity(degradation),
        firstDetected: new Date(),
        affectedRuns: averages[metric].runs
      });
    }
  }

  return regressions;
}
```

**File Location**: `packages/agentdb/src/cli/lib/history-tracker.ts`

---

### 2.5 Health Monitor

**Purpose**: Track system resources and enable self-healing.

**Key Features**:
- **Resource Tracking**: CPU, memory, disk I/O during simulations
- **Memory Leak Detection**: Monitor memory growth over iterations
- **Performance Alerts**: Configurable thresholds for alerts
- **Self-Healing**: Use MPC algorithm to recover from failures

**Monitoring Metrics**:
```typescript
interface HealthMetrics {
  timestamp: Date;
  cpu: {
    usage: number;      // Percentage
    temperature?: number;
  };
  memory: {
    used: number;       // MB
    available: number;
    heapUsed: number;
    heapTotal: number;
  };
  disk: {
    readMBps: number;
    writeMBps: number;
  };
  simulation: {
    iterationsCompleted: number;
    itemsProcessed: number;
    errorsEncountered: number;
  };
}

interface Alert {
  level: 'info' | 'warning' | 'critical';
  metric: string;
  threshold: number;
  actual: number;
  timestamp: Date;
  action?: 'log' | 'throttle' | 'abort' | 'heal';
}
```

**Self-Healing with MPC**:
From Swarm 1's discovery, the Message Passing with Coordination (MPC) algorithm achieved 97.9% recall. We use this for automatic recovery:

```typescript
class HealthMonitor extends EventEmitter {
  async monitorSimulation(runner: SimulationRunner): Promise<void> {
    const interval = setInterval(() => {
      const metrics = this.collectMetrics();

      // Check thresholds
      const alerts = this.checkThresholds(metrics);

      for (const alert of alerts) {
        this.emit('alert', alert);

        if (alert.action === 'heal') {
          this.triggerSelfHealing(runner, alert);
        }
      }
    }, 1000); // 1-second monitoring interval

    runner.on('complete', () => clearInterval(interval));
  }

  private triggerSelfHealing(runner: SimulationRunner, alert: Alert): void {
    console.log(`🔧 Self-healing triggered for ${alert.metric}`);

    // Use MPC algorithm to recover
    // 1. Pause current simulation
    runner.pause();

    // 2. Apply MPC-based recovery strategy
    // (coordination between nodes to find stable state)
    const recovery = this.mpcCoordination(runner.getCurrentState());

    // 3. Resume with adjusted parameters
    runner.resume(recovery.adjustedConfig);
  }
}
```

**File Location**: `packages/agentdb/src/cli/lib/health-monitor.ts`

---

## 3. Integration Workflows

### 3.1 Direct Execution Flow

```
User: agentdb simulate hnsw-exploration
  ↓
CLI parses command → Load config (production profile)
  ↓
Registry.get('hnsw-exploration') → Validate scenario
  ↓
Runner.execute(scenario, config) → Start monitoring
  ↓
Scenario runs → Emit progress events
  ↓
Health monitor checks resources → No alerts
  ↓
Results generated → Report store saves
  ↓
History tracker analyzes trends → No regressions
  ↓
Display summary + report path
```

### 3.2 Wizard Flow

```
User: agentdb simulate --wizard
  ↓
Inquirer prompts:
  1. "What are you optimizing for?" → Select profile
  2. "Dataset size?" → Adjust memory settings
  3. "Advanced options?" → Fine-tune parameters
  ↓
Config manager validates inputs → Generate config
  ↓
Registry.list() → Show compatible scenarios
  ↓
User selects scenario → Execute (same as direct flow)
```

### 3.3 Custom Builder Flow

```
User: agentdb simulate --custom
  ↓
Interactive builder:
  1. HNSW parameters (M, efConstruction, efSearch)
  2. Attention configuration (heads, dimension)
  3. Traversal strategy (beam width, algorithm)
  4. Clustering settings (algorithm, resolution)
  5. Neural augmentation (mode, RL enabled)
  6. Hypergraph options (enabled, edge size)
  ↓
Config manager validates → Save to .agentdb.json
  ↓
Execute with custom config
```

### 3.4 Comparison Flow

```
User: agentdb simulate --compare 1,2,3
  ↓
Report store loads simulations [1, 2, 3]
  ↓
Generate comparison report:
  - Side-by-side metrics
  - Difference analysis
  - Winner determination
  - Statistical significance
  ↓
Display comparison table + charts
```

---

## 4. Extension API

### 4.1 Creating Custom Scenarios

Developers can create custom simulation scenarios:

**Step 1: Create scenario directory**
```bash
mkdir -p ~/.agentdb/plugins/my-scenario
```

**Step 2: Implement scenario**
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
    agentdbVersion: '^2.0.0'
  },

  async execute(config: AgentDBConfig): Promise<SimulationResult> {
    // Your simulation logic here
    return {
      scenario: this.metadata.id,
      timestamp: new Date(),
      config,
      metrics: {
        recall: 0.95,
        latency: 120,
        throughput: 1000,
        memoryUsage: 512
      },
      insights: ['Custom insight 1', 'Custom insight 2'],
      recommendations: ['Try increasing M parameter']
    };
  },

  validate(config: AgentDBConfig): ValidationResult {
    // Optional validation logic
    return { valid: true };
  }
};
```

**Step 3: Register plugin**
```bash
agentdb plugin install ~/.agentdb/plugins/my-scenario
```

**Step 4: Use plugin**
```bash
agentdb simulate my-custom-scenario
```

### 4.2 Component Interfaces

**SearchStrategy Interface**:
```typescript
interface SearchStrategy {
  name: string;
  search(query: Vector, k: number): Promise<SearchResult[]>;
  build(vectors: Vector[]): Promise<void>;
  getStats(): SearchStats;
}
```

**ClusteringAlgorithm Interface**:
```typescript
interface ClusteringAlgorithm {
  name: string;
  cluster(graph: Graph): Promise<Community[]>;
  getModularity(): number;
  refine(): Promise<void>;
}
```

**NeuralAugmentation Interface**:
```typescript
interface NeuralAugmentation {
  name: string;
  augment(features: Tensor): Promise<Tensor>;
  train(samples: TrainingSample[]): Promise<void>;
  evaluate(): Promise<EvaluationMetrics>;
}
```

See `/workspaces/agentic-flow/packages/agentdb/simulation/docs/architecture/EXTENSION-API.md` for full details.

---

## 5. Event System

**Purpose**: Real-time progress tracking and integration hooks.

**Events Emitted**:
```typescript
// Simulation lifecycle
runner.on('start', (scenario: string, config: AgentDBConfig) => {});
runner.on('progress', (progress: ProgressUpdate) => {});
runner.on('complete', (result: SimulationResult) => {});
runner.on('error', (error: Error) => {});
runner.on('cancelled', () => {});

// Health monitoring
monitor.on('alert', (alert: Alert) => {});
monitor.on('metrics', (metrics: HealthMetrics) => {});
monitor.on('healing', (action: HealingAction) => {});

// Registry events
registry.on('scenario-discovered', (scenario: SimulationScenario) => {});
registry.on('plugin-registered', (plugin: SimulationScenario) => {});
```

**Integration with External Systems**:
```typescript
// Example: Send progress to webhook
runner.on('progress', async (progress) => {
  await fetch('https://my-monitoring.com/webhook', {
    method: 'POST',
    body: JSON.stringify(progress)
  });
});

// Example: Abort on memory threshold
monitor.on('alert', (alert) => {
  if (alert.level === 'critical' && alert.metric === 'memory') {
    runner.cancel();
  }
});
```

---

## 6. Production Deployment

### 6.1 System Requirements

**Minimum**:
- CPU: 2 cores
- RAM: 4 GB
- Disk: 10 GB free space
- Node.js: 18.x or later

**Recommended**:
- CPU: 8 cores (for parallel iterations)
- RAM: 16 GB (for large datasets)
- Disk: 50 GB SSD
- GPU: Optional (for neural augmentation)

### 6.2 Installation Methods

**1. npm (Development)**:
```bash
npm install -g agentdb
agentdb --version
```

**2. Docker (Production)**:
```bash
docker pull agentdb/agentdb:2.0
docker run -v /data:/app/data agentdb/agentdb simulate hnsw-exploration
```

**3. Standalone Binary (Air-gapped)**:
```bash
curl -O https://releases.agentdb.io/agentdb-linux-x64
chmod +x agentdb-linux-x64
./agentdb-linux-x64 simulate hnsw-exploration
```

### 6.3 Configuration Best Practices

**Production .agentdb.json**:
```json
{
  "profile": "production",
  "storage": {
    "reportPath": "/data/agentdb/reports.db",
    "autoBackup": true
  },
  "monitoring": {
    "enabled": true,
    "alertThresholds": {
      "memoryMB": 12288,
      "latencyMs": 1000
    }
  },
  "logging": {
    "level": "info",
    "file": "/var/log/agentdb/simulation.log"
  }
}
```

### 6.4 Monitoring & Alerting

**Prometheus Integration**:
```typescript
// Expose metrics endpoint
const prometheus = require('prom-client');
const register = new prometheus.Registry();

// Define metrics
const simulationDuration = new prometheus.Histogram({
  name: 'agentdb_simulation_duration_seconds',
  help: 'Simulation execution time',
  labelNames: ['scenario']
});

const memoryUsage = new prometheus.Gauge({
  name: 'agentdb_memory_usage_bytes',
  help: 'Memory usage during simulation'
});

register.registerMetric(simulationDuration);
register.registerMetric(memoryUsage);

// Update metrics
monitor.on('metrics', (metrics) => {
  memoryUsage.set(metrics.memory.used * 1024 * 1024);
});

// Expose endpoint
app.get('/metrics', (req, res) => {
  res.set('Content-Type', register.contentType);
  res.end(register.metrics());
});
```

### 6.5 Scaling Considerations

**Distributed Simulations**:
For large-scale benchmarking, distribute scenarios across multiple machines:

```typescript
// Coordinator node
const scenarios = registry.list();
const workers = ['worker1:3000', 'worker2:3000', 'worker3:3000'];

for (let i = 0; i < scenarios.length; i++) {
  const worker = workers[i % workers.length];
  await fetch(`http://${worker}/simulate`, {
    method: 'POST',
    body: JSON.stringify({ scenario: scenarios[i].metadata.id })
  });
}
```

---

## 7. Architecture Decision Records (ADRs)

### ADR-001: SQLite for Report Storage

**Status**: Accepted

**Context**: Need persistent storage for simulation results with queryable history.

**Decision**: Use SQLite as embedded database.

**Rationale**:
- ✅ Zero dependencies (no external database server)
- ✅ SQL query power for complex comparisons
- ✅ Portable (single file, easy backup/restore)
- ✅ Upgrade path to PostgreSQL if needed

**Consequences**:
- Limited to ~1TB database size (sufficient for millions of runs)
- No concurrent writes (but simulations are sequential)
- Can migrate to PostgreSQL for distributed deployments

---

### ADR-002: Registry Pattern for Scenarios

**Status**: Accepted

**Context**: Need dynamic loading of simulation scenarios with plugin support.

**Decision**: Use registry pattern with auto-discovery.

**Rationale**:
- ✅ Supports plugin architecture
- ✅ Version management built-in
- ✅ Easy to mock for testing
- ✅ Decouples CLI from scenario implementations

**Consequences**:
- Slight overhead for discovery (mitigated by caching)
- Need clear plugin API contract

---

### ADR-003: Profile-Based Configuration

**Status**: Accepted

**Context**: Different use cases require different optimal configurations.

**Decision**: Preset profiles (production, memory, latency, recall).

**Rationale**:
- ✅ Prevents misconfiguration
- ✅ Aligns with simulation discoveries (optimal settings per use case)
- ✅ Easy to switch between environments
- ✅ Reduces cognitive load for users

**Consequences**:
- Need to maintain profiles as new discoveries emerge
- Users may not understand profile internals (mitigated by docs)

---

### ADR-004: Event-Driven Progress Tracking

**Status**: Accepted

**Context**: Need real-time feedback during long-running simulations.

**Decision**: Use EventEmitter for progress events.

**Rationale**:
- ✅ Decouples progress tracking from execution logic
- ✅ Supports multiple listeners (CLI, webhooks, monitoring)
- ✅ Enables cancellation and pause/resume
- ✅ Future-proof for web UI integration

**Consequences**:
- Memory overhead for event listeners (mitigated by cleanup)
- Need careful error handling in listeners

---

### ADR-005: MPC-Based Self-Healing

**Status**: Accepted

**Context**: Simulations may fail due to resource exhaustion or transient errors.

**Decision**: Use Message Passing with Coordination (MPC) for automatic recovery.

**Rationale**:
- ✅ MPC achieved 97.9% recall in simulation (proven reliability)
- ✅ Coordination between components enables stable recovery
- ✅ Reduces manual intervention
- ✅ Aligns with distributed systems best practices

**Consequences**:
- Requires MPC implementation in health monitor
- May introduce slight overhead during normal execution

---

## 8. Security Considerations

### 8.1 Plugin Validation

**Risk**: Malicious plugins could execute arbitrary code.

**Mitigation**:
1. **Code Signing**: Verify plugin signatures
2. **Sandboxing**: Run plugins in isolated context (VM2)
3. **Permission System**: Plugins declare required permissions
4. **Audit Logging**: Log all plugin activities

### 8.2 Configuration Injection

**Risk**: Malicious `.agentdb.json` files could override security settings.

**Mitigation**:
1. **Schema Validation**: Strict JSON schema validation
2. **Whitelist**: Only allow known configuration keys
3. **Sanitization**: Escape all user inputs
4. **Read-Only Defaults**: Core settings cannot be overridden

### 8.3 Report Storage

**Risk**: Unauthorized access to simulation results.

**Mitigation**:
1. **File Permissions**: Restrict SQLite database to owner only
2. **Encryption**: Optional at-rest encryption for sensitive data
3. **Access Control**: API-level permissions for multi-user setups

---

## 9. Testing Strategy

### 9.1 Unit Tests

- Configuration manager validation
- Registry discovery logic
- Report store CRUD operations
- Health monitor threshold checks

### 9.2 Integration Tests

**End-to-End Workflow**:
```typescript
describe('Integration: CLI → Simulation → Report', () => {
  it('should execute scenario and save results', async () => {
    // 1. Initialize components
    const registry = new SimulationRegistry();
    const store = new ReportStore(':memory:');
    const runner = new SimulationRunner(registry, store);

    // 2. Load scenario
    const scenario = registry.get('hnsw-exploration');
    expect(scenario).toBeDefined();

    // 3. Execute simulation
    const result = await runner.execute(scenario, productionConfig);

    // 4. Verify results
    expect(result.metrics.recall).toBeGreaterThan(0.95);
    expect(result.scenario).toBe('hnsw-exploration');

    // 5. Verify storage
    const saved = await store.get(result.id);
    expect(saved).toEqual(result);
  });
});
```

### 9.3 Performance Benchmarking

Continuous benchmarking to detect regressions:

```bash
# Run benchmark suite
agentdb benchmark --suite full --iterations 100

# Compare against baseline
agentdb benchmark --compare baseline.json
```

---

## 10. Migration Path

### 10.1 From v1.x to v2.0

**Breaking Changes**:
- CLI command structure changed (`agentdb simulate` instead of `agentdb run`)
- Configuration file format (.agentdb.json replaces .agentdbrc)
- Report storage moved from JSON files to SQLite

**Migration Steps**:
1. Install v2.0: `npm install -g agentdb@2.0`
2. Migrate config: `agentdb migrate config .agentdbrc`
3. Import old reports: `agentdb migrate reports ./old-reports/`
4. Verify: `agentdb simulate hnsw-exploration --dry-run`

See `/workspaces/agentic-flow/packages/agentdb/simulation/docs/guides/MIGRATION-GUIDE.md` for details.

---

## 11. Future Enhancements

### 11.1 Web UI

Interactive dashboard for:
- Real-time simulation monitoring
- Visual comparison of runs
- Configuration builder (drag-and-drop)
- Trend charts (Chart.js/D3.js)

### 11.2 Cloud Integration

- AWS/GCP/Azure deployment templates
- Managed AgentDB service
- Distributed simulation orchestration
- Centralized report aggregation

### 11.3 Advanced Analytics

- Machine learning for configuration optimization
- Anomaly detection in metrics
- Automated A/B testing
- Predictive modeling for performance

---

## 12. References

- **Simulation Discoveries**: `/workspaces/agentic-flow/packages/agentdb/simulation/docs/SIMULATION-FINDINGS.md`
- **CLI Integration Plan**: `/workspaces/agentic-flow/packages/agentdb/simulation/docs/CLI-INTEGRATION-PLAN.md`
- **Extension API**: `/workspaces/agentic-flow/packages/agentdb/simulation/docs/architecture/EXTENSION-API.md`
- **Deployment Guide**: `/workspaces/agentic-flow/packages/agentdb/simulation/docs/guides/DEPLOYMENT.md`
- **Migration Guide**: `/workspaces/agentic-flow/packages/agentdb/simulation/docs/guides/MIGRATION-GUIDE.md`

---

**Document Version**: 1.0
**Last Updated**: 2025-11-30
**Maintainer**: AgentDB Architecture Team
