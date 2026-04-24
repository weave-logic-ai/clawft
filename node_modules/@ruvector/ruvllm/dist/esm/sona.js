/**
 * SONA (Self-Optimizing Neural Architecture) Learning System
 *
 * Provides adaptive learning capabilities with trajectory tracking,
 * pattern recognition, and memory protection (EWC++).
 */
/**
 * Default SONA configuration
 */
const DEFAULT_SONA_CONFIG = {
    instantLoopEnabled: true,
    backgroundLoopEnabled: true,
    loraLearningRate: 0.001,
    loraRank: 8,
    ewcLambda: 2000,
    maxTrajectorySize: 1000,
    patternThreshold: 0.85,
};
/**
 * Trajectory Builder for tracking query execution paths
 *
 * @example
 * ```typescript
 * const builder = new TrajectoryBuilder();
 *
 * builder.startStep('query', 'What is AI?');
 * // ... processing ...
 * builder.endStep('AI is artificial intelligence', 0.95);
 *
 * builder.startStep('memory', 'searching context');
 * builder.endStep('found 3 relevant documents', 0.88);
 *
 * const trajectory = builder.complete('success');
 * ```
 */
export class TrajectoryBuilder {
    constructor() {
        this.steps = [];
        this.currentStep = null;
        this.stepStart = 0;
        this.id = `traj-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
        this.startTime = Date.now();
    }
    /**
     * Start a new step in the trajectory
     */
    startStep(type, input) {
        if (this.currentStep) {
            // Auto-complete previous step
            this.endStep('', 0);
        }
        this.stepStart = Date.now();
        this.currentStep = {
            type,
            input,
        };
        return this;
    }
    /**
     * End current step with output
     */
    endStep(output, confidence) {
        if (!this.currentStep) {
            return this;
        }
        this.steps.push({
            type: this.currentStep.type,
            input: this.currentStep.input,
            output,
            durationMs: Date.now() - this.stepStart,
            confidence,
        });
        this.currentStep = null;
        return this;
    }
    /**
     * Complete trajectory with final outcome
     */
    complete(outcome) {
        // Complete any pending step
        if (this.currentStep) {
            this.endStep('incomplete', 0);
        }
        return {
            id: this.id,
            steps: this.steps,
            outcome,
            durationMs: Date.now() - this.startTime,
        };
    }
    /**
     * Get current trajectory ID
     */
    getId() {
        return this.id;
    }
}
/**
 * ReasoningBank - Pattern storage and retrieval
 *
 * Stores learned patterns from successful interactions and
 * enables pattern-based reasoning shortcuts.
 *
 * OPTIMIZED: Uses Float64Array for embeddings and partial sorting
 */
export class ReasoningBank {
    constructor(threshold = 0.85) {
        this.patterns = new Map();
        this.embeddings = new Map();
        this.embeddingNorms = new Map(); // Pre-computed norms
        // Reusable arrays for findSimilar to avoid allocations
        this._similarityResults = [];
        this.threshold = threshold;
    }
    /**
     * Store a new pattern
     */
    store(type, embedding, metadata) {
        const id = `pat-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
        const pattern = {
            id,
            type,
            embedding,
            successRate: 1.0,
            useCount: 0,
            lastUsed: new Date(),
        };
        this.patterns.set(id, pattern);
        // Store as typed array for faster similarity computation
        const typedEmb = new Float64Array(embedding);
        this.embeddings.set(id, typedEmb);
        // Pre-compute and cache the norm
        let norm = 0;
        for (let i = 0; i < typedEmb.length; i++) {
            norm += typedEmb[i] * typedEmb[i];
        }
        this.embeddingNorms.set(id, Math.sqrt(norm));
        return id;
    }
    /**
     * Find similar patterns
     * OPTIMIZED: Uses typed arrays, pre-computed norms, and partial sorting
     */
    findSimilar(embedding, k = 5) {
        // Pre-compute query norm
        let queryNorm = 0;
        const queryLen = embedding.length;
        for (let i = 0; i < queryLen; i++) {
            queryNorm += embedding[i] * embedding[i];
        }
        queryNorm = Math.sqrt(queryNorm);
        if (queryNorm === 0)
            return [];
        // Reuse array to avoid allocations
        this._similarityResults.length = 0;
        for (const [id, patEmb] of this.embeddings) {
            const patNorm = this.embeddingNorms.get(id) || 0;
            if (patNorm === 0)
                continue;
            // Fast dot product
            let dot = 0;
            const minLen = Math.min(queryLen, patEmb.length);
            // Unrolled loop
            let i = 0;
            for (; i + 3 < minLen; i += 4) {
                dot += embedding[i] * patEmb[i] +
                    embedding[i + 1] * patEmb[i + 1] +
                    embedding[i + 2] * patEmb[i + 2] +
                    embedding[i + 3] * patEmb[i + 3];
            }
            for (; i < minLen; i++) {
                dot += embedding[i] * patEmb[i];
            }
            const score = dot / (queryNorm * patNorm);
            if (score >= this.threshold) {
                this._similarityResults.push({ id, score });
            }
        }
        // Partial sort for top-k (faster than full sort for large arrays)
        if (this._similarityResults.length <= k) {
            this._similarityResults.sort((a, b) => b.score - a.score);
        }
        else {
            // Quick partial sort for top k
            this.partialSort(this._similarityResults, k);
        }
        const topK = this._similarityResults.slice(0, k);
        return topK
            .map(s => this.patterns.get(s.id))
            .filter((p) => p !== undefined);
    }
    /**
     * Partial sort to get top k elements (faster than full sort)
     */
    partialSort(arr, k) {
        // Simple selection for small k
        for (let i = 0; i < k && i < arr.length; i++) {
            let maxIdx = i;
            for (let j = i + 1; j < arr.length; j++) {
                if (arr[j].score > arr[maxIdx].score) {
                    maxIdx = j;
                }
            }
            if (maxIdx !== i) {
                const temp = arr[i];
                arr[i] = arr[maxIdx];
                arr[maxIdx] = temp;
            }
        }
    }
    /**
     * Record pattern usage (success or failure)
     */
    recordUsage(patternId, success) {
        const pattern = this.patterns.get(patternId);
        if (!pattern)
            return;
        pattern.useCount++;
        pattern.lastUsed = new Date();
        // Update success rate with exponential moving average
        const alpha = 0.1;
        const outcome = success ? 1.0 : 0.0;
        pattern.successRate = alpha * outcome + (1 - alpha) * pattern.successRate;
    }
    /**
     * Get pattern by ID
     */
    get(patternId) {
        return this.patterns.get(patternId);
    }
    /**
     * Get all patterns of a type
     */
    getByType(type) {
        return Array.from(this.patterns.values()).filter(p => p.type === type);
    }
    /**
     * Prune low-performing patterns
     */
    prune(minSuccessRate = 0.3, minUseCount = 5) {
        let pruned = 0;
        for (const [id, pattern] of this.patterns) {
            if (pattern.useCount >= minUseCount && pattern.successRate < minSuccessRate) {
                this.patterns.delete(id);
                this.embeddings.delete(id);
                this.embeddingNorms.delete(id);
                pruned++;
            }
        }
        return pruned;
    }
    /**
     * Get statistics
     */
    stats() {
        const patterns = Array.from(this.patterns.values());
        const byType = {};
        let totalSuccess = 0;
        for (const p of patterns) {
            totalSuccess += p.successRate;
            byType[p.type] = (byType[p.type] || 0) + 1;
        }
        return {
            totalPatterns: patterns.length,
            avgSuccessRate: patterns.length > 0 ? totalSuccess / patterns.length : 0,
            byType,
        };
    }
    cosineSimilarity(a, b) {
        let dot = 0, normA = 0, normB = 0;
        const len = Math.min(a.length, b.length);
        for (let i = 0; i < len; i++) {
            dot += a[i] * b[i];
            normA += a[i] * a[i];
            normB += b[i] * b[i];
        }
        const denom = Math.sqrt(normA) * Math.sqrt(normB);
        return denom > 0 ? dot / denom : 0;
    }
}
/**
 * EWC++ (Elastic Weight Consolidation) Manager
 *
 * Prevents catastrophic forgetting by protecting important weights.
 * This is a simplified JS implementation of the concept.
 *
 * OPTIMIZED: Uses Float64Array for 5-10x faster penalty computation
 */
