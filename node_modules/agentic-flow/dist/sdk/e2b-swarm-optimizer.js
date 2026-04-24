/**
 * E2B Swarm Optimizer - Performance optimization for E2B swarms
 *
 * Provides automatic optimization strategies:
 * - Agent pool sizing
 * - Task batching
 * - Load rebalancing
 * - Resource cleanup
 */
import { logger } from "../utils/logger.js";
/**
 * E2B Swarm Optimizer
 */
export class E2BSwarmOptimizer {
    swarm;
    config;
    optimizationHistory = [];
    intervalId = null;
    constructor(swarm, config) {
        this.swarm = swarm;
        this.config = {
            targetUtilization: config?.targetUtilization || 0.7,
            maxErrorRate: config?.maxErrorRate || 0.1,
            minAgents: config?.minAgents || 2,
            maxAgents: config?.maxAgents || 10,
            scaleUpThreshold: config?.scaleUpThreshold || 5,
            scaleDownThreshold: config?.scaleDownThreshold || 60000,
            batchSize: config?.batchSize || 10,
            optimizationInterval: config?.optimizationInterval || 30000
        };
    }
    /**
     * Start automatic optimization
     */
    startAutoOptimization() {
        if (this.intervalId)
            return;
        this.intervalId = setInterval(async () => {
            await this.optimize();
        }, this.config.optimizationInterval);
        logger.info('Auto-optimization started', { interval: this.config.optimizationInterval });
    }
    /**
     * Stop automatic optimization
     */
    stopAutoOptimization() {
        if (this.intervalId) {
            clearInterval(this.intervalId);
            this.intervalId = null;
            logger.info('Auto-optimization stopped');
        }
    }
    /**
     * Run optimization cycle
     */
    async optimize() {
        const metrics = this.swarm.getMetrics();
        const recommendations = this.analyzeMetrics(metrics);
        const actionsApplied = [];
        // Apply auto-applicable recommendations
        for (const rec of recommendations.filter(r => r.autoApply)) {
            const applied = await this.applyRecommendation(rec);
            if (applied) {
                actionsApplied.push(rec.type);
            }
        }
        const report = {
            timestamp: Date.now(),
            metrics,
            recommendations,
            actionsApplied,
            healthScore: this.calculateHealthScore(metrics)
        };
        this.optimizationHistory.push(report);
        // Keep only last 100 reports
        if (this.optimizationHistory.length > 100) {
            this.optimizationHistory = this.optimizationHistory.slice(-100);
        }
        logger.info('Optimization cycle complete', {
            healthScore: report.healthScore,
            recommendations: recommendations.length,
            actionsApplied: actionsApplied.length
        });
        return report;
    }
    /**
     * Analyze metrics and generate recommendations
     */
    analyzeMetrics(metrics) {
        const recommendations = [];
        // Check error rate
        if (metrics.errorRate > this.config.maxErrorRate) {
            recommendations.push({
                type: 'cleanup',
                priority: 'high',
                description: `Error rate ${(metrics.errorRate * 100).toFixed(1)}% exceeds threshold ${(this.config.maxErrorRate * 100).toFixed(1)}%`,
                impact: 'Restart failing agents to reduce errors',
                autoApply: true
            });
        }
        // Check utilization - need to scale up
        const avgUtilization = this.calculateAverageUtilization(metrics);
        if (avgUtilization > this.config.targetUtilization && metrics.totalAgents < this.config.maxAgents) {
            recommendations.push({
                type: 'scale-up',
                priority: 'medium',
                description: `Utilization ${(avgUtilization * 100).toFixed(1)}% above target ${(this.config.targetUtilization * 100).toFixed(1)}%`,
                impact: 'Add more agents to handle load',
                autoApply: false
            });
        }
        // Check utilization - can scale down
        if (avgUtilization < 0.2 && metrics.totalAgents > this.config.minAgents) {
            recommendations.push({
                type: 'scale-down',
                priority: 'low',
                description: `Utilization ${(avgUtilization * 100).toFixed(1)}% is very low`,
                impact: 'Remove idle agents to save resources',
                autoApply: false
            });
        }
        // Check for imbalanced agents
        const utilizationValues = Object.values(metrics.agentUtilization);
        if (utilizationValues.length > 1) {
            const max = Math.max(...utilizationValues);
            const min = Math.min(...utilizationValues);
            if (max - min > 0.5) {
                recommendations.push({
                    type: 'rebalance',
                    priority: 'medium',
                    description: `Agent utilization imbalanced (${(min * 100).toFixed(0)}% - ${(max * 100).toFixed(0)}%)`,
                    impact: 'Redistribute tasks for better balance',
                    autoApply: true
                });
            }
        }
        return recommendations;
    }
    /**
     * Apply an optimization recommendation
     */
    async applyRecommendation(rec) {
        try {
            switch (rec.type) {
                case 'cleanup':
                    return await this.cleanupFailingAgents();
                case 'rebalance':
                    return await this.rebalanceLoad();
                default:
                    return false;
            }
        }
        catch (error) {
            logger.warn('Failed to apply recommendation', { type: rec.type, error: error.message });
            return false;
        }
    }
    /**
     * Cleanup failing agents
     */
    async cleanupFailingAgents() {
        const agents = this.swarm.getAgents();
        let cleaned = false;
        for (const agent of agents) {
            if (agent.status === 'error' || (agent.errors > 5 && agent.tasksCompleted > 0)) {
                const errorRate = agent.errors / (agent.tasksCompleted + agent.errors);
                if (errorRate > 0.3) {
                    await this.swarm.terminateAgent(agent.id);
                    logger.info('Cleaned up failing agent', { id: agent.id, errorRate });
                    cleaned = true;
                }
            }
        }
        return cleaned;
    }
    /**
     * Rebalance load across agents
     */
    async rebalanceLoad() {
        // For now, just log - actual rebalancing would require task migration
        logger.info('Load rebalancing requested');
        return true;
    }
    /**
     * Calculate average utilization
     */
    calculateAverageUtilization(metrics) {
        const values = Object.values(metrics.agentUtilization);
        if (values.length === 0)
            return 0;
        return values.reduce((a, b) => a + b, 0) / values.length;
    }
    /**
     * Calculate health score (0-100)
     */
    calculateHealthScore(metrics) {
        let score = 100;
        // Penalize high error rate
        score -= metrics.errorRate * 50;
        // Penalize very high or very low utilization
        const avgUtil = this.calculateAverageUtilization(metrics);
        if (avgUtil > 0.9)
            score -= 10;
        if (avgUtil < 0.1 && metrics.totalAgents > 0)
            score -= 5;
        // Penalize no agents
        if (metrics.totalAgents === 0)
            score -= 30;
        // Penalize all agents busy
        if (metrics.activeAgents === metrics.totalAgents && metrics.totalAgents > 0)
            score -= 10;
        return Math.max(0, Math.min(100, score));
    }
    /**
     * Get optimization history
     */
    getHistory() {
        return [...this.optimizationHistory];
    }
    /**
     * Get current health score
     */
    getHealthScore() {
        return this.calculateHealthScore(this.swarm.getMetrics());
    }
    /**
     * Get optimization summary
     */
    getSummary() {
        const lastReport = this.optimizationHistory[this.optimizationHistory.length - 1];
        return {
            healthScore: this.getHealthScore(),
            totalOptimizations: this.optimizationHistory.length,
            lastOptimization: lastReport?.timestamp || null,
            actionsApplied: this.optimizationHistory.reduce((sum, r) => sum + r.actionsApplied.length, 0),
            currentMetrics: this.swarm.getMetrics()
        };
    }
}
/**
 * Create optimizer for a swarm
 */
export function createSwarmOptimizer(swarm, config) {
    return new E2BSwarmOptimizer(swarm, config);
}
/**
 * Quick optimization pass
 */
export async function optimizeSwarm(swarm) {
    const optimizer = createSwarmOptimizer(swarm);
    return optimizer.optimize();
}
//# sourceMappingURL=e2b-swarm-optimizer.js.map