/**
 * Performance Metrics and Monitoring for Attention Mechanisms
 * Real-time tracking of latency, throughput, and memory usage
 */

export interface AttentionMetrics {
  mechanism: string;
  operationCount: number;
  totalLatencyMs: number;
  avgLatencyUs: number;
  minLatencyUs: number;
  maxLatencyUs: number;
  p50LatencyUs: number;
  p95LatencyUs: number;
  p99LatencyUs: number;
  throughputOpsPerSec: number;
  memoryUsageBytes: number;
  peakMemoryBytes: number;
  allocationCount: number;
}

export interface OperationMetrics {
  startTime: number;
  endTime: number;
  latencyUs: number;
  memoryBefore: number;
  memoryAfter: number;
  memoryDelta: number;
}

export class AttentionMetricsCollector {
  private metrics: Map<string, OperationMetrics[]> = new Map();
  private startMemory: Map<string, number> = new Map();

  /**
   * Start measuring an operation
   */
  startOperation(mechanism: string): void {
    const memoryBefore = process.memoryUsage().heapUsed;
    this.startMemory.set(mechanism, memoryBefore);
  }

  /**
   * End measuring an operation and record metrics
   */
  endOperation(mechanism: string, startTime: number): void {
    const endTime = performance.now();
    const memoryBefore = this.startMemory.get(mechanism) || 0;
    const memoryAfter = process.memoryUsage().heapUsed;

    const operationMetrics: OperationMetrics = {
      startTime,
      endTime,
      latencyUs: (endTime - startTime) * 1000, // Convert ms to µs
      memoryBefore,
      memoryAfter,
      memoryDelta: memoryAfter - memoryBefore,
    };

    if (!this.metrics.has(mechanism)) {
      this.metrics.set(mechanism, []);
    }
    this.metrics.get(mechanism)!.push(operationMetrics);
    this.startMemory.delete(mechanism);
  }

  /**
   * Get aggregated metrics for a mechanism
   */
  getMetrics(mechanism: string): AttentionMetrics | null {
    const operations = this.metrics.get(mechanism);
    if (!operations || operations.length === 0) {
      return null;
    }

    const latencies = operations.map(op => op.latencyUs).sort((a, b) => a - b);
    const memoryDeltas = operations.map(op => op.memoryDelta);
    const totalLatencyMs = latencies.reduce((sum, lat) => sum + lat, 0) / 1000;
    const totalTimeMs = operations[operations.length - 1].endTime - operations[0].startTime;

    // Calculate percentiles
    const p50Index = Math.floor(latencies.length * 0.5);
    const p95Index = Math.floor(latencies.length * 0.95);
    const p99Index = Math.floor(latencies.length * 0.99);

    return {
      mechanism,
      operationCount: operations.length,
      totalLatencyMs,
      avgLatencyUs: latencies.reduce((sum, lat) => sum + lat, 0) / latencies.length,
      minLatencyUs: latencies[0],
      maxLatencyUs: latencies[latencies.length - 1],
      p50LatencyUs: latencies[p50Index],
      p95LatencyUs: latencies[p95Index],
      p99LatencyUs: latencies[p99Index],
      throughputOpsPerSec: totalTimeMs > 0 ? (operations.length / totalTimeMs) * 1000 : 0,
      memoryUsageBytes: operations[operations.length - 1].memoryAfter,
      peakMemoryBytes: Math.max(...operations.map(op => op.memoryAfter)),
      allocationCount: memoryDeltas.filter(delta => delta > 0).length,
    };
  }

  /**
   * Get all collected metrics
   */
  getAllMetrics(): Map<string, AttentionMetrics> {
    const allMetrics = new Map<string, AttentionMetrics>();
    for (const mechanism of this.metrics.keys()) {
      const metrics = this.getMetrics(mechanism);
      if (metrics) {
        allMetrics.set(mechanism, metrics);
      }
    }
    return allMetrics;
  }

  /**
   * Reset all metrics
   */
  reset(): void {
    this.metrics.clear();
    this.startMemory.clear();
  }