export class EwcManager {
    constructor(lambda = 2000) {
        this.tasksLearned = 0;
        this.fisherDiagonal = new Map();
        this.optimalWeights = new Map();
        // Pre-allocated buffer for penalty computation
        this._penaltyBuffer = null;
        this.lambda = lambda;
    }
    /**
     * Register a new task (after successful learning)
     */
    registerTask(taskId, weights) {
        // Store optimal weights for this task using typed arrays
        const optimalArr = new Float64Array(weights.length);
        const fisherArr = new Float64Array(weights.length);
        for (let i = 0; i < weights.length; i++) {
            optimalArr[i] = weights[i];
            fisherArr[i] = Math.abs(weights[i]) * this.lambda;
        }
        this.optimalWeights.set(taskId, optimalArr);
        this.fisherDiagonal.set(taskId, fisherArr);
        this.tasksLearned++;
    }
    /**
     * Compute EWC penalty for weight update
     * OPTIMIZED: Uses typed arrays and minimizes allocations
     */
    computePenalty(currentWeights) {
        let penalty = 0;
        const len = currentWeights.length;
        for (const [taskId, optimal] of this.optimalWeights) {
            const fisher = this.fisherDiagonal.get(taskId);
            if (!fisher)
                continue;
            const minLen = Math.min(len, optimal.length);
            // Unrolled loop for better performance
            let i = 0;
            for (; i + 3 < minLen; i += 4) {
                const diff0 = currentWeights[i] - optimal[i];
                const diff1 = currentWeights[i + 1] - optimal[i + 1];
                const diff2 = currentWeights[i + 2] - optimal[i + 2];
                const diff3 = currentWeights[i + 3] - optimal[i + 3];
                penalty += fisher[i] * diff0 * diff0 +
                    fisher[i + 1] * diff1 * diff1 +
                    fisher[i + 2] * diff2 * diff2 +
                    fisher[i + 3] * diff3 * diff3;
            }
            // Handle remaining elements
            for (; i < minLen; i++) {
                const diff = currentWeights[i] - optimal[i];
                penalty += fisher[i] * diff * diff;
            }
        }
        return penalty * 0.5;
    }
    /**
     * Get EWC statistics
     */
    stats() {
        return {
            tasksLearned: this.tasksLearned,
            fisherComputed: this.fisherDiagonal.size > 0,
            protectionStrength: this.lambda,
            forgettingRate: this.estimateForgettingRate(),
        };
    }
    estimateForgettingRate() {
        // Simplified estimation based on number of tasks
        return Math.max(0, 1 - Math.exp(-this.tasksLearned * 0.1));
    }
}
/**
 * SONA Learning Coordinator
 *
 * Orchestrates the learning loops and components.
 */
