# Provider Fallback & Dynamic Switching Guide

**Production-grade LLM provider fallback for long-running agents**

## Overview

The `ProviderManager` and `LongRunningAgent` classes provide enterprise-grade provider fallback, health monitoring, cost optimization, and automatic recovery for long-running AI agents.

### Key Features

- ✅ **Automatic Fallback** - Seamless switching between providers on failure
- ✅ **Circuit Breaker** - Prevents cascading failures with automatic recovery
- ✅ **Health Monitoring** - Real-time provider health tracking
- ✅ **Cost Optimization** - Intelligent provider selection based on cost/performance
- ✅ **Retry Logic** - Exponential/linear backoff for transient errors
- ✅ **Checkpointing** - Save/restore agent state for crash recovery
- ✅ **Budget Control** - Hard limits on spending and runtime
- ✅ **Performance Tracking** - Latency, success rate, and token usage metrics

## Quick Start

### Basic Provider Fallback

```typescript
import { ProviderManager, ProviderConfig } from 'agentic-flow/core/provider-manager';

// Configure providers
const providers: ProviderConfig[] = [
  {
    name: 'gemini',
    apiKey: process.env.GOOGLE_GEMINI_API_KEY,
    priority: 1, // Try first
    maxRetries: 3,
    timeout: 30000,
    costPerToken: 0.00015,
    enabled: true
  },
  {
    name: 'anthropic',
    apiKey: process.env.ANTHROPIC_API_KEY,
    priority: 2, // Fallback
    maxRetries: 3,
    timeout: 60000,
    costPerToken: 0.003,
    enabled: true
  },
  {
    name: 'onnx',
    priority: 3, // Last resort (free, local)
    maxRetries: 2,
    timeout: 120000,
    costPerToken: 0,
    enabled: true
  }
];

// Initialize manager
const manager = new ProviderManager(providers, {
  type: 'priority', // or 'cost-optimized', 'performance-optimized', 'round-robin'
  maxFailures: 3,
  recoveryTime: 60000,
  retryBackoff: 'exponential'
});

// Execute with automatic fallback
const { result, provider, attempts } = await manager.executeWithFallback(
  async (providerName) => {
    // Your LLM API call here
    return await callLLM(providerName, prompt);
  }
);

console.log(`Success with ${provider} after ${attempts} attempts`);
```

### Long-Running Agent

```typescript
import { LongRunningAgent } from 'agentic-flow/core/long-running-agent';

// Create agent
const agent = new LongRunningAgent({
  agentName: 'research-agent',
  providers,
  fallbackStrategy: {
    type: 'cost-optimized',
    maxFailures: 3,
    recoveryTime: 60000,
    retryBackoff: 'exponential',
    costThreshold: 0.50, // Max $0.50 per request
    latencyThreshold: 30000 // Max 30s per request
  },
  checkpointInterval: 30000, // Save state every 30s
  maxRuntime: 3600000, // Max 1 hour
  costBudget: 5.00 // Max $5 total
});

await agent.start();

// Execute tasks with automatic provider selection
const result = await agent.executeTask({
  name: 'analyze-code',
  complexity: 'complex', // 'simple' | 'medium' | 'complex'
  estimatedTokens: 5000,
  execute: async (provider) => {
    return await analyzeCode(provider, code);
  }
});

// Get status
const status = agent.getStatus();
console.log(`Completed: ${status.completedTasks}, Cost: $${status.totalCost}`);

await agent.stop();
```

## Fallback Strategies

### 1. Priority-Based (Default)

Tries providers in priority order (1 = highest).

```typescript
{
  type: 'priority',
  maxFailures: 3,
  recoveryTime: 60000,
  retryBackoff: 'exponential'
}
```

**Use Case:** Prefer specific provider (e.g., Claude for quality)

### 2. Cost-Optimized

Selects cheapest provider for estimated token count.

