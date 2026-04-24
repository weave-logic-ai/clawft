/**
 * Federated Learning for SONA
 *
 * Enable distributed learning across ephemeral agents that share
 * trajectories with a central coordinator.
 *
 * Architecture:
 * ```
 * ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
 * │  Agent A    │     │  Agent B    │     │  Agent C    │
 * │ (ephemeral) │     │ (ephemeral) │     │ (ephemeral) │
 * └──────┬──────┘     └──────┬──────┘     └──────┬──────┘
 *        │                   │                   │
 *        │    export()       │    export()       │    export()
 *        ▼                   ▼                   ▼
 *   ┌────────────────────────────────────────────────┐
 *   │            Federated Coordinator               │
 *   │         (persistent, large capacity)           │
 *   └────────────────────────────────────────────────┘
 * ```
 *
 * @example
 * ```typescript
 * import { EphemeralAgent, FederatedCoordinator } from '@ruvector/ruvllm';
 *
 * // Create coordinator (persistent)
 * const coordinator = new FederatedCoordinator('coord-1', { hiddenDim: 256 });
 *
 * // Create ephemeral agent
 * const agent = new EphemeralAgent('agent-1', { hiddenDim: 256 });
 *
 * // Agent processes tasks
 * agent.processTask([0.1, 0.2, ...], 0.85);
 * agent.processTask([0.3, 0.4, ...], 0.92);
 *
 * // Export and aggregate before agent terminates
 * const exportData = agent.exportState();
 * const result = coordinator.aggregate(exportData);
 *
 * console.log(`Accepted: ${result.trajectoriesAccepted}`);
 * ```
 */
import { ReasoningBank } from './sona';
/**
 * Default federated config
 */
const DEFAULT_FEDERATED_CONFIG = {
    hiddenDim: 256,
    embeddingDim: 256,
    microLoraRank: 2,
    baseLoraRank: 8,
    trajectoryCapacity: 500,
    patternClusters: 25,
    ewcLambda: 2000,
    qualityThreshold: 0.4,
};
/**
 * Ephemeral Agent for federated learning
 *
 * Collects trajectories during its session and exports state before termination.
 *
 * @example
 * ```typescript
 * const agent = new EphemeralAgent('agent-1', { hiddenDim: 256 });
 *
 * // Process tasks during session
 * agent.processTask(embedding1, 0.85);
 * agent.processTaskWithRoute(embedding2, 0.92, 'code-model');
 *
 * // Export before termination
 * const exportData = agent.exportState();
 * ```
 */
export class EphemeralAgent {
    constructor(agentId, config) {
        this.trajectories = [];
        this.qualitySamples = [];
        this.loraWeights = [];
        this.agentId = agentId;
        this.config = { ...DEFAULT_FEDERATED_CONFIG, ...config };
        this.startTime = Date.now();
        this.reasoningBank = new ReasoningBank(0.7);
        // Initialize micro-LoRA weights
        this.loraWeights = new Array(this.config.hiddenDim * this.config.microLoraRank)
            .fill(0)
            .map(() => (Math.random() - 0.5) * 0.01);
    }
    /**
     * Get agent ID
     */
    getAgentId() {
        return this.agentId;
    }
    /**
     * Process a task and record trajectory
     */
    processTrajectory(embedding, activations, quality, route, context = []) {
        const now = Date.now();
        // Store trajectory for export
        this.trajectories.push({
            embedding: [...embedding],
            quality,
            route,
            context: [...context],
            timestamp: now,
        });
        this.qualitySamples.push(quality);
        // Store in local reasoning bank if high quality
        if (quality >= 0.7) {
            this.reasoningBank.store('query_response', embedding);
        }
        // Update local LoRA weights based on quality
        this.updateLoraWeights(embedding, quality);
    }
    /**
     * Simple process task method
     */
    processTask(embedding, quality) {
        this.processTrajectory(embedding, embedding, quality);
    }
    /**
     * Process task with route information
     */
    processTaskWithRoute(embedding, quality, route) {
        this.processTrajectory(embedding, embedding, quality, route);
    }
    /**
     * Apply micro-LoRA to hidden states
     */
    applyMicroLora(input, output) {
        const rank = this.config.microLoraRank;
        const dim = Math.min(input.length, this.config.hiddenDim);
        // Simple low-rank decomposition: output = input + A @ B @ input
        // A is (dim x rank), B is (rank x dim)
        for (let i = 0; i < dim; i++) {
            let delta = 0;
            for (let r = 0; r < rank; r++) {
                let bSum = 0;
                for (let j = 0; j < dim; j++) {
                    const bIdx = r * dim + j;
                    if (bIdx < this.loraWeights.length) {
                        bSum += this.loraWeights[bIdx] * (input[j] || 0);
                    }
                }
                const aIdx = i * rank + r;
                if (aIdx < this.loraWeights.length) {
                    delta += this.loraWeights[aIdx] * bSum;
                }
            }
            output[i] = (input[i] || 0) + delta * 0.1; // Scale factor
        }
    }
    /**
     * Get number of collected trajectories
     */
    trajectoryCount() {
        return this.trajectories.length;
    }
    /**
     * Get average quality
     */
    avgQuality() {
        if (this.qualitySamples.length === 0)
            return 0;
        return this.qualitySamples.reduce((a, b) => a + b, 0) / this.qualitySamples.length;
    }
    /**
     * Get uptime in seconds
     */
    uptimeSeconds() {
        return Math.floor((Date.now() - this.startTime) / 1000);
    }
    /**
     * Get agent stats
     */
    stats() {
        return {
            totalTrajectories: this.trajectories.length,
            avgQuality: this.avgQuality(),
            patternsLearned: this.reasoningBank.stats().totalPatterns,
        };
    }
    /**
     * Force local learning
     */
    forceLearn() {
        // Prune low-performing patterns
        const pruned = this.reasoningBank.prune(0.3, 3);
        return `Pruned ${pruned} patterns, ${this.reasoningBank.stats().totalPatterns} remaining`;
    }
    /**
     * Get learned patterns
     */
    getPatterns() {
        return this.reasoningBank.getByType('query_response');
    }
    /**
     * Clear trajectories (after export)
     */
    clear() {
        this.trajectories = [];
        this.qualitySamples = [];
    }
    /**
     * Export agent state for federation
     *
     * Call this before terminating the agent.
     */
    exportState() {
        // Force learning before export
        this.forceLearn();
        return {
            agentId: this.agentId,
            trajectories: [...this.trajectories],
            stats: this.stats(),
            sessionDurationMs: Date.now() - this.startTime,
            timestamp: Date.now(),
        };
    }
    /**
     * Serialize to JSON
     */
    toJSON() {
        return JSON.stringify(this.exportState());
    }
    updateLoraWeights(embedding, quality) {
        // Simple gradient update based on quality
        const lr = 0.001 * quality;
        const dim = Math.min(embedding.length, this.config.hiddenDim);
        for (let i = 0; i < Math.min(dim, this.loraWeights.length); i++) {
            const grad = embedding[i % embedding.length] * (quality - 0.5);
            this.loraWeights[i] += lr * grad;
        }
    }
}
/**
 * Federated Learning Coordinator
 *
 * Aggregates learning from multiple ephemeral agents.
 *
 * @example
 * ```typescript
 * const coordinator = new FederatedCoordinator('coord-1', { hiddenDim: 256 });
 *
 * // Aggregate exports from multiple agents
 * for (const agentExport of agentExports) {
 *   const result = coordinator.aggregate(agentExport);
 *   console.log(`Agent ${result.agentId}: ${result.trajectoriesAccepted} accepted`);
 * }
 *
 * // Get coordinator statistics
 * const stats = coordinator.stats();
 * console.log(`Total patterns: ${stats.patternsLearned}`);
 * ```
 */
