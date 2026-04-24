/**
 * Long-Running Agent with Provider Fallback
 *
 * Demonstrates how to use ProviderManager for resilient, cost-optimized agents
 * that can run for hours or days with automatic provider switching.
 */
import { ProviderManager } from './provider-manager.js';
import { logger } from '../utils/logger.js';
export class LongRunningAgent {
    providerManager;
    config;
    startTime;
    checkpoints = [];
    currentState = {};
    isRunning = false;
    checkpointInterval;
    constructor(config) {
        this.config = config;
        this.startTime = new Date();
        // Initialize provider manager
        this.providerManager = new ProviderManager(config.providers, config.fallbackStrategy);
        logger.info('Long-running agent initialized', {
            agentName: config.agentName,
            providers: config.providers.map(p => p.name)
        });
    }
    /**
     * Start the agent with automatic checkpointing
     */
    async start() {
        this.isRunning = true;
        this.startTime = new Date();
        // Start checkpoint interval
        if (this.config.checkpointInterval) {
            this.checkpointInterval = setInterval(() => {
                this.saveCheckpoint();
            }, this.config.checkpointInterval);
        }
        logger.info('Long-running agent started', {
            agentName: this.config.agentName,
            startTime: this.startTime
        });
    }
    /**
     * Execute a task with automatic provider fallback
     */
    async executeTask(task) {
        if (!this.isRunning) {
            throw new Error('Agent not running. Call start() first.');
        }
        // Check budget constraint
        if (this.config.costBudget) {
            const currentCost = this.providerManager.getCostSummary().total;
            if (currentCost >= this.config.costBudget) {
                throw new Error(`Cost budget exceeded: $${currentCost.toFixed(2)} >= $${this.config.costBudget}`);
            }
        }
        // Check runtime constraint
        if (this.config.maxRuntime) {
            const runtime = Date.now() - this.startTime.getTime();
            if (runtime >= this.config.maxRuntime) {
                throw new Error(`Max runtime exceeded: ${runtime}ms >= ${this.config.maxRuntime}ms`);
            }
        }
        logger.info('Executing task', {
            agentName: this.config.agentName,
            taskName: task.name,
            complexity: task.complexity
        });
        try {
            // Execute with automatic fallback
            const { result, provider, attempts } = await this.providerManager.executeWithFallback(task.execute, task.complexity, task.estimatedTokens);
            // Update state
            this.currentState.lastTask = task.name;
            this.currentState.lastProvider = provider;
            this.currentState.completedTasks = (this.currentState.completedTasks || 0) + 1;
            logger.info('Task completed', {
                agentName: this.config.agentName,
                taskName: task.name,
                provider,
                attempts
            });
            return result;
        }
        catch (error) {
            this.currentState.failedTasks = (this.currentState.failedTasks || 0) + 1;
            logger.error('Task failed', {
                agentName: this.config.agentName,
                taskName: task.name,
                error: error.message
            });
            throw error;
        }
    }
    /**
     * Save checkpoint of current state
     */
    saveCheckpoint() {
        const costSummary = this.providerManager.getCostSummary();
        const health = this.providerManager.getHealth();
        const checkpoint = {
            timestamp: new Date(),
            taskProgress: this.calculateProgress(),
            currentProvider: this.currentState.lastProvider || 'none',
            totalCost: costSummary.total,
            totalTokens: costSummary.totalTokens,
            completedTasks: this.currentState.completedTasks || 0,
            failedTasks: this.currentState.failedTasks || 0,
            state: { ...this.currentState }
        };
        this.checkpoints.push(checkpoint);
        logger.info('Checkpoint saved', {
            agentName: this.config.agentName,
            checkpoint: {
                ...checkpoint,
                state: undefined // Don't log full state
            }
        });
        // Alert if cost approaching budget
        if (this.config.costBudget) {
            const costPercentage = (costSummary.total / this.config.costBudget) * 100;
            if (costPercentage >= 80) {
                logger.warn('Cost budget warning', {
                    agentName: this.config.agentName,
                    currentCost: costSummary.total,
                    budget: this.config.costBudget,
                    percentage: costPercentage.toFixed(1) + '%'
                });
            }
        }
        // Alert if providers unhealthy
        const unhealthyProviders = health.filter(h => !h.isHealthy || h.circuitBreakerOpen);
        if (unhealthyProviders.length > 0) {
            logger.warn('Unhealthy providers detected', {
                agentName: this.config.agentName,
                unhealthy: unhealthyProviders.map(h => ({
                    provider: h.provider,
                    circuitBreakerOpen: h.circuitBreakerOpen,
                    consecutiveFailures: h.consecutiveFailures
                }))
            });
        }
    }
    /**
     * Calculate task progress (override in subclass)
     */
    calculateProgress() {
        // Default: based on completed vs total tasks
        const completed = this.currentState.completedTasks || 0;
        const failed = this.currentState.failedTasks || 0;
        const total = completed + failed;
        return total > 0 ? completed / total : 0;
    }
    /**
     * Get current status
     */
    getStatus() {
        const costSummary = this.providerManager.getCostSummary();
        const health = this.providerManager.getHealth();
        const runtime = Date.now() - this.startTime.getTime();
        return {
            isRunning: this.isRunning,
            runtime,
            completedTasks: this.currentState.completedTasks || 0,
            failedTasks: this.currentState.failedTasks || 0,
            totalCost: costSummary.total,
            totalTokens: costSummary.totalTokens,
            providers: health.map(h => ({
                name: h.provider,
                healthy: h.isHealthy,
                circuitBreakerOpen: h.circuitBreakerOpen,
                successRate: (h.successRate * 100).toFixed(1) + '%',
                avgLatency: h.averageLatency.toFixed(0) + 'ms'
            })),
            lastCheckpoint: this.checkpoints[this.checkpoints.length - 1]
        };
    }
    /**
     * Get detailed metrics
     */
    getMetrics() {
        return {
            providers: this.providerManager.getMetrics(),
            health: this.providerManager.getHealth(),
            costs: this.providerManager.getCostSummary(),
            checkpoints: this.checkpoints
        };
    }
    /**
     * Restore from checkpoint
     */
    restoreFromCheckpoint(checkpoint) {
        this.currentState = { ...checkpoint.state };
        logger.info('Restored from checkpoint', {
            agentName: this.config.agentName,
            checkpoint: checkpoint.timestamp
        });
    }
    /**
     * Stop the agent
     */
    async stop() {
        this.isRunning = false;
        // Clear checkpoint interval
        if (this.checkpointInterval) {
            clearInterval(this.checkpointInterval);
        }
        // Save final checkpoint
        this.saveCheckpoint();
        // Cleanup provider manager
        this.providerManager.destroy();
        logger.info('Long-running agent stopped', {
            agentName: this.config.agentName,
            runtime: Date.now() - this.startTime.getTime(),
            completedTasks: this.currentState.completedTasks,
            failedTasks: this.currentState.failedTasks
        });
    }
}
//# sourceMappingURL=long-running-agent.js.map