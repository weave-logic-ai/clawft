"use strict";
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
Object.defineProperty(exports, "__esModule", { value: true });
exports.FederatedCoordinator = exports.EphemeralAgent = void 0;
const sona_1 = require("./sona");
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
class EphemeralAgent {
    constructor(agentId, config) {
        this.trajectories = [];
        this.qualitySamples = [];
        this.loraWeights = [];
        this.agentId = agentId;
        this.config = { ...DEFAULT_FEDERATED_CONFIG, ...config };
        this.startTime = Date.now();
        this.reasoningBank = new sona_1.ReasoningBank(0.7);
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
exports.EphemeralAgent = EphemeralAgent;
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
class FederatedCoordinator {
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
        this.reasoningBank = new sona_1.ReasoningBank(this.config.qualityThreshold);
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
exports.FederatedCoordinator = FederatedCoordinator;
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiZmVkZXJhdGVkLmpzIiwic291cmNlUm9vdCI6IiIsInNvdXJjZXMiOlsiLi4vLi4vc3JjL2ZlZGVyYXRlZC50cyJdLCJuYW1lcyI6W10sIm1hcHBpbmdzIjoiO0FBQUE7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7O0dBeUNHOzs7QUFjSCxpQ0FBdUM7QUFFdkM7O0dBRUc7QUFDSCxNQUFNLHdCQUF3QixHQUE4QjtJQUMxRCxTQUFTLEVBQUUsR0FBRztJQUNkLFlBQVksRUFBRSxHQUFHO0lBQ2pCLGFBQWEsRUFBRSxDQUFDO0lBQ2hCLFlBQVksRUFBRSxDQUFDO0lBQ2Ysa0JBQWtCLEVBQUUsR0FBRztJQUN2QixlQUFlLEVBQUUsRUFBRTtJQUNuQixTQUFTLEVBQUUsSUFBSTtJQUNmLGdCQUFnQixFQUFFLEdBQUc7Q0FDdEIsQ0FBQztBQUVGOzs7Ozs7Ozs7Ozs7Ozs7O0dBZ0JHO0FBQ0gsTUFBYSxjQUFjO0lBU3pCLFlBQVksT0FBZSxFQUFFLE1BQXdCO1FBTjdDLGlCQUFZLEdBQXVCLEVBQUUsQ0FBQztRQUV0QyxtQkFBYyxHQUFhLEVBQUUsQ0FBQztRQUU5QixnQkFBVyxHQUFhLEVBQUUsQ0FBQztRQUdqQyxJQUFJLENBQUMsT0FBTyxHQUFHLE9BQU8sQ0FBQztRQUN2QixJQUFJLENBQUMsTUFBTSxHQUFHLEVBQUUsR0FBRyx3QkFBd0IsRUFBRSxHQUFHLE1BQU0sRUFBRSxDQUFDO1FBQ3pELElBQUksQ0FBQyxTQUFTLEdBQUcsSUFBSSxDQUFDLEdBQUcsRUFBRSxDQUFDO1FBQzVCLElBQUksQ0FBQyxhQUFhLEdBQUcsSUFBSSxvQkFBYSxDQUFDLEdBQUcsQ0FBQyxDQUFDO1FBRTVDLGdDQUFnQztRQUNoQyxJQUFJLENBQUMsV0FBVyxHQUFHLElBQUksS0FBSyxDQUFDLElBQUksQ0FBQyxNQUFNLENBQUMsU0FBUyxHQUFHLElBQUksQ0FBQyxNQUFNLENBQUMsYUFBYSxDQUFDO2FBQzVFLElBQUksQ0FBQyxDQUFDLENBQUM7YUFDUCxH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LENBQUMsSUFBSSxPQUFPLENBQUM7UUFDdkMsQ0FBQztJQUNILENBQUM7Q0FDRjtBQTFTRCxvREEwU0MiLCJzb3VyY2VzQ29udGVudCI6WyIvKipcbiAqIEZlZGVyYXRlZCBMZWFybmluZyBmb3IgU09OQVxuICpcbiAqIEVuYWJsZSBkaXN0cmlidXRlZCBsZWFybmluZyBhY3Jvc3MgZXBoZW1lcmFsIGFnZW50cyB0aGF0IHNoYXJlXG4gKiB0cmFqZWN0b3JpZXMgd2l0aCBhIGNlbnRyYWwgY29vcmRpbmF0b3IuXG4gKlxuICogQXJjaGl0ZWN0dXJlOlxuICogYGBgXG4gKiDilIzilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilIDilJAgICAgIOKUjOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUkCAgICAg4pSM4pSA4pSA4pSA4pSA4pSA4pSA4pSA4pSA4pSA4pSA4pSA4pSA4pSA4pSQXG4gKiDilIIgIEFnZW50IEEgICAg4pSCICAgICDilIIgIEFnZW50IEIgICAg4pSCICAgICDilIIgIEFnZW50IEMgICAg4pSCXG4gKiDilIIgKGVwaGVtZXJhbCkg4pSCICAgICDilIIgKGVwaGVtZXJhbCkg4pSCICAgICDilIIgKGVwaGVtZXJhbCkg4pSCXG4gKiDilJTilIDilIDilIDilIDilIDilIDilKzilIDilIDilIDilIDilIDilIDilJggICAgIOKUlOKUgOKUgOKUgOKUgOKUgOKUgOKUrOKUgOKUgOKUgOKUgOKUgOKUgOKUmCAgICAg4pSU4pSA4pSA4pSA4pSA4pSA4pSA4pSs4pSA4pSA4pSA4pSA4pSA4pSA4pSYXG4gKiAgICAgICAg4pSCICAgICAgICAgICAgICAgICAgIOKUgiAgICAgICAgICAgICAgICAgICDilIJcbiAqICAgICAgICDilIIgICAgZXhwb3J0KCkgICAgICAg4pSCICAgIGV4cG9ydCgpICAgICAgIOKUgiAgICBleHBvcnQoKVxuICogICAgICAgIOKWvCAgICAgICAgICAgICAgICAgICDilrwgICAgICAgICAgICAgICAgICAg4pa8XG4gKiAgIOKUjOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUkFxuICogICDilIIgICAgICAgICAgICBGZWRlcmF0ZWQgQ29vcmRpbmF0b3IgICAgICAgICAgICAgICDilIJcbiAqICAg4pSCICAgICAgICAgKHBlcnNpc3RlbnQsIGxhcmdlIGNhcGFjaXR5KSAgICAgICAgICAg4pSCXG4gKiAgIOKUlOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUgOKUmFxuICogYGBgXG4gKlxuICogQGV4YW1wbGVcbiAqIGBgYHR5cGVzY3JpcHRcbiAqIGltcG9ydCB7IEVwaGVtZXJhbEFnZW50LCBGZWRlcmF0ZWRDb29yZGluYXRvciB9IGZyb20gJ0BydXZlY3Rvci9ydXZsbG0nO1xuICpcbiAqIC8vIENyZWF0ZSBjb29yZGluYXRvciAocGVyc2lzdGVudClcbiAqIGNvbnN0IGNvb3JkaW5hdG9yID0gbmV3IEZlZGVyYXRlZENvb3JkaW5hdG9yKCdjb29yZC0xJywgeyBoaWRkZW5EaW06IDI1NiB9KTtcbiAqXG4gKiAvLyBDcmVhdGUgZXBoZW1lcmFsIGFnZW50XG4gKiBjb25zdCBhZ2VudCA9IG5ldyBFcGhlbWVyYWxBZ2VudCgnYWdlbnQtMScsIHsgaGlkZGVuRGltOiAyNTYgfSk7XG4gKlxuICogLy8gQWdlbnQgcHJvY2Vzc2VzIHRhc2tzXG4gKiBhZ2VudC5wcm9jZXNzVGFzayhbMC4xLCAwLjIsIC4uLl0sIDAuODUpO1xuICogYWdlbnQucHJvY2Vzc1Rhc2soWzAuMywgMC40LCAuLi5dLCAwLjkyKTtcbiAqXG4gKiAvLyBFeHBvcnQgYW5kIGFnZ3JlZ2F0ZSBiZWZvcmUgYWdlbnQgdGVybWluYXRlc1xuICogY29uc3QgZXhwb3J0RGF0YSA9IGFnZW50LmV4cG9ydFN0YXRlKCk7XG4gKiBjb25zdCByZXN1bHQgPSBjb29yZGluYXRvci5hZ2dyZWdhdGUoZXhwb3J0RGF0YSk7XG4gKlxuICogY29uc29sZS5sb2coYEFjY2VwdGVkOiAke3Jlc3VsdC50cmFqZWN0b3JpZXNBY2NlcHRlZH1gKTtcbiAqIGBgYFxuICovXG5cbmltcG9ydCB7XG4gIEVtYmVkZGluZyxcbiAgTGVhcm5lZFBhdHRlcm4sXG4gIFBhdHRlcm5UeXBlLFxuICBGZWRlcmF0ZWRDb25maWcsXG4gIFRyYWplY3RvcnlFeHBvcnQsXG4gIEFnZW50RXhwb3J0U3RhdHMsXG4gIEFnZW50RXhwb3J0LFxuICBBZ2VudENvbnRyaWJ1dGlvbixcbiAgQWdncmVnYXRpb25SZXN1bHQsXG4gIENvb3JkaW5hdG9yU3RhdHMsXG59IGZyb20gJy4vdHlwZXMnO1xuaW1wb3J0IHsgUmVhc29uaW5nQmFuayB9IGZyb20gJy4vc29uYSc7XG5cbi8qKlxuICogRGVmYXVsdCBmZWRlcmF0ZWQgY29uZmlnXG4gKi9cbmNvbnN0IERFRkFVTFRfRkVERVJBVEVEX0NPTkZJRzogUmVxdWlyZWQ8RmVkZXJhdGVkQ29uZmlnPiA9IHtcbiAgaGlkZGVuRGltOiAyNTYsXG4gIGVtYmVkZGluZ0RpbTogMjU2LFxuICBtaWNyb0xvcmFSYW5rOiAyLFxuICBiYXNlTG9yYVJhbms6IDgsXG4gIHRyYWplY3RvcnlDYXBhY2l0eTogNTAwLFxuICBwYXR0ZXJuQ2x1c3RlcnM6IDI1LFxuICBld2NMYW1iZGE6IDIwMDAsXG4gIHF1YWxpdHlUaHJlc2hvbGQ6IDAuNCxcbn07XG5cbi8qKlxuICogRXBoZW1lcmFsIEFnZW50IGZvciBmZWRlcmF0ZWQgbGVhcm5pbmdcbiAqXG4gKiBDb2xsZWN0cyB0cmFqZWN0b3JpZXMgZHVyaW5nIGl0cyBzZXNzaW9uIGFuZCBleHBvcnRzIHN0YXRlIGJlZm9yZSB0ZXJtaW5hdGlvbi5cbiAqXG4gKiBAZXhhbXBsZVxuICogYGBgdHlwZXNjcmlwdFxuICogY29uc3QgYWdlbnQgPSBuZXcgRXBoZW1lcmFsQWdlbnQoJ2FnZW50LTEnLCB7IGhpZGRlbkRpbTogMjU2IH0pO1xuICpcbiAqIC8vIFByb2Nlc3MgdGFza3MgZHVyaW5nIHNlc3Npb25cbiAqIGFnZW50LnByb2Nlc3NUYXNrKGVtYmVkZGluZzEsIDAuODUpO1xuICogYWdlbnQucHJvY2Vzc1Rhc2tXaXRoUm91dGUoZW1iZWRkaW5nMiwgMC45MiwgJ2NvZGUtbW9kZWwnKTtcbiAqXG4gKiAvLyBFeHBvcnQgYmVmb3JlIHRlcm1pbmF0aW9uXG4gKiBjb25zdCBleHBvcnREYXRhID0gYWdlbnQuZXhwb3J0U3RhdGUoKTtcbiAqIGBgYFxuICovXG5leHBvcnQgY2xhc3MgRXBoZW1lcmFsQWdlbnQge1xuICBwcml2YXRlIGFnZW50SWQ6IHN0cmluZztcbiAgcHJpdmF0ZSBjb25maWc6IFJlcXVpcmVkPEZlZGVyYXRlZENvbmZpZz47XG4gIHByaXZhdGUgdHJhamVjdG9yaWVzOiBUcmFqZWN0b3J5RXhwb3J0W10gPSBbXTtcbiAgcHJpdmF0ZSBzdGFydFRpbWU6IG51bWJlcjtcbiAgcHJpdmF0ZSBxdWFsaXR5U2FtcGxlczogbnVtYmVyW10gPSBbXTtcbiAgcHJpdmF0ZSByZWFzb25pbmdCYW5rOiBSZWFzb25pbmdCYW5rO1xuICBwcml2YXRlIGxvcmFXZWlnaHRzOiBudW1iZXJbXSA9IFtdO1xuXG4gIGNvbnN0cnVjdG9yKGFnZW50SWQ6IHN0cmluZywgY29uZmlnPzogRmVkZXJhdGVkQ29uZmlnKSB7XG4gICAgdGhpcy5hZ2VudElkID0gYWdlbnRJZDtcbiAgICB0aGlzLmNvbmZpZyA9IHsgLi4uREVGQVVMVF9GRURFUkFURURfQ09ORklHLCAuLi5jb25maWcgfTtcbiAgICB0aGlzLnN0YXJ0VGltZSA9IERhdGUubm93KCk7XG4gICAgdGhpcy5yZWFzb25pbmdCYW5rID0gbmV3IFJlYXNvbmluZ0JhbmsoMC43KTtcblxuICAgIC8vIEluaXRpYWxpemUgbWljcm8tTG9SQSB3ZWlnaHRzXG4gICAgdGhpcy5sb3JhV2VpZ2h0cyA9IG5ldyBBcnJheSh0aGlzLmNvbmZpZy5oaWRkZW5EaW0gKiB0aGlzLmNvbmZpZy5taWNyb0xvcmFSYW5rKVxuICAgICAgLmZpbGwoMClcbiAgICAgIC5tYXAoKCkgPT4gKE1hdGgucmFuZG9tKCkgLSAwLjUpICogMC4wMSk7XG4gIH1cblxuICAvKipcbiAgICogR2V0IGFnZW50IElEXG4gICAqL1xuICBnZXRBZ2VudElkKCk6IHN0cmluZyB7XG4gICAgcmV0dXJuIHRoaXMuYWdlbnRJZDtcbiAgfVxuXG4gIC8qKlxuICAgKiBQcm9jZXNzIGEgdGFzayBhbmQgcmVjb3JkIHRyYWplY3RvcnlcbiAgICovXG4gIHByb2Nlc3NUcmFqZWN0b3J5KFxuICAgIGVtYmVkZGluZzogRW1iZWRkaW5nLFxuICAgIGFjdGl2YXRpb25zOiBFbWJlZGRpbmcsXG4gICAgcXVhbGl0eTogbnVtYmVyLFxuICAgIHJvdXRlPzogc3RyaW5nLFxuICAgIGNvbnRleHQ6IHN0cmluZ1tdID0gW11cbiAgKTogdm9pZCB7XG4gICAgY29uc3Qgbm93ID0gRGF0ZS5ub3coKTtcblxuICAgIC8vIFN0b3JlIHRyYWplY3RvcnkgZm9yIGV4cG9ydFxuICAgIHRoaXMudHJhamVjdG9yaWVzLnB1c2goe1xuICAgICAgZW1iZWRkaW5nOiBbLi4uZW1iZWRkaW5nXSxcbiAgICAgIHF1YWxpdHksXG4gICAgICByb3V0ZSxcbiAgICAgIGNvbnRleHQ6IFsuLi5jb250ZXh0XSxcbiAgICAgIHRpbWVzdGFtcDogbm93LFxuICAgIH0pO1xuXG4gICAgdGhpcy5xdWFsaXR5U2FtcGxlcy5wdXNoKHF1YWxpdHkpO1xuXG4gICAgLy8gU3RvcmUgaW4gbG9jYWwgcmVhc29uaW5nIGJhbmsgaWYgaGlnaCBxdWFsaXR5XG4gICAgaWYgKHF1YWxpdHkgPj0gMC43KSB7XG4gICAgICB0aGlzLnJlYXNvbmluZ0Jhbmsuc3RvcmUoJ3F1ZXJ5X3Jlc3BvbnNlJywgZW1iZWRkaW5nKTtcbiAgICB9XG5cbiAgICAvLyBVcGRhdGUgbG9jYWwgTG9SQSB3ZWlnaHRzIGJhc2VkIG9uIHF1YWxpdHlcbiAgICB0aGlzLnVwZGF0ZUxvcmFXZWlnaHRzKGVtYmVkZGluZywgcXVhbGl0eSk7XG4gIH1cblxuICAvKipcbiAgICogU2ltcGxlIHByb2Nlc3MgdGFzayBtZXRob2RcbiAgICovXG4gIHByb2Nlc3NUYXNrKGVtYmVkZGluZzogRW1iZWRkaW5nLCBxdWFsaXR5OiBudW1iZXIpOiB2b2lkIHtcbiAgICB0aGlzLnByb2Nlc3NUcmFqZWN0b3J5KGVtYmVkZGluZywgZW1iZWRkaW5nLCBxdWFsaXR5KTtcbiAgfVxuXG4gIC8qKlxuICAgKiBQcm9jZXNzIHRhc2sgd2l0aCByb3V0ZSBpbmZvcm1hdGlvblxuICAgKi9cbiAgcHJvY2Vzc1Rhc2tXaXRoUm91dGUoZW1iZWRkaW5nOiBFbWJlZGRpbmcsIHF1YWxpdHk6IG51bWJlciwgcm91dGU6IHN0cmluZyk6IHZvaWQge1xuICAgIHRoaXMucHJvY2Vzc1RyYWplY3RvcnkoZW1iZWRkaW5nLCBlbWJlZGRpbmcsIHF1YWxpdHksIHJvdXRlKTtcbiAgfVxuXG4gIC8qKlxuICAgKiBBcHBseSBtaWNyby1Mb1JBIHRvIGhpZGRlbiBzdGF0ZXNcbiAgICovXG4gIGFwcGx5TWljcm9Mb3JhKGlucHV0OiBudW1iZXJbXSwgb3V0cHV0OiBudW1iZXJbXSk6IHZvaWQge1xuICAgIGNvbnN0IHJhbmsgPSB0aGlzLmNvbmZpZy5taWNyb0xvcmFSYW5rO1xuICAgIGNvbnN0IGRpbSA9IE1hdGgubWluKGlucHV0Lmxlbmd0aCwgdGhpcy5jb25maWcuaGlkZGVuRGltKTtcblxuICAgIC8vIFNpbXBsZSBsb3ctcmFuayBkZWNvbXBvc2l0aW9uOiBvdXRwdXQgPSBpbnB1dCArIEEgQCBCIEAgaW5wdXRcbiAgICAvLyBBIGlzIChkaW0geCByYW5rKSwgQiBpcyAocmFuayB4IGRpbSlcbiAgICBmb3IgKGxldCBpID0gMDsgaSA8IGRpbTsgaSsrKSB7XG4gICAgICBsZXQgZGVsdGEgPSAwO1xuICAgICAgZm9yIChsZXQgciA9IDA7IHIgPCByYW5rOyByKyspIHtcbiAgICAgICAgbGV0IGJTdW0gPSAwO1xuICAgICAgICBmb3IgKGxldCBqID0gMDsgaiA8IGRpbTsgaisrKSB7XG4gICAgICAgICAgY29uc3QgYklkeCA9IHIgKiBkaW0gKyBqO1xuICAgICAgICAgIGlmIChiSWR4IDwgdGhpcy5sb3JhV2VpZ2h0cy5sZW5ndGgpIHtcbiAgICAgICAgICAgIGJTdW0gKz0gdGhpcy5sb3JhV2VpZ2h0c1tiSWR4XSAqIChpbnB1dFtqXSB8fCAwKTtcbiAgICAgICAgICB9XG4gICAgICAgIH1cbiAgICAgICAgY29uc3QgYUlkeCA9IGkgKiByYW5rICsgcjtcbiAgICAgICAgaWYgKGFJZHggPCB0aGlzLmxvcmFXZWlnaHRzLmxlbmd0aCkge1xuICAgICAgICAgIGRlbHRhICs9IHRoaXMubG9yYVdlaWdodHNbYUlkeF0gKiBiU3VtO1xuICAgICAgICB9XG4gICAgICB9XG4gICAgICBvdXRwdXRbaV0gPSAoaW5wdXRbaV0gfHwgMCkgKyBkZWx0YSAqIDAuMTsgLy8gU2NhbGUgZmFjdG9yXG4gICAgfVxuICB9XG5cbiAgLyoqXG4gICAqIEdldCBudW1iZXIgb2YgY29sbGVjdGVkIHRyYWplY3Rvcmllc1xuICAgKi9cbiAgdHJhamVjdG9yeUNvdW50KCk6IG51bWJlciB7XG4gICAgcmV0dXJuIHRoaXMudHJhamVjdG9yaWVzLmxlbmd0aDtcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgYXZlcmFnZSBxdWFsaXR5XG4gICAqL1xuICBhdmdRdWFsaXR5KCk6IG51bWJlciB7XG4gICAgaWYgKHRoaXMucXVhbGl0eVNhbXBsZXMubGVuZ3RoID09PSAwKSByZXR1cm4gMDtcbiAgICByZXR1cm4gdGhpcy5xdWFsaXR5U2FtcGxlcy5yZWR1Y2UoKGEsIGIpID0+IGEgKyBiLCAwKSAvIHRoaXMucXVhbGl0eVNhbXBsZXMubGVuZ3RoO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCB1cHRpbWUgaW4gc2Vjb25kc1xuICAgKi9cbiAgdXB0aW1lU2Vjb25kcygpOiBudW1iZXIge1xuICAgIHJldHVybiBNYXRoLmZsb29yKChEYXRlLm5vdygpIC0gdGhpcy5zdGFydFRpbWUpIC8gMTAwMCk7XG4gIH1cblxuICAvKipcbiAgICogR2V0IGFnZW50IHN0YXRzXG4gICAqL1xuICBzdGF0cygpOiBBZ2VudEV4cG9ydFN0YXRzIHtcbiAgICByZXR1cm4ge1xuICAgICAgdG90YWxUcmFqZWN0b3JpZXM6IHRoaXMudHJhamVjdG9yaWVzLmxlbmd0aCxcbiAgICAgIGF2Z1F1YWxpdHk6IHRoaXMuYXZnUXVhbGl0eSgpLFxuICAgICAgcGF0dGVybnNMZWFybmVkOiB0aGlzLnJlYXNvbmluZ0Jhbmsuc3RhdHMoKS50b3RhbFBhdHRlcm5zLFxuICAgIH07XG4gIH1cblxuICAvKipcbiAgICogRm9yY2UgbG9jYWwgbGVhcm5pbmdcbiAgICovXG4gIGZvcmNlTGVhcm4oKTogc3RyaW5nIHtcbiAgICAvLyBQcnVuZSBsb3ctcGVyZm9ybWluZyBwYXR0ZXJuc1xuICAgIGNvbnN0IHBydW5lZCA9IHRoaXMucmVhc29uaW5nQmFuay5wcnVuZSgwLjMsIDMpO1xuICAgIHJldHVybiBgUHJ1bmVkICR7cHJ1bmVkfSBwYXR0ZXJucywgJHt0aGlzLnJlYXNvbmluZ0Jhbmsuc3RhdHMoKS50b3RhbFBhdHRlcm5zfSByZW1haW5pbmdgO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCBsZWFybmVkIHBhdHRlcm5zXG4gICAqL1xuICBnZXRQYXR0ZXJucygpOiBMZWFybmVkUGF0dGVybltdIHtcbiAgICByZXR1cm4gdGhpcy5yZWFzb25pbmdCYW5rLmdldEJ5VHlwZSgncXVlcnlfcmVzcG9uc2UnKTtcbiAgfVxuXG4gIC8qKlxuICAgKiBDbGVhciB0cmFqZWN0b3JpZXMgKGFmdGVyIGV4cG9ydClcbiAgICovXG4gIGNsZWFyKCk6IHZvaWQge1xuICAgIHRoaXMudHJhamVjdG9yaWVzID0gW107XG4gICAgdGhpcy5xdWFsaXR5U2FtcGxlcyA9IFtdO1xuICB9XG5cbiAgLyoqXG4gICAqIEV4cG9ydCBhZ2VudCBzdGF0ZSBmb3IgZmVkZXJhdGlvblxuICAgKlxuICAgKiBDYWxsIHRoaXMgYmVmb3JlIHRlcm1pbmF0aW5nIHRoZSBhZ2VudC5cbiAgICovXG4gIGV4cG9ydFN0YXRlKCk6IEFnZW50RXhwb3J0IHtcbiAgICAvLyBGb3JjZSBsZWFybmluZyBiZWZvcmUgZXhwb3J0XG4gICAgdGhpcy5mb3JjZUxlYXJuKCk7XG5cbiAgICByZXR1cm4ge1xuICAgICAgYWdlbnRJZDogdGhpcy5hZ2VudElkLFxuICAgICAgdHJhamVjdG9yaWVzOiBbLi4udGhpcy50cmFqZWN0b3JpZXNdLFxuICAgICAgc3RhdHM6IHRoaXMuc3RhdHMoKSxcbiAgICAgIHNlc3Npb25EdXJhdGlvbk1zOiBEYXRlLm5vdygpIC0gdGhpcy5zdGFydFRpbWUsXG4gICAgICB0aW1lc3RhbXA6IERhdGUubm93KCksXG4gICAgfTtcbiAgfVxuXG4gIC8qKlxuICAgKiBTZXJpYWxpemUgdG8gSlNPTlxuICAgKi9cbiAgdG9KU09OKCk6IHN0cmluZyB7XG4gICAgcmV0dXJuIEpTT04uc3RyaW5naWZ5KHRoaXMuZXhwb3J0U3RhdGUoKSk7XG4gIH1cblxuICBwcml2YXRlIHVwZGF0ZUxvcmFXZWlnaHRzKGVtYmVkZGluZzogRW1iZWRkaW5nLCBxdWFsaXR5OiBudW1iZXIpOiB2b2lkIHtcbiAgICAvLyBTaW1wbGUgZ3JhZGllbnQgdXBkYXRlIGJhc2VkIG9uIHF1YWxpdHlcbiAgICBjb25zdCBsciA9IDAuMDAxICogcXVhbGl0eTtcbiAgICBjb25zdCBkaW0gPSBNYXRoLm1pbihlbWJlZGRpbmcubGVuZ3RoLCB0aGlzLmNvbmZpZy5oaWRkZW5EaW0pO1xuXG4gICAgZm9yIChsZXQgaSA9IDA7IGkgPCBNYXRoLm1pbihkaW0sIHRoaXMubG9yYVdlaWdodHMubGVuZ3RoKTsgaSsrKSB7XG4gICAgICBjb25zdCBncmFkID0gZW1iZWRkaW5nW2kgJSBlbWJlZGRpbmcubGVuZ3RoXSAqIChxdWFsaXR5IC0gMC41KTtcbiAgICAgIHRoaXMubG9yYVdlaWdodHNbaV0gKz0gbHIgKiBncmFkO1xuICAgIH1cbiAgfVxufVxuXG4vKipcbiAqIEZlZGVyYXRlZCBMZWFybmluZyBDb29yZGluYXRvclxuICpcbiAqIEFnZ3JlZ2F0ZXMgbGVhcm5pbmcgZnJvbSBtdWx0aXBsZSBlcGhlbWVyYWwgYWdlbnRzLlxuICpcbiAqIEBleGFtcGxlXG4gKiBgYGB0eXBlc2NyaXB0XG4gKiBjb25zdCBjb29yZGluYXRvciA9IG5ldyBGZWRlcmF0ZWRDb29yZGluYXRvcignY29vcmQtMScsIHsgaGlkZGVuRGltOiAyNTYgfSk7XG4gKlxuICogLy8gQWdncmVnYXRlIGV4cG9ydHMgZnJvbSBtdWx0aXBsZSBhZ2VudHNcbiAqIGZvciAoY29uc3QgYWdlbnRFeHBvcnQgb2YgYWdlbnRFeHBvcnRzKSB7XG4gKiAgIGNvbnN0IHJlc3VsdCA9IGNvb3JkaW5hdG9yLmFnZ3JlZ2F0ZShhZ2VudEV4cG9ydCk7XG4gKiAgIGNvbnNvbGUubG9nKGBBZ2VudCAke3Jlc3VsdC5hZ2VudElkfTogJHtyZXN1bHQudHJhamVjdG9yaWVzQWNjZXB0ZWR9IGFjY2VwdGVkYCk7XG4gKiB9XG4gKlxuICogLy8gR2V0IGNvb3JkaW5hdG9yIHN0YXRpc3RpY3NcbiAqIGNvbnN0IHN0YXRzID0gY29vcmRpbmF0b3Iuc3RhdHMoKTtcbiAqIGNvbnNvbGUubG9nKGBUb3RhbCBwYXR0ZXJuczogJHtzdGF0cy5wYXR0ZXJuc0xlYXJuZWR9YCk7XG4gKiBgYGBcbiAqL1xuZXhwb3J0IGNsYXNzIEZlZGVyYXRlZENvb3JkaW5hdG9yIHtcbiAgcHJpdmF0ZSBjb29yZGluYXRvcklkOiBzdHJpbmc7XG4gIHByaXZhdGUgY29uZmlnOiBSZXF1aXJlZDxGZWRlcmF0ZWRDb25maWc+O1xuICBwcml2YXRlIGNvbnRyaWJ1dGlvbnM6IE1hcDxzdHJpbmcsIEFnZW50Q29udHJpYnV0aW9uPiA9IG5ldyBNYXAoKTtcbiAgcHJpdmF0ZSB0b3RhbFRyYWplY3RvcmllczogbnVtYmVyID0gMDtcbiAgcHJpdmF0ZSBjb25zb2xpZGF0aW9uSW50ZXJ2YWw6IG51bWJlciA9IDUwO1xuICBwcml2YXRlIHJlYXNvbmluZ0Jhbms6IFJlYXNvbmluZ0Jhbms7XG4gIHByaXZhdGUgcXVhbGl0eVNhbXBsZXM6IG51bWJlcltdID0gW107XG4gIHByaXZhdGUgbWFzdGVyTG9yYVdlaWdodHM6IG51bWJlcltdID0gW107XG5cbiAgY29uc3RydWN0b3IoY29vcmRpbmF0b3JJZDogc3RyaW5nLCBjb25maWc/OiBGZWRlcmF0ZWRDb25maWcpIHtcbiAgICB0aGlzLmNvb3JkaW5hdG9ySWQgPSBjb29yZGluYXRvcklkO1xuICAgIHRoaXMuY29uZmlnID0ge1xuICAgICAgLi4uREVGQVVMVF9GRURFUkFURURfQ09ORklHLFxuICAgICAgdHJhamVjdG9yeUNhcGFjaXR5OiA1MDAwMCwgLy8gTGFyZ2UgY2FwYWNpdHkgZm9yIGNvb3JkaW5hdG9yXG4gICAgICBwYXR0ZXJuQ2x1c3RlcnM6IDIwMCxcbiAgICAgIGJhc2VMb3JhUmFuazogMTYsIC8vIERlZXBlciBmb3IgYWdncmVnYXRpb25cbiAgICAgIC4uLmNvbmZpZyxcbiAgICB9O1xuICAgIHRoaXMucmVhc29uaW5nQmFuayA9IG5ldyBSZWFzb25pbmdCYW5rKHRoaXMuY29uZmlnLnF1YWxpdHlUaHJlc2hvbGQpO1xuXG4gICAgLy8gSW5pdGlhbGl6ZSBtYXN0ZXIgTG9SQSB3ZWlnaHRzXG4gICAgdGhpcy5tYXN0ZXJMb3JhV2VpZ2h0cyA9IG5ldyBBcnJheSh0aGlzLmNvbmZpZy5oaWRkZW5EaW0gKiB0aGlzLmNvbmZpZy5iYXNlTG9yYVJhbmspXG4gICAgICAuZmlsbCgwKVxuICAgICAgLm1hcCgoKSA9PiAoTWF0aC5yYW5kb20oKSAtIDAuNSkgKiAwLjAxKTtcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgY29vcmRpbmF0b3IgSURcbiAgICovXG4gIGdldENvb3JkaW5hdG9ySWQoKTogc3RyaW5nIHtcbiAgICByZXR1cm4gdGhpcy5jb29yZGluYXRvcklkO1xuICB9XG5cbiAgLyoqXG4gICAqIFNldCBxdWFsaXR5IHRocmVzaG9sZCBmb3IgYWNjZXB0aW5nIHRyYWplY3Rvcmllc1xuICAgKi9cbiAgc2V0UXVhbGl0eVRocmVzaG9sZCh0aHJlc2hvbGQ6IG51bWJlcik6IHZvaWQge1xuICAgIHRoaXMuY29uZmlnLnF1YWxpdHlUaHJlc2hvbGQgPSB0aHJlc2hvbGQ7XG4gIH1cblxuICAvKipcbiAgICogU2V0IGNvbnNvbGlkYXRpb24gaW50ZXJ2YWxcbiAgICovXG4gIHNldENvbnNvbGlkYXRpb25JbnRlcnZhbChpbnRlcnZhbDogbnVtYmVyKTogdm9pZCB7XG4gICAgdGhpcy5jb25zb2xpZGF0aW9uSW50ZXJ2YWwgPSBpbnRlcnZhbDtcbiAgfVxuXG4gIC8qKlxuICAgKiBBZ2dyZWdhdGUgYWdlbnQgZXhwb3J0IGludG8gY29vcmRpbmF0b3JcbiAgICovXG4gIGFnZ3JlZ2F0ZShleHBvcnREYXRhOiBBZ2VudEV4cG9ydCk6IEFnZ3JlZ2F0aW9uUmVzdWx0IHtcbiAgICBsZXQgYWNjZXB0ZWQgPSAwO1xuICAgIGxldCByZWplY3RlZCA9IDA7XG5cbiAgICAvLyBSZXBsYXkgdHJhamVjdG9yaWVzIGludG8gbWFzdGVyXG4gICAgZm9yIChjb25zdCB0cmFqIG9mIGV4cG9ydERhdGEudHJhamVjdG9yaWVzKSB7XG4gICAgICBpZiAodHJhai5xdWFsaXR5ID49IHRoaXMuY29uZmlnLnF1YWxpdHlUaHJlc2hvbGQpIHtcbiAgICAgICAgLy8gU3RvcmUgcGF0dGVyblxuICAgICAgICBjb25zdCBwYXR0ZXJuVHlwZSA9IHRoaXMucm91dGVUb1BhdHRlcm5UeXBlKHRyYWoucm91dGUpO1xuICAgICAgICB0aGlzLnJlYXNvbmluZ0Jhbmsuc3RvcmUocGF0dGVyblR5cGUsIHRyYWouZW1iZWRkaW5nKTtcbiAgICAgICAgdGhpcy5xdWFsaXR5U2FtcGxlcy5wdXNoKHRyYWoucXVhbGl0eSk7XG5cbiAgICAgICAgLy8gVXBkYXRlIG1hc3RlciBMb1JBIHdlaWdodHNcbiAgICAgICAgdGhpcy51cGRhdGVNYXN0ZXJMb3JhKHRyYWouZW1iZWRkaW5nLCB0cmFqLnF1YWxpdHkpO1xuXG4gICAgICAgIGFjY2VwdGVkKys7XG4gICAgICB9IGVsc2Uge1xuICAgICAgICByZWplY3RlZCsrO1xuICAgICAgfVxuICAgIH1cblxuICAgIHRoaXMudG90YWxUcmFqZWN0b3JpZXMgKz0gYWNjZXB0ZWQ7XG5cbiAgICAvLyBSZWNvcmQgY29udHJpYnV0aW9uXG4gICAgdGhpcy5jb250cmlidXRpb25zLnNldChleHBvcnREYXRhLmFnZW50SWQsIHtcbiAgICAgIHRyYWplY3RvcnlDb3VudDogZXhwb3J0RGF0YS50cmFqZWN0b3JpZXMubGVuZ3RoLFxuICAgICAgYXZnUXVhbGl0eTogZXhwb3J0RGF0YS5zdGF0cy5hdmdRdWFsaXR5LFxuICAgICAgdGltZXN0YW1wOiBEYXRlLm5vdygpLFxuICAgICAgc2Vzc2lvbkR1cmF0aW9uTXM6IGV4cG9ydERhdGEuc2Vzc2lvbkR1cmF0aW9uTXMsXG4gICAgfSk7XG5cbiAgICAvLyBBdXRvLWNvbnNvbGlkYXRlIGlmIG5lZWRlZFxuICAgIGNvbnN0IGNvbnNvbGlkYXRlZCA9IHRoaXMuc2hvdWxkQ29uc29saWRhdGUoKTtcbiAgICBpZiAoY29uc29saWRhdGVkKSB7XG4gICAgICB0aGlzLmZvcmNlQ29uc29saWRhdGUoKTtcbiAgICB9XG5cbiAgICByZXR1cm4ge1xuICAgICAgYWdlbnRJZDogZXhwb3J0RGF0YS5hZ2VudElkLFxuICAgICAgdHJhamVjdG9yaWVzQWNjZXB0ZWQ6IGFjY2VwdGVkLFxuICAgICAgdHJhamVjdG9yaWVzUmVqZWN0ZWQ6IHJlamVjdGVkLFxuICAgICAgY29uc29saWRhdGVkLFxuICAgICAgdG90YWxBZ2VudHM6IHRoaXMuY29udHJpYnV0aW9ucy5zaXplLFxuICAgICAgdG90YWxUcmFqZWN0b3JpZXM6IHRoaXMudG90YWxUcmFqZWN0b3JpZXMsXG4gICAgfTtcbiAgfVxuXG4gIC8qKlxuICAgKiBGb3JjZSBjb25zb2xpZGF0aW9uIChsZWFybmluZylcbiAgICovXG4gIGZvcmNlQ29uc29saWRhdGUoKTogc3RyaW5nIHtcbiAgICBjb25zdCBwcnVuZWQgPSB0aGlzLnJlYXNvbmluZ0JhbmsucHJ1bmUoMC4zLCA1KTtcbiAgICByZXR1cm4gYENvbnNvbGlkYXRlZDogcHJ1bmVkICR7cHJ1bmVkfSBwYXR0ZXJucywgJHt0aGlzLnJlYXNvbmluZ0Jhbmsuc3RhdHMoKS50b3RhbFBhdHRlcm5zfSByZW1haW5pbmdgO1xuICB9XG5cbiAgLyoqXG4gICAqIENvbnNvbGlkYXRlIGxlYXJuaW5nIChhbGlhcylcbiAgICovXG4gIGNvbnNvbGlkYXRlKCk6IHN0cmluZyB7XG4gICAgcmV0dXJuIHRoaXMuZm9yY2VDb25zb2xpZGF0ZSgpO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCBpbml0aWFsIHBhdHRlcm5zIGZvciBuZXcgYWdlbnRzICh3YXJtIHN0YXJ0KVxuICAgKi9cbiAgZ2V0SW5pdGlhbFBhdHRlcm5zKGs6IG51bWJlciA9IDEwKTogTGVhcm5lZFBhdHRlcm5bXSB7XG4gICAgY29uc3QgYWxsUGF0dGVybnMgPSBbXG4gICAgICAuLi50aGlzLnJlYXNvbmluZ0JhbmsuZ2V0QnlUeXBlKCdxdWVyeV9yZXNwb25zZScpLFxuICAgICAgLi4udGhpcy5yZWFzb25pbmdCYW5rLmdldEJ5VHlwZSgncm91dGluZycpLFxuICAgIF07XG5cbiAgICAvLyBTb3J0IGJ5IHN1Y2Nlc3MgcmF0ZSBhbmQgcmV0dXJuIHRvcCBrXG4gICAgcmV0dXJuIGFsbFBhdHRlcm5zXG4gICAgICAuc29ydCgoYSwgYikgPT4gYi5zdWNjZXNzUmF0ZSAtIGEuc3VjY2Vzc1JhdGUpXG4gICAgICAuc2xpY2UoMCwgayk7XG4gIH1cblxuICAvKipcbiAgICogR2V0IGFsbCBsZWFybmVkIHBhdHRlcm5zXG4gICAqL1xuICBnZXRBbGxQYXR0ZXJucygpOiBMZWFybmVkUGF0dGVybltdIHtcbiAgICByZXR1cm4gW1xuICAgICAgLi4udGhpcy5yZWFzb25pbmdCYW5rLmdldEJ5VHlwZSgncXVlcnlfcmVzcG9uc2UnKSxcbiAgICAgIC4uLnRoaXMucmVhc29uaW5nQmFuay5nZXRCeVR5cGUoJ3JvdXRpbmcnKSxcbiAgICAgIC4uLnRoaXMucmVhc29uaW5nQmFuay5nZXRCeVR5cGUoJ2NvbnRleHRfcmV0cmlldmFsJyksXG4gICAgICAuLi50aGlzLnJlYXNvbmluZ0JhbmsuZ2V0QnlUeXBlKCdjb3JyZWN0aW9uJyksXG4gICAgXTtcbiAgfVxuXG4gIC8qKlxuICAgKiBGaW5kIHNpbWlsYXIgcGF0dGVybnNcbiAgICovXG4gIGZpbmRQYXR0ZXJucyhxdWVyeTogRW1iZWRkaW5nLCBrOiBudW1iZXIpOiBMZWFybmVkUGF0dGVybltdIHtcbiAgICByZXR1cm4gdGhpcy5yZWFzb25pbmdCYW5rLmZpbmRTaW1pbGFyKHF1ZXJ5LCBrKTtcbiAgfVxuXG4gIC8qKlxuICAgKiBBcHBseSBjb29yZGluYXRvcidzIExvUkEgdG8gaW5wdXRcbiAgICogT1BUSU1JWkVEOiBQcmUtY29tcHV0ZSBoaWRkZW4gbGF5ZXIgb25jZSwgcmV1c2UgdHlwZWQgYXJyYXlzXG4gICAqL1xuICBhcHBseUxvcmEoaW5wdXQ6IG51bWJlcltdKTogbnVtYmVyW10ge1xuICAgIGNvbnN0IHJhbmsgPSB0aGlzLmNvbmZpZy5iYXNlTG9yYVJhbms7XG4gICAgY29uc3QgZGltID0gTWF0aC5taW4oaW5wdXQubGVuZ3RoLCB0aGlzLmNvbmZpZy5oaWRkZW5EaW0pO1xuICAgIGNvbnN0IHdlaWdodHNMZW4gPSB0aGlzLm1hc3RlckxvcmFXZWlnaHRzLmxlbmd0aDtcblxuICAgIC8vIFByZS1jb21wdXRlIGhpZGRlbiBsYXllciAoaW5wdXQgQCBCKVxuICAgIGNvbnN0IGhpZGRlbiA9IG5ldyBGbG9hdDY0QXJyYXkocmFuayk7XG4gICAgZm9yIChsZXQgciA9IDA7IHIgPCByYW5rOyByKyspIHtcbiAgICAgIGxldCBzdW0gPSAwO1xuICAgICAgY29uc3QgYmFzZUlkeCA9IHIgKiBkaW07XG4gICAgICAvLyBVbnJvbGwgdGhlIGlubmVyIGxvb3BcbiAgICAgIGxldCBqID0gMDtcbiAgICAgIGZvciAoOyBqICsgMyA8IGRpbSAmJiBiYXNlSWR4ICsgaiArIDMgPCB3ZWlnaHRzTGVuOyBqICs9IDQpIHtcbiAgICAgICAgc3VtICs9IHRoaXMubWFzdGVyTG9yYVdlaWdodHNbYmFzZUlkeCArIGpdICogKGlucHV0W2pdIHx8IDApICtcbiAgICAgICAgICAgICAgIHRoaXMubWFzdGVyTG9yYVdlaWdodHNbYmFzZUlkeCArIGogKyAxXSAqIChpbnB1dFtqICsgMV0gfHwgMCkgK1xuICAgICAgICAgICAgICAgdGhpcy5tYXN0ZXJMb3JhV2VpZ2h0c1tiYXNlSWR4ICsgaiArIDJdICogKGlucHV0W2ogKyAyXSB8fCAwKSArXG4gICAgICAgICAgICAgICB0aGlzLm1hc3RlckxvcmFXZWlnaHRzW2Jhc2VJZHggKyBqICsgM10gKiAoaW5wdXRbaiArIDNdIHx8IDApO1xuICAgICAgfVxuICAgICAgZm9yICg7IGogPCBkaW0gJiYgYmFzZUlkeCArIGogPCB3ZWlnaHRzTGVuOyBqKyspIHtcbiAgICAgICAgc3VtICs9IHRoaXMubWFzdGVyTG9yYVdlaWdodHNbYmFzZUlkeCArIGpdICogKGlucHV0W2pdIHx8IDApO1xuICAgICAgfVxuICAgICAgaGlkZGVuW3JdID0gc3VtO1xuICAgIH1cblxuICAgIC8vIENvbXB1dGUgb3V0cHV0IChoaWRkZW4gQCBBICsgaW5wdXQpXG4gICAgY29uc3Qgb3V0cHV0ID0gbmV3IEFycmF5KGlucHV0Lmxlbmd0aCk7XG4gICAgZm9yIChsZXQgaSA9IDA7IGkgPCBpbnB1dC5sZW5ndGg7IGkrKykge1xuICAgICAgaWYgKGkgPCBkaW0pIHtcbiAgICAgICAgbGV0IGRlbHRhID0gMDtcbiAgICAgICAgY29uc3QgYmFzZUlkeCA9IGkgKiByYW5rO1xuICAgICAgICBmb3IgKGxldCByID0gMDsgciA8IHJhbmsgJiYgYmFzZUlkeCArIHIgPCB3ZWlnaHRzTGVuOyByKyspIHtcbiAgICAgICAgICBkZWx0YSArPSB0aGlzLm1hc3RlckxvcmFXZWlnaHRzW2Jhc2VJZHggKyByXSAqIGhpZGRlbltyXTtcbiAgICAgICAgfVxuICAgICAgICBvdXRwdXRbaV0gPSAoaW5wdXRbaV0gfHwgMCkgKyBkZWx0YSAqIDAuMTtcbiAgICAgIH0gZWxzZSB7XG4gICAgICAgIG91dHB1dFtpXSA9IGlucHV0W2ldIHx8IDA7XG4gICAgICB9XG4gICAgfVxuXG4gICAgcmV0dXJuIG91dHB1dDtcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgY29vcmRpbmF0b3Igc3RhdGlzdGljc1xuICAgKi9cbiAgc3RhdHMoKTogQ29vcmRpbmF0b3JTdGF0cyB7XG4gICAgY29uc3QgYXZnUXVhbGl0eSA9IHRoaXMucXVhbGl0eVNhbXBsZXMubGVuZ3RoID4gMFxuICAgICAgPyB0aGlzLnF1YWxpdHlTYW1wbGVzLnJlZHVjZSgoYSwgYikgPT4gYSArIGIsIDApIC8gdGhpcy5xdWFsaXR5U2FtcGxlcy5sZW5ndGhcbiAgICAgIDogMDtcblxuICAgIHJldHVybiB7XG4gICAgICBjb29yZGluYXRvcklkOiB0aGlzLmNvb3JkaW5hdG9ySWQsXG4gICAgICB0b3RhbEFnZW50czogdGhpcy5jb250cmlidXRpb25zLnNpemUsXG4gICAgICB0b3RhbFRyYWplY3RvcmllczogdGhpcy50b3RhbFRyYWplY3RvcmllcyxcbiAgICAgIHBhdHRlcm5zTGVhcm5lZDogdGhpcy5yZWFzb25pbmdCYW5rLnN0YXRzKCkudG90YWxQYXR0ZXJucyxcbiAgICAgIGF2Z1F1YWxpdHksXG4gICAgICBxdWFsaXR5VGhyZXNob2xkOiB0aGlzLmNvbmZpZy5xdWFsaXR5VGhyZXNob2xkLFxuICAgIH07XG4gIH1cblxuICAvKipcbiAgICogR2V0IGNvbnRyaWJ1dGlvbiBoaXN0b3J5XG4gICAqL1xuICBnZXRDb250cmlidXRpb25zKCk6IE1hcDxzdHJpbmcsIEFnZW50Q29udHJpYnV0aW9uPiB7XG4gICAgcmV0dXJuIG5ldyBNYXAodGhpcy5jb250cmlidXRpb25zKTtcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgdG90YWwgYWdlbnQgY291bnRcbiAgICovXG4gIGFnZW50Q291bnQoKTogbnVtYmVyIHtcbiAgICByZXR1cm4gdGhpcy5jb250cmlidXRpb25zLnNpemU7XG4gIH1cblxuICAvKipcbiAgICogR2V0IHRvdGFsIHRyYWplY3RvcnkgY291bnRcbiAgICovXG4gIGdldFRvdGFsVHJhamVjdG9yaWVzKCk6IG51bWJlciB7XG4gICAgcmV0dXJuIHRoaXMudG90YWxUcmFqZWN0b3JpZXM7XG4gIH1cblxuICAvKipcbiAgICogQ2xlYXIgYWxsIGNvbnRyaWJ1dGlvbnNcbiAgICovXG4gIGNsZWFyKCk6IHZvaWQge1xuICAgIHRoaXMuY29udHJpYnV0aW9ucy5jbGVhcigpO1xuICAgIHRoaXMudG90YWxUcmFqZWN0b3JpZXMgPSAwO1xuICAgIHRoaXMucXVhbGl0eVNhbXBsZXMgPSBbXTtcbiAgfVxuXG4gIC8qKlxuICAgKiBFeHBvcnQgY29vcmRpbmF0b3Igc3RhdGVcbiAgICovXG4gIHRvSlNPTigpOiBzdHJpbmcge1xuICAgIHJldHVybiBKU09OLnN0cmluZ2lmeSh7XG4gICAgICBjb29yZGluYXRvcklkOiB0aGlzLmNvb3JkaW5hdG9ySWQsXG4gICAgICBzdGF0czogdGhpcy5zdGF0cygpLFxuICAgICAgY29udHJpYnV0aW9uczogT2JqZWN0LmZyb21FbnRyaWVzKHRoaXMuY29udHJpYnV0aW9ucyksXG4gICAgICBwYXR0ZXJuczogdGhpcy5nZXRBbGxQYXR0ZXJucygpLFxuICAgIH0pO1xuICB9XG5cbiAgLyoqXG4gICAqIENyZWF0ZSBhZ2VudCB3aXRoIGNvb3JkaW5hdG9yJ3MgbGVhcm5lZCBwYXR0ZXJuc1xuICAgKi9cbiAgY3JlYXRlQWdlbnQoYWdlbnRJZDogc3RyaW5nKTogRXBoZW1lcmFsQWdlbnQge1xuICAgIGNvbnN0IGFnZW50ID0gbmV3IEVwaGVtZXJhbEFnZW50KGFnZW50SWQsIHtcbiAgICAgIGhpZGRlbkRpbTogdGhpcy5jb25maWcuaGlkZGVuRGltLFxuICAgICAgZW1iZWRkaW5nRGltOiB0aGlzLmNvbmZpZy5lbWJlZGRpbmdEaW0sXG4gICAgICBtaWNyb0xvcmFSYW5rOiB0aGlzLmNvbmZpZy5taWNyb0xvcmFSYW5rLFxuICAgIH0pO1xuXG4gICAgLy8gV2FybSBzdGFydDogcHJvY2VzcyBpbml0aWFsIHBhdHRlcm5zIGFzIHBvc2l0aXZlIGV4YW1wbGVzXG4gICAgY29uc3QgaW5pdGlhbFBhdHRlcm5zID0gdGhpcy5nZXRJbml0aWFsUGF0dGVybnMoNSk7XG4gICAgZm9yIChjb25zdCBwYXR0ZXJuIG9mIGluaXRpYWxQYXR0ZXJucykge1xuICAgICAgYWdlbnQucHJvY2Vzc1Rhc2socGF0dGVybi5lbWJlZGRpbmcsIHBhdHRlcm4uc3VjY2Vzc1JhdGUpO1xuICAgIH1cblxuICAgIHJldHVybiBhZ2VudDtcbiAgfVxuXG4gIHByaXZhdGUgc2hvdWxkQ29uc29saWRhdGUoKTogYm9vbGVhbiB7XG4gICAgcmV0dXJuIHRoaXMuY29udHJpYnV0aW9ucy5zaXplICUgdGhpcy5jb25zb2xpZGF0aW9uSW50ZXJ2YWwgPT09IDAgJiZcbiAgICAgICAgICAgdGhpcy5jb250cmlidXRpb25zLnNpemUgPiAwO1xuICB9XG5cbiAgcHJpdmF0ZSByb3V0ZVRvUGF0dGVyblR5cGUocm91dGU/OiBzdHJpbmcpOiBQYXR0ZXJuVHlwZSB7XG4gICAgaWYgKCFyb3V0ZSkgcmV0dXJuICdxdWVyeV9yZXNwb25zZSc7XG4gICAgaWYgKHJvdXRlLmluY2x1ZGVzKCdjb2RlJykpIHJldHVybiAncXVlcnlfcmVzcG9uc2UnO1xuICAgIGlmIChyb3V0ZS5pbmNsdWRlcygncm91dGUnKSkgcmV0dXJuICdyb3V0aW5nJztcbiAgICBpZiAocm91dGUuaW5jbHVkZXMoJ21lbW9yeScpKSByZXR1cm4gJ2NvbnRleHRfcmV0cmlldmFsJztcbiAgICByZXR1cm4gJ3F1ZXJ5X3Jlc3BvbnNlJztcbiAgfVxuXG4gIHByaXZhdGUgdXBkYXRlTWFzdGVyTG9yYShlbWJlZGRpbmc6IEVtYmVkZGluZywgcXVhbGl0eTogbnVtYmVyKTogdm9pZCB7XG4gICAgY29uc3QgbHIgPSAwLjAwMDUgKiBxdWFsaXR5OyAvLyBTbG93ZXIgbGVhcm5pbmcgZm9yIGNvb3JkaW5hdG9yXG4gICAgY29uc3QgZGltID0gTWF0aC5taW4oZW1iZWRkaW5nLmxlbmd0aCwgdGhpcy5jb25maWcuaGlkZGVuRGltKTtcblxuICAgIGZvciAobGV0IGkgPSAwOyBpIDwgTWF0aC5taW4oZGltLCB0aGlzLm1hc3RlckxvcmFXZWlnaHRzLmxlbmd0aCk7IGkrKykge1xuICAgICAgY29uc3QgZ3JhZCA9IGVtYmVkZGluZ1tpICUgZW1iZWRkaW5nLmxlbmd0aF0gKiAocXVhbGl0eSAtIDAuNSk7XG4gICAgICB0aGlzLm1hc3RlckxvcmFXZWlnaHRzW2ldICs9IGxyICogZ3JhZDtcblxuICAgICAgLy8gRVdDIHJlZ3VsYXJpemF0aW9uIC0gcHJldmVudCBsYXJnZSB3ZWlnaHQgY2hhbmdlc1xuICAgICAgY29uc3QgcGVuYWx0eSA9IHRoaXMuY29uZmlnLmV3Y0xhbWJkYSAqIHRoaXMubWFzdGVyTG9yYVdlaWdodHNbaV0gKiAwLjAwMDE7XG4gICAgICB0aGlzLm1hc3RlckxvcmFXZWlnaHRzW2ldIC09IHBlbmFsdHk7XG4gICAgfVxuICB9XG59XG4iXX0=