  /**
   * Export metrics as JSON
   */
  exportJSON(): string {
    const allMetrics: Record<string, AttentionMetrics> = {};
    for (const [mechanism, metrics] of this.getAllMetrics()) {
      allMetrics[mechanism] = metrics;
    }
    return JSON.stringify(allMetrics, null, 2);
  }

  /**
   * Export metrics in markdown format
   */
  exportMarkdown(): string {
    const lines: string[] = [
      '# Attention Mechanism Performance Metrics',
      '',
      '## Summary',
      '',
      '| Mechanism | Ops | Avg Latency (µs) | P95 (µs) | P99 (µs) | Throughput (ops/s) | Memory (MB) |',
      '|-----------|-----|------------------|----------|----------|-------------------|-------------|',
    ];

    for (const [mechanism, metrics] of this.getAllMetrics()) {
      lines.push(
        `| ${mechanism} | ${metrics.operationCount} | ${metrics.avgLatencyUs.toFixed(2)} | ${metrics.p95LatencyUs.toFixed(2)} | ${metrics.p99LatencyUs.toFixed(2)} | ${metrics.throughputOpsPerSec.toFixed(2)} | ${(metrics.memoryUsageBytes / 1024 / 1024).toFixed(2)} |`
      );
    }

    lines.push('', '## Detailed Metrics', '');

    for (const [mechanism, metrics] of this.getAllMetrics()) {
      lines.push(
        `### ${mechanism}`,
        '',
        `- **Operations**: ${metrics.operationCount}`,
        `- **Total Latency**: ${metrics.totalLatencyMs.toFixed(2)} ms`,
        `- **Average Latency**: ${metrics.avgLatencyUs.toFixed(2)} µs`,
        `- **Min Latency**: ${metrics.minLatencyUs.toFixed(2)} µs`,
        `- **Max Latency**: ${metrics.maxLatencyUs.toFixed(2)} µs`,
        `- **P50 Latency**: ${metrics.p50LatencyUs.toFixed(2)} µs`,
        `- **P95 Latency**: ${metrics.p95LatencyUs.toFixed(2)} µs`,
        `- **P99 Latency**: ${metrics.p99LatencyUs.toFixed(2)} µs`,
        `- **Throughput**: ${metrics.throughputOpsPerSec.toFixed(2)} ops/sec`,
        `- **Memory Usage**: ${(metrics.memoryUsageBytes / 1024 / 1024).toFixed(2)} MB`,
        `- **Peak Memory**: ${(metrics.peakMemoryBytes / 1024 / 1024).toFixed(2)} MB`,
        `- **Allocations**: ${metrics.allocationCount}`,
        ''
      );
    }

    return lines.join('\n');
  }
}

/**
 * Global metrics collector instance
 */
export const metricsCollector = new AttentionMetricsCollector();

/**
 * Decorator for measuring function performance
 */
export function measurePerformance(mechanism: string) {
  return function (
    target: any,
    propertyKey: string,
    descriptor: PropertyDescriptor
  ) {
    const originalMethod = descriptor.value;

    descriptor.value = async function (...args: any[]) {
      metricsCollector.startOperation(mechanism);
      const startTime = performance.now();

      try {
        const result = await originalMethod.apply(this, args);
        metricsCollector.endOperation(mechanism, startTime);
        return result;
      } catch (error) {
        metricsCollector.endOperation(mechanism, startTime);
        throw error;
      }
    };

    return descriptor;
  };
}

/**
 * Helper to measure a synchronous operation
 */
export function measureSync<T>(
  mechanism: string,
  operation: () => T
): T {
  metricsCollector.startOperation(mechanism);
  const startTime = performance.now();

  try {
    const result = operation();
    metricsCollector.endOperation(mechanism, startTime);
    return result;
  } catch (error) {
    metricsCollector.endOperation(mechanism, startTime);
    throw error;
  }
}

/**
 * Helper to measure an async operation
 */
export async function measureAsync<T>(
  mechanism: string,
  operation: () => Promise<T>
): Promise<T> {
  metricsCollector.startOperation(mechanism);
  const startTime = performance.now();

  try {
    const result = await operation();
    metricsCollector.endOperation(mechanism, startTime);
    return result;
  } catch (error) {
    metricsCollector.endOperation(mechanism, startTime);
    throw error;
  }
}
