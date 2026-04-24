/**
 * LoRA (Low-Rank Adaptation) Runtime
 *
 * Efficient parameter-efficient fine-tuning adapters for LLMs.
 * Supports micro-LoRA (fast, small updates) and base-LoRA (deeper adaptation).
 *
 * @example
 * ```typescript
 * import { LoraAdapter, LoraManager } from '@ruvector/ruvllm';
 *
 * // Create adapter
 * const adapter = new LoraAdapter({
 *   rank: 8,
 *   alpha: 16,
 *   dropout: 0.1,
 *   targetModules: ['query', 'value'],
 * });
 *
 * // Apply to hidden states
 * const output = adapter.forward(hiddenStates);
 *
 * // Manage multiple adapters
 * const manager = new LoraManager();
 * manager.register('task-1', adapter);
 * manager.activate('task-1');
 * ```
 */
/**
 * Default LoRA configuration
 */
const DEFAULT_LORA_CONFIG = {
    rank: 8,
    alpha: 16,
    dropout: 0.1,
    targetModules: ['query', 'value'],
};
/**
 * LoRA Adapter
 *
 * Implements low-rank decomposition for parameter-efficient fine-tuning.
 * W' = W + BA where A is (d x r) and B is (r x d), r << d
 *
 * @example
 * ```typescript
 * const adapter = new LoraAdapter({
 *   rank: 8,
 *   alpha: 16,
 *   inputDim: 768,
 *   outputDim: 768,
 * });
 *
 * // Forward pass
 * const output = adapter.forward(input);
 *
 * // Training step
 * adapter.backward(input, gradOutput, 0.001);
 * ```
 */
export class LoraAdapter {
    constructor(config, inputDim = 256, outputDim = 256) {
        this.trainingState = null;
        this.frozen = false;
        this.config = { ...DEFAULT_LORA_CONFIG, ...config };
        this.inputDim = inputDim;
        this.outputDim = outputDim;
        // Initialize weights
        this.weights = this.initializeWeights();
    }
    /**
     * Forward pass through LoRA adapter
     * OPTIMIZED: Uses Float64Array and loop unrolling
     *
     * output = input + scaling * (input @ A @ B)
     */
    forward(input) {
        const rank = this.config.rank;
        const dim = Math.min(input.length, this.inputDim);
        const scaling = this.weights.scaling;
        // Apply dropout during training (simplified check)
        const applyDropout = this.trainingState !== null && this.config.dropout > 0;
        // input @ A (d -> r) - use typed array for hidden
        const hidden = new Float64Array(rank);
        for (let r = 0; r < rank; r++) {
            let sum = 0;
            const loraACol = this.weights.loraA;
            // Unroll loop for better performance
            let i = 0;
            if (applyDropout) {
                for (; i < dim; i++) {
                    if (Math.random() > this.config.dropout) {
                        sum += input[i] * loraACol[i][r];
                    }
                }
            }
            else {
                for (; i + 3 < dim; i += 4) {
                    sum += input[i] * loraACol[i][r] +
                        input[i + 1] * loraACol[i + 1][r] +
                        input[i + 2] * loraACol[i + 2][r] +
                        input[i + 3] * loraACol[i + 3][r];
                }
                for (; i < dim; i++) {
                    sum += input[i] * loraACol[i][r];
                }
            }
            hidden[r] = sum;
        }
        // hidden @ B (r -> d) + residual
        const output = new Array(this.outputDim);
        const loraB = this.weights.loraB;
        for (let i = 0; i < this.outputDim; i++) {
            let delta = 0;
            for (let r = 0; r < rank; r++) {
                delta += hidden[r] * loraB[r][i];
            }
            // Add scaled delta to input (residual connection)
            output[i] = (input[i] || 0) + scaling * delta;
        }
        return output;
    }
    /**
     * Forward with batch processing
     */
    forwardBatch(inputs) {
        return inputs.map(input => this.forward(input));
    }
    /**
     * Backward pass and weight update
     */
    backward(input, gradOutput, learningRate) {
        if (this.frozen)
            return 0;
        const rank = this.config.rank;
        const dim = Math.min(input.length, this.inputDim);
        // Compute hidden activations (for gradient)
        const hidden = new Array(rank).fill(0);
        for (let r = 0; r < rank; r++) {
            for (let i = 0; i < dim; i++) {
                hidden[r] += input[i] * this.weights.loraA[i][r];
            }
        }
        // Gradient for B: hidden^T @ gradOutput
        const gradB = Array(rank).fill(null).map(() => Array(this.outputDim).fill(0));
        for (let r = 0; r < rank; r++) {
            for (let i = 0; i < this.outputDim; i++) {
                gradB[r][i] = hidden[r] * (gradOutput[i] || 0) * this.weights.scaling;
            }
        }
        // Gradient for hidden: gradOutput @ B^T
        const gradHidden = new Array(rank).fill(0);
        for (let r = 0; r < rank; r++) {
            for (let i = 0; i < this.outputDim; i++) {
                gradHidden[r] += (gradOutput[i] || 0) * this.weights.loraB[r][i] * this.weights.scaling;
            }
        }
        // Gradient for A: input^T @ gradHidden
        const gradA = Array(dim).fill(null).map(() => Array(rank).fill(0));
        for (let i = 0; i < dim; i++) {
            for (let r = 0; r < rank; r++) {
                gradA[i][r] = input[i] * gradHidden[r];
            }
        }
        // Update weights
        let totalGrad = 0;
        for (let i = 0; i < dim; i++) {
            for (let r = 0; r < rank; r++) {
                this.weights.loraA[i][r] -= learningRate * gradA[i][r];
                totalGrad += Math.abs(gradA[i][r]);
            }
        }
        for (let r = 0; r < rank; r++) {
            for (let i = 0; i < this.outputDim; i++) {
                this.weights.loraB[r][i] -= learningRate * gradB[r][i];
                totalGrad += Math.abs(gradB[r][i]);
            }
        }
        // Track training state
        if (this.trainingState) {
            this.trainingState.step++;
            this.trainingState.lossHistory.push(totalGrad);
        }
        return totalGrad;
    }
    /**
     * Start training mode
     */
    startTraining(learningRate = 0.001) {
        this.trainingState = {
            step: 0,
            learningRate,
            gradA: Array(this.inputDim).fill(null).map(() => Array(this.config.rank).fill(0)),
            gradB: Array(this.config.rank).fill(null).map(() => Array(this.outputDim).fill(0)),
            lossHistory: [],
        };
    }
    /**
     * End training mode
     */
    endTraining() {
        const state = this.trainingState;
        this.trainingState = null;
        return state;
    }
    /**
     * Freeze adapter (no more updates)
     */
    freeze() {
        this.frozen = true;
    }
    /**
     * Unfreeze adapter
     */
    unfreeze() {
        this.frozen = false;
    }
    /**
     * Check if frozen
     */
    isFrozen() {
        return this.frozen;
    }
    /**
     * Get adapter config
     */
    getConfig() {
        return { ...this.config };
    }
    /**
     * Get adapter weights
     */
    getWeights() {
        return {
            loraA: this.weights.loraA.map(row => [...row]),
            loraB: this.weights.loraB.map(row => [...row]),
            scaling: this.weights.scaling,
        };
    }
    /**
     * Set adapter weights
     */
    setWeights(weights) {
        this.weights = {
            loraA: weights.loraA.map(row => [...row]),
            loraB: weights.loraB.map(row => [...row]),
            scaling: weights.scaling,
        };
    }
    /**
     * Merge adapter into base weights
     *
     * Returns delta to add to base model weights
     */
    merge() {
        const delta = Array(this.inputDim)
            .fill(null)
            .map(() => Array(this.outputDim).fill(0));
        const rank = this.config.rank;
        for (let i = 0; i < this.inputDim; i++) {
            for (let j = 0; j < this.outputDim; j++) {
                for (let r = 0; r < rank; r++) {
                    delta[i][j] += this.weights.loraA[i][r] * this.weights.loraB[r][j];
                }
                delta[i][j] *= this.weights.scaling;
            }
        }
        return delta;
    }
    /**
     * Get number of trainable parameters
     */
    numParameters() {
        return (this.inputDim * this.config.rank) + (this.config.rank * this.outputDim);
    }
    /**
     * Reset to initial weights
     */
    reset() {
        this.weights = this.initializeWeights();
        this.trainingState = null;
        this.frozen = false;
    }
    /**
     * Clone adapter
     */
    clone() {
        const adapter = new LoraAdapter(this.config, this.inputDim, this.outputDim);
        adapter.setWeights(this.getWeights());
        return adapter;
    }
    /**
     * Serialize to JSON
     */
    toJSON() {
        return JSON.stringify({
            config: this.config,
            inputDim: this.inputDim,
            outputDim: this.outputDim,
            weights: this.weights,
            frozen: this.frozen,
        });
    }
    /**
     * Deserialize from JSON
     */
    static fromJSON(json) {
        const data = JSON.parse(json);
        const adapter = new LoraAdapter(data.config, data.inputDim, data.outputDim);
        adapter.setWeights(data.weights);
        if (data.frozen)
            adapter.freeze();
        return adapter;
    }
    initializeWeights() {
        const rank = this.config.rank;
        // Kaiming initialization for A, zero initialization for B
        const loraA = Array(this.inputDim)
            .fill(null)
            .map(() => Array(rank)
            .fill(0)
            .map(() => (Math.random() - 0.5) * Math.sqrt(2 / this.inputDim)));
        const loraB = Array(rank)
            .fill(null)
            .map(() => Array(this.outputDim).fill(0));
        return {
            loraA,
            loraB,
            scaling: this.config.alpha / this.config.rank,
        };
    }
}
/**
 * LoRA Manager for multiple adapters
 *
 * Manages a collection of LoRA adapters for different tasks/domains.
 */
