/**
 * Swarm Learning Optimizer
 *
 * Enhances parallel execution with adaptive learning, pattern recognition,
 * and automated topology selection using ReasoningBank intelligence.
 */
export class SwarmLearningOptimizer {
    reasoningBank;
    NAMESPACE = 'swarm/optimization';
    constructor(reasoningBank) {
        this.reasoningBank = reasoningBank;
    }
    /**
     * Store swarm execution metrics for learning
     */
    async storeExecutionPattern(taskDescription, metrics, success) {
        const sessionId = `${this.NAMESPACE}/${metrics.topology}/${Date.now()}`;
        await this.reasoningBank.storePattern({
            sessionId,
            task: taskDescription,
            input: JSON.stringify({ taskComplexity: metrics.taskComplexity, agentCount: metrics.agentCount }),
            output: JSON.stringify(metrics),
            reward: this.calculateReward(metrics, success),
            success,
            latencyMs: metrics.totalTimeMs,
            tokensUsed: 0, // Not tracking tokens for swarm ops
            critique: this.generateCritique(metrics, success)
        });
    }
    /**
     * Calculate reward score for swarm execution (0-1)
     */
    calculateReward(metrics, success) {
        if (!success)
            return 0.0;
        let reward = 0.5; // Base reward for success
        // Reward for high success rate (+0.2)
        if (metrics.successRate >= 90) {
            reward += 0.2;
        }
        else if (metrics.successRate >= 75) {
            reward += 0.1;
        }
        // Reward for speedup (+0.2)
        if (metrics.speedup) {
            if (metrics.speedup >= 3.0) {
                reward += 0.2;
            }
            else if (metrics.speedup >= 2.0) {
                reward += 0.15;
            }
            else if (metrics.speedup >= 1.5) {
                reward += 0.1;
            }
        }
        // Reward for efficiency (operations/time) (+0.1)
        const opsPerSecond = (metrics.operations / metrics.totalTimeMs) * 1000;
        if (opsPerSecond > 0.1) {
            reward += 0.1;
        }
        return Math.min(1.0, reward);
    }
    /**
     * Generate critique for learning
     */
    generateCritique(metrics, success) {
        const critiques = [];
        if (!success) {
            critiques.push('Swarm execution failed - investigate error handling');
        }
        if (metrics.successRate < 80) {
            critiques.push(`Low success rate (${metrics.successRate}%) - review agent reliability`);
        }
        if (metrics.speedup && metrics.speedup < 1.2) {
            critiques.push(`Minimal speedup (${metrics.speedup}x) - consider different topology or larger batches`);
        }
        if (metrics.batchSize < 3) {
            critiques.push('Small batch size - may not fully utilize parallel capabilities');
        }
        if (metrics.topology === 'mesh' && metrics.agentCount > 10) {
            critiques.push('Mesh topology with many agents (O(n²) coordination) - consider hierarchical');
        }
        if (critiques.length === 0) {
            if (metrics.speedup && metrics.speedup >= 3.0) {
                return 'Excellent parallel execution - pattern worth reusing';
            }
            return 'Good swarm execution - successful pattern';
        }
        return critiques.join('. ');
    }
    /**
     * Get optimization recommendations based on learned patterns
     */
    async getOptimization(taskDescription, taskComplexity, estimatedAgentCount) {
        // Search for similar successful patterns
        const similarPatterns = await this.reasoningBank.searchPatterns(taskDescription, {
            k: 10,
            minReward: 0.7,
            onlySuccesses: true
        });
        if (similarPatterns.length === 0) {
            // No learned patterns - return default recommendations
            return this.getDefaultRecommendation(taskComplexity, estimatedAgentCount);
        }
        // Analyze patterns to find optimal configuration
        const topologyScores = new Map();
        for (const pattern of similarPatterns) {
            try {
                const metrics = JSON.parse(pattern.output);
                const topology = metrics.topology;
                if (!topologyScores.has(topology)) {
                    topologyScores.set(topology, { totalReward: 0, count: 0, avgSpeedup: 0 });
                }
                const score = topologyScores.get(topology);
                score.totalReward += pattern.reward;
                score.count += 1;
                score.avgSpeedup += metrics.speedup || 1.0;
            }
            catch (e) {
                // Skip invalid pattern data
                continue;
            }
        }
        // Find best topology based on average reward and speedup
        let bestTopology = 'hierarchical';
        let bestScore = 0;
        let bestSpeedup = 1.0;
        const alternatives = [];
        for (const [topology, data] of topologyScores.entries()) {
            const avgReward = data.totalReward / data.count;
            const avgSpeedup = data.avgSpeedup / data.count;
            const score = avgReward * 0.6 + (avgSpeedup / 5.0) * 0.4; // Weighted score
            if (score > bestScore) {
                if (bestScore > 0) {
                    // Previous best becomes alternative
                    alternatives.push({
                        topology: bestTopology,
                        confidence: Math.round(bestScore * 100) / 100,
                        reasoning: `Average speedup: ${bestSpeedup.toFixed(2)}x from ${topologyScores.get(bestTopology).count} executions`
                    });
                }
                bestScore = score;
                bestTopology = topology;
                bestSpeedup = avgSpeedup;
            }
            else {
                alternatives.push({
                    topology,
                    confidence: Math.round(score * 100) / 100,
                    reasoning: `Average speedup: ${avgSpeedup.toFixed(2)}x from ${data.count} executions`
                });
            }
        }
        // Determine optimal batch size based on task complexity and learned patterns
        const optimalBatchSize = this.determineOptimalBatchSize(taskComplexity, estimatedAgentCount, similarPatterns);
        return {
            recommendedTopology: bestTopology,
            recommendedBatchSize: optimalBatchSize,
            recommendedAgentCount: Math.min(estimatedAgentCount, this.getMaxAgentsForTopology(bestTopology)),
            expectedSpeedup: bestSpeedup,
            confidence: Math.round(bestScore * 100) / 100,
            reasoning: `Based on ${similarPatterns.length} similar successful executions. ` +
                `${bestTopology} topology achieved ${bestSpeedup.toFixed(2)}x average speedup.`,
            alternatives: alternatives.sort((a, b) => b.confidence - a.confidence).slice(0, 2)
        };
    }
    /**
     * Get default recommendations when no learned patterns exist
     */
    getDefaultRecommendation(taskComplexity, estimatedAgentCount) {
        // Default topology selection based on agent count and complexity
        let topology;
        let expectedSpeedup;
        let reasoning;
        if (estimatedAgentCount <= 5) {
            topology = 'mesh';
            expectedSpeedup = 2.5;
            reasoning = 'Mesh topology optimal for small swarms (≤5 agents) - full peer-to-peer coordination';
        }
        else if (estimatedAgentCount <= 10) {
            topology = 'hierarchical';
            expectedSpeedup = 3.5;
            reasoning = 'Hierarchical topology optimal for medium swarms (6-10 agents) - efficient delegation';
        }
        else {
            topology = 'hierarchical';
            expectedSpeedup = 4.0;
            reasoning = 'Hierarchical topology required for large swarms (>10 agents) - avoids O(n²) coordination overhead';
        }
        // Adjust for task complexity
        if (taskComplexity === 'critical' || taskComplexity === 'high') {
            expectedSpeedup *= 1.2; // Higher complexity benefits more from parallelization
        }
        const batchSize = this.determineOptimalBatchSize(taskComplexity, estimatedAgentCount, []);
        return {
            recommendedTopology: topology,
            recommendedBatchSize: batchSize,
            recommendedAgentCount: Math.min(estimatedAgentCount, this.getMaxAgentsForTopology(topology)),
            expectedSpeedup,
            confidence: 0.6, // Lower confidence without learned patterns
            reasoning: `${reasoning} (default recommendation - no learned patterns yet)`,
            alternatives: [
                {
                    topology: topology === 'mesh' ? 'hierarchical' : 'mesh',
                    confidence: 0.5,
                    reasoning: 'Alternative topology if default does not perform well'
                }
            ]
        };
    }
    /**
     * Determine optimal batch size based on complexity and learned patterns
     */
    determineOptimalBatchSize(taskComplexity, estimatedAgentCount, learnedPatterns) {
        // Analyze learned patterns for optimal batch size
        if (learnedPatterns.length > 0) {
            const batchSizes = learnedPatterns
                .map(p => {
                try {
                    const metrics = JSON.parse(p.output);
                    return { batchSize: metrics.batchSize, reward: p.reward };
                }
                catch (e) {
                    return null;
                }
            })
                .filter(x => x !== null);
            if (batchSizes.length > 0) {
                // Find batch size with highest average reward
                const batchMap = new Map();
                for (const { batchSize, reward } of batchSizes) {
                    if (!batchMap.has(batchSize)) {
                        batchMap.set(batchSize, { totalReward: 0, count: 0 });
                    }
                    const entry = batchMap.get(batchSize);
                    entry.totalReward += reward;
                    entry.count += 1;
                }
                let bestBatchSize = 3;
                let bestAvgReward = 0;
                for (const [batchSize, { totalReward, count }] of batchMap.entries()) {
                    const avgReward = totalReward / count;
                    if (avgReward > bestAvgReward) {
                        bestAvgReward = avgReward;
                        bestBatchSize = batchSize;
                    }
                }
                return bestBatchSize;
            }
        }
        // Default batch size based on complexity
        switch (taskComplexity) {
            case 'low':
                return Math.min(3, estimatedAgentCount);
            case 'medium':
                return Math.min(5, estimatedAgentCount);
            case 'high':
                return Math.min(7, estimatedAgentCount);
            case 'critical':
                return Math.min(10, estimatedAgentCount);
        }
    }
    /**
     * Get maximum agents for topology (to avoid coordination overhead)
     */
    getMaxAgentsForTopology(topology) {
        switch (topology) {
            case 'mesh':
                return 10; // O(n²) coordination overhead beyond this
            case 'hierarchical':
                return 50; // Scales well with delegation
            case 'ring':
                return 20; // Sequential token passing limits scale
            case 'star':
                return 30; // Central coordinator bottleneck
            default:
                return 10;
        }
    }
    /**
     * Generate statistics report on learned patterns
     */
    async getOptimizationStats() {
        const allPatterns = await this.reasoningBank.searchPatterns(this.NAMESPACE, {
            k: 1000,
            onlySuccesses: true
        });
        const stats = {
            totalPatterns: allPatterns.length,
            topologiesUsed: {},
            speedupByTopology: {},
            successRates: [],
            avgSpeedupByTopology: {},
            avgSuccessRate: 0,
            bestPerformingTopology: 'hierarchical'
        };
        for (const pattern of allPatterns) {
            try {
                const metrics = JSON.parse(pattern.output);
                // Count topology usage
                stats.topologiesUsed[metrics.topology] = (stats.topologiesUsed[metrics.topology] || 0) + 1;
                // Track speedup
                if (!stats.speedupByTopology[metrics.topology]) {
                    stats.speedupByTopology[metrics.topology] = [];
                }
                if (metrics.speedup) {
                    stats.speedupByTopology[metrics.topology].push(metrics.speedup);
                }
                // Track success rate
                stats.successRates.push(metrics.successRate);
            }
            catch (e) {
                continue;
            }
        }
        // Calculate averages
        for (const [topology, speedups] of Object.entries(stats.speedupByTopology)) {
            if (speedups.length > 0) {
                stats.avgSpeedupByTopology[topology] = speedups.reduce((a, b) => a + b, 0) / speedups.length;
            }
        }
        stats.avgSuccessRate = stats.successRates.length > 0
            ? stats.successRates.reduce((a, b) => a + b, 0) / stats.successRates.length
            : 0;
        // Find best performing topology
        let bestSpeedup = 0;
        for (const [topology, avgSpeedup] of Object.entries(stats.avgSpeedupByTopology)) {
            if (avgSpeedup > bestSpeedup) {
                bestSpeedup = avgSpeedup;
                stats.bestPerformingTopology = topology;
            }
        }
        return {
            totalPatterns: stats.totalPatterns,
            topologiesUsed: stats.topologiesUsed,
            avgSpeedupByTopology: stats.avgSpeedupByTopology,
            avgSuccessRate: Math.round(stats.avgSuccessRate * 10) / 10,
            bestPerformingTopology: stats.bestPerformingTopology
        };
    }
}
/**
 * Auto-select optimal swarm configuration
 */
export async function autoSelectSwarmConfig(reasoningBank, taskDescription, options = {}) {
    const optimizer = new SwarmLearningOptimizer(reasoningBank);
    const complexity = options.taskComplexity || 'medium';
    const agentCount = options.estimatedAgentCount || 5;
    return await optimizer.getOptimization(taskDescription, complexity, agentCount);
}
//# sourceMappingURL=swarm-learning-optimizer.js.map