```typescript
{
  type: 'cost-optimized',
  maxFailures: 3,
  recoveryTime: 60000,
  retryBackoff: 'exponential',
  costThreshold: 0.50 // Max $0.50 per request
}
```

**Use Case:** High-volume applications, budget constraints

### 3. Performance-Optimized

Selects provider with best latency and success rate.

```typescript
{
  type: 'performance-optimized',
  maxFailures: 3,
  recoveryTime: 60000,
  retryBackoff: 'exponential',
  latencyThreshold: 30000 // Max 30s
}
```

**Use Case:** Real-time applications, user-facing services

### 4. Round-Robin

Distributes load evenly across providers.

```typescript
{
  type: 'round-robin',
  maxFailures: 3,
  recoveryTime: 60000,
  retryBackoff: 'exponential'
}
```

**Use Case:** Load balancing, testing multiple providers

## Task Complexity Heuristics

The system applies intelligent heuristics based on task complexity:

### Simple Tasks → Prefer Gemini/ONNX
```typescript
await agent.executeTask({
  name: 'format-code',
  complexity: 'simple', // Fast, cheap providers preferred
  estimatedTokens: 200,
  execute: async (provider) => formatCode(code)
});
```

**Rationale:** Simple tasks don't need Claude's reasoning power

### Medium Tasks → Auto-Optimized
```typescript
await agent.executeTask({
  name: 'refactor-function',
  complexity: 'medium', // Balance cost/quality
  estimatedTokens: 1500,
  execute: async (provider) => refactorFunction(code)
});
```

**Rationale:** Uses fallback strategy (cost/performance)

### Complex Tasks → Prefer Claude
```typescript
await agent.executeTask({
  name: 'design-architecture',
  complexity: 'complex', // Quality matters most
  estimatedTokens: 5000,
  execute: async (provider) => designArchitecture(requirements)
});
```

**Rationale:** Complex reasoning benefits from Claude's capabilities

## Circuit Breaker

Prevents cascading failures by temporarily disabling failing providers.

### How It Works

1. **Failure Tracking:** Count consecutive failures per provider
2. **Threshold:** Open circuit after N failures (configurable)
3. **Recovery:** Automatically recover after timeout
4. **Fallback:** Use next available provider

### Configuration

```typescript
{
  maxFailures: 3, // Open circuit after 3 consecutive failures
  recoveryTime: 60000, // Try recovery after 60 seconds
  retryBackoff: 'exponential' // 1s, 2s, 4s, 8s, 16s...
}
```

### Monitoring

```typescript
const health = manager.getHealth();

health.forEach(h => {
  console.log(`${h.provider}:`);
  console.log(`  Circuit Breaker: ${h.circuitBreakerOpen ? 'OPEN' : 'CLOSED'}`);
  console.log(`  Consecutive Failures: ${h.consecutiveFailures}`);
  console.log(`  Success Rate: ${(h.successRate * 100).toFixed(1)}%`);
});
```

## Cost Tracking & Optimization

### Real-Time Cost Monitoring

```typescript
const costs = manager.getCostSummary();

console.log(`Total: $${costs.total.toFixed(4)}`);
console.log(`Tokens: ${costs.totalTokens.toLocaleString()}`);

for (const [provider, cost] of Object.entries(costs.byProvider)) {
  console.log(`  ${provider}: $${cost.toFixed(4)}`);
}
```

### Budget Constraints

```typescript
const agent = new LongRunningAgent({
  agentName: 'budget-agent',
  providers,
  costBudget: 10.00, // Hard limit: $10
  // ... other config
});

// Agent automatically stops when budget exceeded
```

### Cost-Per-Provider Configuration

```typescript
const providers: ProviderConfig[] = [
  {
    name: 'gemini',
    costPerToken: 0.00015, // $0.15 per 1M tokens
    // ...
  },
  {
    name: 'anthropic',
    costPerToken: 0.003, // $3 per 1M tokens (Sonnet)
    // ...
  },
  {
    name: 'onnx',
    costPerToken: 0, // FREE (local)
    // ...
  }
];
```