export class LoraManager {
    constructor(defaultConfig) {
        this.adapters = new Map();
        this.activeAdapterId = null;
        this.defaultConfig = { ...DEFAULT_LORA_CONFIG, ...defaultConfig };
    }
    /**
     * Register a new adapter
     */
    register(id, adapter) {
        this.adapters.set(id, adapter);
    }
    /**
     * Create and register a new adapter
     */
    create(id, config, inputDim, outputDim) {
        const mergedConfig = { ...this.defaultConfig, ...config };
        const adapter = new LoraAdapter(mergedConfig, inputDim, outputDim);
        this.register(id, adapter);
        return adapter;
    }
    /**
     * Get adapter by ID
     */
    get(id) {
        return this.adapters.get(id);
    }
    /**
     * Remove adapter
     */
    remove(id) {
        if (this.activeAdapterId === id) {
            this.activeAdapterId = null;
        }
        return this.adapters.delete(id);
    }
    /**
     * Activate an adapter
     */
    activate(id) {
        if (this.adapters.has(id)) {
            this.activeAdapterId = id;
            return true;
        }
        return false;
    }
    /**
     * Deactivate current adapter
     */
    deactivate() {
        this.activeAdapterId = null;
    }
    /**
     * Get active adapter
     */
    getActive() {
        return this.activeAdapterId ? this.adapters.get(this.activeAdapterId) || null : null;
    }
    /**
     * Get active adapter ID
     */
    getActiveId() {
        return this.activeAdapterId;
    }
    /**
     * Apply active adapter
     */
    forward(input) {
        const active = this.getActive();
        return active ? active.forward(input) : [...input];
    }
    /**
     * List all adapter IDs
     */
    list() {
        return Array.from(this.adapters.keys());
    }
    /**
     * Get adapter count
     */
    count() {
        return this.adapters.size;
    }
    /**
     * Freeze all adapters
     */
    freezeAll() {
        for (const adapter of this.adapters.values()) {
            adapter.freeze();
        }
    }
    /**
     * Unfreeze all adapters
     */
    unfreezeAll() {
        for (const adapter of this.adapters.values()) {
            adapter.unfreeze();
        }
    }
    /**
     * Merge multiple adapters into one
     */
    mergeAdapters(ids, outputId) {
        const adapters = ids.map(id => this.adapters.get(id)).filter(Boolean);
        if (adapters.length === 0)
            return null;
        // Use first adapter as base
        const merged = adapters[0].clone();
        const weights = merged.getWeights();
        // Average weights from other adapters
        for (let i = 1; i < adapters.length; i++) {
            const otherWeights = adapters[i].getWeights();
            for (let row = 0; row < weights.loraA.length && row < otherWeights.loraA.length; row++) {
                for (let col = 0; col < weights.loraA[row].length && col < otherWeights.loraA[row].length; col++) {
                    weights.loraA[row][col] = (weights.loraA[row][col] + otherWeights.loraA[row][col]) / 2;
                }
            }
            for (let row = 0; row < weights.loraB.length && row < otherWeights.loraB.length; row++) {
                for (let col = 0; col < weights.loraB[row].length && col < otherWeights.loraB[row].length; col++) {
                    weights.loraB[row][col] = (weights.loraB[row][col] + otherWeights.loraB[row][col]) / 2;
                }
            }
        }
        merged.setWeights(weights);
        this.register(outputId, merged);
        return merged;
    }
    /**
     * Get statistics
     */
    stats() {
        let totalParams = 0;
        let frozenCount = 0;
        for (const adapter of this.adapters.values()) {
            totalParams += adapter.numParameters();
            if (adapter.isFrozen())
                frozenCount++;
        }
        return {
            totalAdapters: this.adapters.size,
            activeAdapter: this.activeAdapterId,
            totalParameters: totalParams,
            frozenCount,
        };
    }
    /**
     * Clear all adapters
     */
    clear() {
        this.adapters.clear();
        this.activeAdapterId = null;
    }
}
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoibG9yYS5qcyIsInNvdXJjZVJvb3QiOiIiLCJzb3VyY2VzIjpbIi4uLy4uL3NyYy9sb3JhLnRzIl0sIm5hbWVzIjpbXSwibWFwcGluZ3MiOiJBQUFBOzs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7OztHQTBCRztBQUlIOztHQUVHO0FBQ0gsTUFBTSxtQkFBbUIsR0FBeUI7SUFDaEQsSUFBSSxFQUFFLENBQUM7SUFDUCxLQUFLLEVBQUUsRUFBRTtJQUNULE9BQU8sRUFBRSxHQUFHO0lBQ1osYUFBYSxFQUFFLENBQUMsT0FBTyxFQUFFLE9BQU8sQ0FBQztDQUNsQyxDQUFDO0FBOEJGOzs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7R0FxQkc7QUFDSCxNQUFNLE9BQU8sV0FBVztJQVF0QixZQUFZLE1BQTRCLEVBQUUsUUFBUSxHQUFHLEdBQUcsRUFBRSxTQUFTLEdBQUcsR0FBRztRQUhqRSxrQkFBYSxHQUE2QixJQUFJLENBQUM7UUFDL0MsV0FBTSxHQUFZLEtBQUssQ0FBQztRQUc5QixJQUFJLENBQUMsTUFBTSxHQUFHLEVBQUUsR0FBRyxtQkFBbUIsRUFBRSxHQUFHLE1BQU0sRUFBRSxDQUFDO1FBQ3BELElBQUksQ0FBQyxRQUFRLEdBQUcsUUFBUSxDQUFDO1FBQ3pCLElBQUksQ0FBQyxTQUFTLEdBQUcsU0FBUyxDQUFDO1FBRTNCLHFCQUFxQjtRQUNyQixJQUFJLENBQUMsT0FBTyxHQUFHLElBQUksQ0FBQyxpQkFBaUIsRUFBRSxDQUFDO0lBQzFDLENBQUM7SUFFRDs7Ozs7T0FLRztJQUNILE9BQU8sQ0FBQyxLQUFlO1FBQ3JCLE1BQU0sSUFBSSxHQUFHLElBQUksQ0FBQyxNQUFNLENBQUMsSUFBSSxDQUFDO1FBQzlCLE1BQU0sR0FBRyxHQUFHLElBQUksQ0FBQyxHQUFHLENBQUMsS0FBSyxDQUFDLE1BQU0sRUFBRSxJQUFJLENBQUMsUUFBUSxDQUFDLENBQUM7UUFDbEQsTUFBTSxPQUFPLEdBQUcsSUFBSSxDQUFDLE9BQU8sQ0FBQyxPQUFPLENBQUM7UUFFckMsbURBQW1EO1FB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T0FFRztJQUNILEtBQUs7UUFNSCxJQUFJLFdBQVcsR0FBRyxDQUFDLENBQUM7UUFDcEIsSUFBSSxXQUFXLEdBQUcsQ0FBQyxDQUFDO1FBRXBCLEtBQUssTUFBTSxPQUFPLElBQUksSUFBSSxDQUFDLFFBQVEsQ0FBQyxNQUFNLEVBQUUsRUFBRSxDQUFDO1lBQzdDLFdBQVcsSUFBSSxPQUFPLENBQUMsYUFBYSxFQUFFLENBQUM7WUFDdkMsSUFBSSxPQUFPLENBQUMsUUFBUSxFQUFFO2dCQUFFLFdBQVcsRUFBRSxDQUFDO1FBQ3hDLENBQUM7UUFFRCxPQUFPO1lBQ0wsYUFBYSxFQUFFLElBQUksQ0FBQyxRQUFRLENBQUMsSUFBSTtZQUNqQyxhQUFhLEVBQUUsSUFBSSxDQUFDLGVBQWU7WUFDbkMsZUFBZSxFQUFFLFdBQVc7WUFDNUIsV0FBVztTQUNaLENBQUM7SUFDSixDQUFDO0lBRUQ7O09BRUc7SUFDSCxLQUFLO1FBQ0gsSUFBSSxDQUFDLFFBQVEsQ0FBQyxLQUFLLEVBQUUsQ0FBQztRQUN0QixJQUFJLENBQUMsZUFBZSxHQUFHLElBQUksQ0FBQztJQUM5QixDQUFDO0NBQ0YiLCJzb3VyY2VzQ29udGVudCI6WyIvKipcbiAqIExvUkEgKExvdy1SYW5rIEFkYXB0YXRpb24pIFJ1bnRpbWVcbiAqXG4gKiBFZmZpY2llbnQgcGFyYW1ldGVyLWVmZmljaWVudCBmaW5lLXR1bmluZyBhZGFwdGVycyBmb3IgTExNcy5cbiAqIFN1cHBvcnRzIG1pY3JvLUxvUkEgKGZhc3QsIHNtYWxsIHVwZGF0ZXMpIGFuZCBiYXNlLUxvUkEgKGRlZXBlciBhZGFwdGF0aW9uKS5cbiAqXG4gKiBAZXhhbXBsZVxuICogYGBgdHlwZXNjcmlwdFxuICogaW1wb3J0IHsgTG9yYUFkYXB0ZXIsIExvcmFNYW5hZ2VyIH0gZnJvbSAnQHJ1dmVjdG9yL3J1dmxsbSc7XG4gKlxuICogLy8gQ3JlYXRlIGFkYXB0ZXJcbiAqIGNvbnN0IGFkYXB0ZXIgPSBuZXcgTG9yYUFkYXB0ZXIoe1xuICogICByYW5rOiA4LFxuICogICBhbHBoYTogMTYsXG4gKiAgIGRyb3BvdXQ6IDAuMSxcbiAqICAgdGFyZ2V0TW9kdWxlczogWydxdWVyeScsICd2YWx1ZSddLFxuICogfSk7XG4gKlxuICogLy8gQXBwbHkgdG8gaGlkZGVuIHN0YXRlc1xuICogY29uc3Qgb3V0cHV0ID0gYWRhcHRlci5mb3J3YXJkKGhpZGRlblN0YXRlcyk7XG4gKlxuICogLy8gTWFuYWdlIG11bHRpcGxlIGFkYXB0ZXJzXG4gKiBjb25zdCBtYW5hZ2VyID0gbmV3IExvcmFNYW5hZ2VyKCk7XG4gKiBtYW5hZ2VyLnJlZ2lzdGVyKCd0YXNrLTEnLCBhZGFwdGVyKTtcbiAqIG1hbmFnZXIuYWN0aXZhdGUoJ3Rhc2stMScpO1xuICogYGBgXG4gKi9cblxuaW1wb3J0IHsgTG9SQUNvbmZpZywgRW1iZWRkaW5nIH0gZnJvbSAnLi90eXBlcyc7XG5cbi8qKlxuICogRGVmYXVsdCBMb1JBIGNvbmZpZ3VyYXRpb25cbiAqL1xuY29uc3QgREVGQVVMVF9MT1JBX0NPTkZJRzogUmVxdWlyZWQ8TG9SQUNvbmZpZz4gPSB7XG4gIHJhbms6IDgsXG4gIGFscGhhOiAxNixcbiAgZHJvcG91dDogMC4xLFxuICB0YXJnZXRNb2R1bGVzOiBbJ3F1ZXJ5JywgJ3ZhbHVlJ10sXG59O1xuXG4vKipcbiAqIExvUkEgYWRhcHRlciB3ZWlnaHRzXG4gKi9cbmV4cG9ydCBpbnRlcmZhY2UgTG9yYVdlaWdodHMge1xuICAvKiogRG93biBwcm9qZWN0aW9uIG1hdHJpeCAoZCB4IHIpICovXG4gIGxvcmFBOiBudW1iZXJbXVtdO1xuICAvKiogVXAgcHJvamVjdGlvbiBtYXRyaXggKHIgeCBkKSAqL1xuICBsb3JhQjogbnVtYmVyW11bXTtcbiAgLyoqIFNjYWxpbmcgZmFjdG9yICovXG4gIHNjYWxpbmc6IG51bWJlcjtcbn1cblxuLyoqXG4gKiBMb1JBIHRyYWluaW5nIHN0YXRlXG4gKi9cbmV4cG9ydCBpbnRlcmZhY2UgTG9yYVRyYWluaW5nU3RhdGUge1xuICAvKiogQ3VycmVudCBzdGVwICovXG4gIHN0ZXA6IG51bWJlcjtcbiAgLyoqIExlYXJuaW5nIHJhdGUgKi9cbiAgbGVhcm5pbmdSYXRlOiBudW1iZXI7XG4gIC8qKiBBY2N1bXVsYXRlZCBncmFkaWVudHMgZm9yIEEgKi9cbiAgZ3JhZEE6IG51bWJlcltdW107XG4gIC8qKiBBY2N1bXVsYXRlZCBncmFkaWVudHMgZm9yIEIgKi9cbiAgZ3JhZEI6IG51bWJlcltdW107XG4gIC8qKiBMb3NzIGhpc3RvcnkgKi9cbiAgbG9zc0hpc3Rvcnk6IG51bWJlcltdO1xufVxuXG4vKipcbiAqIExvUkEgQWRhcHRlclxuICpcbiAqIEltcGxlbWVudHMgbG93LXJhbmsgZGVjb21wb3NpdGlvbiBmb3IgcGFyYW1ldGVyLWVmZmljaWVudCBmaW5lLXR1bmluZy5cbiAqIFcnID0gVyArIEJBIHdoZXJlIEEgaXMgKGQgeCByKSBhbmQgQiBpcyAociB4IGQpLCByIDw8IGRcbiAqXG4gKiBAZXhhbXBsZVxuICogYGBgdHlwZXNjcmlwdFxuICogY29uc3QgYWRhcHRlciA9IG5ldyBMb3JhQWRhcHRlcih7XG4gKiAgIHJhbms6IDgsXG4gKiAgIGFscGhhOiAxNixcbiAqICAgaW5wdXREaW06IDc2OCxcbiAqICAgb3V0cHV0RGltOiA3NjgsXG4gKiB9KTtcbiAqXG4gKiAvLyBGb3J3YXJkIHBhc3NcbiAqIGNvbnN0IG91dHB1dCA9IGFkYXB0ZXIuZm9yd2FyZChpbnB1dCk7XG4gKlxuICogLy8gVHJhaW5pbmcgc3RlcFxuICogYWRhcHRlci5iYWNrd2FyZChpbnB1dCwgZ3JhZE91dHB1dCwgMC4wMDEpO1xuICogYGBgXG4gKi9cbmV4cG9ydCBjbGFzcyBMb3JhQWRhcHRlciB7XG4gIHByaXZhdGUgY29uZmlnOiBSZXF1aXJlZDxMb1JBQ29uZmlnPjtcbiAgcHJpdmF0ZSBpbnB1dERpbTogbnVtYmVyO1xuICBwcml2YXRlIG91dHB1dERpbTogbnVtYmVyO1xuICBwcml2YXRlIHdlaWdodHM6IExvcmFXZWlnaHRzO1xuICBwcml2YXRlIHRyYWluaW5nU3RhdGU6IExvcmFUcmFpbmluZ1N0YXRlIHwgbnVsbCA9IG51bGw7XG4gIHByaXZhdGUgZnJvemVuOiBib29sZWFuID0gZmFsc2U7XG5cbiAgY29uc3RydWN0b3IoY29uZmlnPzogUGFydGlhbDxMb1JBQ29uZmlnPiwgaW5wdXREaW0gPSAyNTYsIG91dHB1dERpbSA9IDI1Nikge1xuICAgIHRoaXMuY29uZmlnID0geyAuLi5ERUZBVUxUX0xPUkFfQ09ORklHLCAuLi5jb25maWcgfTtcbiAgICB0aGlzLmlucHV0RGltID0gaW5wdXREaW07XG4gICAgdGhpcy5vdXRwdXREaW0gPSBvdXRwdXREaW07XG5cbiAgICAvLyBJbml0aWFsaXplIHdlaWdodHNcbiAgICB0aGlzLndlaWdodHMgPSB0aGlzLmluaXRpYWxpemVXZWlnaHRzKCk7XG4gIH1cblxuICAvKipcbiAgICogRm9yd2FyZCBwYXNzIHRocm91Z2ggTG9SQSBhZGFwdGVyXG4gICAqIE9QVElNSVpFRDogVXNlcyBGbG9hdDY0QXJyYXkgYW5kIGxvb3AgdW5yb2xsaW5nXG4gICAqXG4gICAqIG91dHB1dCA9IGlucHV0ICsgc2NhbGluZyAqIChpbnB1dCBAIEEgQCBCKVxuICAgKi9cbiAgZm9yd2FyZChpbnB1dDogbnVtYmVyW10pOiBudW1iZXJbXSB7XG4gICAgY29uc3QgcmFuayA9IHRoaXMuY29uZmlnLnJhbms7XG4gICAgY29uc3QgZGltID0gTWF0aC5taW4oaW5wdXQubGVuZ3RoLCB0aGlzLmlucHV0RGltKTtcbiAgICBjb25zdCBzY2FsaW5nID0gdGhpcy53ZWlnaHRzLnNjYWxpbmc7XG5cbiAgICAvLyBBcHBseSBkcm9wb3V0IGR1cmluZyB0cmFpbmluZyAoc2ltcGxpZmllZCBjaGVjaylcbiAgICBjb25zdCBhcHBseURyb3BvdXQgPSB0aGlzLnRyYWluaW5nU3RhdGUgIT09IG51bGwgJiYgdGhpcy5jb25maWcuZHJvcG91dCA+IDA7XG5cbiAgICAvLyBpbnB1dCBAIEEgKGQgLT4gcikgLSB1c2UgdHlwZWQgYXJyYXkgZm9yIGhpZGRlblxuICAgIGNvbnN0IGhpZGRlbiA9IG5ldyBGbG9hdDY0QXJyYXkocmFuayk7XG4gICAgZm9yIChsZXQgciA9IDA7IHIgPCByYW5rOyByKyspIHtcbiAgICAgIGxldCBzdW0gPSAwO1xuICAgICAgY29uc3QgbG9yYUFDb2wgPSB0aGlzLndlaWdodHMubG9yYUE7XG4gICAgICAvLyBVbnJvbGwgbG9vcCBmb3IgYmV0dGVyIHBlcmZvcm1hbmNlXG4gICAgICBsZXQgaSA9IDA7XG4gICAgICBpZiAoYXBwbHlEcm9wb3V0KSB7XG4gICAgICAgIGZvciAoOyBpIDwgZGltOyBpKyspIHtcbiAgICAgICAgICBpZiAoTWF0aC5yYW5kb20oKSA+IHRoaXMuY29uZmlnLmRyb3BvdXQpIHtcbiAgICAgICAgICAgIHN1bSArPSBpbnB1dFtpXSAqIGxvcmFBQ29sW2ldW3JdO1xuICAgICAgICAgIH1cbiAgICAgICAgfVxuICAgICAgfSBlbHNlIHtcbiAgICAgICAgZm9yICg7IGkgKyAzIDwgZGltOyBpICs9IDQpIHtcbiAgICAgICAgICBzdW0gKz0gaW5wdXRbaV0gKiBsb3JhQUNvbFtpXVtyXSArXG4gICAgICAgICAgICAgICAgIGlucHV0W2kgKyAxXSAqIGxvcmFBQ29sW2kgKyAxXVtyXSArXG4gICAgICAgICAgICAgICAgIGlucHV0W2kgKyAyXSAqIGxvcmFBQ29sW2kgKyAyXVtyXSArXG4gICAgICAgICAgICAgICAgIGlucHV0W2kgKyAzXSAqIGxvcmFBQ29sW2kgKyAzXVtyXTtcbiAgICAgICAgfVxuICAgICAgICBmb3IgKDsgaSA8IGRpbTsgaSsrKSB7XG4gICAgICAgICAgc3VtICs9IGlucHV0W2ldICogbG9yYUFDb2xbaV1bcl07XG4gICAgICAgIH1cbiAgICAgIH1cbiAgICAgIGhpZGRlbltyXSA9IHN1bTtcbiAgICB9XG5cbiAgICAvLyBoaWRkZW4gQCBCIChyIC0+IGQpICsgcmVzaWR1YWxcbiAgICBjb25zdCBvdXRwdXQgPSBuZXcgQXJyYXkodGhpcy5vdXRwdXREaW0pO1xuICAgIGNvbnN0IGxvcmFCID0gdGhpcy53ZWlnaHRzLmxvcmFCO1xuICAgIGZvciAobGV0IGkgPSAwOyBpIDwgdGhpcy5vdXRwdXREaW07IGkrKykge1xuICAgICAgbGV0IGRlbHRhID0gMDtcbiAgICAgIGZvciAobGV0IHIgPSAwOyByIDwgcmFuazsgcisrKSB7XG4gICAgICAgIGRlbHRhICs9IGhpZGRlbltyXSAqIGxvcmFCW3JdW2ldO1xuICAgICAgfVxuICAgICAgLy8gQWRkIHNjYWxlZCBkZWx0YSB0byBpbnB1dCAocmVzaWR1YWwgY29ubmVjdGlvbilcbiAgICAgIG91dHB1dFtpXSA9IChpbnB1dFtpXSB8fCAwKSArIHNjYWxpbmcgKiBkZWx0YTtcbiAgICB9XG5cbiAgICByZXR1cm4gb3V0cHV0O1xuICB9XG5cbiAgLyoqXG4gICAqIEZvcndhcmQgd2l0aCBiYXRjaCBwcm9jZXNzaW5nXG4gICAqL1xuICBmb3J3YXJkQmF0Y2goaW5wdXRzOiBudW1iZXJbXVtdKTogbnVtYmVyW11bXSB7XG4gICAgcmV0dXJuIGlucHV0cy5tYXAoaW5wdXQgPT4gdGhpcy5mb3J3YXJkKGlucHV0KSk7XG4gIH1cblxuICAvKipcbiAgICogQmFja3dhcmQgcGFzcyBhbmQgd2VpZ2h0IHVwZGF0ZVxuICAgKi9cbiAgYmFja3dhcmQoaW5wdXQ6IG51bWJlcltdLCBncmFkT3V0cHV0OiBudW1iZXJbXSwgbGVhcm5pbmdSYXRlOiBudW1iZXIpOiBudW1iZXIge1xuICAgIGlmICh0aGlzLmZyb3plbikgcmV0dXJuIDA7XG5cbiAgICBjb25zdCByYW5rID0gdGhpcy5jb25maWcucmFuaztcbiAgICBjb25zdCBkaW0gPSBNYXRoLm1pbihpbnB1dC5sZW5ndGgsIHRoaXMuaW5wdXREaW0pO1xuXG4gICAgLy8gQ29tcHV0ZSBoaWRkZW4gYWN0aXZhdGlvbnMgKGZvciBncmFkaWVudClcbiAgICBjb25zdCBoaWRkZW4gPSBuZXcgQXJyYXkocmFuaykuZmlsbCgwKTtcbiAgICBmb3IgKGxldCByID0gMDsgciA8IHJhbms7IHIrKykge1xuICAgICAgZm9yIChsZXQgaSA9IDA7IGkgPCBkaW07IGkrKykge1xuICAgICAgICBoaWRkZW5bcl0gKz0gaW5wdXRbaV0gKiB0aGlzLndlaWdodHMubG9yYUFbaV1bcl07XG4gICAgICB9XG4gICAgfVxuXG4gICAgLy8gR3JhZGllbnQgZm9yIEI6IGhpZGRlbl5UIEAgZ3JhZE91dHB1dFxuICAgIGNvbnN0IGdyYWRCOiBudW1iZXJbXVtdID0gQXJyYXkocmFuaykuZmlsbChudWxsKS5tYXAoKCkgPT4gQXJyYXkodGhpcy5vdXRwdXREaW0pLmZpbGwoMCkpO1xuICAgIGZvciAobGV0IHIgPSAwOyByIDwgcmFuazsgcisrKSB7XG4gICAgICBmb3IgKGxldCBpID0gMDsgaSA8IHRoaXMub3V0cHV0RGltOyBpKyspIHtcbiAgICAgICAgZ3JhZEJbcl1baV0gPSBoaWRkZW5bcl0gKiAoZ3JhZE91dHB1dFtpXSB8fCAwKSAqIHRoaXMud2VpZ2h0cy5zY2FsaW5nO1xuICAgICAgfVxuICAgIH1cblxuICAgIC8vIEdyYWRpZW50IGZvciBoaWRkZW46IGdyYWRPdXRwdXQgQCBCXlRcbiAgICBjb25zdCBncmFkSGlkZGVuID0gbmV3IEFycmF5KHJhbmspLmZpbGwoMCk7XG4gICAgZm9yIChsZXQgciA9IDA7IHIgPCByYW5rOyByKyspIHtcbiAgICAgIGZvciAobGV0IGkgPSAwOyBpIDwgdGhpcy5vdXRwdXREaW07IGkrKykge1xuICAgICAgICBncmFkSGlkZGVuW3JdICs9IChncmFkT3V0cHV0W2ldIHx8IDApICogdGhpcy53ZWlnaHRzLmxvcmFCW3JdW2ldICogdGhpcy53ZWlnaHRzLnNjYWxpbmc7XG4gICAgICB9XG4gICAgfVxuXG4gICAgLy8gR3JhZGllbnQgZm9yIEE6IGlucHV0XlQgQCBncmFkSGlkZGVuXG4gICAgY29uc3QgZ3JhZEE6IG51bWJlcltdW10gPSBBcnJheShkaW0pLmZpbGwobnVsbCkubWFwKCgpID0+IEFycmF5KHJhbmspLmZpbGwoMCkpO1xuICAgIGZvciAobGV0IGkgPSAwOyBpIDwgZGltOyBpKyspIHtcbiAgICAgIGZvciAobGV0IHIgPSAwOyByIDwgcmFuazsgcisrKSB7XG4gICAgICAgIGdyYWRBW2ldW3JdID0gaW5wdXRbaV0gKiBncmFkSGlkZGVuW3JdO1xuICAgICAgfVxuICAgIH1cblxuICAgIC8vIFVwZGF0ZSB3ZWlnaHRzXG4gICAgbGV0IHRvdGFsR3JhZCA9IDA7XG4gICAgZm9yIChsZXQgaSA9IDA7IGkgPCBkaW07IGkrKykge1xuICAgICAgZm9yIChsZXQgciA9IDA7IHIgPCByYW5rOyByKyspIHtcbiAgICAgICAgdGhpcy53ZWlnaHRzLmxvcmFBW2ldW3JdIC09IGxlYXJuaW5nUmF0ZSAqIGdyYWRBW2ldW3JdO1xuICAgICAgICB0b3RhbEdyYWQgKz0gTWF0aC5hYnMoZ3JhZEFbaV1bcl0pO1xuICAgICAgfVxuICAgIH1cbiAgICBmb3IgKGxldCByID0gMDsgciA8IHJhbms7IHIrKykge1xuICAgICAgZm9yIChsZXQgaSA9IDA7IGkgPCB0aGlzLm91dHB1dERpbTsgaSsrKSB7XG4gICAgICAgIHRoaXMud2VpZ2h0cy5sb3JhQltyXVtpXSAtPSBsZWFybmluZ1JhdGUgKiBncmFkQltyXVtpXTtcbiAgICAgICAgdG90YWxHcmFkICs9IE1hdGguYWJzKGdyYWRCW3JdW2ldKTtcbiAgICAgIH1cbiAgICB9XG5cbiAgICAvLyBUcmFjayB0cmFpbmluZyBzdGF0ZVxuICAgIGlmICh0aGlzLnRyYWluaW5nU3RhdGUpIHtcbiAgICAgIHRoaXMudHJhaW5pbmdTdGF0ZS5zdGVwKys7XG4gICAgICB0aGlzLnRyYWluaW5nU3RhdGUubG9zc0hpc3RvcnkucHVzaCh0b3RhbEdyYWQpO1xuICAgIH1cblxuICAgIHJldHVybiB0b3RhbEdyYWQ7XG4gIH1cblxuICAvKipcbiAgICogU3RhcnQgdHJhaW5pbmcgbW9kZVxuICAgKi9cbiAgc3RhcnRUcmFpbmluZyhsZWFybmluZ1JhdGUgPSAwLjAwMSk6IHZvaWQge1xuICAgIHRoaXMudHJhaW5pbmdTdGF0ZSA9IHtcbiAgICAgIHN0ZXA6IDAsXG4gICAgICBsZWFybmluZ1JhdGUsXG4gICAgICBncmFkQTogQXJyYXkodGhpcy5pbnB1dERpbSkuZmlsbChudWxsKS5tYXAoKCkgPT4gQXJyYXkodGhpcy5jb25maWcucmFuaykuZmlsbCgwKSksXG4gICAgICBncmFkQjogQXJyYXkodGhpcy5jb25maWcucmFuaykuZmlsbChudWxsKS5tYXAoKCkgPT4gQXJyYXkodGhpcy5vdXRwdXREaW0pLmZpbGwoMCkpLFxuICAgICAgbG9zc0hpc3Rvcnk6IFtdLFxuICAgIH07XG4gIH1cblxuICAvKipcbiAgICogRW5kIHRyYWluaW5nIG1vZGVcbiAgICovXG4gIGVuZFRyYWluaW5nKCk6IExvcmFUcmFpbmluZ1N0YXRlIHwgbnVsbCB7XG4gICAgY29uc3Qgc3RhdGUgPSB0aGlzLnRyYWluaW5nU3RhdGU7XG4gICAgdGhpcy50cmFpbmluZ1N0YXRlID0gbnVsbDtcbiAgICByZXR1cm4gc3RhdGU7XG4gIH1cblxuICAvKipcbiAgICogRnJlZXplIGFkYXB0ZXIgKG5vIG1vcmUgdXBkYXRlcylcbiAgICovXG4gIGZyZWV6ZSgpOiB2b2lkIHtcbiAgICB0aGlzLmZyb3plbiA9IHRydWU7XG4gIH1cblxuICAvKipcbiAgICogVW5mcmVlemUgYWRhcHRlclxuICAgKi9cbiAgdW5mcmVlemUoKTogdm9pZCB7XG4gICAgdGhpcy5mcm96ZW4gPSBmYWxzZTtcbiAgfVxuXG4gIC8qKlxuICAgKiBDaGVjayBpZiBmcm96ZW5cbiAgICovXG4gIGlzRnJvemVuKCk6IGJvb2xlYW4ge1xuICAgIHJldHVybiB0aGlzLmZyb3plbjtcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgYWRhcHRlciBjb25maWdcbiAgICovXG4gIGdldENvbmZpZygpOiBSZXF1aXJlZDxMb1JBQ29uZmlnPiB7XG4gICAgcmV0dXJuIHsgLi4udGhpcy5jb25maWcgfTtcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgYWRhcHRlciB3ZWlnaHRzXG4gICAqL1xuICBnZXRXZWlnaHRzKCk6IExvcmFXZWlnaHRzIHtcbiAgICByZXR1cm4ge1xuICAgICAgbG9yYUE6IHRoaXMud2VpZ2h0cy5sb3JhQS5tYXAocm93ID0+IFsuLi5yb3ddKSxcbiAgICAgIGxvcmFCOiB0aGlzLndlaWdodHMubG9yYUIubWFwKHJvdyA9PiBbLi4ucm93XSksXG4gICAgICBzY2FsaW5nOiB0aGlzLndlaWdodHMuc2NhbGluZyxcbiAgICB9O1xuICB9XG5cbiAgLyoqXG4gICAqIFNldCBhZGFwdGVyIHdlaWdodHNcbiAgICovXG4gIHNldFdlaWdodHMod2VpZ2h0czogTG9yYVdlaWdodHMpOiB2b2lkIHtcbiAgICB0aGlzLndlaWdodHMgPSB7XG4gICAgICBsb3JhQTogd2VpZ2h0cy5sb3JhQS5tYXAocm93ID0+IFsuLi5yb3ddKSxcbiAgICAgIGxvcmFCOiB3ZWlnaHRzLmxvcmFCLm1hcChyb3cgPT4gWy4uLnJvd10pLFxuICAgICAgc2NhbGluZzogd2VpZ2h0cy5zY2FsaW5nLFxuICAgIH07XG4gIH1cblxuICAvKipcbiAgICogTWVyZ2UgYWRhcHRlciBpbnRvIGJhc2Ugd2VpZ2h0c1xuICAgKlxuICAgKiBSZXR1cm5zIGRlbHRhIHRvIGFkZCB0byBiYXNlIG1vZGVsIHdlaWdodHNcbiAgICovXG4gIG1lcmdlKCk6IG51bWJlcltdW10ge1xuICAgIGNvbnN0IGRlbHRhOiBudW1iZXJbXVtdID0gQXJyYXkodGhpcy5pbnB1dERpbSlcbiAgICAgIC5maWxsKG51bGwpXG4gICAgICAubWFwKCgpID0+IEFycmF5KHRoaXMub3V0cHV0RGltKS5maWxsKDApKTtcblxuICAgIGNvbnN0IHJhbmsgPSB0aGlzLmNvbmZpZy5yYW5rO1xuICAgIGZvciAobGV0IGkgPSAwOyBpIDwgdGhpcy5pbnB1dERpbTsgaSsrKSB7XG4gICAgICBmb3IgKGxldCBqID0gMDsgaiA8IHRoaXMub3V0cHV0RGltOyBqKyspIHtcbiAgICAgICAgZm9yIChsZXQgciA9IDA7IHIgPCByYW5rOyByKyspIHtcbiAgICAgICAgICBkZWx0YVtpXVtqXSArPSB0aGlzLndlaWdodHMubG9yYUFbaV1bcl0gKiB0aGlzLndlaWdodHMubG9yYUJbcl1bal07XG4gICAgICAgIH1cbiAgICAgICAgZGVsdGFbaV1bal0gKj0gdGhpcy53ZWlnaHRzLnNjYWxpbmc7XG4gICAgICB9XG4gICAgfVxuXG4gICAgcmV0dXJuIGRlbHRhO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCBudW1iZXIgb2YgdHJhaW5hYmxlIHBhcmFtZXRlcnNcbiAgICovXG4gIG51bVBhcmFtZXRlcnMoKTogbnVtYmVyIHtcbiAgICByZXR1cm4gKHRoaXMuaW5wdXREaW0gKiB0aGlzLmNvbmZpZy5yYW5rKSArICh0aGlzLmNvbmZpZy5yYW5rICogdGhpcy5vdXRwdXREaW0pO1xuICB9XG5cbiAgLyoqXG4gICAqIFJlc2V0IHRvIGluaXRpYWwgd2VpZ2h0c1xuICAgKi9cbiAgcmVzZXQoKTogdm9pZCB7XG4gICAgdGhpcy53ZWlnaHRzID0gdGhpcy5pbml0aWFsaXplV2VpZ2h0cygpO1xuICAgIHRoaXMudHJhaW5pbmdTdGF0ZSA9IG51bGw7XG4gICAgdGhpcy5mcm96ZW4gPSBmYWxzZTtcbiAgfVxuXG4gIC8qKlxuICAgKiBDbG9uZSBhZGFwdGVyXG4gICAqL1xuICBjbG9uZSgpOiBMb3JhQWRhcHRlciB7XG4gICAgY29uc3QgYWRhcHRlciA9IG5ldyBMb3JhQWRhcHRlcih0aGlzLmNvbmZpZywgdGhpcy5pbnB1dERpbSwgdGhpcy5vdXRwdXREaW0pO1xuICAgIGFkYXB0ZXIuc2V0V2VpZ2h0cyh0aGlzLmdldFdlaWdodHMoKSk7XG4gICAgcmV0dXJuIGFkYXB0ZXI7XG4gIH1cblxuICAvKipcbiAgICogU2VyaWFsaXplIHRvIEpTT05cbiAgICovXG4gIHRvSlNPTigpOiBzdHJpbmcge1xuICAgIHJldHVybiBKU09OLnN0cmluZ2lmeSh7XG4gICAgICBjb25maWc6IHRoaXMuY29uZmlnLFxuICAgICAgaW5wdXREaW06IHRoaXMuaW5wdXREaW0sXG4gICAgICBvdXRwdXREaW06IHRoaXMub3V0cHV0RGltLFxuICAgICAgd2VpZ2h0czogdGhpcy53ZWlnaHRzLFxuICAgICAgZnJvemVuOiB0aGlzLmZyb3plbixcbiAgICB9KTtcbiAgfVxuXG4gIC8qKlxuICAgKiBEZXNlcmlhbGl6ZSBmcm9tIEpTT05cbiAgICovXG4gIHN0YXRpYyBmcm9tSlNPTihqc29uOiBzdHJpbmcpOiBMb3JhQWRhcHRlciB7XG4gICAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UoanNvbik7XG4gICAgY29uc3QgYWRhcHRlciA9IG5ldyBMb3JhQWRhcHRlcihkYXRhLmNvbmZpZywgZGF0YS5pbnB1dERpbSwgZGF0YS5vdXRwdXREaW0pO1xuICAgIGFkYXB0ZXIuc2V0V2VpZ2h0cyhkYXRhLndlaWdodHMpO1xuICAgIGlmIChkYXRhLmZyb3plbikgYWRhcHRlci5mcmVlemUoKTtcbiAgICByZXR1cm4gYWRhcHRlcjtcbiAgfVxuXG4gIHByaXZhdGUgaW5pdGlhbGl6ZVdlaWdodHMoKTogTG9yYVdlaWdodHMge1xuICAgIGNvbnN0IHJhbmsgPSB0aGlzLmNvbmZpZy5yYW5rO1xuXG4gICAgLy8gS2FpbWluZyBpbml0aWFsaXphdGlvbiBmb3IgQSwgemVybyBpbml0aWFsaXphdGlvbiBmb3IgQlxuICAgIGNvbnN0IGxvcmFBOiBudW1iZXJbXVtdID0gQXJyYXkodGhpcy5pbnB1dERpbSlcbiAgICAgIC5maWxsKG51bGwpXG4gICAgICAubWFwKCgpID0+XG4gICAgICAgIEFycmF5KHJhbmspXG4gICAgICAgICAgLmZpbGwoMClcbiAgICAgICAgICAubWFwKCgpID0+IChNYXRoLnJhbmRvbSgpIC0gMC41KSAqIE1hdGguc3FydCgyIC8gdGhpcy5pbnB1dERpbSkpXG4gICAgICApO1xuXG4gICAgY29uc3QgbG9yYUI6IG51bWJlcltdW10gPSBBcnJheShyYW5rKVxuICAgICAgLmZpbGwobnVsbClcbiAgICAgIC5tYXAoKCkgPT4gQXJyYXkodGhpcy5vdXRwdXREaW0pLmZpbGwoMCkpO1xuXG4gICAgcmV0dXJuIHtcbiAgICAgIGxvcmFBLFxuICAgICAgbG9yYUIsXG4gICAgICBzY2FsaW5nOiB0aGlzLmNvbmZpZy5hbHBoYSAvIHRoaXMuY29uZmlnLnJhbmssXG4gICAgfTtcbiAgfVxufVxuXG4vKipcbiAqIExvUkEgTWFuYWdlciBmb3IgbXVsdGlwbGUgYWRhcHRlcnNcbiAqXG4gKiBNYW5hZ2VzIGEgY29sbGVjdGlvbiBvZiBMb1JBIGFkYXB0ZXJzIGZvciBkaWZmZXJlbnQgdGFza3MvZG9tYWlucy5cbiAqL1xuZXhwb3J0IGNsYXNzIExvcmFNYW5hZ2VyIHtcbiAgcHJpdmF0ZSBhZGFwdGVyczogTWFwPHN0cmluZywgTG9yYUFkYXB0ZXI+ID0gbmV3IE1hcCgpO1xuICBwcml2YXRlIGFjdGl2ZUFkYXB0ZXJJZDogc3RyaW5nIHwgbnVsbCA9IG51bGw7XG4gIHByaXZhdGUgZGVmYXVsdENvbmZpZzogUmVxdWlyZWQ8TG9SQUNvbmZpZz47XG5cbiAgY29uc3RydWN0b3IoZGVmYXVsdENvbmZpZz86IFBhcnRpYWw8TG9SQUNvbmZpZz4pIHtcbiAgICB0aGlzLmRlZmF1bHRDb25maWcgPSB7IC4uLkRFRkFVTFRfTE9SQV9DT05GSUcsIC4uLmRlZmF1bHRDb25maWcgfTtcbiAgfVxuXG4gIC8qKlxuICAgKiBSZWdpc3RlciBhIG5ldyBhZGFwdGVyXG4gICAqL1xuICByZWdpc3RlcihpZDogc3RyaW5nLCBhZGFwdGVyOiBMb3JhQWRhcHRlcik6IHZvaWQge1xuICAgIHRoaXMuYWRhcHRlcnMuc2V0KGlkLCBhZGFwdGVyKTtcbiAgfVxuXG4gIC8qKlxuICAgKiBDcmVhdGUgYW5kIHJlZ2lzdGVyIGEgbmV3IGFkYXB0ZXJcbiAgICovXG4gIGNyZWF0ZShpZDogc3RyaW5nLCBjb25maWc/OiBQYXJ0aWFsPExvUkFDb25maWc+LCBpbnB1dERpbT86IG51bWJlciwgb3V0cHV0RGltPzogbnVtYmVyKTogTG9yYUFkYXB0ZXIge1xuICAgIGNvbnN0IG1lcmdlZENvbmZpZyA9IHsgLi4udGhpcy5kZWZhdWx0Q29uZmlnLCAuLi5jb25maWcgfTtcbiAgICBjb25zdCBhZGFwdGVyID0gbmV3IExvcmFBZGFwdGVyKG1lcmdlZENvbmZpZywgaW5wdXREaW0sIG91dHB1dERpbSk7XG4gICAgdGhpcy5yZWdpc3RlcihpZCwgYWRhcHRlcik7XG4gICAgcmV0dXJuIGFkYXB0ZXI7XG4gIH1cblxuICAvKipcbiAgICogR2V0IGFkYXB0ZXIgYnkgSURcbiAgICovXG4gIGdldChpZDogc3RyaW5nKTogTG9yYUFkYXB0ZXIgfCB1bmRlZmluZWQge1xuICAgIHJldHVybiB0aGlzLmFkYXB0ZXJzLmdldChpZCk7XG4gIH1cblxuICAvKipcbiAgICogUmVtb3ZlIGFkYXB0ZXJcbiAgICovXG4gIHJlbW92ZShpZDogc3RyaW5nKTogYm9vbGVhbiB7XG4gICAgaWYgKHRoaXMuYWN0aXZlQWRhcHRlcklkID09PSBpZCkge1xuICAgICAgdGhpcy5hY3RpdmVBZGFwdGVySWQgPSBudWxsO1xuICAgIH1cbiAgICByZXR1cm4gdGhpcy5hZGFwdGVycy5kZWxldGUoaWQpO1xuICB9XG5cbiAgLyoqXG4gICAqIEFjdGl2YXRlIGFuIGFkYXB0ZXJcbiAgICovXG4gIGFjdGl2YXRlKGlkOiBzdHJpbmcpOiBib29sZWFuIHtcbiAgICBpZiAodGhpcy5hZGFwdGVycy5oYXMoaWQpKSB7XG4gICAgICB0aGlzLmFjdGl2ZUFkYXB0ZXJJZCA9IGlkO1xuICAgICAgcmV0dXJuIHRydWU7XG4gICAgfVxuICAgIHJldHVybiBmYWxzZTtcbiAgfVxuXG4gIC8qKlxuICAgKiBEZWFjdGl2YXRlIGN1cnJlbnQgYWRhcHRlclxuICAgKi9cbiAgZGVhY3RpdmF0ZSgpOiB2b2lkIHtcbiAgICB0aGlzLmFjdGl2ZUFkYXB0ZXJJZCA9IG51bGw7XG4gIH1cblxuICAvKipcbiAgICogR2V0IGFjdGl2ZSBhZGFwdGVyXG4gICAqL1xuICBnZXRBY3RpdmUoKTogTG9yYUFkYXB0ZXIgfCBudWxsIHtcbiAgICByZXR1cm4gdGhpcy5hY3RpdmVBZGFwdGVySWQgPyB0aGlzLmFkYXB0ZXJzLmdldCh0aGlzLmFjdGl2ZUFkYXB0ZXJJZCkgfHwgbnVsbCA6IG51bGw7XG4gIH1cblxuICAvKipcbiAgICogR2V0IGFjdGl2ZSBhZGFwdGVyIElEXG4gICAqL1xuICBnZXRBY3RpdmVJZCgpOiBzdHJpbmcgfCBudWxsIHtcbiAgICByZXR1cm4gdGhpcy5hY3RpdmVBZGFwdGVySWQ7XG4gIH1cblxuICAvKipcbiAgICogQXBwbHkgYWN0aXZlIGFkYXB0ZXJcbiAgICovXG4gIGZvcndhcmQoaW5wdXQ6IG51bWJlcltdKTogbnVtYmVyW10ge1xuICAgIGNvbnN0IGFjdGl2ZSA9IHRoaXMuZ2V0QWN0aXZlKCk7XG4gICAgcmV0dXJuIGFjdGl2ZSA/IGFjdGl2ZS5mb3J3YXJkKGlucHV0KSA6IFsuLi5pbnB1dF07XG4gIH1cblxuICAvKipcbiAgICogTGlzdCBhbGwgYWRhcHRlciBJRHNcbiAgICovXG4gIGxpc3QoKTogc3RyaW5nW10ge1xuICAgIHJldHVybiBBcnJheS5mcm9tKHRoaXMuYWRhcHRlcnMua2V5cygpKTtcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgYWRhcHRlciBjb3VudFxuICAgKi9cbiAgY291bnQoKTogbnVtYmVyIHtcbiAgICByZXR1cm4gdGhpcy5hZGFwdGVycy5zaXplO1xuICB9XG5cbiAgLyoqXG4gICAqIEZyZWV6ZSBhbGwgYWRhcHRlcnNcbiAgICovXG4gIGZyZWV6ZUFsbCgpOiB2b2lkIHtcbiAgICBmb3IgKGNvbnN0IGFkYXB0ZXIgb2YgdGhpcy5hZGFwdGVycy52YWx1ZXMoKSkge1xuICAgICAgYWRhcHRlci5mcmVlemUoKTtcbiAgICB9XG4gIH1cblxuICAvKipcbiAgICogVW5mcmVlemUgYWxsIGFkYXB0ZXJzXG4gICAqL1xuICB1bmZyZWV6ZUFsbCgpOiB2b2lkIHtcbiAgICBmb3IgKGNvbnN0IGFkYXB0ZXIgb2YgdGhpcy5hZGFwdGVycy52YWx1ZXMoKSkge1xuICAgICAgYWRhcHRlci51bmZyZWV6ZSgpO1xuICAgIH1cbiAgfVxuXG4gIC8qKlxuICAgKiBNZXJnZSBtdWx0aXBsZSBhZGFwdGVycyBpbnRvIG9uZVxuICAgKi9cbiAgbWVyZ2VBZGFwdGVycyhpZHM6IHN0cmluZ1tdLCBvdXRwdXRJZDogc3RyaW5nKTogTG9yYUFkYXB0ZXIgfCBudWxsIHtcbiAgICBjb25zdCBhZGFwdGVycyA9IGlkcy5tYXAoaWQgPT4gdGhpcy5hZGFwdGVycy5nZXQoaWQpKS5maWx0ZXIoQm9vbGVhbikgYXMgTG9yYUFkYXB0ZXJbXTtcbiAgICBpZiAoYWRhcHRlcnMubGVuZ3RoID09PSAwKSByZXR1cm4gbnVsbDtcblxuICAgIC8vIFVzZSBmaXJzdCBhZGFwdGVyIGFzIGJhc2VcbiAgICBjb25zdCBtZXJnZWQgPSBhZGFwdGVyc1swXS5jbG9uZSgpO1xuICAgIGNvbnN0IHdlaWdodHMgPSBtZXJnZWQuZ2V0V2VpZ2h0cygpO1xuXG4gICAgLy8gQXZlcmFnZSB3ZWlnaHRzIGZyb20gb3RoZXIgYWRhcHRlcnNcbiAgICBmb3IgKGxldCBpID0gMTsgaSA8IGFkYXB0ZXJzLmxlbmd0aDsgaSsrKSB7XG4gICAgICBjb25zdCBvdGhlcldlaWdodHMgPSBhZGFwdGVyc1tpXS5nZXRXZWlnaHRzKCk7XG5cbiAgICAgIGZvciAobGV0IHJvdyA9IDA7IHJvdyA8IHdlaWdodHMubG9yYUEubGVuZ3RoICYmIHJvdyA8IG90aGVyV2VpZ2h0cy5sb3JhQS5sZW5ndGg7IHJvdysrKSB7XG4gICAgICAgIGZvciAobGV0IGNvbCA9IDA7IGNvbCA8IHdlaWdodHMubG9yYUFbcm93XS5sZW5ndGggJiYgY29sIDwgb3RoZXJXZWlnaHRzLmxvcmFBW3Jvd10ubGVuZ3RoOyBjb2wrKykge1xuICAgICAgICAgIHdlaWdodHMubG9yYUFbcm93XVtjb2xdID0gKHdlaWdodHMubG9yYUFbcm93XVtjb2xdICsgb3RoZXJXZWlnaHRzLmxvcmFBW3Jvd11bY29sXSkgLyAyO1xuICAgICAgICB9XG4gICAgICB9XG4gICAgICBmb3IgKGxldCByb3cgPSAwOyByb3cgPCB3ZWlnaHRzLmxvcmFCLmxlbmd0aCAmJiByb3cgPCBvdGhlcldlaWdodHMubG9yYUIubGVuZ3RoOyByb3crKykge1xuICAgICAgICBmb3IgKGxldCBjb2wgPSAwOyBjb2wgPCB3ZWlnaHRzLmxvcmFCW3Jvd10ubGVuZ3RoICYmIGNvbCA8IG90aGVyV2VpZ2h0cy5sb3JhQltyb3ddLmxlbmd0aDsgY29sKyspIHtcbiAgICAgICAgICB3ZWlnaHRzLmxvcmFCW3Jvd11bY29sXSA9ICh3ZWlnaHRzLmxvcmFCW3Jvd11bY29sXSArIG90aGVyV2VpZ2h0cy5sb3JhQltyb3ddW2NvbF0pIC8gMjtcbiAgICAgICAgfVxuICAgICAgfVxuICAgIH1cblxuICAgIG1lcmdlZC5zZXRXZWlnaHRzKHdlaWdodHMpO1xuICAgIHRoaXMucmVnaXN0ZXIob3V0cHV0SWQsIG1lcmdlZCk7XG4gICAgcmV0dXJuIG1lcmdlZDtcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgc3RhdGlzdGljc1xuICAgKi9cbiAgc3RhdHMoKToge1xuICAgIHRvdGFsQWRhcHRlcnM6IG51bWJlcjtcbiAgICBhY3RpdmVBZGFwdGVyOiBzdHJpbmcgfCBudWxsO1xuICAgIHRvdGFsUGFyYW1ldGVyczogbnVtYmVyO1xuICAgIGZyb3plbkNvdW50OiBudW1iZXI7XG4gIH0ge1xuICAgIGxldCB0b3RhbFBhcmFtcyA9IDA7XG4gICAgbGV0IGZyb3plbkNvdW50ID0gMDtcblxuICAgIGZvciAoY29uc3QgYWRhcHRlciBvZiB0aGlzLmFkYXB0ZXJzLnZhbHVlcygpKSB7XG4gICAgICB0b3RhbFBhcmFtcyArPSBhZGFwdGVyLm51bVBhcmFtZXRlcnMoKTtcbiAgICAgIGlmIChhZGFwdGVyLmlzRnJvemVuKCkpIGZyb3plbkNvdW50Kys7XG4gICAgfVxuXG4gICAgcmV0dXJuIHtcbiAgICAgIHRvdGFsQWRhcHRlcnM6IHRoaXMuYWRhcHRlcnMuc2l6ZSxcbiAgICAgIGFjdGl2ZUFkYXB0ZXI6IHRoaXMuYWN0aXZlQWRhcHRlcklkLFxuICAgICAgdG90YWxQYXJhbWV0ZXJzOiB0b3RhbFBhcmFtcyxcbiAgICAgIGZyb3plbkNvdW50LFxuICAgIH07XG4gIH1cblxuICAvKipcbiAgICogQ2xlYXIgYWxsIGFkYXB0ZXJzXG4gICAqL1xuICBjbGVhcigpOiB2b2lkIHtcbiAgICB0aGlzLmFkYXB0ZXJzLmNsZWFyKCk7XG4gICAgdGhpcy5hY3RpdmVBZGFwdGVySWQgPSBudWxsO1xuICB9XG59XG4iXX0=