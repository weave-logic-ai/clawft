/**
 * Usage Metering Engine
 * Real-time usage tracking and aggregation
 */
import { EventEmitter } from 'events';
import { UsageMetric } from '../types.js';
export class MeteringEngine extends EventEmitter {
    config;
    storage;
    buffer = [];
    flushTimer;
    usageCache = new Map();
    constructor(storage, config) {
        super();
        this.storage = storage;
        this.config = {
            enabled: true,
            bufferSize: 100,
            flushInterval: 5000, // 5 seconds
            softLimitPercent: 80,
            hardLimitPercent: 100,
            ...config
        };
        if (this.config.enabled) {
            this.startFlushTimer();
        }
    }
    /**
     * Record a usage event
     */
    async recordUsage(record) {
        if (!this.config.enabled)
            return;
        const fullRecord = {
            ...record,
            id: this.generateId(),
            timestamp: new Date(),
            billingPeriod: this.getCurrentBillingPeriod()
        };
        // Update cache
        const userMetrics = this.usageCache.get(record.subscriptionId) || new Map();
        const current = userMetrics.get(record.metric) || 0;
        userMetrics.set(record.metric, current + record.amount);
        this.usageCache.set(record.subscriptionId, userMetrics);
        // Buffer the record
        this.buffer.push(fullRecord);
        // Emit event
        this.emit('usage.recorded', fullRecord);
        // Flush if buffer is full
        if (this.buffer.length >= this.config.bufferSize) {
            await this.flush();
        }
    }
    /**
     * Check if usage is within quota
     */
    async checkQuota(subscriptionId, metric, limits) {
        const current = await this.getCurrentUsage(subscriptionId, metric);
        const limit = this.getLimitForMetric(metric, limits);
        // Unlimited
        if (limit === -1) {
            return {
                allowed: true,
                metric,
                current,
                limit,
                percentUsed: 0,
                remaining: Infinity,
                overage: 0
            };
        }
        const percentUsed = (current / limit) * 100;
        const remaining = Math.max(0, limit - current);
        const overage = Math.max(0, current - limit);
        // Check soft limit
        if (percentUsed >= this.config.softLimitPercent && percentUsed < this.config.hardLimitPercent) {
            this.emit('quota.warning', {
                subscriptionId,
                metric,
                percentUsed,
                current,
                limit
            });
        }
        // Check hard limit
        if (percentUsed >= this.config.hardLimitPercent) {
            this.emit('quota.exceeded', {
                subscriptionId,
                metric,
                current,
                limit,
                overage
            });
            return {
                allowed: false,
                metric,
                current,
                limit,
                percentUsed,
                remaining,
                overage,
                warning: `Quota exceeded for ${metric}. Current: ${current}, Limit: ${limit}`
            };
        }
        return {
            allowed: true,
            metric,
            current,
            limit,
            percentUsed,
            remaining,
            overage
        };
    }
    /**
     * Get usage summary for a subscription
     */
    async getUsageSummary(subscriptionId, limits) {
        const period = this.getCurrentBillingPeriod();
        const metrics = new Map();
        const percentUsed = new Map();
        const overages = new Map();
        // Get all metrics
        const allMetrics = Object.values(UsageMetric);
        for (const metric of allMetrics) {
            const current = await this.getCurrentUsage(subscriptionId, metric);
            const limit = this.getLimitForMetric(metric, limits);
            metrics.set(metric, current);
            if (limit !== -1) {
                const percent = (current / limit) * 100;
                percentUsed.set(metric, percent);
                if (current > limit) {
                    overages.set(metric, current - limit);
                }
            }
        }
        // Calculate estimated cost based on overages
        const estimatedCost = this.calculateOverageCost(overages);
        return {
            subscriptionId,
            userId: '', // Will be set by caller
            period,
            metrics,
            limits,
            percentUsed,
            overages,
            estimatedCost
        };
    }
    /**
     * Get current usage for a metric
     */
    async getCurrentUsage(subscriptionId, metric) {
        // Check cache first
        const cached = this.usageCache.get(subscriptionId)?.get(metric);
        if (cached !== undefined) {
            return cached;
        }
        // Query storage
        const period = this.getCurrentBillingPeriod();
        const records = await this.storage.getUsageRecords(subscriptionId, period);
        const total = records
            .filter(r => r.metric === metric)
            .reduce((sum, r) => sum + r.amount, 0);
        // Update cache
        const userMetrics = this.usageCache.get(subscriptionId) || new Map();
        userMetrics.set(metric, total);
        this.usageCache.set(subscriptionId, userMetrics);
        return total;
    }
    /**
     * Flush buffered records to storage
     */
    async flush() {
        if (this.buffer.length === 0)
            return;
        const records = [...this.buffer];
        this.buffer = [];
        try {
            await Promise.all(records.map(r => this.storage.saveUsageRecord(r)));
            this.emit('flush.success', { count: records.length });
        }
        catch (error) {
            this.emit('flush.error', error);
            // Re-add to buffer on error
            this.buffer.unshift(...records);
        }
    }
    /**
     * Start automatic flush timer
     */
    startFlushTimer() {
        this.flushTimer = setInterval(() => {
            this.flush();
        }, this.config.flushInterval);
    }
    /**
     * Stop the metering engine
     */
    async stop() {
        if (this.flushTimer) {
            clearInterval(this.flushTimer);
        }
        await this.flush();
    }
    /**
     * Clear cache for a subscription
     */
    clearCache(subscriptionId) {
        if (subscriptionId) {
            this.usageCache.delete(subscriptionId);
        }
        else {
            this.usageCache.clear();
        }
    }
    getLimitForMetric(metric, limits) {
        const metricToLimitMap = {
            [UsageMetric.AgentHours]: 'maxAgentHours',
            [UsageMetric.Deployments]: 'maxDeployments',
            [UsageMetric.APIRequests]: 'maxAPIRequests',
            [UsageMetric.StorageGB]: 'maxStorageGB',
            [UsageMetric.SwarmSize]: 'maxSwarmSize',
            [UsageMetric.GPUHours]: 'maxGPUHours',
            [UsageMetric.BandwidthGB]: 'maxBandwidthGB',
            [UsageMetric.ConcurrentJobs]: 'maxConcurrentJobs',
            [UsageMetric.TeamMembers]: 'maxTeamMembers',
            [UsageMetric.CustomDomains]: 'maxCustomDomains'
        };
        return limits[metricToLimitMap[metric]];
    }
    calculateOverageCost(overages) {
        // Example overage rates (per unit)
        const rates = {
            [UsageMetric.AgentHours]: 0.50,
            [UsageMetric.Deployments]: 5.00,
            [UsageMetric.APIRequests]: 0.0001,
            [UsageMetric.StorageGB]: 0.10,
            [UsageMetric.SwarmSize]: 10.00,
            [UsageMetric.GPUHours]: 2.00,
            [UsageMetric.BandwidthGB]: 0.05,
            [UsageMetric.ConcurrentJobs]: 5.00,
            [UsageMetric.TeamMembers]: 15.00,
            [UsageMetric.CustomDomains]: 10.00
        };
        let total = 0;
        overages.forEach((amount, metric) => {
            total += amount * (rates[metric] || 0);
        });
        return total;
    }
    getCurrentBillingPeriod() {
        const now = new Date();
        return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`;
    }
    generateId() {
        return `usage_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    }
}
//# sourceMappingURL=engine.js.map