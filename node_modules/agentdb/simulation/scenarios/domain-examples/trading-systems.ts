/**
 * Trading Systems: Ultra-Low Latency Vector Search
 *
 * Use Case: High-frequency trading systems need sub-microsecond
 * similarity search for pattern matching and strategy execution.
 *
 * Optimization Priority: LATENCY (quality trade-off acceptable)
 */

import { UnifiedMetrics } from '../../types';

export const TRADING_ATTENTION_CONFIG = {
  heads: 4,                        // Fewer heads = faster (vs 8-head optimal)
  forwardPassTargetUs: 500,        // Sub-millisecond target (500μs)
  batchSize: 1,                    // Single-query latency critical
  precision: 'float16' as const,   // Reduced precision for speed
  cachingStrategy: 'aggressive' as const, // Pre-compute common patterns

  // Dynamic-k optimized for market conditions
  dynamicK: {
    min: 3,                        // Minimum candidates (fast fallback)
    max: 10,                       // Maximum candidates (volatile markets)
    adaptationStrategy: 'market-volatility' as const
  },

  // MPC self-healing for 24/7 operation
  selfHealing: {
    enabled: true,
    adaptationIntervalMs: 50,      // 50ms response (vs 100ms general)
    degradationThreshold: 0.02     // 2% tolerance (vs 5% general)
  }
};

// Trading-specific metrics
export interface TradingMetrics extends UnifiedMetrics {
  executionLatencyUs: number;      // Order execution latency
  marketDataLatencyUs: number;     // Market data processing latency
  strategyMatchAccuracy: number;   // Pattern match accuracy
  falsePositiveRate: number;       // False signal rate (critical for trading)
  uptime99_99: number;             // 99.99% uptime requirement
}

// Trading signal interface
export interface TradingSignal {
  strategy: string;
  confidence: number;
  executionTimeUs: number;
}

// Example: Pattern matching for trading strategies
export async function matchTradingPattern(
  marketData: Float32Array,
  strategyDatabase: any, // HNSWGraph type
  getCurrentVolatility: () => number,
  applyAttention: (data: Float32Array, config: any) => Promise<Float32Array>,
  adaptKToVolatility: (volatility: number) => number
): Promise<TradingSignal[]> {
  const config = TRADING_ATTENTION_CONFIG;

  // 4-head attention for fast pattern matching
  const enhanced = await applyAttention(marketData, config);

  // Dynamic-k based on market volatility
  const k = adaptKToVolatility(getCurrentVolatility());

  // Search for matching strategies
  const matches = await strategyDatabase.search(enhanced, k);

  return matches.map((m: any) => ({
    strategy: m.id,
    confidence: m.score,
    executionTimeUs: m.latencyUs  // Track latency for each match
  }));
}

// Performance targets for trading
export const TRADING_PERFORMANCE_TARGETS = {
  p50LatencyUs: 500,               // 500μs median latency
  p99LatencyUs: 2000,              // 2ms p99 latency
  throughputQPS: 100000,           // 100K queries/sec
  accuracy: 0.92,                  // 92% pattern match accuracy (vs 96.8% general)
  uptimePercent: 99.99             // 99.99% uptime (4 nines)
};

// Example configuration variations
export const TRADING_CONFIG_VARIATIONS = {
  // Ultra-low latency (300μs target)
  ultraLowLatency: {
    ...TRADING_ATTENTION_CONFIG,
    heads: 2,                      // Even fewer heads
    forwardPassTargetUs: 300,
    precision: 'int8' as const     // Quantized for speed
  },

  // Balanced (1ms target, better accuracy)
  balanced: {
    ...TRADING_ATTENTION_CONFIG,
    heads: 6,
    forwardPassTargetUs: 1000,
    precision: 'float32' as const
  },

  // High-frequency scalping (extreme speed)
  scalping: {
    ...TRADING_ATTENTION_CONFIG,
    heads: 2,
    forwardPassTargetUs: 200,
    batchSize: 1,
    precision: 'int8' as const,
    cachingStrategy: 'precompute' as const
  }
};

// Market condition adaptations
export function adaptConfigToMarket(
  baseConfig: typeof TRADING_ATTENTION_CONFIG,
  marketCondition: 'calm' | 'volatile' | 'trending'
): typeof TRADING_ATTENTION_CONFIG {
  switch (marketCondition) {
    case 'calm':
      return {
        ...baseConfig,
        dynamicK: { ...baseConfig.dynamicK, min: 3, max: 7 }
      };
    case 'volatile':
      return {
        ...baseConfig,
        dynamicK: { ...baseConfig.dynamicK, min: 5, max: 15 },
        selfHealing: { ...baseConfig.selfHealing, adaptationIntervalMs: 25 }
      };
    case 'trending':
      return {
        ...baseConfig,
        heads: 6, // More heads for pattern recognition
        dynamicK: { ...baseConfig.dynamicK, min: 7, max: 12 }
      };
  }
}