## Health Monitoring

### Automatic Health Checks

```typescript
const providers: ProviderConfig[] = [
  {
    name: 'gemini',
    healthCheckInterval: 60000, // Check every minute
    // ...
  }
];
```

### Manual Health Check

```typescript
const health = manager.getHealth();

health.forEach(h => {
  console.log(`${h.provider}:`);
  console.log(`  Healthy: ${h.isHealthy}`);
  console.log(`  Success Rate: ${(h.successRate * 100).toFixed(1)}%`);
  console.log(`  Avg Latency: ${h.averageLatency.toFixed(0)}ms`);
  console.log(`  Error Rate: ${(h.errorRate * 100).toFixed(1)}%`);
});
```

### Metrics Collection

```typescript
const metrics = manager.getMetrics();

metrics.forEach(m => {
  console.log(`${m.provider}:`);
  console.log(`  Total Requests: ${m.totalRequests}`);
  console.log(`  Successful: ${m.successfulRequests}`);
  console.log(`  Failed: ${m.failedRequests}`);
  console.log(`  Avg Latency: ${m.averageLatency.toFixed(0)}ms`);
  console.log(`  Total Cost: $${m.totalCost.toFixed(4)}`);
});
```

## Checkpointing & Recovery

### Automatic Checkpoints

```typescript
const agent = new LongRunningAgent({
  agentName: 'checkpoint-agent',
  providers,
  checkpointInterval: 30000, // Save every 30 seconds
  // ...
});

await agent.start();

// Agent automatically saves checkpoints every 30s
// On crash, restore from last checkpoint
```

### Manual Checkpoint Management

```typescript
// Get all checkpoints
const metrics = agent.getMetrics();
const checkpoints = metrics.checkpoints;

// Restore from specific checkpoint
const lastCheckpoint = checkpoints[checkpoints.length - 1];
agent.restoreFromCheckpoint(lastCheckpoint);
```

### Checkpoint Data

```typescript
interface AgentCheckpoint {
  timestamp: Date;
  taskProgress: number; // 0-1
  currentProvider: string;
  totalCost: number;
  totalTokens: number;
  completedTasks: number;
  failedTasks: number;
  state: Record<string, any>; // Custom state
}
```

## Retry Logic

### Exponential Backoff (Recommended)

```typescript
{
  retryBackoff: 'exponential'
}
```

**Delays:** 1s, 2s, 4s, 8s, 16s, 30s (max)

**Use Case:** Rate limits, transient errors

### Linear Backoff

```typescript
{
  retryBackoff: 'linear'
}
```

**Delays:** 1s, 2s, 3s, 4s, 5s, 10s (max)

**Use Case:** Predictable retry patterns

### Retryable Errors

Automatically retried:
- `rate limit`
- `timeout`
- `connection`
- `network`
- HTTP 503, 502, 429

Non-retryable errors fail immediately:
- Authentication errors
- Invalid requests
- HTTP 4xx (except 429)

## Production Best Practices

### 1. Multi-Provider Strategy

```typescript
const providers: ProviderConfig[] = [
  // Primary: Fast & cheap for simple tasks
  { name: 'gemini', priority: 1, costPerToken: 0.00015 },

  // Fallback: High quality for complex tasks
  { name: 'anthropic', priority: 2, costPerToken: 0.003 },

  // Emergency: Free local inference
  { name: 'onnx', priority: 3, costPerToken: 0 }
];
```

### 2. Cost Optimization

```typescript
// Use cost-optimized strategy for high-volume
const agent = new LongRunningAgent({
  agentName: 'production-agent',
  providers,
  fallbackStrategy: {
    type: 'cost-optimized',
    costThreshold: 0.50
  },
  costBudget: 100.00 // Daily budget
});
```

### 3. Health Monitoring