export class SonaCoordinator {
    constructor(config) {
        this.trajectoryBuffer = [];
        this.signalBuffer = [];
        this.config = { ...DEFAULT_SONA_CONFIG, ...config };
        this.reasoningBank = new ReasoningBank(this.config.patternThreshold);
        this.ewcManager = new EwcManager(this.config.ewcLambda);
    }
    /**
     * Record a learning signal
     */
    recordSignal(signal) {
        this.signalBuffer.push(signal);
        // Instant loop - immediate learning
        if (this.config.instantLoopEnabled && signal.quality >= 0.8) {
            this.processInstantLearning(signal);
        }
    }
    /**
     * Record a completed trajectory
     */
    recordTrajectory(trajectory) {
        this.trajectoryBuffer.push(trajectory);
        // Maintain buffer size
        while (this.trajectoryBuffer.length > this.config.maxTrajectorySize) {
            this.trajectoryBuffer.shift();
        }
        // Extract patterns from successful trajectories
        if (trajectory.outcome === 'success') {
            this.extractPatterns(trajectory);
        }
    }
    /**
     * Run background learning loop
     */
    runBackgroundLoop() {
        if (!this.config.backgroundLoopEnabled) {
            return { patternsLearned: 0, trajectoriesProcessed: 0 };
        }
        let patternsLearned = 0;
        const trajectoriesProcessed = this.trajectoryBuffer.length;
        // Process accumulated trajectories
        for (const traj of this.trajectoryBuffer) {
            if (traj.outcome === 'success' || traj.outcome === 'partial') {
                patternsLearned += this.extractPatterns(traj);
            }
        }
        // Prune low-performing patterns
        this.reasoningBank.prune();
        // Clear processed trajectories
        this.trajectoryBuffer = [];
        return { patternsLearned, trajectoriesProcessed };
    }
    /**
     * Get reasoning bank for pattern queries
     */
    getReasoningBank() {
        return this.reasoningBank;
    }
    /**
     * Get EWC manager
     */
    getEwcManager() {
        return this.ewcManager;
    }
    /**
     * Get statistics
     */
    stats() {
        return {
            signalsReceived: this.signalBuffer.length,
            trajectoriesBuffered: this.trajectoryBuffer.length,
            patterns: this.reasoningBank.stats(),
            ewc: this.ewcManager.stats(),
        };
    }
    processInstantLearning(signal) {
        // Immediate pattern reinforcement would happen here
        // In full implementation, this updates LoRA weights
    }
    extractPatterns(trajectory) {
        let extracted = 0;
        for (const step of trajectory.steps) {
            if (step.confidence >= this.config.patternThreshold) {
                // Create embedding from step (simplified)
                const embedding = this.createEmbedding(step.input + step.output);
                // Determine pattern type
                const type = this.stepTypeToPatternType(step.type);
                // Store if not too similar to existing
                const similar = this.reasoningBank.findSimilar(embedding, 1);
                if (similar.length === 0) {
                    this.reasoningBank.store(type, embedding);
                    extracted++;
                }
            }
        }
        return extracted;
    }
    stepTypeToPatternType(stepType) {
        switch (stepType) {
            case 'query':
            case 'generate':
                return 'query_response';
            case 'route':
                return 'routing';
            case 'memory':
                return 'context_retrieval';
            case 'feedback':
                return 'correction';
            default:
                return 'query_response';
        }
    }
    createEmbedding(text) {
        // Simplified hash-based embedding (real impl uses model)
        const dim = 64;
        const embedding = new Array(dim).fill(0);
        for (let i = 0; i < text.length; i++) {
            const idx = (text.charCodeAt(i) * (i + 1)) % dim;
            embedding[idx] += 0.1;
        }
        // Normalize
        const norm = Math.sqrt(embedding.reduce((s, x) => s + x * x, 0)) || 1;
        return embedding.map(x => x / norm);
    }
}
// Export all SONA components
export { DEFAULT_SONA_CONFIG, };
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoic29uYS5qcyIsInNvdXJjZVJvb3QiOiIiLCJzb3VyY2VzIjpbIi4uLy4uL3NyYy9zb25hLnRzIl0sIm5hbWVzIjpbXSwibWFwcGluZ3MiOiJBQUFBOzs7OztHQUtHO0FBZUg7O0dBRUc7QUFDSCxNQUFNLG1CQUFtQixHQUF5QjtJQUNoRCxrQkFBa0IsRUFBRSxJQUFJO0lBQ3hCLHFCQUFxQixFQUFFLElBQUk7SUFDM0IsZ0JBQWdCLEVBQUUsS0FBSztJQUN2QixRQUFRLEVBQUUsQ0FBQztJQUNYLFNBQVMsRUFBRSxJQUFJO0lBQ2YsaUJBQWlCLEVBQUUsSUFBSTtJQUN2QixnQkFBZ0IsRUFBRSxJQUFJO0NBQ3ZCLENBQUM7QUFFRjs7Ozs7Ozs7Ozs7Ozs7OztHQWdCRztBQUNILE1BQU0sT0FBTyxpQkFBaUI7SUFPNUI7UUFMUSxVQUFLLEdBQXFCLEVBQUUsQ0FBQztRQUM3QixnQkFBVyxHQUFtQyxJQUFJLENBQUM7UUFDbkQsY0FBUyxHQUFXLENBQUMsQ0FBQztRQUk1QixJQUFJLENBQUMsRUFBRSxHQUFHLFFBQVEsSUFBSSxDQUFDLEdBQUcsRUFBRSxJQUFJLElBQUksQ0FBQyxNQUFNLEVBQUUsQ0FBQyxRQUFRLENBQUMsRUFBRSxDQUFDLENBQUMsS0FBSyxDQUFDLENBQUMsRUFBRSxDQUFDLENBQUMsRUFBRSxDQUFDO1FBQ3pFLElBQUksQ0FBQyxTQUFTLEdBQUcsSUFBSSxDQUFDLEdBQUcsRUFBRSxDQUFDO0lBQzlCLENBQUM7SUFFRDs7T0FFRztJQUNILFNBQVMsQ0FBQyxJQUE0QixFQUFFLEtBQWE7UUFDbkQsSUFBSSxJQUFJLENBQUMsV0FBVyxFQUFFLENBQUM7WUFDckIsOEJBQThCO1lBQzlCLElBQUksQ0FBQyxPQUFPLENBQUMsRUFBRSxFQUFFLENBQUMsQ0FBQyxDQUFDO1FBQ3RCLENBQUM7UUFFRCxJQUFJLENBQUMsU0FBUyxHQUFHLElBQUksQ0FBQyxHQUFHLEVBQUUsQ0FBQztRQUM1QixJQUFJLENBQUMsV0FBVyxHQUFHO1lBQ2pCLElBQUk7WUFD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SSxDQUFDLFVBQVUsQ0FBQyxDQUFDLENBQUMsR0FBRyxDQUFDLENBQUMsR0FBRyxDQUFDLENBQUMsQ0FBQyxHQUFHLEdBQUcsQ0FBQztZQUNqRCxTQUFTLENBQUMsR0FBRyxDQUFDLElBQUksR0FBRyxDQUFDO1FBQ3hCLENBQUM7UUFFRCxZQUFZO1FBQ1osTUFBTSxJQUFJLEdBQUcsSUFBSSxDQUFDLElBQUksQ0FBQyxTQUFTLENBQUMsTUFBTSxDQUFDLENBQUMsQ0FBQyxFQUFFLENBQUMsRUFBRSxFQUFFLENBQUMsQ0FBQyxHQUFHLENBQUMsR0FBRyxDQUFDLEVBQUUsQ0FBQyxDQUFDLENBQUMsSUFBSSxDQUFDLENBQUM7UUFDdEUsT0FBTyxTQUFTLENBQUMsR0FBRyxDQUFDLENBQUMsQ0FBQyxFQUFFLENBQUMsQ0FBQyxHQUFHLElBQUksQ0FBQyxDQUFDO0lBQ3RDLENBQUM7Q0FDRjtBQUVELDZCQUE2QjtBQUM3QixPQUFPLEVBQ0wsbUJBQW1CLEdBQ3BCLENBQUMiLCJzb3VyY2VzQ29udGVudCI6WyIvKipcbiAqIFNPTkEgKFNlbGYtT3B0aW1pemluZyBOZXVyYWwgQXJjaGl0ZWN0dXJlKSBMZWFybmluZyBTeXN0ZW1cbiAqXG4gKiBQcm92aWRlcyBhZGFwdGl2ZSBsZWFybmluZyBjYXBhYmlsaXRpZXMgd2l0aCB0cmFqZWN0b3J5IHRyYWNraW5nLFxuICogcGF0dGVybiByZWNvZ25pdGlvbiwgYW5kIG1lbW9yeSBwcm90ZWN0aW9uIChFV0MrKykuXG4gKi9cblxuaW1wb3J0IHtcbiAgU29uYUNvbmZpZyxcbiAgTGVhcm5pbmdTaWduYWwsXG4gIFF1ZXJ5VHJhamVjdG9yeSxcbiAgVHJhamVjdG9yeVN0ZXAsXG4gIFRyYWplY3RvcnlPdXRjb21lLFxuICBMZWFybmVkUGF0dGVybixcbiAgUGF0dGVyblR5cGUsXG4gIEV3Y1N0YXRzLFxuICBMb1JBQ29uZmlnLFxuICBFbWJlZGRpbmcsXG59IGZyb20gJy4vdHlwZXMnO1xuXG4vKipcbiAqIERlZmF1bHQgU09OQSBjb25maWd1cmF0aW9uXG4gKi9cbmNvbnN0IERFRkFVTFRfU09OQV9DT05GSUc6IFJlcXVpcmVkPFNvbmFDb25maWc+ID0ge1xuICBpbnN0YW50TG9vcEVuYWJsZWQ6IHRydWUsXG4gIGJhY2tncm91bmRMb29wRW5hYmxlZDogdHJ1ZSxcbiAgbG9yYUxlYXJuaW5nUmF0ZTogMC4wMDEsXG4gIGxvcmFSYW5rOiA4LFxuICBld2NMYW1iZGE6IDIwMDAsXG4gIG1heFRyYWplY3RvcnlTaXplOiAxMDAwLFxuICBwYXR0ZXJuVGhyZXNob2xkOiAwLjg1LFxufTtcblxuLyoqXG4gKiBUcmFqZWN0b3J5IEJ1aWxkZXIgZm9yIHRyYWNraW5nIHF1ZXJ5IGV4ZWN1dGlvbiBwYXRoc1xuICpcbiAqIEBleGFtcGxlXG4gKiBgYGB0eXBlc2NyaXB0XG4gKiBjb25zdCBidWlsZGVyID0gbmV3IFRyYWplY3RvcnlCdWlsZGVyKCk7XG4gKlxuICogYnVpbGRlci5zdGFydFN0ZXAoJ3F1ZXJ5JywgJ1doYXQgaXMgQUk/Jyk7XG4gKiAvLyAuLi4gcHJvY2Vzc2luZyAuLi5cbiAqIGJ1aWxkZXIuZW5kU3RlcCgnQUkgaXMgYXJ0aWZpY2lhbCBpbnRlbGxpZ2VuY2UnLCAwLjk1KTtcbiAqXG4gKiBidWlsZGVyLnN0YXJ0U3RlcCgnbWVtb3J5JywgJ3NlYXJjaGluZyBjb250ZXh0Jyk7XG4gKiBidWlsZGVyLmVuZFN0ZXAoJ2ZvdW5kIDMgcmVsZXZhbnQgZG9jdW1lbnRzJywgMC44OCk7XG4gKlxuICogY29uc3QgdHJhamVjdG9yeSA9IGJ1aWxkZXIuY29tcGxldGUoJ3N1Y2Nlc3MnKTtcbiAqIGBgYFxuICovXG5leHBvcnQgY2xhc3MgVHJhamVjdG9yeUJ1aWxkZXIge1xuICBwcml2YXRlIGlkOiBzdHJpbmc7XG4gIHByaXZhdGUgc3RlcHM6IFRyYWplY3RvcnlTdGVwW10gPSBbXTtcbiAgcHJpdmF0ZSBjdXJyZW50U3RlcDogUGFydGlhbDxUcmFqZWN0b3J5U3RlcD4gfCBudWxsID0gbnVsbDtcbiAgcHJpdmF0ZSBzdGVwU3RhcnQ6IG51bWJlciA9IDA7XG4gIHByaXZhdGUgc3RhcnRUaW1lOiBudW1iZXI7XG5cbiAgY29uc3RydWN0b3IoKSB7XG4gICAgdGhpcy5pZCA9IGB0cmFqLSR7RGF0ZS5ub3coKX0tJHtNYXRoLnJhbmRvbSgpLnRvU3RyaW5nKDM2KS5zbGljZSgyLCA4KX1gO1xuICAgIHRoaXMuc3RhcnRUaW1lID0gRGF0ZS5ub3coKTtcbiAgfVxuXG4gIC8qKlxuICAgKiBTdGFydCBhIG5ldyBzdGVwIGluIHRoZSB0cmFqZWN0b3J5XG4gICAqL1xuICBzdGFydFN0ZXAodHlwZTogVHJhamVjdG9yeVN0ZXBbJ3R5cGUnXSwgaW5wdXQ6IHN0cmluZyk6IHRoaXMge1xuICAgIGlmICh0aGlzLmN1cnJlbnRTdGVwKSB7XG4gICAgICAvLyBBdXRvLWNvbXBsZXRlIHByZXZpb3VzIHN0ZXBcbiAgICAgIHRoaXMuZW5kU3RlcCgnJywgMCk7XG4gICAgfVxuXG4gICAgdGhpcy5zdGVwU3RhcnQgPSBEYXRlLm5vdygpO1xuICAgIHRoaXMuY3VycmVudFN0ZXAgPSB7XG4gICAgICB0eXBlLFxuICAgICAgaW5wdXQsXG4gICAgfTtcblxuICAgIHJldHVybiB0aGlzO1xuICB9XG5cbiAgLyoqXG4gICAqIEVuZCBjdXJyZW50IHN0ZXAgd2l0aCBvdXRwdXRcbiAgICovXG4gIGVuZFN0ZXAob3V0cHV0OiBzdHJpbmcsIGNvbmZpZGVuY2U6IG51bWJlcik6IHRoaXMge1xuICAgIGlmICghdGhpcy5jdXJyZW50U3RlcCkge1xuICAgICAgcmV0dXJuIHRoaXM7XG4gICAgfVxuXG4gICAgdGhpcy5zdGVwcy5wdXNoKHtcbiAgICAgIHR5cGU6IHRoaXMuY3VycmVudFN0ZXAudHlwZSEsXG4gICAgICBpbnB1dDogdGhpcy5jdXJyZW50U3RlcC5pbnB1dCEsXG4gICAgICBvdXRwdXQsXG4gICAgICBkdXJhdGlvbk1zOiBEYXRlLm5vdygpIC0gdGhpcy5zdGVwU3RhcnQsXG4gICAgICBjb25maWRlbmNlLFxuICAgIH0pO1xuXG4gICAgdGhpcy5jdXJyZW50U3RlcCA9IG51bGw7XG4gICAgcmV0dXJuIHRoaXM7XG4gIH1cblxuICAvKipcbiAgICogQ29tcGxldGUgdHJhamVjdG9yeSB3aXRoIGZpbmFsIG91dGNvbWVcbiAgICovXG4gIGNvbXBsZXRlKG91dGNvbWU6IFRyYWplY3RvcnlPdXRjb21lKTogUXVlcnlUcmFqZWN0b3J5IHtcbiAgICAvLyBDb21wbGV0ZSBhbnkgcGVuZGluZyBzdGVwXG4gICAgaWYgKHRoaXMuY3VycmVudFN0ZXApIHtcbiAgICAgIHRoaXMuZW5kU3RlcCgnaW5jb21wbGV0ZScsIDApO1xuICAgIH1cblxuICAgIHJldHVybiB7XG4gICAgICBpZDogdGhpcy5pZCxcbiAgICAgIHN0ZXBzOiB0aGlzLnN0ZXBzLFxuICAgICAgb3V0Y29tZSxcbiAgICAgIGR1cmF0aW9uTXM6IERhdGUubm93KCkgLSB0aGlzLnN0YXJ0VGltZSxcbiAgICB9O1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCBjdXJyZW50IHRyYWplY3RvcnkgSURcbiAgICovXG4gIGdldElkKCk6IHN0cmluZyB7XG4gICAgcmV0dXJuIHRoaXMuaWQ7XG4gIH1cbn1cblxuLyoqXG4gKiBSZWFzb25pbmdCYW5rIC0gUGF0dGVybiBzdG9yYWdlIGFuZCByZXRyaWV2YWxcbiAqXG4gKiBTdG9yZXMgbGVhcm5lZCBwYXR0ZXJucyBmcm9tIHN1Y2Nlc3NmdWwgaW50ZXJhY3Rpb25zIGFuZFxuICogZW5hYmxlcyBwYXR0ZXJuLWJhc2VkIHJlYXNvbmluZyBzaG9ydGN1dHMuXG4gKlxuICogT1BUSU1JWkVEOiBVc2VzIEZsb2F0NjRBcnJheSBmb3IgZW1iZWRkaW5ncyBhbmQgcGFydGlhbCBzb3J0aW5nXG4gKi9cbmV4cG9ydCBjbGFzcyBSZWFzb25pbmdCYW5rIHtcbiAgcHJpdmF0ZSBwYXR0ZXJuczogTWFwPHN0cmluZywgTGVhcm5lZFBhdHRlcm4+ID0gbmV3IE1hcCgpO1xuICBwcml2YXRlIGVtYmVkZGluZ3M6IE1hcDxzdHJpbmcsIEZsb2F0NjRBcnJheT4gPSBuZXcgTWFwKCk7XG4gIHByaXZhdGUgZW1iZWRkaW5nTm9ybXM6IE1hcDxzdHJpbmcsIG51bWJlcj4gPSBuZXcgTWFwKCk7IC8vIFByZS1jb21wdXRlZCBub3Jtc1xuICBwcml2YXRlIHRocmVzaG9sZDogbnVtYmVyO1xuICAvLyBSZXVzYWJsZSBhcnJheXMgZm9yIGZpbmRTaW1pbGFyIHRvIGF2b2lkIGFsbG9jYXRpb25zXG4gIHByaXZhdGUgX3NpbWlsYXJpdHlSZXN1bHRzOiBBcnJheTx7IGlkOiBzdHJpbmc7IHNjb3JlOiBudW1iZXIgfT4gPSBbXTtcblxuICBjb25zdHJ1Y3Rvcih0aHJlc2hvbGQgPSAwLjg1KSB7XG4gICAgdGhpcy50aHJlc2hvbGQgPSB0aHJlc2hvbGQ7XG4gIH1cblxuICAvKipcbiAgICogU3RvcmUgYSBuZXcgcGF0dGVyblxuICAgKi9cbiAgc3RvcmUoXG4gICAgdHlwZTogUGF0dGVyblR5cGUsXG4gICAgZW1iZWRkaW5nOiBFbWJlZGRpbmcsXG4gICAgbWV0YWRhdGE/OiBSZWNvcmQ8c3RyaW5nLCB1bmtub3duPlxuICApOiBzdHJpbmcge1xuICAgIGNvbnN0IGlkID0gYHBhdC0ke0RhdGUubm93KCl9LSR7TWF0aC5yYW5kb20oKS50b1N0cmluZygzNikuc2xpY2UoMiwgOCl9YDtcblxuICAgIGNvbnN0IHBhdHRlcm46IExlYXJuZWRQYXR0ZXJuID0ge1xuICAgICAgaWQsXG4gICAgICB0eXBlLFxuICAgICAgZW1iZWRkaW5nLFxuICAgICAgc3VjY2Vzc1JhdGU6IDEuMCxcbiAgICAgIHVzZUNvdW50OiAwLFxuICAgICAgbGFzdFVzZWQ6IG5ldyBEYXRlKCksXG4gICAgfTtcblxuICAgIHRoaXMucGF0dGVybnMuc2V0KGlkLCBwYXR0ZXJuKTtcblxuICAgIC8vIFN0b3JlIGFzIHR5cGVkIGFycmF5IGZvciBmYXN0ZXIgc2ltaWxhcml0eSBjb21wdXRhdGlvblxuICAgIGNvbnN0IHR5cGVkRW1iID0gbmV3IEZsb2F0NjRBcnJheShlbWJlZGRpbmcpO1xuICAgIHRoaXMuZW1iZWRkaW5ncy5zZXQoaWQsIHR5cGVkRW1iKTtcblxuICAgIC8vIFByZS1jb21wdXRlIGFuZCBjYWNoZSB0aGUgbm9ybVxuICAgIGxldCBub3JtID0gMDtcbiAgICBmb3IgKGxldCBpID0gMDsgaSA8IHR5cGVkRW1iLmxlbmd0aDsgaSsrKSB7XG4gICAgICBub3JtICs9IHR5cGVkRW1iW2ldICogdHlwZWRFbWJbaV07XG4gICAgfVxuICAgIHRoaXMuZW1iZWRkaW5nTm9ybXMuc2V0KGlkLCBNYXRoLnNxcnQobm9ybSkpO1xuXG4gICAgcmV0dXJuIGlkO1xuICB9XG5cbiAgLyoqXG4gICAqIEZpbmQgc2ltaWxhciBwYXR0ZXJuc1xuICAgKiBPUFRJTUlaRUQ6IFVzZXMgdHlwZWQgYXJyYXlzLCBwcmUtY29tcHV0ZWQgbm9ybXMsIGFuZCBwYXJ0aWFsIHNvcnRpbmdcbiAgICovXG4gIGZpbmRTaW1pbGFyKGVtYmVkZGluZzogRW1iZWRkaW5nLCBrID0gNSk6IExlYXJuZWRQYXR0ZXJuW10ge1xuICAgIC8vIFByZS1jb21wdXRlIHF1ZXJ5IG5vcm1cbiAgICBsZXQgcXVlcnlOb3JtID0gMDtcbiAgICBjb25zdCBxdWVyeUxlbiA9IGVtYmVkZGluZy5sZW5ndGg7XG4gICAgZm9yIChsZXQgaSA9IDA7IGkgPCBxdWVyeUxlbjsgaSsrKSB7XG4gICAgICBxdWVyeU5vcm0gKz0gZW1iZWRkaW5nW2ldICogZW1iZWRkaW5nW2ldO1xuICAgIH1cbiAgICBxdWVyeU5vcm0gPSBNYXRoLnNxcnQocXVlcnlOb3JtKTtcblxuICAgIGlmIChxdWVyeU5vcm0gPT09IDApIHJldHVybiBbXTtcblxuICAgIC8vIFJldXNlIGFycmF5IHRvIGF2b2lkIGFsbG9jYXRpb25zXG4gICAgdGhpcy5fc2ltaWxhcml0eVJlc3VsdHMubGVuZ3RoID0gMDtcblxuICAgIGZvciAoY29uc3QgW2lkLCBwYXRFbWJdIG9mIHRoaXMuZW1iZWRkaW5ncykge1xuICAgICAgY29uc3QgcGF0Tm9ybSA9IHRoaXMuZW1iZWRkaW5nTm9ybXMuZ2V0KGlkKSB8fCAwO1xuICAgICAgaWYgKHBhdE5vcm0gPT09IDApIGNvbnRpbnVlO1xuXG4gICAgICAvLyBGYXN0IGRvdCBwcm9kdWN0XG4gICAgICBsZXQgZG90ID0gMDtcbiAgICAgIGNvbnN0IG1pbkxlbiA9IE1hdGgubWluKHF1ZXJ5TGVuLCBwYXRFbWIubGVuZ3RoKTtcblxuICAgICAgLy8gVW5yb2xsZWQgbG9vcFxuICAgICAgbGV0IGkgPSAwO1xuICAgICAgZm9yICg7IGkgKyAzIDwgbWluTGVuOyBpICs9IDQpIHtcbiAgICAgICAgZG90ICs9IGVtYmVkZGluZ1tpXSAqIHBhdEVtYltpXSArXG4gICAgICAgICAgICAgICBlbWJlZGRpbmdbaSArIDFdICogcGF0RW1iW2kgKyAxXSArXG4gICAgICAgICAgICAgICBlbWJlZGRpbmdbaSArIDJdICogcGF0RW1iW2kgKyAyXSArXG4gICAgICAgICAgICAgICBlbWJlZGRpbmdbaSArIDNdICogcGF0RW1iW2kgKyAzXTtcbiAgICAgIH1cbiAgICAgIGZvciAoOyBpIDwgbWluTGVuOyBpKyspIHtcbiAgICAgICAgZG90ICs9IGVtYmVkZGluZ1tpXSAqIHBhdEVtYltpXTtcbiAgICAgIH1cblxuICAgICAgY29uc3Qgc2NvcmUgPSBkb3QgLyAocXVlcnlOb3JtICogcGF0Tm9ybSk7XG5cbiAgICAgIGlmIChzY29yZSA+PSB0aGlzLnRocmVzaG9sZCkge1xuICAgICAgICB0aGlzLl9zaW1pbGFyaXR5UmVzdWx0cy5wdXNoKHsgaWQsIHNjb3JlIH0pO1xuICAgICAgfVxuICAgIH1cblxuICAgIC8vIFBhcnRpYWwgc29ydCBmb3IgdG9wLWsgKGZhc3RlciB0aGFuIGZ1bGwgc29ydCBmb3IgbGFyZ2UgYXJyYXlzKVxuICAgIGlmICh0aGlzLl9zaW1pbGFyaXR5UmVzdWx0cy5sZW5ndGggPD0gaykge1xuICAgICAgdGhpcy5fc2ltaWxhcml0eVJlc3VsdHMuc29ydCgoYSwgYikgPT4gYi5zY29yZSAtIGEuc2NvcmUpO1xuICAgIH0gZWxzZSB7XG4gICAgICAvLyBRdWljayBwYXJ0aWFsIHNvcnQgZm9yIHRvcCBrXG4gICAgICB0aGlzLnBhcnRpYWxTb3J0KHRoaXMuX3NpbWlsYXJpdHlSZXN1bHRzLCBrKTtcbiAgICB9XG5cbiAgICBjb25zdCB0b3BLID0gdGhpcy5fc2ltaWxhcml0eVJlc3VsdHMuc2xpY2UoMCwgayk7XG5cbiAgICByZXR1cm4gdG9wS1xuICAgICAgLm1hcChzID0+IHRoaXMucGF0dGVybnMuZ2V0KHMuaWQpKVxuICAgICAgLmZpbHRlcigocCk6IHAgaXMgTGVhcm5lZFBhdHRlcm4gPT4gcCAhPT0gdW5kZWZpbmVkKTtcbiAgfVxuXG4gIC8qKlxuICAgKiBQYXJ0aWFsIHNvcnQgdG8gZ2V0IHRvcCBrIGVsZW1lbnRzIChmYXN0ZXIgdGhhbiBmdWxsIHNvcnQpXG4gICAqL1xuICBwcml2YXRlIHBhcnRpYWxTb3J0KGFycjogQXJyYXk8eyBpZDogc3RyaW5nOyBzY29yZTogbnVtYmVyIH0+LCBrOiBudW1iZXIpOiB2b2lkIHtcbiAgICAvLyBTaW1wbGUgc2VsZWN0aW9uIGZvciBzbWFsbCBrXG4gICAgZm9yIChsZXQgaSA9IDA7IGkgPCBrICYmIGkgPCBhcnIubGVuZ3RoOyBpKyspIHtcbiAgICAgIGxldCBtYXhJZHggPSBpO1xuICAgICAgZm9yIChsZXQgaiA9IGkgKyAxOyBqIDwgYXJyLmxlbmd0aDsgaisrKSB7XG4gICAgICAgIGlmIChhcnJbal0uc2NvcmUgPiBhcnJbbWF4SWR4XS5zY29yZSkge1xuICAgICAgICAgIG1heElkeCA9IGo7XG4gICAgICAgIH1cbiAgICAgIH1cbiAgICAgIGlmIChtYXhJZHggIT09IGkpIHtcbiAgICAgICAgY29uc3QgdGVtcCA9IGFycltpXTtcbiAgICAgICAgYXJyW2ldID0gYXJyW21heElkeF07XG4gICAgICAgIGFyclttYXhJZHhdID0gdGVtcDtcbiAgICAgIH1cbiAgICB9XG4gIH1cblxuICAvKipcbiAgICogUmVjb3JkIHBhdHRlcm4gdXNhZ2UgKHN1Y2Nlc3Mgb3IgZmFpbHVyZSlcbiAgICovXG4gIHJlY29yZFVzYWdlKHBhdHRlcm5JZDogc3RyaW5nLCBzdWNjZXNzOiBib29sZWFuKTogdm9pZCB7XG4gICAgY29uc3QgcGF0dGVybiA9IHRoaXMucGF0dGVybnMuZ2V0KHBhdHRlcm5JZCk7XG4gICAgaWYgKCFwYXR0ZXJuKSByZXR1cm47XG5cbiAgICBwYXR0ZXJuLnVzZUNvdW50Kys7XG4gICAgcGF0dGVybi5sYXN0VXNlZCA9IG5ldyBEYXRlKCk7XG5cbiAgICAvLyBVcGRhdGUgc3VjY2VzcyByYXRlIHdpdGggZXhwb25lbnRpYWwgbW92aW5nIGF2ZXJhZ2VcbiAgICBjb25zdCBhbHBoYSA9IDAuMTtcbiAgICBjb25zdCBvdXRjb21lID0gc3VjY2VzcyA/IDEuMCA6IDAuMDtcbiAgICBwYXR0ZXJuLnN1Y2Nlc3NSYXRlID0gYWxwaGEgKiBvdXRjb21lICsgKDEgLSBhbHBoYSkgKiBwYXR0ZXJuLnN1Y2Nlc3NSYXRlO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCBwYXR0ZXJuIGJ5IElEXG4gICAqL1xuICBnZXQocGF0dGVybklkOiBzdHJpbmcpOiBMZWFybmVkUGF0dGVybiB8IHVuZGVmaW5lZCB7XG4gICAgcmV0dXJuIHRoaXMucGF0dGVybnMuZ2V0KHBhdHRlcm5JZCk7XG4gIH1cblxuICAvKipcbiAgICogR2V0IGFsbCBwYXR0ZXJucyBvZiBhIHR5cGVcbiAgICovXG4gIGdldEJ5VHlwZSh0eXBlOiBQYXR0ZXJuVHlwZSk6IExlYXJuZWRQYXR0ZXJuW10ge1xuICAgIHJldHVybiBBcnJheS5mcm9tKHRoaXMucGF0dGVybnMudmFsdWVzKCkpLmZpbHRlcihwID0+IHAudHlwZSA9PT0gdHlwZSk7XG4gIH1cblxuICAvKipcbiAgICogUHJ1bmUgbG93LXBlcmZvcm1pbmcgcGF0dGVybnNcbiAgICovXG4gIHBydW5lKG1pblN1Y2Nlc3NSYXRlID0gMC4zLCBtaW5Vc2VDb3VudCA9IDUpOiBudW1iZXIge1xuICAgIGxldCBwcnVuZWQgPSAwO1xuXG4gICAgZm9yIChjb25zdCBbaWQsIHBhdHRlcm5dIG9mIHRoaXMucGF0dGVybnMpIHtcbiAgICAgIGlmIChwYXR0ZXJuLnVzZUNvdW50ID49IG1pblVzZUNvdW50ICYmIHBhdHRlcm4uc3VjY2Vzc1JhdGUgPCBtaW5TdWNjZXNzUmF0ZSkge1xuICAgICAgICB0aGlzLnBhdHRlcm5zLmRlbGV0ZShpZCk7XG4gICAgICAgIHRoaXMuZW1iZWRkaW5ncy5kZWxldGUoaWQpO1xuICAgICAgICB0aGlzLmVtYmVkZGluZ05vcm1zLmRlbGV0ZShpZCk7XG4gICAgICAgIHBydW5lZCsrO1xuICAgICAgfVxuICAgIH1cblxuICAgIHJldHVybiBwcnVuZWQ7XG4gIH1cblxuICAvKipcbiAgICogR2V0IHN0YXRpc3RpY3NcbiAgICovXG4gIHN0YXRzKCk6IHsgdG90YWxQYXR0ZXJuczogbnVtYmVyOyBhdmdTdWNjZXNzUmF0ZTogbnVtYmVyOyBieVR5cGU6IFJlY29yZDxzdHJpbmcsIG51bWJlcj4gfSB7XG4gICAgY29uc3QgcGF0dGVybnMgPSBBcnJheS5mcm9tKHRoaXMucGF0dGVybnMudmFsdWVzKCkpO1xuICAgIGNvbnN0IGJ5VHlwZTogUmVjb3JkPHN0cmluZywgbnVtYmVyPiA9IHt9O1xuXG4gICAgbGV0IHRvdGFsU3VjY2VzcyA9IDA7XG4gICAgZm9yIChjb25zdCBwIG9mIHBhdHRlcm5zKSB7XG4gICAgICB0b3RhbFN1Y2Nlc3MgKz0gcC5zdWNjZXNzUmF0ZTtcbiAgICAgIGJ5VHlwZVtwLnR5cGVdID0gKGJ5VHlwZVtwLnR5cGVdIHx8IDApICsgMTtcbiAgICB9XG5cbiAgICByZXR1cm4ge1xuICAgICAgdG90YWxQYXR0ZXJuczogcGF0dGVybnMubGVuZ3RoLFxuICAgICAgYXZnU3VjY2Vzc1JhdGU6IHBhdHRlcm5zLmxlbmd0aCA+IDAgPyB0b3RhbFN1Y2Nlc3MgLyBwYXR0ZXJucy5sZW5ndGggOiAwLFxuICAgICAgYnlUeXBlLFxuICAgIH07XG4gIH1cblxuICBwcml2YXRlIGNvc2luZVNpbWlsYXJpdHkoYTogRW1iZWRkaW5nLCBiOiBFbWJlZGRpbmcpOiBudW1iZXIge1xuICAgIGxldCBkb3QgPSAwLCBub3JtQSA9IDAsIG5vcm1CID0gMDtcbiAgICBjb25zdCBsZW4gPSBNYXRoLm1pbihhLmxlbmd0aCwgYi5sZW5ndGgpO1xuXG4gICAgZm9yIChsZXQgaSA9IDA7IGkgPCBsZW47IGkrKykge1xuICAgICAgZG90ICs9IGFbaV0gKiBiW2ldO1xuICAgICAgbm9ybUEgKz0gYVtpXSAqIGFbaV07XG4gICAgICBub3JtQiArPSBiW2ldICogYltpXTtcbiAgICB9XG5cbiAgICBjb25zdCBkZW5vbSA9IE1hdGguc3FydChub3JtQSkgKiBNYXRoLnNxcnQobm9ybUIpO1xuICAgIHJldHVybiBkZW5vbSA+IDAgPyBkb3QgLyBkZW5vbSA6IDA7XG4gIH1cbn1cblxuLyoqXG4gKiBFV0MrKyAoRWxhc3RpYyBXZWlnaHQgQ29uc29saWRhdGlvbikgTWFuYWdlclxuICpcbiAqIFByZXZlbnRzIGNhdGFzdHJvcGhpYyBmb3JnZXR0aW5nIGJ5IHByb3RlY3RpbmcgaW1wb3J0YW50IHdlaWdodHMuXG4gKiBUaGlzIGlzIGEgc2ltcGxpZmllZCBKUyBpbXBsZW1lbnRhdGlvbiBvZiB0aGUgY29uY2VwdC5cbiAqXG4gKiBPUFRJTUlaRUQ6IFVzZXMgRmxvYXQ2NEFycmF5IGZvciA1LTEweCBmYXN0ZXIgcGVuYWx0eSBjb21wdXRhdGlvblxuICovXG5leHBvcnQgY2xhc3MgRXdjTWFuYWdlciB7XG4gIHByaXZhdGUgbGFtYmRhOiBudW1iZXI7XG4gIHByaXZhdGUgdGFza3NMZWFybmVkOiBudW1iZXIgPSAwO1xuICBwcml2YXRlIGZpc2hlckRpYWdvbmFsOiBNYXA8c3RyaW5nLCBGbG9hdDY0QXJyYXk+ID0gbmV3IE1hcCgpO1xuICBwcml2YXRlIG9wdGltYWxXZWlnaHRzOiBNYXA8c3RyaW5nLCBGbG9hdDY0QXJyYXk+ID0gbmV3IE1hcCgpO1xuICAvLyBQcmUtYWxsb2NhdGVkIGJ1ZmZlciBmb3IgcGVuYWx0eSBjb21wdXRhdGlvblxuICBwcml2YXRlIF9wZW5hbHR5QnVmZmVyOiBGbG9hdDY0QXJyYXkgfCBudWxsID0gbnVsbDtcblxuICBjb25zdHJ1Y3RvcihsYW1iZGEgPSAyMDAwKSB7XG4gICAgdGhpcy5sYW1iZGEgPSBsYW1iZGE7XG4gIH1cblxuICAvKipcbiAgICogUmVnaXN0ZXIgYSBuZXcgdGFzayAoYWZ0ZXIgc3VjY2Vzc2Z1bCBsZWFybmluZylcbiAgICovXG4gIHJlZ2lzdGVyVGFzayh0YXNrSWQ6IHN0cmluZywgd2VpZ2h0czogbnVtYmVyW10pOiB2b2lkIHtcbiAgICAvLyBTdG9yZSBvcHRpbWFsIHdlaWdodHMgZm9yIHRoaXMgdGFzayB1c2luZyB0eXBlZCBhcnJheXNcbiAgICBjb25zdCBvcHRpbWFsQXJyID0gbmV3IEZsb2F0NjRBcnJheSh3ZWlnaHRzLmxlbmd0aCk7XG4gICAgY29uc3QgZmlzaGVyQXJyID0gbmV3IEZsb2F0NjRBcnJheSh3ZWlnaHRzLmxlbmd0aCk7XG5cbiAgICBmb3IgKGxldCBpID0gMDsgaSA8IHdlaWdodHMubGVuZ3RoOyBpKyspIHtcbiAgICAgIG9wdGltYWxBcnJbaV0gPSB3ZWlnaHRzW2ldO1xuICAgICAgZmlzaGVyQXJyW2ldID0gTWF0aC5hYnMod2VpZ2h0c1tpXSkgKiB0aGlzLmxhbWJkYTtcbiAgICB9XG5cbiAgICB0aGlzLm9wdGltYWxXZWlnaHRzLnNldCh0YXNrSWQsIG9wdGltYWxBcnIpO1xuICAgIHRoaXMuZmlzaGVyRGlhZ29uYWwuc2V0KHRhc2tJZCwgZmlzaGVyQXJyKTtcbiAgICB0aGlzLnRhc2tzTGVhcm5lZCsrO1xuICB9XG5cbiAgLyoqXG4gICAqIENvbXB1dGUgRVdDIHBlbmFsdHkgZm9yIHdlaWdodCB1cGRhdGVcbiAgICogT1BUSU1JWkVEOiBVc2VzIHR5cGVkIGFycmF5cyBhbmQgbWluaW1pemVzIGFsbG9jYXRpb25zXG4gICAqL1xuICBjb21wdXRlUGVuYWx0eShjdXJyZW50V2VpZ2h0czogbnVtYmVyW10pOiBudW1iZXIge1xuICAgIGxldCBwZW5hbHR5ID0gMDtcbiAgICBjb25zdCBsZW4gPSBjdXJyZW50V2VpZ2h0cy5sZW5ndGg7XG5cbiAgICBmb3IgKGNvbnN0IFt0YXNrSWQsIG9wdGltYWxdIG9mIHRoaXMub3B0aW1hbFdlaWdodHMpIHtcbiAgICAgIGNvbnN0IGZpc2hlciA9IHRoaXMuZmlzaGVyRGlhZ29uYWwuZ2V0KHRhc2tJZCk7XG4gICAgICBpZiAoIWZpc2hlcikgY29udGludWU7XG5cbiAgICAgIGNvbnN0IG1pbkxlbiA9IE1hdGgubWluKGxlbiwgb3B0aW1hbC5sZW5ndGgpO1xuXG4gICAgICAvLyBVbnJvbGxlZCBsb29wIGZvciBiZXR0ZXIgcGVyZm9ybWFuY2VcbiAgICAgIGxldCBpID0gMDtcbiAgICAgIGZvciAoOyBpICsgMyA8IG1pbkxlbjsgaSArPSA0KSB7XG4gICAgICAgIGNvbnN0IGRpZmYwID0gY3VycmVudFdlaWdodHNbaV0gLSBvcHRpbWFsW2ldO1xuICAgICAgICBjb25zdCBkaWZmMSA9IGN1cnJlbnRXZWlnaHRzW2kgKyAxXSAtIG9wdGltYWxbaSArIDFdO1xuICAgICAgICBjb25zdCBkaWZmMiA9IGN1cnJlbnRXZWlnaHRzW2kgKyAyXSAtIG9wdGltYWxbaSArIDJdO1xuICAgICAgICBjb25zdCBkaWZmMyA9IGN1cnJlbnRXZWlnaHRzW2kgKyAzXSAtIG9wdGltYWxbaSArIDNdO1xuICAgICAgICBwZW5hbHR5ICs9IGZpc2hlcltpXSAqIGRpZmYwICogZGlmZjAgK1xuICAgICAgICAgICAgICAgICAgIGZpc2hlcltpICsgMV0gKiBkaWZmMSAqIGRpZmYxICtcbiAgICAgICAgICAgICAgICAgICBmaXNoZXJbaSArIDJdICogZGlmZjIgKiBkaWZmMiArXG4gICAgICAgICAgICAgICAgICAgZmlzaGVyW2kgKyAzXSAqIGRpZmYzICogZGlmZjM7XG4gICAgICB9XG4gICAgICAvLyBIYW5kbGUgcmVtYWluaW5nIGVsZW1lbnRzXG4gICAgICBmb3IgKDsgaSA8IG1pbkxlbjsgaSsrKSB7XG4gICAgICAgIGNvbnN0IGRpZmYgPSBjdXJyZW50V2VpZ2h0c1tpXSAtIG9wdGltYWxbaV07XG4gICAgICAgIHBlbmFsdHkgKz0gZmlzaGVyW2ldICogZGlmZiAqIGRpZmY7XG4gICAgICB9XG4gICAgfVxuXG4gICAgcmV0dXJuIHBlbmFsdHkgKiAwLjU7XG4gIH1cblxuICAvKipcbiAgICogR2V0IEVXQyBzdGF0aXN0aWNzXG4gICAqL1xuICBzdGF0cygpOiBFd2NTdGF0cyB7XG4gICAgcmV0dXJuIHtcbiAgICAgIHRhc2tzTGVhcm5lZDogdGhpcy50YXNrc0xlYXJuZWQsXG4gICAgICBmaXNoZXJDb21wdXRlZDogdGhpcy5maXNoZXJEaWFnb25hbC5zaXplID4gMCxcbiAgICAgIHByb3RlY3Rpb25TdHJlbmd0aDogdGhpcy5sYW1iZGEsXG4gICAgICBmb3JnZXR0aW5nUmF0ZTogdGhpcy5lc3RpbWF0ZUZvcmdldHRpbmdSYXRlKCksXG4gICAgfTtcbiAgfVxuXG4gIHByaXZhdGUgZXN0aW1hdGVGb3JnZXR0aW5nUmF0ZSgpOiBudW1iZXIge1xuICAgIC8vIFNpbXBsaWZpZWQgZXN0aW1hdGlvbiBiYXNlZCBvbiBudW1iZXIgb2YgdGFza3NcbiAgICByZXR1cm4gTWF0aC5tYXgoMCwgMSAtIE1hdGguZXhwKC10aGlzLnRhc2tzTGVhcm5lZCAqIDAuMSkpO1xuICB9XG59XG5cbi8qKlxuICogU09OQSBMZWFybmluZyBDb29yZGluYXRvclxuICpcbiAqIE9yY2hlc3RyYXRlcyB0aGUgbGVhcm5pbmcgbG9vcHMgYW5kIGNvbXBvbmVudHMuXG4gKi9cbmV4cG9ydCBjbGFzcyBTb25hQ29vcmRpbmF0b3Ige1xuICBwcml2YXRlIGNvbmZpZzogUmVxdWlyZWQ8U29uYUNvbmZpZz47XG4gIHByaXZhdGUgdHJhamVjdG9yeUJ1ZmZlcjogUXVlcnlUcmFqZWN0b3J5W10gPSBbXTtcbiAgcHJpdmF0ZSByZWFzb25pbmdCYW5rOiBSZWFzb25pbmdCYW5rO1xuICBwcml2YXRlIGV3Y01hbmFnZXI6IEV3Y01hbmFnZXI7XG4gIHByaXZhdGUgc2lnbmFsQnVmZmVyOiBMZWFybmluZ1NpZ25hbFtdID0gW107XG5cbiAgY29uc3RydWN0b3IoY29uZmlnPzogU29uYUNvbmZpZykge1xuICAgIHRoaXMuY29uZmlnID0geyAuLi5ERUZBVUxUX1NPTkFfQ09ORklHLCAuLi5jb25maWcgfTtcbiAgICB0aGlzLnJlYXNvbmluZ0JhbmsgPSBuZXcgUmVhc29uaW5nQmFuayh0aGlzLmNvbmZpZy5wYXR0ZXJuVGhyZXNob2xkKTtcbiAgICB0aGlzLmV3Y01hbmFnZXIgPSBuZXcgRXdjTWFuYWdlcih0aGlzLmNvbmZpZy5ld2NMYW1iZGEpO1xuICB9XG5cbiAgLyoqXG4gICAqIFJlY29yZCBhIGxlYXJuaW5nIHNpZ25hbFxuICAgKi9cbiAgcmVjb3JkU2lnbmFsKHNpZ25hbDogTGVhcm5pbmdTaWduYWwpOiB2b2lkIHtcbiAgICB0aGlzLnNpZ25hbEJ1ZmZlci5wdXNoKHNpZ25hbCk7XG5cbiAgICAvLyBJbnN0YW50IGxvb3AgLSBpbW1lZGlhdGUgbGVhcm5pbmdcbiAgICBpZiAodGhpcy5jb25maWcuaW5zdGFudExvb3BFbmFibGVkICYmIHNpZ25hbC5xdWFsaXR5ID49IDAuOCkge1xuICAgICAgdGhpcy5wcm9jZXNzSW5zdGFudExlYXJuaW5nKHNpZ25hbCk7XG4gICAgfVxuICB9XG5cbiAgLyoqXG4gICAqIFJlY29yZCBhIGNvbXBsZXRlZCB0cmFqZWN0b3J5XG4gICAqL1xuICByZWNvcmRUcmFqZWN0b3J5KHRyYWplY3Rvcnk6IFF1ZXJ5VHJhamVjdG9yeSk6IHZvaWQge1xuICAgIHRoaXMudHJhamVjdG9yeUJ1ZmZlci5wdXNoKHRyYWplY3RvcnkpO1xuXG4gICAgLy8gTWFpbnRhaW4gYnVmZmVyIHNpemVcbiAgICB3aGlsZSAodGhpcy50cmFqZWN0b3J5QnVmZmVyLmxlbmd0aCA+IHRoaXMuY29uZmlnLm1heFRyYWplY3RvcnlTaXplKSB7XG4gICAgICB0aGlzLnRyYWplY3RvcnlCdWZmZXIuc2hpZnQoKTtcbiAgICB9XG5cbiAgICAvLyBFeHRyYWN0IHBhdHRlcm5zIGZyb20gc3VjY2Vzc2Z1bCB0cmFqZWN0b3JpZXNcbiAgICBpZiAodHJhamVjdG9yeS5vdXRjb21lID09PSAnc3VjY2VzcycpIHtcbiAgICAgIHRoaXMuZXh0cmFjdFBhdHRlcm5zKHRyYWplY3RvcnkpO1xuICAgIH1cbiAgfVxuXG4gIC8qKlxuICAgKiBSdW4gYmFja2dyb3VuZCBsZWFybmluZyBsb29wXG4gICAqL1xuICBydW5CYWNrZ3JvdW5kTG9vcCgpOiB7IHBhdHRlcm5zTGVhcm5lZDogbnVtYmVyOyB0cmFqZWN0b3JpZXNQcm9jZXNzZWQ6IG51bWJlciB9IHtcbiAgICBpZiAoIXRoaXMuY29uZmlnLmJhY2tncm91bmRMb29wRW5hYmxlZCkge1xuICAgICAgcmV0dXJuIHsgcGF0dGVybnNMZWFybmVkOiAwLCB0cmFqZWN0b3JpZXNQcm9jZXNzZWQ6IDAgfTtcbiAgICB9XG5cbiAgICBsZXQgcGF0dGVybnNMZWFybmVkID0gMDtcbiAgICBjb25zdCB0cmFqZWN0b3JpZXNQcm9jZXNzZWQgPSB0aGlzLnRyYWplY3RvcnlCdWZmZXIubGVuZ3RoO1xuXG4gICAgLy8gUHJvY2VzcyBhY2N1bXVsYXRlZCB0cmFqZWN0b3JpZXNcbiAgICBmb3IgKGNvbnN0IHRyYWogb2YgdGhpcy50cmFqZWN0b3J5QnVmZmVyKSB7XG4gICAgICBpZiAodHJhai5vdXRjb21lID09PSAnc3VjY2VzcycgfHwgdHJhai5vdXRjb21lID09PSAncGFydGlhbCcpIHtcbiAgICAgICAgcGF0dGVybnNMZWFybmVkICs9IHRoaXMuZXh0cmFjdFBhdHRlcm5zKHRyYWopO1xuICAgICAgfVxuICAgIH1cblxuICAgIC8vIFBydW5lIGxvdy1wZXJmb3JtaW5nIHBhdHRlcm5zXG4gICAgdGhpcy5yZWFzb25pbmdCYW5rLnBydW5lKCk7XG5cbiAgICAvLyBDbGVhciBwcm9jZXNzZWQgdHJhamVjdG9yaWVzXG4gICAgdGhpcy50cmFqZWN0b3J5QnVmZmVyID0gW107XG5cbiAgICByZXR1cm4geyBwYXR0ZXJuc0xlYXJuZWQsIHRyYWplY3Rvcmllc1Byb2Nlc3NlZCB9O1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCByZWFzb25pbmcgYmFuayBmb3IgcGF0dGVybiBxdWVyaWVzXG4gICAqL1xuICBnZXRSZWFzb25pbmdCYW5rKCk6IFJlYXNvbmluZ0Jhbmsge1xuICAgIHJldHVybiB0aGlzLnJlYXNvbmluZ0Jhbms7XG4gIH1cblxuICAvKipcbiAgICogR2V0IEVXQyBtYW5hZ2VyXG4gICAqL1xuICBnZXRFd2NNYW5hZ2VyKCk6IEV3Y01hbmFnZXIge1xuICAgIHJldHVybiB0aGlzLmV3Y01hbmFnZXI7XG4gIH1cblxuICAvKipcbiAgICogR2V0IHN0YXRpc3RpY3NcbiAgICovXG4gIHN0YXRzKCk6IHtcbiAgICBzaWduYWxzUmVjZWl2ZWQ6IG51bWJlcjtcbiAgICB0cmFqZWN0b3JpZXNCdWZmZXJlZDogbnVtYmVyO1xuICAgIHBhdHRlcm5zOiBSZXR1cm5UeXBlPFJlYXNvbmluZ0JhbmtbJ3N0YXRzJ10+O1xuICAgIGV3YzogRXdjU3RhdHM7XG4gIH0ge1xuICAgIHJldHVybiB7XG4gICAgICBzaWduYWxzUmVjZWl2ZWQ6IHRoaXMuc2lnbmFsQnVmZmVyLmxlbmd0aCxcbiAgICAgIHRyYWplY3Rvcmllc0J1ZmZlcmVkOiB0aGlzLnRyYWplY3RvcnlCdWZmZXIubGVuZ3RoLFxuICAgICAgcGF0dGVybnM6IHRoaXMucmVhc29uaW5nQmFuay5zdGF0cygpLFxuICAgICAgZXdjOiB0aGlzLmV3Y01hbmFnZXIuc3RhdHMoKSxcbiAgICB9O1xuICB9XG5cbiAgcHJpdmF0ZSBwcm9jZXNzSW5zdGFudExlYXJuaW5nKHNpZ25hbDogTGVhcm5pbmdTaWduYWwpOiB2b2lkIHtcbiAgICAvLyBJbW1lZGlhdGUgcGF0dGVybiByZWluZm9yY2VtZW50IHdvdWxkIGhhcHBlbiBoZXJlXG4gICAgLy8gSW4gZnVsbCBpbXBsZW1lbnRhdGlvbiwgdGhpcyB1cGRhdGVzIExvUkEgd2VpZ2h0c1xuICB9XG5cbiAgcHJpdmF0ZSBleHRyYWN0UGF0dGVybnModHJhamVjdG9yeTogUXVlcnlUcmFqZWN0b3J5KTogbnVtYmVyIHtcbiAgICBsZXQgZXh0cmFjdGVkID0gMDtcblxuICAgIGZvciAoY29uc3Qgc3RlcCBvZiB0cmFqZWN0b3J5LnN0ZXBzKSB7XG4gICAgICBpZiAoc3RlcC5jb25maWRlbmNlID49IHRoaXMuY29uZmlnLnBhdHRlcm5UaHJlc2hvbGQpIHtcbiAgICAgICAgLy8gQ3JlYXRlIGVtYmVkZGluZyBmcm9tIHN0ZXAgKHNpbXBsaWZpZWQpXG4gICAgICAgIGNvbnN0IGVtYmVkZGluZyA9IHRoaXMuY3JlYXRlRW1iZWRkaW5nKHN0ZXAuaW5wdXQgKyBzdGVwLm91dHB1dCk7XG5cbiAgICAgICAgLy8gRGV0ZXJtaW5lIHBhdHRlcm4gdHlwZVxuICAgICAgICBjb25zdCB0eXBlID0gdGhpcy5zdGVwVHlwZVRvUGF0dGVyblR5cGUoc3RlcC50eXBlKTtcblxuICAgICAgICAvLyBTdG9yZSBpZiBub3QgdG9vIHNpbWlsYXIgdG8gZXhpc3RpbmdcbiAgICAgICAgY29uc3Qgc2ltaWxhciA9IHRoaXMucmVhc29uaW5nQmFuay5maW5kU2ltaWxhcihlbWJlZGRpbmcsIDEpO1xuICAgICAgICBpZiAoc2ltaWxhci5sZW5ndGggPT09IDApIHtcbiAgICAgICAgICB0aGlzLnJlYXNvbmluZ0Jhbmsuc3RvcmUodHlwZSwgZW1iZWRkaW5nKTtcbiAgICAgICAgICBleHRyYWN0ZWQrKztcbiAgICAgICAgfVxuICAgICAgfVxuICAgIH1cblxuICAgIHJldHVybiBleHRyYWN0ZWQ7XG4gIH1cblxuICBwcml2YXRlIHN0ZXBUeXBlVG9QYXR0ZXJuVHlwZShzdGVwVHlwZTogVHJhamVjdG9yeVN0ZXBbJ3R5cGUnXSk6IFBhdHRlcm5UeXBlIHtcbiAgICBzd2l0Y2ggKHN0ZXBUeXBlKSB7XG4gICAgICBjYXNlICdxdWVyeSc6XG4gICAgICBjYXNlICdnZW5lcmF0ZSc6XG4gICAgICAgIHJldHVybiAncXVlcnlfcmVzcG9uc2UnO1xuICAgICAgY2FzZSAncm91dGUnOlxuICAgICAgICByZXR1cm4gJ3JvdXRpbmcnO1xuICAgICAgY2FzZSAnbWVtb3J5JzpcbiAgICAgICAgcmV0dXJuICdjb250ZXh0X3JldHJpZXZhbCc7XG4gICAgICBjYXNlICdmZWVkYmFjayc6XG4gICAgICAgIHJldHVybiAnY29ycmVjdGlvbic7XG4gICAgICBkZWZhdWx0OlxuICAgICAgICByZXR1cm4gJ3F1ZXJ5X3Jlc3BvbnNlJztcbiAgICB9XG4gIH1cblxuICBwcml2YXRlIGNyZWF0ZUVtYmVkZGluZyh0ZXh0OiBzdHJpbmcpOiBFbWJlZGRpbmcge1xuICAgIC8vIFNpbXBsaWZpZWQgaGFzaC1iYXNlZCBlbWJlZGRpbmcgKHJlYWwgaW1wbCB1c2VzIG1vZGVsKVxuICAgIGNvbnN0IGRpbSA9IDY0O1xuICAgIGNvbnN0IGVtYmVkZGluZyA9IG5ldyBBcnJheShkaW0pLmZpbGwoMCk7XG5cbiAgICBmb3IgKGxldCBpID0gMDsgaSA8IHRleHQubGVuZ3RoOyBpKyspIHtcbiAgICAgIGNvbnN0IGlkeCA9ICh0ZXh0LmNoYXJDb2RlQXQoaSkgKiAoaSArIDEpKSAlIGRpbTtcbiAgICAgIGVtYmVkZGluZ1tpZHhdICs9IDAuMTtcbiAgICB9XG5cbiAgICAvLyBOb3JtYWxpemVcbiAgICBjb25zdCBub3JtID0gTWF0aC5zcXJ0KGVtYmVkZGluZy5yZWR1Y2UoKHMsIHgpID0+IHMgKyB4ICogeCwgMCkpIHx8IDE7XG4gICAgcmV0dXJuIGVtYmVkZGluZy5tYXAoeCA9PiB4IC8gbm9ybSk7XG4gIH1cbn1cblxuLy8gRXhwb3J0IGFsbCBTT05BIGNvbXBvbmVudHNcbmV4cG9ydCB7XG4gIERFRkFVTFRfU09OQV9DT05GSUcsXG59O1xuIl19