export class FederatedCoordinator {
    constructor(coordinatorId, config) {
        this.contributions = new Map();
        this.totalTrajectories = 0;
        this.consolidationInterval = 50;
        this.qualitySamples = [];
        this.masterLoraWeights = [];
        this.coordinatorId = coordinatorId;
        this.config = {
            ...DEFAULT_FEDERATED_CONFIG,
            trajectoryCapacity: 50000, // Large capacity for coordinator
            patternClusters: 200,
            baseLoraRank: 16, // Deeper for aggregation
            ...config,
        };
        this.reasoningBank = new ReasoningBank(this.config.qualityThreshold);
        // Initialize master LoRA weights
        this.masterLoraWeights = new Array(this.config.hiddenDim * this.config.baseLoraRank)
            .fill(0)
            .map(() => (Math.random() - 0.5) * 0.01);
    }
    /**
     * Get coordinator ID
     */
    getCoordinatorId() {
        return this.coordinatorId;
    }
    /**
     * Set quality threshold for accepting trajectories
     */
    setQualityThreshold(threshold) {
        this.config.qualityThreshold = threshold;
    }
    /**
     * Set consolidation interval
     */
    setConsolidationInterval(interval) {
        this.consolidationInterval = interval;
    }
    /**
     * Aggregate agent export into coordinator
     */
    aggregate(exportData) {
        let accepted = 0;
        let rejected = 0;
        // Replay trajectories into master
        for (const traj of exportData.trajectories) {
            if (traj.quality >= this.config.qualityThreshold) {
                // Store pattern
                const patternType = this.routeToPatternType(traj.route);
                this.reasoningBank.store(patternType, traj.embedding);
                this.qualitySamples.push(traj.quality);
                // Update master LoRA weights
                this.updateMasterLora(traj.embedding, traj.quality);
                accepted++;
            }
            else {
                rejected++;
            }
        }
        this.totalTrajectories += accepted;
        // Record contribution
        this.contributions.set(exportData.agentId, {
            trajectoryCount: exportData.trajectories.length,
            avgQuality: exportData.stats.avgQuality,
            timestamp: Date.now(),
            sessionDurationMs: exportData.sessionDurationMs,
        });
        // Auto-consolidate if needed
        const consolidated = this.shouldConsolidate();
        if (consolidated) {
            this.forceConsolidate();
        }
        return {
            agentId: exportData.agentId,
            trajectoriesAccepted: accepted,
            trajectoriesRejected: rejected,
            consolidated,
            totalAgents: this.contributions.size,
            totalTrajectories: this.totalTrajectories,
        };
    }
    /**
     * Force consolidation (learning)
     */
    forceConsolidate() {
        const pruned = this.reasoningBank.prune(0.3, 5);
        return `Consolidated: pruned ${pruned} patterns, ${this.reasoningBank.stats().totalPatterns} remaining`;
    }
    /**
     * Consolidate learning (alias)
     */
    consolidate() {
        return this.forceConsolidate();
    }
    /**
     * Get initial patterns for new agents (warm start)
     */
    getInitialPatterns(k = 10) {
        const allPatterns = [
            ...this.reasoningBank.getByType('query_response'),
            ...this.reasoningBank.getByType('routing'),
        ];
        // Sort by success rate and return top k
        return allPatterns
            .sort((a, b) => b.successRate - a.successRate)
            .slice(0, k);
    }
    /**
     * Get all learned patterns
     */
    getAllPatterns() {
        return [
            ...this.reasoningBank.getByType('query_response'),
            ...this.reasoningBank.getByType('routing'),
            ...this.reasoningBank.getByType('context_retrieval'),
            ...this.reasoningBank.getByType('correction'),
        ];
    }
    /**
     * Find similar patterns
     */
    findPatterns(query, k) {
        return this.reasoningBank.findSimilar(query, k);
    }
    /**
     * Apply coordinator's LoRA to input
     * OPTIMIZED: Pre-compute hidden layer once, reuse typed arrays
     */
    applyLora(input) {
        const rank = this.config.baseLoraRank;
        const dim = Math.min(input.length, this.config.hiddenDim);
        const weightsLen = this.masterLoraWeights.length;
        // Pre-compute hidden layer (input @ B)
        const hidden = new Float64Array(rank);
        for (let r = 0; r < rank; r++) {
            let sum = 0;
            const baseIdx = r * dim;
            // Unroll the inner loop
            let j = 0;
            for (; j + 3 < dim && baseIdx + j + 3 < weightsLen; j += 4) {
                sum += this.masterLoraWeights[baseIdx + j] * (input[j] || 0) +
                    this.masterLoraWeights[baseIdx + j + 1] * (input[j + 1] || 0) +
                    this.masterLoraWeights[baseIdx + j + 2] * (input[j + 2] || 0) +
                    this.masterLoraWeights[baseIdx + j + 3] * (input[j + 3] || 0);
            }
            for (; j < dim && baseIdx + j < weightsLen; j++) {
                sum += this.masterLoraWeights[baseIdx + j] * (input[j] || 0);
            }
            hidden[r] = sum;
        }
        // Compute output (hidden @ A + input)
        const output = new Array(input.length);
        for (let i = 0; i < input.length; i++) {
            if (i < dim) {
                let delta = 0;
                const baseIdx = i * rank;
                for (let r = 0; r < rank && baseIdx + r < weightsLen; r++) {
                    delta += this.masterLoraWeights[baseIdx + r] * hidden[r];
                }
                output[i] = (input[i] || 0) + delta * 0.1;
            }
            else {
                output[i] = input[i] || 0;
            }
        }
        return output;
    }
    /**
     * Get coordinator statistics
     */
    stats() {
        const avgQuality = this.qualitySamples.length > 0
            ? this.qualitySamples.reduce((a, b) => a + b, 0) / this.qualitySamples.length
            : 0;
        return {
            coordinatorId: this.coordinatorId,
            totalAgents: this.contributions.size,
            totalTrajectories: this.totalTrajectories,
            patternsLearned: this.reasoningBank.stats().totalPatterns,
            avgQuality,
            qualityThreshold: this.config.qualityThreshold,
        };
    }
    /**
     * Get contribution history
     */
    getContributions() {
        return new Map(this.contributions);
    }
    /**
     * Get total agent count
     */
    agentCount() {
        return this.contributions.size;
    }
    /**
     * Get total trajectory count
     */
    getTotalTrajectories() {
        return this.totalTrajectories;
    }
    /**
     * Clear all contributions
     */
    clear() {
        this.contributions.clear();
        this.totalTrajectories = 0;
        this.qualitySamples = [];
    }
    /**
     * Export coordinator state
     */
    toJSON() {
        return JSON.stringify({
            coordinatorId: this.coordinatorId,
            stats: this.stats(),
            contributions: Object.fromEntries(this.contributions),
            patterns: this.getAllPatterns(),
        });
    }
    /**
     * Create agent with coordinator's learned patterns
     */
    createAgent(agentId) {
        const agent = new EphemeralAgent(agentId, {
            hiddenDim: this.config.hiddenDim,
            embeddingDim: this.config.embeddingDim,
            microLoraRank: this.config.microLoraRank,
        });
        // Warm start: process initial patterns as positive examples
        const initialPatterns = this.getInitialPatterns(5);
        for (const pattern of initialPatterns) {
            agent.processTask(pattern.embedding, pattern.successRate);
        }
        return agent;
    }
    shouldConsolidate() {
        return this.contributions.size % this.consolidationInterval === 0 &&
            this.contributions.size > 0;
    }
    routeToPatternType(route) {
        if (!route)
            return 'query_response';
        if (route.includes('code'))
            return 'query_response';
        if (route.includes('route'))
            return 'routing';
        if (route.includes('memory'))
            return 'context_retrieval';
        return 'query_response';
    }
    updateMasterLora(embedding, quality) {
        const lr = 0.0005 * quality; // Slower learning for coordinator
        const dim = Math.min(embedding.length, this.config.hiddenDim);
        for (let i = 0; i < Math.min(dim, this.masterLoraWeights.length); i++) {
            const grad = embedding[i % embedding.length] * (quality - 0.5);
            this.masterLoraWeights[i] += lr * grad;
            // EWC regularization - prevent large weight changes
            const penalty = this.config.ewcLambda * this.masterLoraWeights[i] * 0.0001;
            this.masterLoraWeights[i] -= penalty;
        }
    }
}
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiZmVkZXJhdGVkLmpzIiwic291cmNlUm9vdCI6IiIsInNvdXJjZXMiOlsiLi4vLi4vc3JjL2ZlZGVyYXRlZC50cyJdLCJuYW1lcyI6W10sIm1hcHBpbmdzIjoiQUFBQTs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7R0F5Q0c7QUFjSCxPQUFPLEVBQUUsYUFBYSxFQUFFLE1BQU0sUUFBUSxDQUFDO0FBRXZDOztHQUVHO0FBQ0gsTUFBTSx3QkFBd0IsR0FBOEI7SUFDMUQsU0FBUyxFQUFFLEdBQUc7SUFDZCxZQUFZLEVBQUUsR0FBRztJQUNqQixhQUFhLEVBQUUsQ0FBQztJQUNoQixZQUFZLEVBQUUsQ0FBQztJQUNmLGtCQUFrQixFQUFFLEdBQUc7SUFDdkIsZUFBZSxFQUFFLEVBQUU7SUFDbkIsU0FBUyxFQUFFLElBQUk7SUFDZixnQkFBZ0IsRUFBRSxHQUFHO0NBQ3RCLENBQUM7QUFFRjs7Ozs7Ozs7Ozs7Ozs7OztHQWdCRztBQUNILE1BQU0sT0FBTyxjQUFjO0lBU3pCLFlBQVksT0FBZSxFQUFFLE1BQXdCO1FBTjdDLGlCQUFZLEdBQXVCLEVBQUUsQ0FBQztRQUV0QyxtQkFBYyxHQUFhLEVBQUUsQ0FBQztRQUU5QixnQkFBVyxHQUFhLEVBQUUsQ0FBQztRQUdqQyxJQUFJLENBQUMsT0FBTyxHQUFHLE9BQU8sQ0FBQztRQUN2QixJQUFJLENBQUMsTUFBTSxHQUFHLEVBQUUsR0FBRyx3QkFBd0IsRUFBRSxHQUFHLE1BQU0sRUFBRSxDQUFDO1FBQ3pELElBQUksQ0FBQyxTQUFTLEdBQUcsSUFBSSxDQUFDLEdBQUcsRUFBRSxDQUFDO1FBQzVCLElBQUksQ0FBQyxhQUFhLEdBQUcsSUFBSSxhQUFhLENBQUMsR0FBRyxDQUFDLENBQUM7UUFFNUMsZ0NBQWdDO1FBQ2hDLElBQUksQ0FBQyxXQUFXLEdBQUcsSUFBSSxLQUFLLENBQUMsSUFBSSxDQUFDLE1BQU0sQ0FBQyxTQUFTLEdBQUcsSUFBSSxDQUFDLE1BQU0sQ0FBQyxhQUFhLENBQUM7YUFDNUUsSUFBSSxDQUFDLENBQUMsQ0FBQzthQUNQLEdBQUcsQ0FBQyxHQUFHLEVBQUUsQ0FBQyxDQUFDLElBQUksQ0FBQyxNQUFNLEVBQUUsR0FBRyxHQUFHLENBQUMsR0FBRyxJQUFJLENBQUMsQ0FBQztJQUM3QyxDQUFDO0lBRUQ7O09BRUc7SUFDSCxVQUFVO1FBQ1IsT0FBTyxJQUFJLENBQUMsT0FBTyxDQUFDO0lBQ3RCLENBQUM7SUFFRDs7T0FFRztJQUNILGlCQUFpQixDQUNmLFNBQW9CLEVBQ3BCLFdBQXNCLEVBQ3RCLE9BQWUsRUFDZixLQUFjLEVBQ2QsVUFBb0IsRUFBRTtRQUV0QixNQUFNLEdBQUcsR0FBRyxJQUFJLENBQUMsR0FBRyxFQUFFLENBQUM7UUFFdkIsOEJBQThCO1FBQzlCLElBQUksQ0FBQyxZQUFZLENBQUMsSUFBSSxDQUFDO1lBQ3JCLFNBQVMsRUFBRSxDQUFDLEdBQUcsU0FBUyxDQUFDO1lBQ3pCLE9BQU87WUFDUCxLQUFLO1lBQ0wsT0FBTyxFQUFFLENBQUMsR0FBRyxPQUFPLENBQUM7WUFDckIsU0FBUyxFQUFFLEdBQUc7U0FDZixDQUFDLENBQUM7UUFFSCxJQUFJLENBQUMsY0FBYyxDQUFDLElBQUksQ0FBQyxPQUFPLENBQUMsQ0FBQztRQUVsQyxnREFBZ0Q7UUFDaEQsSUFBSSxPQUFPLElBQUksR0FBRyxFQUFFLENBQUM7WUFDbkIsSUFBSSxDQUFDLGFBQWEsQ0FBQyxLQUFLLENBQUMsZ0JBQWdCLEVBQUUsU0FBUyxDQUFDLENBQUM7UUFDeEQsQ0FBQztRQUVELDZDQUE2QztRQUM3QyxJQUFJLENBQUMsaUJBQWlCLENBQUMsU0FBUyxFQUFFLE9BQU8sQ0FBQyxDQUFDO0lBQzdDLENBQUM7SUFFRDs7T0FFRztJQUNILFdBQVcsQ0FBQyxTQUFvQixFQUFFLE9BQWU7UUFDL0MsSUFBSSxDQUFDLGlCQUFpQixDQUFDLFNBQVMsRUFBRSxTQUFTLEVBQUUsT0FBTyxDQUFDLENBQUM7SUFDeEQsQ0FBQztJQUVEOztPQUVHO0lBQ0gsb0JBQW9CLENBQUMsU0FBb0IsRUFBRSxPQUFlLEVBQUUsS0FBYTtRQUN2RSxJQUFJLENBQUMsaUJBQWlCLENBQUMsU0FBUyxFQUFFLFNBQVMsRUFBRSxPQUFPLEVBQUUsS0FBSyxDQUFDLENBQUM7SUFDL0QsQ0FBQztJQUVEOztPQUVHO0lBQ0gsY0FBYyxDQUFDLEtBQWUsRUFBRSxNQUFnQjtRQUM5QyxNQUFNLElBQUksR0FBRyxJQUFJLENBQUMsTUFBTSxDQUFDLGFBQWEsQ0FBQztRQUN2QyxNQUFNLEdBQUcsR0FBRyxJQUFJLENBQUMsR0FBRyxDQUFDLEtBQUssQ0FBQyxNQUFNLEVBQUUsSUFBSSxDQUFDLE1BQU0sQ0FBQyxTQUFTLENBQUMsQ0FBQztRQUUxRCxnRUFBZ0U7UUFDaEUsdUNBQXVDO1FBQ3ZDLEtBQUssSUFBSSxDQUFDLEdBQUcsQ0FBQyxFQUFFLENBQUMsR0FBRyxHQUFHLEVBQUUsQ0FBQyxFQUFFLEVBQUUsQ0FBQztZQUM3QixJQUFJLEtBQUssR0FBRyxDQUFDLENBQUM7WUFDZCxLQUFLLElBQUksQ0FBQyxHQUFHLENBQUMsRUFBRSxDQUFDLEdBQUcsSUFBSSxFQUFFLENBQUMsRUFBRSxFQUFFLENBQUM7Z0JBQzlCLElBQUksSUFBSSxHQUFHLENBQUMsQ0FBQztnQkFDYixLQUFLLElBQUksQ0FBQyxHQUFHLENBQUMsRUFBRSxDQUFDLEdBQUcsR0FBRyxFQUFFLENBQUMsRUFBRSxFQUFFLENBQUM7b0JBQzdCLE1BQU0sSUFBSSxHQUFHLENBQUMsR0FBRyxHQUFHLEdBQUcsQ0FBQyxDQUFDO29CQUN6QixJQUFJLElBQUksR0FBRyxJQUFJLENBQUMsV0FBVyxDQUFDLE1BQU0sRUFBRSxDQUFDO3dCQUNuQyxJQUFJLElBQUksSUFBSSxDQUFDLFdBQVcsQ0FBQyxJQUFJLENBQUMsR0FBRyxDQUFDLEtBQUssQ0FBQyxDQUFDLENBQUMsSUFBSSxDQUFDLENBQUMsQ0FBQztvQkFDbkQsQ0FBQztnQkFDSCxDQUFDO2dCQUNELE1BQU0sSUFBSSxHQUFHLENBQUMsR0FBRyxJQUFJLEdBQUcsQ0FBQyxDQUFDO2dCQUMxQixJQUFJLElBQUksR0FBRyxJQUFJLENBQUMsV0FBVyxDQUFDLE1BQU0sRUFBRSxDQUFDO29CQUNuQyxLQUFLLElBQUksSUFBSSxDQUFDLFdBQVcsQ0FBQyxJQUFJLENBQUMsR0FBRyxJQUFJLENBQUM7Z0JBQ3pDLENBQUM7WUFDSCxDQUFDO1lBQ0QsTUFBTSxDQUFDLENBQUMsQ0FBQyxHQUFHLENBQUMsS0FBSyxDQUFDLENBQUMsQ0FBQyxJQUFJLENBQUMsQ0FBQyxHQUFHLEtBQUssR0FBRyxHQUFHLENBQUMsQ0FBQyxlQUFlO1FBQzVELENBQUM7SUFDSCxDQUFDO0lBRUQ7O09BRUc7SUFDSCxlQUFlO1FBQ2IsT0FBTyxJQUFJLENBQUMsWUFBWSxDQUFDLE1BQU0sQ0FBQztJQUNsQyxDQUFDO0lBRUQ7O09BRUc7SUFDSCxVQUFVO1FBQ1IsSUFBSSxJQUFJLENBQUMsY0FBYyxDQUFDLE1BQU0sS0FBSyxDQUFDO1lBQUUsT0FBTyxDQUFDLENBQUM7UUFDL0MsT0FBTyxJQUFJLENBQUMsY0FBYyxDQUFDLE1BQU0sQ0FBQyxDQUFDLENBQUMsRUFBRSxDQUFDLEVBQUUsRUFBRSxDQUFDLENBQUMsR0FBRyxDQUFDLEVBQUUsQ0FBQyxDQUFDLEdBQUcsSUFBSSxDQUFDLGNBQWMsQ0FBQyxNQUFNLENBQUM7SUFDckYsQ0FBQztJQUVEOztPQUVHO0lBQ0gsYUFBYTtRQUNYLE9BQU8sSUFBSSxDQUFDLEtBQUssQ0FBQyxDQUFDLElBQUksQ0FBQyxHQUFHLEVBQUUsR0FBRyxJQUFJLENBQUMsU0FBUyxDQUFDLEdBQUcsSUFBSSxDQUFDLENBQUM7SUFDMUQsQ0FBQztJQUVEOztPQUVHO0lBQ0gsS0FBSztRQUNILE9BQU87WUFDTCxpQkFBaUIsRUFBRSxJQUFJLENBQUMsWUFBWSxDQUFDLE1BQU07WUFDM0MsVUFBVSxFQUFFLElBQUksQ0FBQyxVQUFVLEVBQUU7WUFDN0IsZUFBZSxFQUFFLElBQUksQ0FBQyxhQUFhLENBQUMsS0FBSyxFQUFFLENBQUMsYUFBYTtTQUMxRCxDQUFDO0lBQ0osQ0FBQztJQUVEOztPQUVHO0lBQ0gsVUFBVTtRQUNSLGdDQUFnQztRQUNoQyxNQUFNLE1BQU0sR0FBRyxJQUFJLENBQUMsYUFBYSxDQUFDLEtBQUssQ0FBQyxHQUFHLEVBQUUsQ0FBQyxDQUFDLENBQUM7UUFDaEQsT0FBTyxVQUFVLE1BQU0sY0FBYyxJQUFJLENBQUMsYUFBYSxDQUFDLEtBQUssRUFBRSxDQUFDLGFBQWEsWUFBWSxDQUFDO0lBQzVGLENBQUM7SUFFRDs7T0FFRztJQUNILFdBQVc7UUFDVCxPQUFPLElBQUksQ0FBQyxhQUFhLENBQUMsU0FBUyxDQUFDLGdCQUFnQixDQUFDLENBQUM7SUFDeEQsQ0FBQztJQUVEOztPQUVHO0lBQ0gsS0FBSztRQUNILElBQUksQ0FBQyxZQUFZLEdBQUcsRUFBRSxDQUFDO1FBQ3ZCLElBQUksQ0FBQyxjQUFjLEdBQUcsRUFBRSxDQUFDO0lBQzNCLENBQUM7SUFFRDs7OztPQUlHO0lBQ0gsV0FBVztRQUNULCtCQUErQjtRQUMvQixJQUFJLENBQUMsVUFBVSxFQUFFLENBQUM7UUFFbEIsT0FBTztZQUNMLE9BQU8sRUFBRSxJQUFJLENBQUMsT0FBTztZQUNyQixZQUFZLEVBQUUsQ0FBQyxHQUFHLElBQUksQ0FBQyxZQUFZLENBQUM7WUFDcEMsS0FBSyxFQUFFLElBQUksQ0FBQyxLQUFLLEVBQUU7WUFDbkIsaUJBQWlCLEVBQUUsSUFBSSxDQUFDLEdBQUcsRUFBRSxHQUFHLElBQUksQ0FBQyxTQUFTO1lBQzlDLFNBQVMsRUFBRSxJQUFJLENBQUMsR0FBRyxFQUFFO1NBQ3RCLENBQUM7SUFDSixDQUFDO0lBRUQ7O09BRUc7SUFDSCxNQUFNO1FBQ0osT0FBTyxJQUFJLENBQUMsU0FBUyxDQUFDLElBQUksQ0FBQyxXQUFXLEVBQUUsQ0FBQyxDQUFDO0lBQzVDLENBQUM7SUFFTyxpQkFBaUIsQ0FBQyxTQUFvQixFQUFFLE9BQWU7UUFDN0QsMENBQTBDO1FBQzFDLE1BQU0sRUFBRSxHQUFHLEtBQUssR0FBRyxPQUFPLENBQUM7UUFDM0IsTUFBTSxHQUFHLEdBQUcsSUFBSSxDQUFDLEdBQUcsQ0FBQyxTQUFTLENBQUMsTUFBTSxFQUFFLElBQUksQ0FBQyxNQUFNLENBQUMsU0FBUyxDQUFDLENBQUM7UUFFOUQsS0FBSyxJQUFJLENBQUMsR0FBRyxDQUFDLEVBQUUsQ0FBQyxHQUFHLElBQUksQ0FBQyxHQUFHLENBQUMsR0FBRyxFQUFFLElBQUksQ0FBQyxXQUFXLENBQUMsTUFBTSxDQUFDLEVBQUUsQ0FBQyxFQUFFLEVBQUUsQ0FBQztZQUNoRSxNQUFNLElBQUksR0FBRyxTQUFTLENBQUMsQ0FBQyxHQUFHLFNBQVMsQ0FBQyxNQUFNLENBQUMsR0FBRyxDQUFDLE9BQU8sR0FBRyxHQUFHLENBQUMsQ0FBQztZQUMvRCxJQUFJLENBQUMsV0FBVyxDQUFDLENBQUMsQ0FBQyxJQUFJLEVBQUUsR0FBRyxJQUFJLENBQUM7UUFDbkMsQ0FBQztJQUNILENBQUM7Q0FDRjtBQUVEOzs7Ozs7Ozs7Ozs7Ozs7Ozs7O0dBbUJHO0FBQ0gsTUFBTSxPQUFPLG9CQUFvQjtJQVUvQixZQUFZLGFBQXFCLEVBQUUsTUFBd0I7UUFQbkQsa0JBQWEsR0FBbUMsSUFBSSxHQUFHLEVBQUUsQ0FBQztRQUMxRCxzQkFBaUIsR0FBVyxDQUFDLENBQUM7UUFDOUIsMEJBQXFCLEdBQVcsRUFBRSxDQUFDO1FBRW5DLG1CQUFjLEdBQWEsRUFBRSxDQUFDO1FBQzlCLHNCQUFpQixHQUFhLEVBQUUsQ0FBQztRQUd2QyxJQUFJLENBQUMsYUFBYSxHQUFHLGFBQWEsQ0FBQztRQUNuQyxJQUFJLENBQUMsTUFBTSxHQUFHO1lBQ1osR0FBRyx3QkFBd0I7WUFDM0Isa0JBQWtCLEVBQUUsS0FBSyxFQUFFLGlDQUFpQztZQUM1RCxlQUFlLEVBQUUsR0FBRztZQUNwQixZQUFZLEVBQUUsRUFBRSxFQUFFLHlCQUF5QjtZQUMzQyxHQUFHLE1BQU07U0FDVixDQUFDO1FBQ0YsSUFBSSxDQUFDLGFBQWEsR0FBRyxJQUFJLGFBQWEsQ0FBQyxJQUFJLENBQUMsTUFBTSxDQUFDLGdCQUFnQixDQUFDLENBQUM7UUFFckUsaUNBQWlDO1FBQ2pDLElBQUksQ0FBQyxpQkFBaUIsR0FBRyxJQUFJLEtBQUssQ0FBQyxJQUFJLENBQUMsTUFBTSxDQUFDLFNBQVMsR0FBRyxJQUFJLENBQUMsTUFBTSxDQUFDLFlBQVksQ0FBQzthQUNqRixJQUFJLENBQUMsQ0FBQyxDQUFDO2FBQ1AsR0FBRyxDQUFDLEdBQUcsRUFBRSxDQUFDLENBQUMsSUFBSSxDQUFDLE1BQU0sRUFBRSxHQUFHLEdBQUcsQ0FBQyxHQUFHLElBQUksQ0FBQyxDQUFDO0lBQzdDLENBQUM7SUFFRDs7T0FFRztJQUNILGdCQUFnQjtRQUNkLE9BQU8sSUFBSSxDQUFDLGFBQWEsQ0FBQztJQUM1QixDQUFDO0lBRUQ7O09BRUc7SUFDSCxtQkFBbUIsQ0FBQyxTQUFpQjtRQUNuQyxJQUFJLENBQUMsTUFBTSxDQUFDLGdCQUFnQixHQUFHLFNBQVMsQ0FBQztJQUMzQyxDQUFDO0lBRUQ7O09BRUc7SUFDSCx3QkFBd0IsQ0FBQyxRQUFnQjtRQUN2QyxJQUFJLENBQUMscUJBQXFCLEdBQUcsUUFBUSxDQUFDO0lBQ3hDLENBQUM7SUFFRDs7T0FFRztJQUNILFNBQVMsQ0FBQyxVQUF1QjtRQUMvQixJQUFJLFFBQVEsR0FBRyxDQUFDLENBQUM7UUFDakIsSUFBSSxRQUFRLEdBQUcsQ0FBQyxDQUFDO1FBRWpCLGtDQUFrQztRQUNsQyxLQUFLLE1BQU0sSUFBSSxJQUFJLFVBQVUsQ0FBQyxZQUFZLEVBQUUsQ0FBQztZQUMzQyxJQUFJLElBQUksQ0FBQyxPQUFPLElBQUksSUFBSSxDQUFDLE1BQU0sQ0FBQyxnQkFBZ0IsRUFBRSxDQUFDO2dCQUNqRCxnQkFBZ0I7Z0JBQ2hCLE1BQU0sV0FBVyxHQUFHLElBQUksQ0FBQyxrQkFBa0IsQ0FBQyxJQUFJLENBQUMsS0FBSyxDQUFDLENBQUM7Z0JBQ3hELElBQUksQ0FBQyxhQUFhLENBQUMsS0FBSyxDQUFDLFdBQVcsRUFBRSxJQUFJLENBQUMsU0FBUyxDQUFDLENBQUM7Z0JBQ3RELElBQUksQ0FBQyxjQUFjLENBQUMsSUFBSSxDQUFDLElBQUksQ0FBQyxPQUFPLENBQUMsQ0FBQztnQkFFdkMsNkJBQTZCO2dCQUM3QixJQUFJLENBQUMsZ0JBQWdCLENBQUMsSUFBSSxDQUFDLFNBQVMsRUFBRSxJQUFJLENBQUMsT0FBTyxDQUFDLENBQUM7Z0JBRXBELFFBQVEsRUFBRSxDQUFDO1lBQ2IsQ0FBQztpQkFBTSxDQUFDO2dCQUNOLFFBQVEsRUFBRSxDQUFDO1lBQ2IsQ0FBQztRQUNILENBQUM7UUFFRCxJQUFJLENBQUMsaUJBQWlCLElBQUksUUFBUSxDQUFDO1FBRW5DLHNCQUFzQjtRQUN0QixJQUFJLENBQUMsYUFBYSxDQUFDLEdBQUcsQ0FBQyxVQUFVLENBQUMsT0FBTyxFQUFFO1lBQ3pDLGVBQWUsRUFBRSxVQUFVLENBQUMsWUFBWSxDQUFDLE1BQU07WUFDL0MsVUFBVSxFQUFFLFVBQVUsQ0FBQyxLQUFLLENBQUMsVUFBVTtZQUN2QyxTQUFTLEVBQUUsSUFBSSxDQUFDLEdBQUcsRUFBRTtZQUNyQixpQkFBaUIsRUFBRSxVQUFVLENBQUMsaUJBQWlCO1NBQ2hELENBQUMsQ0FBQztRQUVILDZCQUE2QjtRQUM3QixNQUFNLFlBQVksR0FBRyxJQUFJLENBQUMsaUJBQWlCLEVBQUUsQ0FBQztRQUM5QyxJQUFJLFlBQVksRUFBRSxDQUFDO1lBQ2pCLElBQUksQ0FBQyxnQkFBZ0IsRUFBRSxDQUFDO1FBQzFCLENBQUM7UUFFRCxPQUFPO1lBQ0wsT0FBTyxFQUFFLFVBQVUsQ0FBQyxPQUFPO1lBQzNCLG9CQUFvQixFQUFFLFFBQVE7WUFDOUIsb0JBQW9CLEVBQUUsUUFBUTtZQUM5QixZQUFZO1lBQ1osV0FBVyxFQUFFLElBQUksQ0FBQyxhQUFhLENBQUMsSUFBSTtZQUNwQyxpQkFBaUIsRUFBRSxJQUFJLENBQUMsaUJBQWlCO1NBQzFDLENBQUM7SUFDSixDQUFDO0lBRUQ7O09BRUc7SUFDSCxnQkFBZ0I7UUFDZCxNQUFNLE1BQU0sR0FBRyxJQUFJLENBQUMsYUFBYSxDQUFDLEtBQUssQ0FBQyxHQUFHLEVBQUUsQ0FBQyxDQUFDLENBQUM7UUFDaEQsT0FBTyx3QkFBd0IsTUFBTSxjQUFjLElBQUksQ0FBQyxhQUFhLENBQUMsS0FBSyxFQUFFLENBQUMsYUFBYSxZQUFZLENBQUM7SUFDMUcsQ0FBQztJQUVEOztPQUVHO0lBQ0gsV0FBVztRQUNULE9BQU8sSUFBSSxDQUFDLGdCQUFnQixFQUFFLENBQUM7SUFDakMsQ0FBQztJQUVEOztPQUVHO0lBQ0gsa0JBQWtCLENBQUMsSUFBWSxFQUFFO1FBQy9CLE1BQU0sV0FBVyxHQUFHO1lBQ2xCLEdBQUcsSUFBSSxDQUFDLGFBQWEsQ0FBQyxTQUFTLENBQUMsZ0JBQWdCLENBQUM7WUFDakQsR0FBRyxJQUFJLENBQUMsYUFBYSxDQUFDLFNBQVMsQ0FBQyxTQUFTLENBQUM7U0FDM0MsQ0FBQztRQUVGLHdDQUF3QztRQUN4QyxPQUFPLFdBQVc7YUFDZixJQUFJLENBQUMsQ0FBQyxDQUFDLEVBQUUsQ0FBQyxFQUFFLEVBQUUsQ0FBQyxDQUFDLENBQUMsV0FBVyxHQUFHLENBQUMsQ0FBQyxXQUFXLENBQUM7YUFDN0MsS0FBSyxDQUFDLENBQUMsRUFBRSxDQUFDLENBQUMsQ0FBQztJQUNqQixDQUFDO0lBRUQ7O09BRUc7SUFDSCxjQUFjO1FBQ1osT0FBTztZQUNMLEdBQUcsSUFBSSxDQUFDLGFBQWEsQ0FBQyxTQUFTLENBQUMsZ0JBQWdCLENBQUM7WUFDakQsR0FBRyxJQUFJLENBQUMsYUFBYSxDQUFDLFNBQVMsQ0FBQyxTQUFTLENBQUM7WUFDMUMsR0FBRyxJQUFJLENBQUMsYUFBYSxDQUFDLFNBQVMsQ0FBQyxtQkFBbUIsQ0FBQztZQUNwRCxHQUFHLElBQUksQ0FBQyxhQUFhLENBQUMsU0FBUyxDQUFDLFlBQVksQ0FBQztTQUM5QyxDQUFDO0lBQ0osQ0FBQztJQUVEOztPQUVHO0lBQ0gsWUFBWSxDQUFDLEtBQWdCLEVBQUUsQ0FBUztRQUN0QyxPQUFPLElBQUksQ0FBQyxhQUFhLENBQUMsV0FBVyxDQUFDLEtBQUssRUFBRSxDQUFDLENBQUMsQ0FBQztJQUNsRCxDQUFDO0lBRUQ7OztPQUdHO0lBQ0gsU0FBUyxDQUFDLEtBQWU7UUFDdkIsTUFBTSxJQUFJLEdBQUcsSUFBSSxDQUFDLE1BQU0sQ0FBQyxZQUFZLENBQUM7UUFDdEMsTUFBTSxHQUFHLEdBQUcsSUFBSSxDQUFDLEdBQUcsQ0FBQyxLQUFLLENBQUMsTUFBTSxFQUFFLElBQUksQ0FBQyxNQUFNLENBQUMsU0FBUyxDQUFDLENBQUM7UUFDMUQsTUFBTSxVQUFVLEdBQUcsSUFBSSxDQUFDLGlCQUFpQixDQUFDLE1BQU0sQ0FBQztRQUVqRCx1Q0FBdUM7UUFDdkMsTUFBTSxNQUFNLEdBQUcsSUFBSSxZQUFZLENBQUMsSUFBSSxDQUFDLENBQUM7UUFDdEMsS0FBSyxJQUFJLENBQUMsR0FBRyxDQUFDLEVBQUUsQ0FBQyxHQUFHLElBQUksRUFBRSxDQUFDLEVBQUUsRUFBRSxDQUFDO1lBQzlCLElBQUksR0FBRyxHQUFHLENBQUMsQ0FBQztZQUNaLE1BQU0sT0FBTyxHQUFHLENBQUMsR0FBRyxHQUFHLENBQUM7WUFDeEIsd0JBQXdCO1lBQ3hCLElBQUksQ0FBQyxHQUFHLENBQUMsQ0FBQztZQUNWLE9BQU8sQ0FBQyxHQUFHLENBQUMsR0FBRyxHQUFHLElBQUksT0FBTyxHQUFHLENBQUMsR0FBRyxDQUFDLEdBQUcsVUFBVSxFQUFFLENBQUMsSUFBSSxDQUFDLEVBQUUsQ0FBQztnQkFDM0QsR0FBRyxJQUFJLElBQUksQ0FBQyxpQkFBaUIsQ0FBQyxPQUFPLEdBQUcsQ0FBQyxDQUFDLEdBQUcsQ0FBQyxLQUFLLENBQUMsQ0FBQyxDQUFDLElBQUksQ0FBQyxDQUFDO29CQUNyRCxJQUFJLENBQUMsaUJBQWlCLENBQUMsT0FBTyxHQUFHLENBQUMsR0FBRyxDQUFDLENBQUMsR0FBRyxDQUFDLEtBQUssQ0FBQyxDQUFDLEdBQUcsQ0FBQyxDQUFDLElBQUksQ0FBQyxDQUFDO29CQUM3RCxJQUFJLENBQUMsaUJBQWlCLENBQUMsT0FBTyxHQUFHLENBQUMsR0FBRyxDQUFDLENBQUMsR0FBRyxDQUFDLEtBQUssQ0FBQyxDQUFDLEdBQUcsQ0FBQyxDQUFDLElBQUksQ0FBQyxDQUFDO29CQUM3RCxJQUFJLENBQUMsaUJBQWlCLENBQUMsT0FBTyxHQUFHLENBQUMsR0FBRyxDQUFDLENBQUMsR0FBRyxDQUFDLEtBQUssQ0FBQyxDQUFDLEdBQUcsQ0FBQyxDQUFDLElBQUksQ0FBQyxDQUFDLENBQUM7WUFDdkUsQ0FBQztZQUNELE9BQU8sQ0FBQyxHQUFHLEdBQUcsSUFBSSxPQUFPLEdBQUcsQ0FBQyxHQUFHLFVBQVUsRUFBRSxDQUFDLEVBQUUsRUFBRSxDQUFDO2dCQUNoRCxHQUFHLElBQUksSUFBSSxDQUFDLGlCQUFpQixDQUFDLE9BQU8sR0FBRyxDQUFDLENBQUMsR0FBRyxDQUFDLEtBQUssQ0FBQyxDQUFDLENBQUMsSUFBSSxDQUFDLENBQUMsQ0FBQztZQUMvRCxDQUFDO1lBQ0QsTUFBTSxDQUFDLENBQUMsQ0FBQyxHQUFHLEdBQUcsQ0FBQztRQUNsQixDQUFDO1FBRUQsc0NBQXNDO1FBQ3RDLE1BQU0sTUFBTSxHQUFHLElBQUksS0FBSyxDQUFDLEtBQUssQ0FBQyxNQUFNLENBQUMsQ0FBQztRQUN2QyxLQUFLLElBQUksQ0FBQyxHQUFHLENBQUMsRUFBRSxDQUFDLEdBQUcsS0FBSyxDQUFDLE1BQU0sRUFBRSxDQUFDLEVBQUUsRUFBRSxDQUFDO1lBQ3RDLElBQUksQ0FBQyxHQUFHLEdBQUcsRUFBRSxDQUFDO2dCQUNaLElBQUksS0FBSyxHQUFHLENBQUMsQ0FBQztnQkFDZCxNQUFNLE9BQU8sR0FBRyxDQUFDLEdBQUcsSUFBSSxDQUFDO2dCQUN6QixLQUFLLElBQUksQ0FBQyxHQUFHLENBQUMsRUFBRSxDQUFDLEdBQUcsSUFBSSxJQUFJLE9BQU8sR0FBRyxDQUFDLEdBQUcsVUFBVSxFQUFFLENBQUMsRUFBRSxFQUFFLENBQUM7b0JBQzFELEtBQUssSUFBSSxJQUFJLENBQUMsaUJBQWlCLENBQUMsT0FBTyxHQUFHLENBQUMsQ0FBQyxHQUFHLE1BQU0sQ0FBQyxDQUFDLENBQUMsQ0FBQztnQkFDM0QsQ0FBQztnQkFDRCxNQUFNLENBQUMsQ0FBQyxDQUFDLEdBQUcsQ0FBQyxLQUFLLENBQUMsQ0FBQyxDQUFDLElBQUksQ0FBQyxDQUFDLEdBQUcsS0FBSyxHQUFHLEdBQUcsQ0FBQztZQUM1QyxDQUFDO2lCQUFNLENBQUM7Z0JBQ04sTUFBTSxDQUFDLENBQUMsQ0FBQyxHQUFHLEtBQUssQ0FBQyxDQUFDLENBQUMsSUFBSSxDQUFDLENBQUM7WUFDNUIsQ0FBQztRQUNILENBQUM7UUFFRCxPQUFPLE1BQU0sQ0FBQztJQUNoQixDQUFDO0lBRUQ7O09BRUc7SUFDSCxLQUFLO1FBQ0gsTUFBTSxVQUFVLEdBQUcsSUFBSSxDQUFDLGNBQWMsQ0FBQyxNQUFNLEdBQUcsQ0FBQztZQUMvQyxDQUFDLENBQUMsSUFBSSxDQUFDLGNBQWMsQ0FBQyxNQUFNLENBQUMsQ0FBQyxDQUFDLEVBQUUsQ0FBQyxFQUFFLEVBQUUsQ0FBQyxDQUFDLEdBQUcsQ0FBQyxFQUFFLENBQUMsQ0FBQyxHQUFHLElBQUksQ0FBQyxjQUFjLENBQUMsTUFBTTtZQUM3RSxDQUFDLENBQUMsQ0FBQyxDQUFDO1FBRU4sT0FBTztZQUNMLGFBQWEsRUFBRSxJQUFJLENBQUMsYUFBYTtZQUNqQyxXQUFXLEVBQUUsSUFBSSxDQUFDLGFBQWEsQ0FBQyxJQUFJO1lBQ3BDLGlCQUFpQixFQUFFLElBQUksQ0FBQyxpQkFBaUI7WUFDekMsZUFBZSxFQUFFLElBQUksQ0FBQyxhQUFhLENBQUMsS0FBSyxFQUFFLENBQUMsYUFBYTtZQUN6RCxVQUFVO1lBQ1YsZ0JBQWdCLEVBQUUsSUFBSSxDQUFDLE1BQU0sQ0FBQyxnQkFBZ0I7U0FDL0MsQ0FBQztJQUNKLENBQUM7SUFFRDs7T0FFRztJQUNILGdCQUFnQjtRQUNkLE9BQU8sSUFBSSxHQUFHLENBQUMsSUFBSSxDQUFDLGFBQWEsQ0FBQyxDQUFDO0lBQ3JDLENBQUM7SUFFRDs7T0FFRztJQUNILFVBQVU7UUFDUixPQUFPLElBQUksQ0FBQyxhQUFhLENBQUMsSUFBSSxDQUFDO0lBQ2pDLENBQUM7SUFFRDs7T0FFRztJQUNILG9CQUFvQjtRQUNsQixPQUFPLElBQUksQ0FBQyxpQkFBaUIsQ0FBQztJQUNoQyxDQUFDO0lBRUQ7O09BRUc7SUFDSCxLQUFLO1FBQ0gsSUFBSSxDQUFDLGFBQWEsQ0FBQyxLQUFLLEVBQUUsQ0FBQztRQUMzQixJQUFJLENBQUMsaUJBQWlCLEdBQUcsQ0FBQyxDQUFDO1FBQzNCLElBQUksQ0FBQyxjQUFjLEdBQUcsRUFBRSxDQUFDO0lBQzNCLENBQUM7SUFFRDs7T0FFRztJQUNILE1BQU07UUFDSixPQUFPLElBQUksQ0FBQyxTQUFTLENBQUM7WUFDcEIsYUFBYSxFQUFFLElBQUksQ0FBQyxhQUFhO1lBQ2pDLEtBQUssRUFBRSxJQUFJLENBQUMsS0FBSyxFQUFFO1lBQ25CLGFBQWEsRUFBRSxNQUFNLENBQUMsV0FBVyxDQUFDLElBQUksQ0FBQyxhQUFhLENBQUM7WUFDckQsUUFBUSxFQUFFLElBQUksQ0FBQyxjQUFjLEVBQUU7U0FDaEMsQ0FBQyxDQUFDO0lBQ0wsQ0FBQztJQUVEOztPQUVHO0lBQ0gsV0FBVyxDQUFDLE9BQWU7UUFDekIsTUFBTSxLQUFLLEdBQUcsSUFBSSxjQUFjLENBQUMsT0FBTyxFQUFFO1lBQ3hDLFNBQVMsRUFBRSxJQUFJLENBQUMsTUFBTSxDQUFDLFNBQVM7WUFDaEMsWUFBWSxFQUFFLElBQUksQ0FBQyxNQUFNLENBQUMsWUFBWTtZQUN0QyxhQUFhLEVBQUUsSUFBSSxDQUFDLE1BQU0sQ0FBQyxhQUFhO1NBQ3pDLENBQUMsQ0FBQztRQUVILDREQUE0RDtRQUM1RCxNQUFNLGVBQWUsR0FBRyxJQUFJLENBQUMsa0JBQWtCLENBQUMsQ0FBQyxDQUFDLENBQUM7UUFDbkQsS0FBSyxNQUFNLE9BQU8sSUFBSSxlQUFlLEVBQUUsQ0FBQztZQUN0QyxLQUFLLENBQUMsV0FBVyxDQUFDLE9BQU8sQ0FBQyxTQUFTLEVBQUUsT0FBTyxDQUFDLFdBQVcsQ0FBQyxDQUFDO1FBQzVELENBQUM7UUFFRCxPQUFPLEtBQUssQ0FBQztJQUNmLENBQUM7SUFFTyxpQkFBaUI7UUFDdkIsT0FBTyxJQUFJLENBQUMsYUFBYSxDQUFDLElBQUksR0FBRyxJQUFJLENBQUMscUJBQXFCLEtBQUssQ0FBQztZQUMxRCxJQUFJLENBQUMsYUFBYSxDQUFDLElBQUksR0FBRyxDQUFDLENBQUM7SUFDckMsQ0FBQztJQUVPLGtCQUFrQixDQUFDLEtBQWM7UUFDdkMsSUFBSSxDQUFDLEtBQUs7WUFBRSxPQUFPLGdCQUFnQixDQUFDO1FBQ3BDLElBQUksS0FBSyxDQUFDLFFBQVEsQ0FBQyxNQUFNLENBQUM7WUFBRSxPQUFPLGdCQUFnQixDQUFDO1FBQ3BELElBQUksS0FBSyxDQUFDLFFBQVEsQ0FBQyxPQUFPLENBQUM7WUFBRSxPQUFPLFNBQVMsQ0FBQztRQUM5QyxJQUFJLEtBQUssQ0FBQyxRQUFRLENBQUMsUUFBUSxDQUFDO1lBQUUsT0FBTyxtQkFBbUIsQ0FBQztRQUN6RCxPQUFPLGdCQUFnQixDQUFDO0lBQzFCLENBQUM7SUFFTyxnQkFBZ0IsQ0FBQyxTQUFvQixFQUFFLE9BQWU7UUFDNUQsTUFBTSxFQUFFLEdBQUcsTUFBTSxHQUFHLE9BQU8sQ0FBQyxDQUFDLGtDQUFrQztRQUMvRCxNQUFNLEdBQUcsR0FBRyxJQUFJLENBQUMsR0FBRyxDQUFDLFNBQVMsQ0FBQyxNQUFNLEVBQUUsSUFBSSxDQUFDLE1BQU0sQ0FBQyxTQUFTLENBQUMsQ0FBQztRQUU5RCxLQUFLLElBQUksQ0FBQyxHQUFHLENBQUMsRUFBRSxDQUFDLEdBQUcsSUFBSSxDQUFDLEdBQUcsQ0FBQyxHQUFHLEVBQUUsSUFBSSxDQUFDLGlCQUFpQixDQUFDLE1BQU0sQ0FBQyxFQUFFLENBQUMsRUFBRSxFQUFFLENBQUM7WUFDdEUsTUFBTSxJQUFJLEdBQUcsU0FBUyxDQUFDLENBQUMsR0FBRyxTQUFTLENBQUMsTUFBTSxDQUFDLEdBQUcsQ0FBQyxPQUFPLEdBQUcsR0FBRyxDQUFDLENBQUM7WUFDL0QsSUFBSSxDQUFDLGlCQUFpQixDQUFDLENBQUMsQ0FBQyxJQUFJLEVBQUUsR0FBRyxJQUFJLENBQUM7WUFFdkMsb0RBQW9EO1lBQ3BELE1BQU0sT0FBTyxHQUFHLElBQUksQ0FBQyxNQUFNLENBQUMsU0FBUyxHQUFHLElBQUksQ0FBQyxpQkFBaUIsQ0FBQyxDQUFDLENBQUMsR0FBRyxNQUFNLENBQUM7WUFDM0UsSUFBSSxDQUFDLGlCQUFpQixDQUFDLENBQUMsQ0FBQyxJQUFJLE9BQU8sQ0FBQztRQUN2QyxDQUFDO0lBQ0gsQ0FBQztDQUNGIiwic291cmNlc0NvbnRlbnQiOlsiLyoqXG4gKiBGZWRlcmF0ZWQgTGVhcm5pbmcgZm9yIFNPTkFcbiAqXG4gKiBFbmFibGUgZGlzdHJpYnV0ZWQgbGVhcm5pbmcgYWNyb3NzIGVwaGVtZXJhbCBhZ2VudHMgdGhhdCBzaGFyZVxuICogdHJhamVjdG9yaWVzIHdpdGggYSBjZW50cmFsIGNvb3JkaW5hdG9yLlxuICpcbiAqIEFyY2hpdGVjdHVyZTpcbiAqIGBgYFxuICog4pSM4pSA4pSA4pSA4pSA4pSA4pSA4pSA4pSA4pSA4pSA4pSA4pSA4pSA4pSQICAgICDilIzilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilJAgICAgIOKUjOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUkFxuICog4pSCICBBZ2VudCBBICAgIOKUgiAgICAg4pSCICBBZ2VudCBCICAgIOKUgiAgICAg4pSCICBBZ2VudCBDICAgIOKUglxuICog4pSCIChlcGhlbWVyYWwpIOKUgiAgICAg4pSCIChlcGhlbWVyYWwpIOKUgiAgICAg4pSCIChlcGhlbWVyYWwpIOKUglxuICog4pSU4pSA4pSA4pSA4pSA4pSA4pSA4pSs4pSA4pSA4pSA4pSA4pSA4pSA4pSYICAgICDilJTilIDilIDilIDilIDilIDilIDilKzilIDilIDilIDilIDilIDilIDilJggICAgIOKUlOKUgOKUgOKUgOKUgOKUgOKUgOKUrOKUgOKUgOKUgOKUgOKUgOKUgOKUmFxuICogICAgICAgIOKUgiAgICAgICAgICAgICAgICAgICDilIIgICAgICAgICAgICAgICAgICAg4pSCXG4gKiAgICAgICAg4pSCICAgIGV4cG9ydCgpICAgICAgIOKUgiAgICBleHBvcnQoKSAgICAgICDilIIgICAgZXhwb3J0KClcbiAqICAgICAgICDilrwgICAgICAgICAgICAgICAgICAg4pa8ICAgICAgICAgICAgICAgICAgIOKWvFxuICogICDilIzilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilJBcbiAqICAg4pSCICAgICAgICAgICAgRmVkZXJhdGVkIENvb3JkaW5hdG9yICAgICAgICAgICAgICAg4pSCXG4gKiAgIOKUgiAgICAgICAgIChwZXJzaXN0ZW50LCBsYXJnZSBjYXBhY2l0eSkgICAgICAgICAgIOKUglxuICogICDilJTilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilJhcbiAqIGBgYFxuICpcbiAqIEBleGFtcGxlXG4gKiBgYGB0eXBlc2NyaXB0XG4gKiBpbXBvcnQgeyBFcGhlbWVyYWxBZ2VudCwgRmVkZXJhdGVkQ29vcmRpbmF0b3IgfSBmcm9tICdAcnV2ZWN0b3IvcnV2bGxtJztcbiAqXG4gKiAvLyBDcmVhdGUgY29vcmRpbmF0b3IgKHBlcnNpc3RlbnQpXG4gKiBjb25zdCBjb29yZGluYXRvciA9IG5ldyBGZWRlcmF0ZWRDb29yZGluYXRvcignY29vcmQtMScsIHsgaGlkZGVuRGltOiAyNTYgfSk7XG4gKlxuICogLy8gQ3JlYXRlIGVwaGVtZXJhbCBhZ2VudFxuICogY29uc3QgYWdlbnQgPSBuZXcgRXBoZW1lcmFsQWdlbnQoJ2FnZW50LTEnLCB7IGhpZGRlbkRpbTogMjU2IH0pO1xuICpcbiAqIC8vIEFnZW50IHByb2Nlc3NlcyB0YXNrc1xuICogYWdlbnQucHJvY2Vzc1Rhc2soWzAuMSwgMC4yLCAuLi5dLCAwLjg1KTtcbiAqIGFnZW50LnByb2Nlc3NUYXNrKFswLjMsIDAuNCwgLi4uXSwgMC45Mik7XG4gKlxuICogLy8gRXhwb3J0IGFuZCBhZ2dyZWdhdGUgYmVmb3JlIGFnZW50IHRlcm1pbmF0ZXNcbiAqIGNvbnN0IGV4cG9ydERhdGEgPSBhZ2VudC5leHBvcnRTdGF0ZSgpO1xuICogY29uc3QgcmVzdWx0ID0gY29vcmRpbmF0b3IuYWdncmVnYXRlKGV4cG9ydERhdGEpO1xuICpcbiAqIGNvbnNvbGUubG9nKGBBY2NlcHRlZDogJHtyZXN1bHQudHJhamVjdG9yaWVzQWNjZXB0ZWR9YCk7XG4gKiBgYGBcbiAqL1xuXG5pbXBvcnQge1xuICBFbWJlZGRpbmcsXG4gIExlYXJuZWRQYXR0ZXJuLFxuICBQYXR0ZXJuVHlwZSxcbiAgRmVkZXJhdGVkQ29uZmlnLFxuICBUcmFqZWN0b3J5RXhwb3J0LFxuICBBZ2VudEV4cG9ydFN0YXRzLFxuICBBZ2VudEV4cG9ydCxcbiAgQWdlbnRDb250cmlidXRpb24sXG4gIEFnZ3JlZ2F0aW9uUmVzdWx0LFxuICBDb29yZGluYXRvclN0YXRzLFxufSBmcm9tICcuL3R5cGVzJztcbmltcG9ydCB7IFJlYXNvbmluZ0JhbmsgfSBmcm9tICcuL3NvbmEnO1xuXG4vKipcbiAqIERlZmF1bHQgZmVkZXJhdGVkIGNvbmZpZ1xuICovXG5jb25zdCBERUZBVUxUX0ZFREVSQVRFRF9DT05GSUc6IFJlcXVpcmVkPEZlZGVyYXRlZENvbmZpZz4gPSB7XG4gIGhpZGRlbkRpbTogMjU2LFxuICBlbWJlZGRpbmdEaW06IDI1NixcbiAgbWljcm9Mb3JhUmFuazogMixcbiAgYmFzZUxvcmFSYW5rOiA4LFxuICB0cmFqZWN0b3J5Q2FwYWNpdHk6IDUwMCxcbiAgcGF0dGVybkNsdXN0ZXJzOiAyNSxcbiAgZXdjTGFtYmRhOiAyMDAwLFxuICBxdWFsaXR5VGhyZXNob2xkOiAwLjQsXG59O1xuXG4vKipcbiAqIEVwaGVtZXJhbCBBZ2VudCBmb3IgZmVkZXJhdGVkIGxlYXJuaW5nXG4gKlxuICogQ29sbGVjdHMgdHJhamVjdG9yaWVzIGR1cmluZyBpdHMgc2Vzc2lvbiBhbmQgZXhwb3J0cyBzdGF0ZSBiZWZvcmUgdGVybWluYXRpb24uXG4gKlxuICogQGV4YW1wbGVcbiAqIGBgYHR5cGVzY3JpcHRcbiAqIGNvbnN0IGFnZW50ID0gbmV3IEVwaGVtZXJhbEFnZW50KCdhZ2VudC0xJywgeyBoaWRkZW5EaW06IDI1NiB9KTtcbiAqXG4gKiAvLyBQcm9jZXNzIHRhc2tzIGR1cmluZyBzZXNzaW9uXG4gKiBhZ2VudC5wcm9jZXNzVGFzayhlbWJlZGRpbmcxLCAwLjg1KTtcbiAqIGFnZW50LnByb2Nlc3NUYXNrV2l0aFJvdXRlKGVtYmVkZGluZzIsIDAuOTIsICdjb2RlLW1vZGVsJyk7XG4gKlxuICogLy8gRXhwb3J0IGJlZm9yZSB0ZXJtaW5hdGlvblxuICogY29uc3QgZXhwb3J0RGF0YSA9IGFnZW50LmV4cG9ydFN0YXRlKCk7XG4gKiBgYGBcbiAqL1xuZXhwb3J0IGNsYXNzIEVwaGVtZXJhbEFnZW50IHtcbiAgcHJpdmF0ZSBhZ2VudElkOiBzdHJpbmc7XG4gIHByaXZhdGUgY29uZmlnOiBSZXF1aXJlZDxGZWRlcmF0ZWRDb25maWc+O1xuICBwcml2YXRlIHRyYWplY3RvcmllczogVHJhamVjdG9yeUV4cG9ydFtdID0gW107XG4gIHByaXZhdGUgc3RhcnRUaW1lOiBudW1iZXI7XG4gIHByaXZhdGUgcXVhbGl0eVNhbXBsZXM6IG51bWJlcltdID0gW107XG4gIHByaXZhdGUgcmVhc29uaW5nQmFuazogUmVhc29uaW5nQmFuaztcbiAgcHJpdmF0ZSBsb3JhV2VpZ2h0czogbnVtYmVyW10gPSBbXTtcblxuICBjb25zdHJ1Y3RvcihhZ2VudElkOiBzdHJpbmcsIGNvbmZpZz86IEZlZGVyYXRlZENvbmZpZykge1xuICAgIHRoaXMuYWdlbnRJZCA9IGFnZW50SWQ7XG4gICAgdGhpcy5jb25maWcgPSB7IC4uLkRFRkFVTFRfRkVERVJBVEVEX0NPTkZJRywgLi4uY29uZmlnIH07XG4gICAgdGhpcy5zdGFydFRpbWUgPSBEYXRlLm5vdygpO1xuICAgIHRoaXMucmVhc29uaW5nQmFuayA9IG5ldyBSZWFzb25pbmdCYW5rKDAuNyk7XG5cbiAgICAvLyBJbml0aWFsaXplIG1pY3JvLUxvUkEgd2VpZ2h0c1xuICAgIHRoaXMubG9yYVdlaWdodHMgPSBuZXcgQXJyYXkodGhpcy5jb25maWcuaGlkZGVuRGltICogdGhpcy5jb25maWcubWljcm9Mb3JhUmFuaylcbiAgICAgIC5maWxsKDApXG4gICAgICAubWFwKCgpID0+IChNYXRoLnJhbmRvbSgpIC0gMC41KSAqIDAuMDEpO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCBhZ2VudCBJRFxuICAgKi9cbiAgZ2V0QWdlbnRJZCgpOiBzdHJpbmcge1xuICAgIHJldHVybiB0aGlzLmFnZW50SWQ7XG4gIH1cblxuICAvKipcbiAgICogUHJvY2VzcyBhIHRhc2sgYW5kIHJlY29yZCB0cmFqZWN0b3J5XG4gICAqL1xuICBwcm9jZXNzVHJhamVjdG9yeShcbiAgICBlbWJlZGRpbmc6IEVtYmVkZGluZyxcbiAgICBhY3RpdmF0aW9uczogRW1iZWRkaW5nLFxuICAgIHF1YWxpdHk6IG51bWJlcixcbiAgICByb3V0ZT86IHN0cmluZyxcbiAgICBjb250ZXh0OiBzdHJpbmdbXSA9IFtdXG4gICk6IHZvaWQge1xuICAgIGNvbnN0IG5vdyA9IERhdGUubm93KCk7XG5cbiAgICAvLyBTdG9yZSB0cmFqZWN0b3J5IGZvciBleHBvcnRcbiAgICB0aGlzLnRyYWplY3Rvcmllcy5wdXNoKHtcbiAgICAgIGVtYmVkZGluZzogWy4uLmVtYmVkZGluZ10sXG4gICAgICBxdWFsaXR5LFxuICAgICAgcm91dGUsXG4gICAgICBjb250ZXh0OiBbLi4uY29udGV4dF0sXG4gICAgICB0aW1lc3RhbXA6IG5vdyxcbiAgICB9KTtcblxuICAgIHRoaXMucXVhbGl0eVNhbXBsZXMucHVzaChxdWFsaXR5KTtcblxuICAgIC8vIFN0b3JlIGluIGxvY2FsIHJlYXNvbmluZyBiYW5rIGlmIGhpZ2ggcXVhbGl0eVxuICAgIGlmIChxdWFsaXR5ID49IDAuNykge1xuICAgICAgdGhpcy5yZWFzb25pbmdCYW5rLnN0b3JlKCdxdWVyeV9yZXNwb25zZScsIGVtYmVkZGluZyk7XG4gICAgfVxuXG4gICAgLy8gVXBkYXRlIGxvY2FsIExvUkEgd2VpZ2h0cyBiYXNlZCBvbiBxdWFsaXR5XG4gICAgdGhpcy51cGRhdGVMb3JhV2VpZ2h0cyhlbWJlZGRpbmcsIHF1YWxpdHkpO1xuICB9XG5cbiAgLyoqXG4gICAqIFNpbXBsZSBwcm9jZXNzIHRhc2sgbWV0aG9kXG4gICAqL1xuICBwcm9jZXNzVGFzayhlbWJlZGRpbmc6IEVtYmVkZGluZywgcXVhbGl0eTogbnVtYmVyKTogdm9pZCB7XG4gICAgdGhpcy5wcm9jZXNzVHJhamVjdG9yeShlbWJlZGRpbmcsIGVtYmVkZGluZywgcXVhbGl0eSk7XG4gIH1cblxuICAvKipcbiAgICogUHJvY2VzcyB0YXNrIHdpdGggcm91dGUgaW5mb3JtYXRpb25cbiAgICovXG4gIHByb2Nlc3NUYXNrV2l0aFJvdXRlKGVtYmVkZGluZzogRW1iZWRkaW5nLCBxdWFsaXR5OiBudW1iZXIsIHJvdXRlOiBzdHJpbmcpOiB2b2lkIHtcbiAgICB0aGlzLnByb2Nlc3NUcmFqZWN0b3J5KGVtYmVkZGluZywgZW1iZWRkaW5nLCBxdWFsaXR5LCByb3V0ZSk7XG4gIH1cblxuICAvKipcbiAgICogQXBwbHkgbWljcm8tTG9SQSB0byBoaWRkZW4gc3RhdGVzXG4gICAqL1xuICBhcHBseU1pY3JvTG9yYShpbnB1dDogbnVtYmVyW10sIG91dHB1dDogbnVtYmVyW10pOiB2b2lkIHtcbiAgICBjb25zdCByYW5rID0gdGhpcy5jb25maWcubWljcm9Mb3JhUmFuaztcbiAgICBjb25zdCBkaW0gPSBNYXRoLm1pbihpbnB1dC5sZW5ndGgsIHRoaXMuY29uZmlnLmhpZGRlbkRpbSk7XG5cbiAgICAvLyBTaW1wbGUgbG93LXJhbmsgZGVjb21wb3NpdGlvbjogb3V0cHV0ID0gaW5wdXQgKyBBIEAgQiBAIGlucHV0XG4gICAgLy8gQSBpcyAoZGltIHggcmFuayksIEIgaXMgKHJhbmsgeCBkaW0pXG4gICAgZm9yIChsZXQgaSA9IDA7IGkgPCBkaW07IGkrKykge1xuICAgICAgbGV0IGRlbHRhID0gMDtcbiAgICAgIGZvciAobGV0IHIgPSAwOyByIDwgcmFuazsgcisrKSB7XG4gICAgICAgIGxldCBiU3VtID0gMDtcbiAgICAgICAgZm9yIChsZXQgaiA9IDA7IGogPCBkaW07IGorKykge1xuICAgICAgICAgIGNvbnN0IGJJZHggPSByICogZGltICsgajtcbiAgICAgICAgICBpZiAoYklkeCA8IHRoaXMubG9yYVdlaWdodHMubGVuZ3RoKSB7XG4gICAgICAgICAgICBiU3VtICs9IHRoaXMubG9yYVdlaWdodHNbYklkeF0gKiAoaW5wdXRbal0gfHwgMCk7XG4gICAgICAgICAgfVxuICAgICAgICB9XG4gICAgICAgIGNvbnN0IGFJZHggPSBpICogcmFuayArIHI7XG4gICAgICAgIGlmIChhSWR4IDwgdGhpcy5sb3JhV2VpZ2h0cy5sZW5ndGgpIHtcbiAgICAgICAgICBkZWx0YSArPSB0aGlzLmxvcmFXZWlnaHRzW2FJZHhdICogYlN1bTtcbiAgICAgICAgfVxuICAgICAgfVxuICAgICAgb3V0cHV0W2ldID0gKGlucHV0W2ldIHx8IDApICsgZGVsdGEgKiAwLjE7IC8vIFNjYWxlIGZhY3RvclxuICAgIH1cbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgbnVtYmVyIG9mIGNvbGxlY3RlZCB0cmFqZWN0b3JpZXNcbiAgICovXG4gIHRyYWplY3RvcnlDb3VudCgpOiBudW1iZXIge1xuICAgIHJldHVybiB0aGlzLnRyYWplY3Rvcmllcy5sZW5ndGg7XG4gIH1cblxuICAvKipcbiAgICogR2V0IGF2ZXJhZ2UgcXVhbGl0eVxuICAgKi9cbiAgYXZnUXVhbGl0eSgpOiBudW1iZXIge1xuICAgIGlmICh0aGlzLnF1YWxpdHlTYW1wbGVzLmxlbmd0aCA9PT0gMCkgcmV0dXJuIDA7XG4gICAgcmV0dXJuIHRoaXMucXVhbGl0eVNhbXBsZXMucmVkdWNlKChhLCBiKSA9PiBhICsgYiwgMCkgLyB0aGlzLnF1YWxpdHlTYW1wbGVzLmxlbmd0aDtcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgdXB0aW1lIGluIHNlY29uZHNcbiAgICovXG4gIHVwdGltZVNlY29uZHMoKTogbnVtYmVyIHtcbiAgICByZXR1cm4gTWF0aC5mbG9vcigoRGF0ZS5ub3coKSAtIHRoaXMuc3RhcnRUaW1lKSAvIDEwMDApO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCBhZ2VudCBzdGF0c1xuICAgKi9cbiAgc3RhdHMoKTogQWdlbnRFeHBvcnRTdGF0cyB7XG4gICAgcmV0dXJuIHtcbiAgICAgIHRvdGFsVHJhamVjdG9yaWVzOiB0aGlzLnRyYWplY3Rvcmllcy5sZW5ndGgsXG4gICAgICBhdmdRdWFsaXR5OiB0aGlzLmF2Z1F1YWxpdHkoKSxcbiAgICAgIHBhdHRlcm5zTGVhcm5lZDogdGhpcy5yZWFzb25pbmdCYW5rLnN0YXRzKCkudG90YWxQYXR0ZXJucyxcbiAgICB9O1xuICB9XG5cbiAgLyoqXG4gICAqIEZvcmNlIGxvY2FsIGxlYXJuaW5nXG4gICAqL1xuICBmb3JjZUxlYXJuKCk6IHN0cmluZyB7XG4gICAgLy8gUHJ1bmUgbG93LXBlcmZvcm1pbmcgcGF0dGVybnNcbiAgICBjb25zdCBwcnVuZWQgPSB0aGlzLnJlYXNvbmluZ0JhbmsucHJ1bmUoMC4zLCAzKTtcbiAgICByZXR1cm4gYFBydW5lZCAke3BydW5lZH0gcGF0dGVybnMsICR7dGhpcy5yZWFzb25pbmdCYW5rLnN0YXRzKCkudG90YWxQYXR0ZXJuc30gcmVtYWluaW5nYDtcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgbGVhcm5lZCBwYXR0ZXJuc1xuICAgKi9cbiAgZ2V0UGF0dGVybnMoKTogTGVhcm5lZFBhdHRlcm5bXSB7XG4gICAgcmV0dXJuIHRoaXMucmVhc29uaW5nQmFuay5nZXRCeVR5cGUoJ3F1ZXJ5X3Jlc3BvbnNlJyk7XG4gIH1cblxuICAvKipcbiAgICogQ2xlYXIgdHJhamVjdG9yaWVzIChhZnRlciBleHBvcnQpXG4gICAqL1xuICBjbGVhcigpOiB2b2lkIHtcbiAgICB0aGlzLnRyYWplY3RvcmllcyA9IFtdO1xuICAgIHRoaXMucXVhbGl0eVNhbXBsZXMgPSBbXTtcbiAgfVxuXG4gIC8qKlxuICAgKiBFeHBvcnQgYWdlbnQgc3RhdGUgZm9yIGZlZGVyYXRpb25cbiAgICpcbiAgICogQ2FsbCB0aGlzIGJlZm9yZSB0ZXJtaW5hdGluZyB0aGUgYWdlbnQuXG4gICAqL1xuICBleHBvcnRTdGF0ZSgpOiBBZ2VudEV4cG9ydCB7XG4gICAgLy8gRm9yY2UgbGVhcm5pbmcgYmVmb3JlIGV4cG9ydFxuICAgIHRoaXMuZm9yY2VMZWFybigpO1xuXG4gICAgcmV0dXJuIHtcbiAgICAgIGFnZW50SWQ6IHRoaXMuYWdlbnRJZCxcbiAgICAgIHRyYWplY3RvcmllczogWy4uLnRoaXMudHJhamVjdG9yaWVzXSxcbiAgICAgIHN0YXRzOiB0aGlzLnN0YXRzKCksXG4gICAgICBzZXNzaW9uRHVyYXRpb25NczogRGF0ZS5ub3coKSAtIHRoaXMuc3RhcnRUaW1lLFxuICAgICAgdGltZXN0YW1wOiBEYXRlLm5vdygpLFxuICAgIH07XG4gIH1cblxuICAvKipcbiAgICogU2VyaWFsaXplIHRvIEpTT05cbiAgICovXG4gIHRvSlNPTigpOiBzdHJpbmcge1xuICAgIHJldHVybiBKU09OLnN0cmluZ2lmeSh0aGlzLmV4cG9ydFN0YXRlKCkpO1xuICB9XG5cbiAgcHJpdmF0ZSB1cGRhdGVMb3JhV2VpZ2h0cyhlbWJlZGRpbmc6IEVtYmVkZGluZywgcXVhbGl0eTogbnVtYmVyKTogdm9pZCB7XG4gICAgLy8gU2ltcGxlIGdyYWRpZW50IHVwZGF0ZSBiYXNlZCBvbiBxdWFsaXR5XG4gICAgY29uc3QgbHIgPSAwLjAwMSAqIHF1YWxpdHk7XG4gICAgY29uc3QgZGltID0gTWF0aC5taW4oZW1iZWRkaW5nLmxlbmd0aCwgdGhpcy5jb25maWcuaGlkZGVuRGltKTtcblxuICAgIGZvciAobGV0IGkgPSAwOyBpIDwgTWF0aC5taW4oZGltLCB0aGlzLmxvcmFXZWlnaHRzLmxlbmd0aCk7IGkrKykge1xuICAgICAgY29uc3QgZ3JhZCA9IGVtYmVkZGluZ1tpICUgZW1iZWRkaW5nLmxlbmd0aF0gKiAocXVhbGl0eSAtIDAuNSk7XG4gICAgICB0aGlzLmxvcmFXZWlnaHRzW2ldICs9IGxyICogZ3JhZDtcbiAgICB9XG4gIH1cbn1cblxuLyoqXG4gKiBGZWRlcmF0ZWQgTGVhcm5pbmcgQ29vcmRpbmF0b3JcbiAqXG4gKiBBZ2dyZWdhdGVzIGxlYXJuaW5nIGZyb20gbXVsdGlwbGUgZXBoZW1lcmFsIGFnZW50cy5cbiAqXG4gKiBAZXhhbXBsZVxuICogYGBgdHlwZXNjcmlwdFxuICogY29uc3QgY29vcmRpbmF0b3IgPSBuZXcgRmVkZXJhdGVkQ29vcmRpbmF0b3IoJ2Nvb3JkLTEnLCB7IGhpZGRlbkRpbTogMjU2IH0pO1xuICpcbiAqIC8vIEFnZ3JlZ2F0ZSBleHBvcnRzIGZyb20gbXVsdGlwbGUgYWdlbnRzXG4gKiBmb3IgKGNvbnN0IGFnZW50RXhwb3J0IG9mIGFnZW50RXhwb3J0cykge1xuICogICBjb25zdCByZXN1bHQgPSBjb29yZGluYXRvci5hZ2dyZWdhdGUoYWdlbnRFeHBvcnQpO1xuICogICBjb25zb2xlLmxvZyhgQWdlbnQgJHtyZXN1bHQuYWdlbnRJZH06ICR7cmVzdWx0LnRyYWplY3Rvcmllc0FjY2VwdGVkfSBhY2NlcHRlZGApO1xuICogfVxuICpcbiAqIC8vIEdldCBjb29yZGluYXRvciBzdGF0aXN0aWNzXG4gKiBjb25zdCBzdGF0cyA9IGNvb3JkaW5hdG9yLnN0YXRzKCk7XG4gKiBjb25zb2xlLmxvZyhgVG90YWwgcGF0dGVybnM6ICR7c3RhdHMucGF0dGVybnNMZWFybmVkfWApO1xuICogYGBgXG4gKi9cbmV4cG9ydCBjbGFzcyBGZWRlcmF0ZWRDb29yZGluYXRvciB7XG4gIHByaXZhdGUgY29vcmRpbmF0b3JJZDogc3RyaW5nO1xuICBwcml2YXRlIGNvbmZpZzogUmVxdWlyZWQ8RmVkZXJhdGVkQ29uZmlnPjtcbiAgcHJpdmF0ZSBjb250cmlidXRpb25zOiBNYXA8c3RyaW5nLCBBZ2VudENvbnRyaWJ1dGlvbj4gPSBuZXcgTWFwKCk7XG4gIHByaXZhdGUgdG90YWxUcmFqZWN0b3JpZXM6IG51bWJlciA9IDA7XG4gIHByaXZhdGUgY29uc29saWRhdGlvbkludGVydmFsOiBudW1iZXIgPSA1MDtcbiAgcHJpdmF0ZSByZWFzb25pbmdCYW5rOiBSZWFzb25pbmdCYW5rO1xuICBwcml2YXRlIHF1YWxpdHlTYW1wbGVzOiBudW1iZXJbXSA9IFtdO1xuICBwcml2YXRlIG1hc3RlckxvcmFXZWlnaHRzOiBudW1iZXJbXSA9IFtdO1xuXG4gIGNvbnN0cnVjdG9yKGNvb3JkaW5hdG9ySWQ6IHN0cmluZywgY29uZmlnPzogRmVkZXJhdGVkQ29uZmlnKSB7XG4gICAgdGhpcy5jb29yZGluYXRvcklkID0gY29vcmRpbmF0b3JJZDtcbiAgICB0aGlzLmNvbmZpZyA9IHtcbiAgICAgIC4uLkRFRkFVTFRfRkVERVJBVEVEX0NPTkZJRyxcbiAgICAgIHRyYWplY3RvcnlDYXBhY2l0eTogNTAwMDAsIC8vIExhcmdlIGNhcGFjaXR5IGZvciBjb29yZGluYXRvclxuICAgICAgcGF0dGVybkNsdXN0ZXJzOiAyMDAsXG4gICAgICBiYXNlTG9yYVJhbms6IDE2LCAvLyBEZWVwZXIgZm9yIGFnZ3JlZ2F0aW9uXG4gICAgICAuLi5jb25maWcsXG4gICAgfTtcbiAgICB0aGlzLnJlYXNvbmluZ0JhbmsgPSBuZXcgUmVhc29uaW5nQmFuayh0aGlzLmNvbmZpZy5xdWFsaXR5VGhyZXNob2xkKTtcblxuICAgIC8vIEluaXRpYWxpemUgbWFzdGVyIExvUkEgd2VpZ2h0c1xuICAgIHRoaXMubWFzdGVyTG9yYVdlaWdodHMgPSBuZXcgQXJyYXkodGhpcy5jb25maWcuaGlkZGVuRGltICogdGhpcy5jb25maWcuYmFzZUxvcmFSYW5rKVxuICAgICAgLmZpbGwoMClcbiAgICAgIC5tYXAoKCkgPT4gKE1hdGgucmFuZG9tKCkgLSAwLjUpICogMC4wMSk7XG4gIH1cblxuICAvKipcbiAgICogR2V0IGNvb3JkaW5hdG9yIElEXG4gICAqL1xuICBnZXRDb29yZGluYXRvcklkKCk6IHN0cmluZyB7XG4gICAgcmV0dXJuIHRoaXMuY29vcmRpbmF0b3JJZDtcbiAgfVxuXG4gIC8qKlxuICAgKiBTZXQgcXVhbGl0eSB0aHJlc2hvbGQgZm9yIGFjY2VwdGluZyB0cmFqZWN0b3JpZXNcbiAgICovXG4gIHNldFF1YWxpdHlUaHJlc2hvbGQodGhyZXNob2xkOiBudW1iZXIpOiB2b2lkIHtcbiAgICB0aGlzLmNvbmZpZy5xdWFsaXR5VGhyZXNob2xkID0gdGhyZXNob2xkO1xuICB9XG5cbiAgLyoqXG4gICAqIFNldCBjb25zb2xpZGF0aW9uIGludGVydmFsXG4gICAqL1xuICBzZXRDb25zb2xpZGF0aW9uSW50ZXJ2YWwoaW50ZXJ2YWw6IG51bWJlcik6IHZvaWQge1xuICAgIHRoaXMuY29uc29saWRhdGlvbkludGVydmFsID0gaW50ZXJ2YWw7XG4gIH1cblxuICAvKipcbiAgICogQWdncmVnYXRlIGFnZW50IGV4cG9ydCBpbnRvIGNvb3JkaW5hdG9yXG4gICAqL1xuICBhZ2dyZWdhdGUoZXhwb3J0RGF0YTogQWdlbnRFeHBvcnQpOiBBZ2dyZWdhdGlvblJlc3VsdCB7XG4gICAgbGV0IGFjY2VwdGVkID0gMDtcbiAgICBsZXQgcmVqZWN0ZWQgPSAwO1xuXG4gICAgLy8gUmVwbGF5IHRyYWplY3RvcmllcyBpbnRvIG1hc3RlclxuICAgIGZvciAoY29uc3QgdHJhaiBvZiBleHBvcnREYXRhLnRyYWplY3Rvcmllcykge1xuICAgICAgaWYgKHRyYWoucXVhbGl0eSA+PSB0aGlzLmNvbmZpZy5xdWFsaXR5VGhyZXNob2xkKSB7XG4gICAgICAgIC8vIFN0b3JlIHBhdHRlcm5cbiAgICAgICAgY29uc3QgcGF0dGVyblR5cGUgPSB0aGlzLnJvdXRlVG9QYXR0ZXJuVHlwZSh0cmFqLnJvdXRlKTtcbiAgICAgICAgdGhpcy5yZWFzb25pbmdCYW5rLnN0b3JlKHBhdHRlcm5UeXBlLCB0cmFqLmVtYmVkZGluZyk7XG4gICAgICAgIHRoaXMucXVhbGl0eVNhbXBsZXMucHVzaCh0cmFqLnF1YWxpdHkpO1xuXG4gICAgICAgIC8vIFVwZGF0ZSBtYXN0ZXIgTG9SQSB3ZWlnaHRzXG4gICAgICAgIHRoaXMudXBkYXRlTWFzdGVyTG9yYSh0cmFqLmVtYmVkZGluZywgdHJhai5xdWFsaXR5KTtcblxuICAgICAgICBhY2NlcHRlZCsrO1xuICAgICAgfSBlbHNlIHtcbiAgICAgICAgcmVqZWN0ZWQrKztcbiAgICAgIH1cbiAgICB9XG5cbiAgICB0aGlzLnRvdGFsVHJhamVjdG9yaWVzICs9IGFjY2VwdGVkO1xuXG4gICAgLy8gUmVjb3JkIGNvbnRyaWJ1dGlvblxuICAgIHRoaXMuY29udHJpYnV0aW9ucy5zZXQoZXhwb3J0RGF0YS5hZ2VudElkLCB7XG4gICAgICB0cmFqZWN0b3J5Q291bnQ6IGV4cG9ydERhdGEudHJhamVjdG9yaWVzLmxlbmd0aCxcbiAgICAgIGF2Z1F1YWxpdHk6IGV4cG9ydERhdGEuc3RhdHMuYXZnUXVhbGl0eSxcbiAgICAgIHRpbWVzdGFtcDogRGF0ZS5ub3coKSxcbiAgICAgIHNlc3Npb25EdXJhdGlvbk1zOiBleHBvcnREYXRhLnNlc3Npb25EdXJhdGlvbk1zLFxuICAgIH0pO1xuXG4gICAgLy8gQXV0by1jb25zb2xpZGF0ZSBpZiBuZWVkZWRcbiAgICBjb25zdCBjb25zb2xpZGF0ZWQgPSB0aGlzLnNob3VsZENvbnNvbGlkYXRlKCk7XG4gICAgaWYgKGNvbnNvbGlkYXRlZCkge1xuICAgICAgdGhpcy5mb3JjZUNvbnNvbGlkYXRlKCk7XG4gICAgfVxuXG4gICAgcmV0dXJuIHtcbiAgICAgIGFnZW50SWQ6IGV4cG9ydERhdGEuYWdlbnRJZCxcbiAgICAgIHRyYWplY3Rvcmllc0FjY2VwdGVkOiBhY2NlcHRlZCxcbiAgICAgIHRyYWplY3Rvcmllc1JlamVjdGVkOiByZWplY3RlZCxcbiAgICAgIGNvbnNvbGlkYXRlZCxcbiAgICAgIHRvdGFsQWdlbnRzOiB0aGlzLmNvbnRyaWJ1dGlvbnMuc2l6ZSxcbiAgICAgIHRvdGFsVHJhamVjdG9yaWVzOiB0aGlzLnRvdGFsVHJhamVjdG9yaWVzLFxuICAgIH07XG4gIH1cblxuICAvKipcbiAgICogRm9yY2UgY29uc29saWRhdGlvbiAobGVhcm5pbmcpXG4gICAqL1xuICBmb3JjZUNvbnNvbGlkYXRlKCk6IHN0cmluZyB7XG4gICAgY29uc3QgcHJ1bmVkID0gdGhpcy5yZWFzb25pbmdCYW5rLnBydW5lKDAuMywgNSk7XG4gICAgcmV0dXJuIGBDb25zb2xpZGF0ZWQ6IHBydW5lZCAke3BydW5lZH0gcGF0dGVybnMsICR7dGhpcy5yZWFzb25pbmdCYW5rLnN0YXRzKCkudG90YWxQYXR0ZXJuc30gcmVtYWluaW5nYDtcbiAgfVxuXG4gIC8qKlxuICAgKiBDb25zb2xpZGF0ZSBsZWFybmluZyAoYWxpYXMpXG4gICAqL1xuICBjb25zb2xpZGF0ZSgpOiBzdHJpbmcge1xuICAgIHJldHVybiB0aGlzLmZvcmNlQ29uc29saWRhdGUoKTtcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgaW5pdGlhbCBwYXR0ZXJucyBmb3IgbmV3IGFnZW50cyAod2FybSBzdGFydClcbiAgICovXG4gIGdldEluaXRpYWxQYXR0ZXJucyhrOiBudW1iZXIgPSAxMCk6IExlYXJuZWRQYXR0ZXJuW10ge1xuICAgIGNvbnN0IGFsbFBhdHRlcm5zID0gW1xuICAgICAgLi4udGhpcy5yZWFzb25pbmdCYW5rLmdldEJ5VHlwZSgncXVlcnlfcmVzcG9uc2UnKSxcbiAgICAgIC4uLnRoaXMucmVhc29uaW5nQmFuay5nZXRCeVR5cGUoJ3JvdXRpbmcnKSxcbiAgICBdO1xuXG4gICAgLy8gU29ydCBieSBzdWNjZXNzIHJhdGUgYW5kIHJldHVybiB0b3Aga1xuICAgIHJldHVybiBhbGxQYXR0ZXJuc1xuICAgICAgLnNvcnQoKGEsIGIpID0+IGIuc3VjY2Vzc1JhdGUgLSBhLnN1Y2Nlc3NSYXRlKVxuICAgICAgLnNsaWNlKDAsIGspO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCBhbGwgbGVhcm5lZCBwYXR0ZXJuc1xuICAgKi9cbiAgZ2V0QWxsUGF0dGVybnMoKTogTGVhcm5lZFBhdHRlcm5bXSB7XG4gICAgcmV0dXJuIFtcbiAgICAgIC4uLnRoaXMucmVhc29uaW5nQmFuay5nZXRCeVR5cGUoJ3F1ZXJ5X3Jlc3BvbnNlJyksXG4gICAgICAuLi50aGlzLnJlYXNvbmluZ0JhbmsuZ2V0QnlUeXBlKCdyb3V0aW5nJyksXG4gICAgICAuLi50aGlzLnJlYXNvbmluZ0JhbmsuZ2V0QnlUeXBlKCdjb250ZXh0X3JldHJpZXZhbCcpLFxuICAgICAgLi4udGhpcy5yZWFzb25pbmdCYW5rLmdldEJ5VHlwZSgnY29ycmVjdGlvbicpLFxuICAgIF07XG4gIH1cblxuICAvKipcbiAgICogRmluZCBzaW1pbGFyIHBhdHRlcm5zXG4gICAqL1xuICBmaW5kUGF0dGVybnMocXVlcnk6IEVtYmVkZGluZywgazogbnVtYmVyKTogTGVhcm5lZFBhdHRlcm5bXSB7XG4gICAgcmV0dXJuIHRoaXMucmVhc29uaW5nQmFuay5maW5kU2ltaWxhcihxdWVyeSwgayk7XG4gIH1cblxuICAvKipcbiAgICogQXBwbHkgY29vcmRpbmF0b3IncyBMb1JBIHRvIGlucHV0XG4gICAqIE9QVElNSVpFRDogUHJlLWNvbXB1dGUgaGlkZGVuIGxheWVyIG9uY2UsIHJldXNlIHR5cGVkIGFycmF5c1xuICAgKi9cbiAgYXBwbHlMb3JhKGlucHV0OiBudW1iZXJbXSk6IG51bWJlcltdIHtcbiAgICBjb25zdCByYW5rID0gdGhpcy5jb25maWcuYmFzZUxvcmFSYW5rO1xuICAgIGNvbnN0IGRpbSA9IE1hdGgubWluKGlucHV0Lmxlbmd0aCwgdGhpcy5jb25maWcuaGlkZGVuRGltKTtcbiAgICBjb25zdCB3ZWlnaHRzTGVuID0gdGhpcy5tYXN0ZXJMb3JhV2VpZ2h0cy5sZW5ndGg7XG5cbiAgICAvLyBQcmUtY29tcHV0ZSBoaWRkZW4gbGF5ZXIgKGlucHV0IEAgQilcbiAgICBjb25zdCBoaWRkZW4gPSBuZXcgRmxvYXQ2NEFycmF5KHJhbmspO1xuICAgIGZvciAobGV0IHIgPSAwOyByIDwgcmFuazsgcisrKSB7XG4gICAgICBsZXQgc3VtID0gMDtcbiAgICAgIGNvbnN0IGJhc2VJZHggPSByICogZGltO1xuICAgICAgLy8gVW5yb2xsIHRoZSBpbm5lciBsb29wXG4gICAgICBsZXQgaiA9IDA7XG4gICAgICBmb3IgKDsgaiArIDMgPCBkaW0gJiYgYmFzZUlkeCArIGogKyAzIDwgd2VpZ2h0c0xlbjsgaiArPSA0KSB7XG4gICAgICAgIHN1bSArPSB0aGlzLm1hc3RlckxvcmFXZWlnaHRzW2Jhc2VJZHggKyBqXSAqIChpbnB1dFtqXSB8fCAwKSArXG4gICAgICAgICAgICAgICB0aGlzLm1hc3RlckxvcmFXZWlnaHRzW2Jhc2VJZHggKyBqICsgMV0gKiAoaW5wdXRbaiArIDFdIHx8IDApICtcbiAgICAgICAgICAgICAgIHRoaXMubWFzdGVyTG9yYVdlaWdodHNbYmFzZUlkeCArIGogKyAyXSAqIChpbnB1dFtqICsgMl0gfHwgMCkgK1xuICAgICAgICAgICAgICAgdGhpcy5tYXN0ZXJMb3JhV2VpZ2h0c1tiYXNlSWR4ICsgaiArIDNdICogKGlucHV0W2ogKyAzXSB8fCAwKTtcbiAgICAgIH1cbiAgICAgIGZvciAoOyBqIDwgZGltICYmIGJhc2VJZHggKyBqIDwgd2VpZ2h0c0xlbjsgaisrKSB7XG4gICAgICAgIHN1bSArPSB0aGlzLm1hc3RlckxvcmFXZWlnaHRzW2Jhc2VJZHggKyBqXSAqIChpbnB1dFtqXSB8fCAwKTtcbiAgICAgIH1cbiAgICAgIGhpZGRlbltyXSA9IHN1bTtcbiAgICB9XG5cbiAgICAvLyBDb21wdXRlIG91dHB1dCAoaGlkZGVuIEAgQSArIGlucHV0KVxuICAgIGNvbnN0IG91dHB1dCA9IG5ldyBBcnJheShpbnB1dC5sZW5ndGgpO1xuICAgIGZvciAobGV0IGkgPSAwOyBpIDwgaW5wdXQubGVuZ3RoOyBpKyspIHtcbiAgICAgIGlmIChpIDwgZGltKSB7XG4gICAgICAgIGxldCBkZWx0YSA9IDA7XG4gICAgICAgIGNvbnN0IGJhc2VJZHggPSBpICogcmFuaztcbiAgICAgICAgZm9yIChsZXQgciA9IDA7IHIgPCByYW5rICYmIGJhc2VJZHggKyByIDwgd2VpZ2h0c0xlbjsgcisrKSB7XG4gICAgICAgICAgZGVsdGEgKz0gdGhpcy5tYXN0ZXJMb3JhV2VpZ2h0c1tiYXNlSWR4ICsgcl0gKiBoaWRkZW5bcl07XG4gICAgICAgIH1cbiAgICAgICAgb3V0cHV0W2ldID0gKGlucHV0W2ldIHx8IDApICsgZGVsdGEgKiAwLjE7XG4gICAgICB9IGVsc2Uge1xuICAgICAgICBvdXRwdXRbaV0gPSBpbnB1dFtpXSB8fCAwO1xuICAgICAgfVxuICAgIH1cblxuICAgIHJldHVybiBvdXRwdXQ7XG4gIH1cblxuICAvKipcbiAgICogR2V0IGNvb3JkaW5hdG9yIHN0YXRpc3RpY3NcbiAgICovXG4gIHN0YXRzKCk6IENvb3JkaW5hdG9yU3RhdHMge1xuICAgIGNvbnN0IGF2Z1F1YWxpdHkgPSB0aGlzLnF1YWxpdHlTYW1wbGVzLmxlbmd0aCA+IDBcbiAgICAgID8gdGhpcy5xdWFsaXR5U2FtcGxlcy5yZWR1Y2UoKGEsIGIpID0+IGEgKyBiLCAwKSAvIHRoaXMucXVhbGl0eVNhbXBsZXMubGVuZ3RoXG4gICAgICA6IDA7XG5cbiAgICByZXR1cm4ge1xuICAgICAgY29vcmRpbmF0b3JJZDogdGhpcy5jb29yZGluYXRvcklkLFxuICAgICAgdG90YWxBZ2VudHM6IHRoaXMuY29udHJpYnV0aW9ucy5zaXplLFxuICAgICAgdG90YWxUcmFqZWN0b3JpZXM6IHRoaXMudG90YWxUcmFqZWN0b3JpZXMsXG4gICAgICBwYXR0ZXJuc0xlYXJuZWQ6IHRoaXMucmVhc29uaW5nQmFuay5zdGF0cygpLnRvdGFsUGF0dGVybnMsXG4gICAgICBhdmdRdWFsaXR5LFxuICAgICAgcXVhbGl0eVRocmVzaG9sZDogdGhpcy5jb25maWcucXVhbGl0eVRocmVzaG9sZCxcbiAgICB9O1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCBjb250cmlidXRpb24gaGlzdG9yeVxuICAgKi9cbiAgZ2V0Q29udHJpYnV0aW9ucygpOiBNYXA8c3RyaW5nLCBBZ2VudENvbnRyaWJ1dGlvbj4ge1xuICAgIHJldHVybiBuZXcgTWFwKHRoaXMuY29udHJpYnV0aW9ucyk7XG4gIH1cblxuICAvKipcbiAgICogR2V0IHRvdGFsIGFnZW50IGNvdW50XG4gICAqL1xuICBhZ2VudENvdW50KCk6IG51bWJlciB7XG4gICAgcmV0dXJuIHRoaXMuY29udHJpYnV0aW9ucy5zaXplO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCB0b3RhbCB0cmFqZWN0b3J5IGNvdW50XG4gICAqL1xuICBnZXRUb3RhbFRyYWplY3RvcmllcygpOiBudW1iZXIge1xuICAgIHJldHVybiB0aGlzLnRvdGFsVHJhamVjdG9yaWVzO1xuICB9XG5cbiAgLyoqXG4gICAqIENsZWFyIGFsbCBjb250cmlidXRpb25zXG4gICAqL1xuICBjbGVhcigpOiB2b2lkIHtcbiAgICB0aGlzLmNvbnRyaWJ1dGlvbnMuY2xlYXIoKTtcbiAgICB0aGlzLnRvdGFsVHJhamVjdG9yaWVzID0gMDtcbiAgICB0aGlzLnF1YWxpdHlTYW1wbGVzID0gW107XG4gIH1cblxuICAvKipcbiAgICogRXhwb3J0IGNvb3JkaW5hdG9yIHN0YXRlXG4gICAqL1xuICB0b0pTT04oKTogc3RyaW5nIHtcbiAgICByZXR1cm4gSlNPTi5zdHJpbmdpZnkoe1xuICAgICAgY29vcmRpbmF0b3JJZDogdGhpcy5jb29yZGluYXRvcklkLFxuICAgICAgc3RhdHM6IHRoaXMuc3RhdHMoKSxcbiAgICAgIGNvbnRyaWJ1dGlvbnM6IE9iamVjdC5mcm9tRW50cmllcyh0aGlzLmNvbnRyaWJ1dGlvbnMpLFxuICAgICAgcGF0dGVybnM6IHRoaXMuZ2V0QWxsUGF0dGVybnMoKSxcbiAgICB9KTtcbiAgfVxuXG4gIC8qKlxuICAgKiBDcmVhdGUgYWdlbnQgd2l0aCBjb29yZGluYXRvcidzIGxlYXJuZWQgcGF0dGVybnNcbiAgICovXG4gIGNyZWF0ZUFnZW50KGFnZW50SWQ6IHN0cmluZyk6IEVwaGVtZXJhbEFnZW50IHtcbiAgICBjb25zdCBhZ2VudCA9IG5ldyBFcGhlbWVyYWxBZ2VudChhZ2VudElkLCB7XG4gICAgICBoaWRkZW5EaW06IHRoaXMuY29uZmlnLmhpZGRlbkRpbSxcbiAgICAgIGVtYmVkZGluZ0RpbTogdGhpcy5jb25maWcuZW1iZWRkaW5nRGltLFxuICAgICAgbWljcm9Mb3JhUmFuazogdGhpcy5jb25maWcubWljcm9Mb3JhUmFuayxcbiAgICB9KTtcblxuICAgIC8vIFdhcm0gc3RhcnQ6IHByb2Nlc3MgaW5pdGlhbCBwYXR0ZXJucyBhcyBwb3NpdGl2ZSBleGFtcGxlc1xuICAgIGNvbnN0IGluaXRpYWxQYXR0ZXJucyA9IHRoaXMuZ2V0SW5pdGlhbFBhdHRlcm5zKDUpO1xuICAgIGZvciAoY29uc3QgcGF0dGVybiBvZiBpbml0aWFsUGF0dGVybnMpIHtcbiAgICAgIGFnZW50LnByb2Nlc3NUYXNrKHBhdHRlcm4uZW1iZWRkaW5nLCBwYXR0ZXJuLnN1Y2Nlc3NSYXRlKTtcbiAgICB9XG5cbiAgICByZXR1cm4gYWdlbnQ7XG4gIH1cblxuICBwcml2YXRlIHNob3VsZENvbnNvbGlkYXRlKCk6IGJvb2xlYW4ge1xuICAgIHJldHVybiB0aGlzLmNvbnRyaWJ1dGlvbnMuc2l6ZSAlIHRoaXMuY29uc29saWRhdGlvbkludGVydmFsID09PSAwICYmXG4gICAgICAgICAgIHRoaXMuY29udHJpYnV0aW9ucy5zaXplID4gMDtcbiAgfVxuXG4gIHByaXZhdGUgcm91dGVUb1BhdHRlcm5UeXBlKHJvdXRlPzogc3RyaW5nKTogUGF0dGVyblR5cGUge1xuICAgIGlmICghcm91dGUpIHJldHVybiAncXVlcnlfcmVzcG9uc2UnO1xuICAgIGlmIChyb3V0ZS5pbmNsdWRlcygnY29kZScpKSByZXR1cm4gJ3F1ZXJ5X3Jlc3BvbnNlJztcbiAgICBpZiAocm91dGUuaW5jbHVkZXMoJ3JvdXRlJykpIHJldHVybiAncm91dGluZyc7XG4gICAgaWYgKHJvdXRlLmluY2x1ZGVzKCdtZW1vcnknKSkgcmV0dXJuICdjb250ZXh0X3JldHJpZXZhbCc7XG4gICAgcmV0dXJuICdxdWVyeV9yZXNwb25zZSc7XG4gIH1cblxuICBwcml2YXRlIHVwZGF0ZU1hc3RlckxvcmEoZW1iZWRkaW5nOiBFbWJlZGRpbmcsIHF1YWxpdHk6IG51bWJlcik6IHZvaWQge1xuICAgIGNvbnN0IGxyID0gMC4wMDA1ICogcXVhbGl0eTsgLy8gU2xvd2VyIGxlYXJuaW5nIGZvciBjb29yZGluYXRvclxuICAgIGNvbnN0IGRpbSA9IE1hdGgubWluKGVtYmVkZGluZy5sZW5ndGgsIHRoaXMuY29uZmlnLmhpZGRlbkRpbSk7XG5cbiAgICBmb3IgKGxldCBpID0gMDsgaSA8IE1hdGgubWluKGRpbSwgdGhpcy5tYXN0ZXJMb3JhV2VpZ2h0cy5sZW5ndGgpOyBpKyspIHtcbiAgICAgIGNvbnN0IGdyYWQgPSBlbWJlZGRpbmdbaSAlIGVtYmVkZGluZy5sZW5ndGhdICogKHF1YWxpdHkgLSAwLjUpO1xuICAgICAgdGhpcy5tYXN0ZXJMb3JhV2VpZ2h0c1tpXSArPSBsciAqIGdyYWQ7XG5cbiAgICAgIC8vIEVXQyByZWd1bGFyaXphdGlvbiAtIHByZXZlbnQgbGFyZ2Ugd2VpZ2h0IGNoYW5nZXNcbiAgICAgIGNvbnN0IHBlbmFsdHkgPSB0aGlzLmNvbmZpZy5ld2NMYW1iZGEgKiB0aGlzLm1hc3RlckxvcmFXZWlnaHRzW2ldICogMC4wMDAxO1xuICAgICAgdGhpcy5tYXN0ZXJMb3JhV2VpZ2h0c1tpXSAtPSBwZW5hbHR5O1xuICAgIH1cbiAgfVxufVxuIl19