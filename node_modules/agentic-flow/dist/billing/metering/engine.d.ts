/**
 * Usage Metering Engine
 * Real-time usage tracking and aggregation
 */
import { EventEmitter } from 'events';
import { UsageRecord, UsageMetric, UsageSummary, UsageLimits, QuotaCheckResult, StorageAdapter } from '../types.js';
export interface MeteringConfig {
    enabled: boolean;
    bufferSize: number;
    flushInterval: number;
    softLimitPercent: number;
    hardLimitPercent: number;
}
export declare class MeteringEngine extends EventEmitter {
    private config;
    private storage;
    private buffer;
    private flushTimer?;
    private usageCache;
    constructor(storage: StorageAdapter, config?: Partial<MeteringConfig>);
    /**
     * Record a usage event
     */
    recordUsage(record: Omit<UsageRecord, 'id' | 'timestamp' | 'billingPeriod'>): Promise<void>;
    /**
     * Check if usage is within quota
     */
    checkQuota(subscriptionId: string, metric: UsageMetric, limits: UsageLimits): Promise<QuotaCheckResult>;
    /**
     * Get usage summary for a subscription
     */
    getUsageSummary(subscriptionId: string, limits: UsageLimits): Promise<UsageSummary>;
    /**
     * Get current usage for a metric
     */
    private getCurrentUsage;
    /**
     * Flush buffered records to storage
     */
    private flush;
    /**
     * Start automatic flush timer
     */
    private startFlushTimer;
    /**
     * Stop the metering engine
     */
    stop(): Promise<void>;
    /**
     * Clear cache for a subscription
     */
    clearCache(subscriptionId?: string): void;
    private getLimitForMetric;
    private calculateOverageCost;
    private getCurrentBillingPeriod;
    private generateId;
}
//# sourceMappingURL=engine.d.ts.map