```typescript
// Monitor provider health every minute
const providers: ProviderConfig[] = [
  {
    name: 'gemini',
    healthCheckInterval: 60000,
    enabled: true
  }
];

// Check health before critical operations
const health = manager.getHealth();
const unhealthy = health.filter(h => !h.isHealthy);

if (unhealthy.length > 0) {
  console.warn('Unhealthy providers:', unhealthy.map(h => h.provider));
}
```

### 4. Graceful Degradation

```typescript
// Prefer quality, fallback to cost
const providers: ProviderConfig[] = [
  { name: 'anthropic', priority: 1 }, // Best quality
  { name: 'gemini', priority: 2 },     // Cheaper fallback
  { name: 'onnx', priority: 3 }        // Always available
];
```

### 5. Circuit Breaker Tuning

```typescript
{
  maxFailures: 5, // More tolerant in production
  recoveryTime: 300000, // 5 minutes before retry
  retryBackoff: 'exponential'
}
```

## Docker Validation

### Build Image

```bash
docker build -f Dockerfile.provider-fallback -t agentic-flow-provider-fallback .
```

### Run Tests

```bash
# With Gemini API key
docker run --rm \
  -e GOOGLE_GEMINI_API_KEY=your_key_here \
  agentic-flow-provider-fallback

# ONNX only (no API key needed)
docker run --rm agentic-flow-provider-fallback
```

### Expected Output

```
✅ Provider Fallback Validation Test
====================================

📋 Testing Provider Manager...

1️⃣  Building TypeScript...
✅ Build complete

2️⃣  Running provider fallback example...
   Using Gemini API key: AIza...
🚀 Starting Long-Running Agent with Provider Fallback

📋 Task 1: Simple Code Generation (Gemini optimal)
  Using provider: gemini
  ✅ Result: { code: 'console.log("Hello World");', provider: 'gemini' }

📋 Task 2: Complex Architecture Design (Claude optimal)
  Using provider: anthropic
  ✅ Result: { architecture: 'Event-driven microservices', provider: 'anthropic' }

📈 Provider Health:
gemini:
  Healthy: true
  Success Rate: 100.0%
  Circuit Breaker: CLOSED

✅ All provider fallback tests passed!
```

## API Reference

### ProviderManager

```typescript
class ProviderManager {
  constructor(providers: ProviderConfig[], strategy: FallbackStrategy);

  selectProvider(
    taskComplexity?: 'simple' | 'medium' | 'complex',
    estimatedTokens?: number
  ): Promise<ProviderType>;

  executeWithFallback<T>(
    requestFn: (provider: ProviderType) => Promise<T>,
    taskComplexity?: 'simple' | 'medium' | 'complex',
    estimatedTokens?: number
  ): Promise<{ result: T; provider: ProviderType; attempts: number }>;

  getMetrics(): ProviderMetrics[];
  getHealth(): ProviderHealth[];
  getCostSummary(): { total: number; byProvider: Record<ProviderType, number>; totalTokens: number };
  destroy(): void;
}
```

### LongRunningAgent

```typescript
class LongRunningAgent {
  constructor(config: LongRunningAgentConfig);

  start(): Promise<void>;
  stop(): Promise<void>;

  executeTask<T>(task: {
    name: string;
    complexity: 'simple' | 'medium' | 'complex';
    estimatedTokens?: number;
    execute: (provider: string) => Promise<T>;
  }): Promise<T>;

  getStatus(): AgentStatus;
  getMetrics(): AgentMetrics;
  restoreFromCheckpoint(checkpoint: AgentCheckpoint): void;
}
```

## Examples

See `src/examples/use-provider-fallback.ts` for complete working examples.

## Support

- **GitHub Issues:** https://github.com/ruvnet/agentic-flow/issues
- **Documentation:** https://github.com/ruvnet/agentic-flow#readme
- **Discord:** Coming soon

## License

MIT - See LICENSE file for details
