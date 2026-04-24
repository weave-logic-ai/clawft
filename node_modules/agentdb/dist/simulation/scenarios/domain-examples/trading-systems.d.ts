/**
 * Trading Systems: Ultra-Low Latency Vector Search
 *
 * Use Case: High-frequency trading systems need sub-microsecond
 * similarity search for pattern matching and strategy execution.
 *
 * Optimization Priority: LATENCY (quality trade-off acceptable)
 */
import { UnifiedMetrics } from '../../types';
export declare const TRADING_ATTENTION_CONFIG: {
    heads: number;
    forwardPassTargetUs: number;
    batchSize: number;
    precision: "float16";
    cachingStrategy: "aggressive";
    dynamicK: {
        min: number;
        max: number;
        adaptationStrategy: "market-volatility";
    };
    selfHealing: {
        enabled: boolean;
        adaptationIntervalMs: number;
        degradationThreshold: number;
    };
};
export interface TradingMetrics extends UnifiedMetrics {
    executionLatencyUs: number;
    marketDataLatencyUs: number;
    strategyMatchAccuracy: number;
    falsePositiveRate: number;
    uptime99_99: number;
}
export interface TradingSignal {
    strategy: string;
    confidence: number;
    executionTimeUs: number;
}
export declare function matchTradingPattern(marketData: Float32Array, strategyDatabase: any, // HNSWGraph type
getCurrentVolatility: () => number, applyAttention: (data: Float32Array, config: any) => Promise<Float32Array>, adaptKToVolatility: (volatility: number) => number): Promise<TradingSignal[]>;
export declare const TRADING_PERFORMANCE_TARGETS: {
    p50LatencyUs: number;
    p99LatencyUs: number;
    throughputQPS: number;
    accuracy: number;
    uptimePercent: number;
};
export declare const TRADING_CONFIG_VARIATIONS: {
    ultraLowLatency: {
        heads: number;
        forwardPassTargetUs: number;
        precision: "int8";
        batchSize: number;
        cachingStrategy: "aggressive";
        dynamicK: {
            min: number;
            max: number;
            adaptationStrategy: "market-volatility";
        };
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
        };
    };
    balanced: {
        heads: number;
        forwardPassTargetUs: number;
        precision: "float32";
        batchSize: number;
        cachingStrategy: "aggressive";
        dynamicK: {
            min: number;
            max: number;
            adaptationStrategy: "market-volatility";
        };
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
        };
    };
    scalping: {
        heads: number;
        forwardPassTargetUs: number;
        batchSize: number;
        precision: "int8";
        cachingStrategy: "precompute";
        dynamicK: {
            min: number;
            max: number;
            adaptationStrategy: "market-volatility";
        };
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
        };
    };
};
export declare function adaptConfigToMarket(baseConfig: typeof TRADING_ATTENTION_CONFIG, marketCondition: 'calm' | 'volatile' | 'trending'): typeof TRADING_ATTENTION_CONFIG;
//# sourceMappingURL=trading-systems.d.ts.map