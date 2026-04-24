"use strict";
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
Object.defineProperty(exports, "__esModule", { value: true });
exports.LoraManager = exports.LoraAdapter = void 0;
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
class LoraAdapter {
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
exports.LoraAdapter = LoraAdapter;
/**
 * LoRA Manager for multiple adapters
 *
 * Manages a collection of LoRA adapters for different tasks/domains.
 */
class LoraManager {
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
exports.LoraManager = LoraManager;
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoibG9yYS5qcyIsInNvdXJjZVJvb3QiOiIiLCJzb3VyY2VzIjpbIi4uLy4uL3NyYy9sb3JhLnRzIl0sIm5hbWVzIjpbXSwibWFwcGluZ3MiOiI7QUFBQTs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7R0EwQkc7OztBQUlIOztHQUVHO0FBQ0gsTUFBTSxtQkFBbUIsR0FBeUI7SUFDaEQsSUFBSSxFQUFFLENBQUM7SUFDUCxLQUFLLEVBQUUsRUFBRTtJQUNULE9BQU8sRUFBRSxHQUFHO0lBQ1osYUFBYSxFQUFFLENBQUMsT0FBTyxFQUFFLE9BQU8sQ0FBQztDQUNsQyxDQUFDO0FBOEJGOzs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7R0FxQkc7QUFDSCxNQUFhLFdBQVc7SUFRdEIsWUFBWSxNQUE0QixFQUFFLFFBQVEsR0FBRyxHQUFHLEVBQUUsU0FBUyxHQUFHLEdBQUc7UUFIakUsa0JBQWEsR0FBNkIsSUFBSSxDQUFDO1FBQy9DLFdBQU0sR0FBWSxLQUFLLENBQUM7UUFHOUIsSUFBSSxDQUFDLE1BQU0sR0FBRyxFQUFFLEdBQUcsbUJBQW1CLEVBQUUsR0FBRyxNQUFNLEVBQUUsQ0FBQztRQUNwRCxJQUFJLENBQUMsUUFBUSxHQUFHLFFBQVEsQ0FBQztRQUN6QixJQUFJLENBQUMsU0FBUyxHQUFHLFNBQVMsQ0FBQztRQUUzQixxQkFBcUI7UUFDckIsSUFBSSxDQUFDLE9BQU8sR0FBRyxJQUFJLENBQUMsaUJBQWlCLEVBQUUsQ0FBQztJQUMxQyxDQUFDO0lBRUQ7Ozs7O09BS0c7SUFDSCxPQUFPLENBQUMsS0FBZTtRQUNyQixNQUFNLElBQUksR0FBRyxJQUFJLENBQUMsTUFBTSxDQUFDLElBQUksQ0FBQztRQUM5QixNQUFNLEdBQUcsR0FBRyxJQUFJLENBQUMsR0FBRyxDQUFDLEtBQUssQ0FBQyxNQUFNLEVBQUUsSUFBSSxDQUFDLFFBQVEsQ0FBQyxDQUFDO1FBQ2xELE1BQU0sT0FBTyxHQUFHLElBQUksQ0FBQyxPQUFPLENBQUMsT0FBTyxDQUFDO1FBRXJDLG1EQUFtRDtRQUNu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SUFFRDs7T0FFRztJQUNILEtBQUs7UUFNSCxJQUFJLFdBQVcsR0FBRyxDQUFDLENBQUM7UUFDcEIsSUFBSSxXQUFXLEdBQUcsQ0FBQyxDQUFDO1FBRXBCLEtBQUssTUFBTSxPQUFPLElBQUksSUFBSSxDQUFDLFFBQVEsQ0FBQyxNQUFNLEVBQUUsRUFBRSxDQUFDO1lBQzdDLFdBQVcsSUFBSSxPQUFPLENBQUMsYUFBYSxFQUFFLENBQUM7WUFDdkMsSUFBSSxPQUFPLENBQUMsUUFBUSxFQUFFO2dCQUFFLFdBQVcsRUFBRSxDQUFDO1FBQ3hDLENBQUM7UUFFRCxPQUFPO1lBQ0wsYUFBYSxFQUFFLElBQUksQ0FBQyxRQUFRLENBQUMsSUFBSTtZQUNqQyxhQUFhLEVBQUUsSUFBSSxDQUFDLGVBQWU7WUFDbkMsZUFBZSxFQUFFLFdBQVc7WUFDNUIsV0FBVztTQUNaLENBQUM7SUFDSixDQUFDO0lBRUQ7O09BRUc7SUFDSCxLQUFLO1FBQ0gsSUFBSSxDQUFDLFFBQVEsQ0FBQyxLQUFLLEVBQUUsQ0FBQztRQUN0QixJQUFJLENBQUMsZUFBZSxHQUFHLElBQUksQ0FBQztJQUM5QixDQUFDO0NBQ0Y7QUFuTEQsa0NBbUxDIiwic291cmNlc0NvbnRlbnQiOlsiLyoqXG4gKiBMb1JBIChMb3ctUmFuayBBZGFwdGF0aW9uKSBSdW50aW1lXG4gKlxuICogRWZmaWNpZW50IHBhcmFtZXRlci1lZmZpY2llbnQgZmluZS10dW5pbmcgYWRhcHRlcnMgZm9yIExMTXMuXG4gKiBTdXBwb3J0cyBtaWNyby1Mb1JBIChmYXN0LCBzbWFsbCB1cGRhdGVzKSBhbmQgYmFzZS1Mb1JBIChkZWVwZXIgYWRhcHRhdGlvbikuXG4gKlxuICogQGV4YW1wbGVcbiAqIGBgYHR5cGVzY3JpcHRcbiAqIGltcG9ydCB7IExvcmFBZGFwdGVyLCBMb3JhTWFuYWdlciB9IGZyb20gJ0BydXZlY3Rvci9ydXZsbG0nO1xuICpcbiAqIC8vIENyZWF0ZSBhZGFwdGVyXG4gKiBjb25zdCBhZGFwdGVyID0gbmV3IExvcmFBZGFwdGVyKHtcbiAqICAgcmFuazogOCxcbiAqICAgYWxwaGE6IDE2LFxuICogICBkcm9wb3V0OiAwLjEsXG4gKiAgIHRhcmdldE1vZHVsZXM6IFsncXVlcnknLCAndmFsdWUnXSxcbiAqIH0pO1xuICpcbiAqIC8vIEFwcGx5IHRvIGhpZGRlbiBzdGF0ZXNcbiAqIGNvbnN0IG91dHB1dCA9IGFkYXB0ZXIuZm9yd2FyZChoaWRkZW5TdGF0ZXMpO1xuICpcbiAqIC8vIE1hbmFnZSBtdWx0aXBsZSBhZGFwdGVyc1xuICogY29uc3QgbWFuYWdlciA9IG5ldyBMb3JhTWFuYWdlcigpO1xuICogbWFuYWdlci5yZWdpc3RlcigndGFzay0xJywgYWRhcHRlcik7XG4gKiBtYW5hZ2VyLmFjdGl2YXRlKCd0YXNrLTEnKTtcbiAqIGBgYFxuICovXG5cbmltcG9ydCB7IExvUkFDb25maWcsIEVtYmVkZGluZyB9IGZyb20gJy4vdHlwZXMnO1xuXG4vKipcbiAqIERlZmF1bHQgTG9SQSBjb25maWd1cmF0aW9uXG4gKi9cbmNvbnN0IERFRkFVTFRfTE9SQV9DT05GSUc6IFJlcXVpcmVkPExvUkFDb25maWc+ID0ge1xuICByYW5rOiA4LFxuICBhbHBoYTogMTYsXG4gIGRyb3BvdXQ6IDAuMSxcbiAgdGFyZ2V0TW9kdWxlczogWydxdWVyeScsICd2YWx1ZSddLFxufTtcblxuLyoqXG4gKiBMb1JBIGFkYXB0ZXIgd2VpZ2h0c1xuICovXG5leHBvcnQgaW50ZXJmYWNlIExvcmFXZWlnaHRzIHtcbiAgLyoqIERvd24gcHJvamVjdGlvbiBtYXRyaXggKGQgeCByKSAqL1xuICBsb3JhQTogbnVtYmVyW11bXTtcbiAgLyoqIFVwIHByb2plY3Rpb24gbWF0cml4IChyIHggZCkgKi9cbiAgbG9yYUI6IG51bWJlcltdW107XG4gIC8qKiBTY2FsaW5nIGZhY3RvciAqL1xuICBzY2FsaW5nOiBudW1iZXI7XG59XG5cbi8qKlxuICogTG9SQSB0cmFpbmluZyBzdGF0ZVxuICovXG5leHBvcnQgaW50ZXJmYWNlIExvcmFUcmFpbmluZ1N0YXRlIHtcbiAgLyoqIEN1cnJlbnQgc3RlcCAqL1xuICBzdGVwOiBudW1iZXI7XG4gIC8qKiBMZWFybmluZyByYXRlICovXG4gIGxlYXJuaW5nUmF0ZTogbnVtYmVyO1xuICAvKiogQWNjdW11bGF0ZWQgZ3JhZGllbnRzIGZvciBBICovXG4gIGdyYWRBOiBudW1iZXJbXVtdO1xuICAvKiogQWNjdW11bGF0ZWQgZ3JhZGllbnRzIGZvciBCICovXG4gIGdyYWRCOiBudW1iZXJbXVtdO1xuICAvKiogTG9zcyBoaXN0b3J5ICovXG4gIGxvc3NIaXN0b3J5OiBudW1iZXJbXTtcbn1cblxuLyoqXG4gKiBMb1JBIEFkYXB0ZXJcbiAqXG4gKiBJbXBsZW1lbnRzIGxvdy1yYW5rIGRlY29tcG9zaXRpb24gZm9yIHBhcmFtZXRlci1lZmZpY2llbnQgZmluZS10dW5pbmcuXG4gKiBXJyA9IFcgKyBCQSB3aGVyZSBBIGlzIChkIHggcikgYW5kIEIgaXMgKHIgeCBkKSwgciA8PCBkXG4gKlxuICogQGV4YW1wbGVcbiAqIGBgYHR5cGVzY3JpcHRcbiAqIGNvbnN0IGFkYXB0ZXIgPSBuZXcgTG9yYUFkYXB0ZXIoe1xuICogICByYW5rOiA4LFxuICogICBhbHBoYTogMTYsXG4gKiAgIGlucHV0RGltOiA3NjgsXG4gKiAgIG91dHB1dERpbTogNzY4LFxuICogfSk7XG4gKlxuICogLy8gRm9yd2FyZCBwYXNzXG4gKiBjb25zdCBvdXRwdXQgPSBhZGFwdGVyLmZvcndhcmQoaW5wdXQpO1xuICpcbiAqIC8vIFRyYWluaW5nIHN0ZXBcbiAqIGFkYXB0ZXIuYmFja3dhcmQoaW5wdXQsIGdyYWRPdXRwdXQsIDAuMDAxKTtcbiAqIGBgYFxuICovXG5leHBvcnQgY2xhc3MgTG9yYUFkYXB0ZXIge1xuICBwcml2YXRlIGNvbmZpZzogUmVxdWlyZWQ8TG9SQUNvbmZpZz47XG4gIHByaXZhdGUgaW5wdXREaW06IG51bWJlcjtcbiAgcHJpdmF0ZSBvdXRwdXREaW06IG51bWJlcjtcbiAgcHJpdmF0ZSB3ZWlnaHRzOiBMb3JhV2VpZ2h0cztcbiAgcHJpdmF0ZSB0cmFpbmluZ1N0YXRlOiBMb3JhVHJhaW5pbmdTdGF0ZSB8IG51bGwgPSBudWxsO1xuICBwcml2YXRlIGZyb3plbjogYm9vbGVhbiA9IGZhbHNlO1xuXG4gIGNvbnN0cnVjdG9yKGNvbmZpZz86IFBhcnRpYWw8TG9SQUNvbmZpZz4sIGlucHV0RGltID0gMjU2LCBvdXRwdXREaW0gPSAyNTYpIHtcbiAgICB0aGlzLmNvbmZpZyA9IHsgLi4uREVGQVVMVF9MT1JBX0NPTkZJRywgLi4uY29uZmlnIH07XG4gICAgdGhpcy5pbnB1dERpbSA9IGlucHV0RGltO1xuICAgIHRoaXMub3V0cHV0RGltID0gb3V0cHV0RGltO1xuXG4gICAgLy8gSW5pdGlhbGl6ZSB3ZWlnaHRzXG4gICAgdGhpcy53ZWlnaHRzID0gdGhpcy5pbml0aWFsaXplV2VpZ2h0cygpO1xuICB9XG5cbiAgLyoqXG4gICAqIEZvcndhcmQgcGFzcyB0aHJvdWdoIExvUkEgYWRhcHRlclxuICAgKiBPUFRJTUlaRUQ6IFVzZXMgRmxvYXQ2NEFycmF5IGFuZCBsb29wIHVucm9sbGluZ1xuICAgKlxuICAgKiBvdXRwdXQgPSBpbnB1dCArIHNjYWxpbmcgKiAoaW5wdXQgQCBBIEAgQilcbiAgICovXG4gIGZvcndhcmQoaW5wdXQ6IG51bWJlcltdKTogbnVtYmVyW10ge1xuICAgIGNvbnN0IHJhbmsgPSB0aGlzLmNvbmZpZy5yYW5rO1xuICAgIGNvbnN0IGRpbSA9IE1hdGgubWluKGlucHV0Lmxlbmd0aCwgdGhpcy5pbnB1dERpbSk7XG4gICAgY29uc3Qgc2NhbGluZyA9IHRoaXMud2VpZ2h0cy5zY2FsaW5nO1xuXG4gICAgLy8gQXBwbHkgZHJvcG91dCBkdXJpbmcgdHJhaW5pbmcgKHNpbXBsaWZpZWQgY2hlY2spXG4gICAgY29uc3QgYXBwbHlEcm9wb3V0ID0gdGhpcy50cmFpbmluZ1N0YXRlICE9PSBudWxsICYmIHRoaXMuY29uZmlnLmRyb3BvdXQgPiAwO1xuXG4gICAgLy8gaW5wdXQgQCBBIChkIC0+IHIpIC0gdXNlIHR5cGVkIGFycmF5IGZvciBoaWRkZW5cbiAgICBjb25zdCBoaWRkZW4gPSBuZXcgRmxvYXQ2NEFycmF5KHJhbmspO1xuICAgIGZvciAobGV0IHIgPSAwOyByIDwgcmFuazsgcisrKSB7XG4gICAgICBsZXQgc3VtID0gMDtcbiAgICAgIGNvbnN0IGxvcmFBQ29sID0gdGhpcy53ZWlnaHRzLmxvcmFBO1xuICAgICAgLy8gVW5yb2xsIGxvb3AgZm9yIGJldHRlciBwZXJmb3JtYW5jZVxuICAgICAgbGV0IGkgPSAwO1xuICAgICAgaWYgKGFwcGx5RHJvcG91dCkge1xuICAgICAgICBmb3IgKDsgaSA8IGRpbTsgaSsrKSB7XG4gICAgICAgICAgaWYgKE1hdGgucmFuZG9tKCkgPiB0aGlzLmNvbmZpZy5kcm9wb3V0KSB7XG4gICAgICAgICAgICBzdW0gKz0gaW5wdXRbaV0gKiBsb3JhQUNvbFtpXVtyXTtcbiAgICAgICAgICB9XG4gICAgICAgIH1cbiAgICAgIH0gZWxzZSB7XG4gICAgICAgIGZvciAoOyBpICsgMyA8IGRpbTsgaSArPSA0KSB7XG4gICAgICAgICAgc3VtICs9IGlucHV0W2ldICogbG9yYUFDb2xbaV1bcl0gK1xuICAgICAgICAgICAgICAgICBpbnB1dFtpICsgMV0gKiBsb3JhQUNvbFtpICsgMV1bcl0gK1xuICAgICAgICAgICAgICAgICBpbnB1dFtpICsgMl0gKiBsb3JhQUNvbFtpICsgMl1bcl0gK1xuICAgICAgICAgICAgICAgICBpbnB1dFtpICsgM10gKiBsb3JhQUNvbFtpICsgM11bcl07XG4gICAgICAgIH1cbiAgICAgICAgZm9yICg7IGkgPCBkaW07IGkrKykge1xuICAgICAgICAgIHN1bSArPSBpbnB1dFtpXSAqIGxvcmFBQ29sW2ldW3JdO1xuICAgICAgICB9XG4gICAgICB9XG4gICAgICBoaWRkZW5bcl0gPSBzdW07XG4gICAgfVxuXG4gICAgLy8gaGlkZGVuIEAgQiAociAtPiBkKSArIHJlc2lkdWFsXG4gICAgY29uc3Qgb3V0cHV0ID0gbmV3IEFycmF5KHRoaXMub3V0cHV0RGltKTtcbiAgICBjb25zdCBsb3JhQiA9IHRoaXMud2VpZ2h0cy5sb3JhQjtcbiAgICBmb3IgKGxldCBpID0gMDsgaSA8IHRoaXMub3V0cHV0RGltOyBpKyspIHtcbiAgICAgIGxldCBkZWx0YSA9IDA7XG4gICAgICBmb3IgKGxldCByID0gMDsgciA8IHJhbms7IHIrKykge1xuICAgICAgICBkZWx0YSArPSBoaWRkZW5bcl0gKiBsb3JhQltyXVtpXTtcbiAgICAgIH1cbiAgICAgIC8vIEFkZCBzY2FsZWQgZGVsdGEgdG8gaW5wdXQgKHJlc2lkdWFsIGNvbm5lY3Rpb24pXG4gICAgICBvdXRwdXRbaV0gPSAoaW5wdXRbaV0gfHwgMCkgKyBzY2FsaW5nICogZGVsdGE7XG4gICAgfVxuXG4gICAgcmV0dXJuIG91dHB1dDtcbiAgfVxuXG4gIC8qKlxuICAgKiBGb3J3YXJkIHdpdGggYmF0Y2ggcHJvY2Vzc2luZ1xuICAgKi9cbiAgZm9yd2FyZEJhdGNoKGlucHV0czogbnVtYmVyW11bXSk6IG51bWJlcltdW10ge1xuICAgIHJldHVybiBpbnB1dHMubWFwKGlucHV0ID0+IHRoaXMuZm9yd2FyZChpbnB1dCkpO1xuICB9XG5cbiAgLyoqXG4gICAqIEJhY2t3YXJkIHBhc3MgYW5kIHdlaWdodCB1cGRhdGVcbiAgICovXG4gIGJhY2t3YXJkKGlucHV0OiBudW1iZXJbXSwgZ3JhZE91dHB1dDogbnVtYmVyW10sIGxlYXJuaW5nUmF0ZTogbnVtYmVyKTogbnVtYmVyIHtcbiAgICBpZiAodGhpcy5mcm96ZW4pIHJldHVybiAwO1xuXG4gICAgY29uc3QgcmFuayA9IHRoaXMuY29uZmlnLnJhbms7XG4gICAgY29uc3QgZGltID0gTWF0aC5taW4oaW5wdXQubGVuZ3RoLCB0aGlzLmlucHV0RGltKTtcblxuICAgIC8vIENvbXB1dGUgaGlkZGVuIGFjdGl2YXRpb25zIChmb3IgZ3JhZGllbnQpXG4gICAgY29uc3QgaGlkZGVuID0gbmV3IEFycmF5KHJhbmspLmZpbGwoMCk7XG4gICAgZm9yIChsZXQgciA9IDA7IHIgPCByYW5rOyByKyspIHtcbiAgICAgIGZvciAobGV0IGkgPSAwOyBpIDwgZGltOyBpKyspIHtcbiAgICAgICAgaGlkZGVuW3JdICs9IGlucHV0W2ldICogdGhpcy53ZWlnaHRzLmxvcmFBW2ldW3JdO1xuICAgICAgfVxuICAgIH1cblxuICAgIC8vIEdyYWRpZW50IGZvciBCOiBoaWRkZW5eVCBAIGdyYWRPdXRwdXRcbiAgICBjb25zdCBncmFkQjogbnVtYmVyW11bXSA9IEFycmF5KHJhbmspLmZpbGwobnVsbCkubWFwKCgpID0+IEFycmF5KHRoaXMub3V0cHV0RGltKS5maWxsKDApKTtcbiAgICBmb3IgKGxldCByID0gMDsgciA8IHJhbms7IHIrKykge1xuICAgICAgZm9yIChsZXQgaSA9IDA7IGkgPCB0aGlzLm91dHB1dERpbTsgaSsrKSB7XG4gICAgICAgIGdyYWRCW3JdW2ldID0gaGlkZGVuW3JdICogKGdyYWRPdXRwdXRbaV0gfHwgMCkgKiB0aGlzLndlaWdodHMuc2NhbGluZztcbiAgICAgIH1cbiAgICB9XG5cbiAgICAvLyBHcmFkaWVudCBmb3IgaGlkZGVuOiBncmFkT3V0cHV0IEAgQl5UXG4gICAgY29uc3QgZ3JhZEhpZGRlbiA9IG5ldyBBcnJheShyYW5rKS5maWxsKDApO1xuICAgIGZvciAobGV0IHIgPSAwOyByIDwgcmFuazsgcisrKSB7XG4gICAgICBmb3IgKGxldCBpID0gMDsgaSA8IHRoaXMub3V0cHV0RGltOyBpKyspIHtcbiAgICAgICAgZ3JhZEhpZGRlbltyXSArPSAoZ3JhZE91dHB1dFtpXSB8fCAwKSAqIHRoaXMud2VpZ2h0cy5sb3JhQltyXVtpXSAqIHRoaXMud2VpZ2h0cy5zY2FsaW5nO1xuICAgICAgfVxuICAgIH1cblxuICAgIC8vIEdyYWRpZW50IGZvciBBOiBpbnB1dF5UIEAgZ3JhZEhpZGRlblxuICAgIGNvbnN0IGdyYWRBOiBudW1iZXJbXVtdID0gQXJyYXkoZGltKS5maWxsKG51bGwpLm1hcCgoKSA9PiBBcnJheShyYW5rKS5maWxsKDApKTtcbiAgICBmb3IgKGxldCBpID0gMDsgaSA8IGRpbTsgaSsrKSB7XG4gICAgICBmb3IgKGxldCByID0gMDsgciA8IHJhbms7IHIrKykge1xuICAgICAgICBncmFkQVtpXVtyXSA9IGlucHV0W2ldICogZ3JhZEhpZGRlbltyXTtcbiAgICAgIH1cbiAgICB9XG5cbiAgICAvLyBVcGRhdGUgd2VpZ2h0c1xuICAgIGxldCB0b3RhbEdyYWQgPSAwO1xuICAgIGZvciAobGV0IGkgPSAwOyBpIDwgZGltOyBpKyspIHtcbiAgICAgIGZvciAobGV0IHIgPSAwOyByIDwgcmFuazsgcisrKSB7XG4gICAgICAgIHRoaXMud2VpZ2h0cy5sb3JhQVtpXVtyXSAtPSBsZWFybmluZ1JhdGUgKiBncmFkQVtpXVtyXTtcbiAgICAgICAgdG90YWxHcmFkICs9IE1hdGguYWJzKGdyYWRBW2ldW3JdKTtcbiAgICAgIH1cbiAgICB9XG4gICAgZm9yIChsZXQgciA9IDA7IHIgPCByYW5rOyByKyspIHtcbiAgICAgIGZvciAobGV0IGkgPSAwOyBpIDwgdGhpcy5vdXRwdXREaW07IGkrKykge1xuICAgICAgICB0aGlzLndlaWdodHMubG9yYUJbcl1baV0gLT0gbGVhcm5pbmdSYXRlICogZ3JhZEJbcl1baV07XG4gICAgICAgIHRvdGFsR3JhZCArPSBNYXRoLmFicyhncmFkQltyXVtpXSk7XG4gICAgICB9XG4gICAgfVxuXG4gICAgLy8gVHJhY2sgdHJhaW5pbmcgc3RhdGVcbiAgICBpZiAodGhpcy50cmFpbmluZ1N0YXRlKSB7XG4gICAgICB0aGlzLnRyYWluaW5nU3RhdGUuc3RlcCsrO1xuICAgICAgdGhpcy50cmFpbmluZ1N0YXRlLmxvc3NIaXN0b3J5LnB1c2godG90YWxHcmFkKTtcbiAgICB9XG5cbiAgICByZXR1cm4gdG90YWxHcmFkO1xuICB9XG5cbiAgLyoqXG4gICAqIFN0YXJ0IHRyYWluaW5nIG1vZGVcbiAgICovXG4gIHN0YXJ0VHJhaW5pbmcobGVhcm5pbmdSYXRlID0gMC4wMDEpOiB2b2lkIHtcbiAgICB0aGlzLnRyYWluaW5nU3RhdGUgPSB7XG4gICAgICBzdGVwOiAwLFxuICAgICAgbGVhcm5pbmdSYXRlLFxuICAgICAgZ3JhZEE6IEFycmF5KHRoaXMuaW5wdXREaW0pLmZpbGwobnVsbCkubWFwKCgpID0+IEFycmF5KHRoaXMuY29uZmlnLnJhbmspLmZpbGwoMCkpLFxuICAgICAgZ3JhZEI6IEFycmF5KHRoaXMuY29uZmlnLnJhbmspLmZpbGwobnVsbCkubWFwKCgpID0+IEFycmF5KHRoaXMub3V0cHV0RGltKS5maWxsKDApKSxcbiAgICAgIGxvc3NIaXN0b3J5OiBbXSxcbiAgICB9O1xuICB9XG5cbiAgLyoqXG4gICAqIEVuZCB0cmFpbmluZyBtb2RlXG4gICAqL1xuICBlbmRUcmFpbmluZygpOiBMb3JhVHJhaW5pbmdTdGF0ZSB8IG51bGwge1xuICAgIGNvbnN0IHN0YXRlID0gdGhpcy50cmFpbmluZ1N0YXRlO1xuICAgIHRoaXMudHJhaW5pbmdTdGF0ZSA9IG51bGw7XG4gICAgcmV0dXJuIHN0YXRlO1xuICB9XG5cbiAgLyoqXG4gICAqIEZyZWV6ZSBhZGFwdGVyIChubyBtb3JlIHVwZGF0ZXMpXG4gICAqL1xuICBmcmVlemUoKTogdm9pZCB7XG4gICAgdGhpcy5mcm96ZW4gPSB0cnVlO1xuICB9XG5cbiAgLyoqXG4gICAqIFVuZnJlZXplIGFkYXB0ZXJcbiAgICovXG4gIHVuZnJlZXplKCk6IHZvaWQge1xuICAgIHRoaXMuZnJvemVuID0gZmFsc2U7XG4gIH1cblxuICAvKipcbiAgICogQ2hlY2sgaWYgZnJvemVuXG4gICAqL1xuICBpc0Zyb3plbigpOiBib29sZWFuIHtcbiAgICByZXR1cm4gdGhpcy5mcm96ZW47XG4gIH1cblxuICAvKipcbiAgICogR2V0IGFkYXB0ZXIgY29uZmlnXG4gICAqL1xuICBnZXRDb25maWcoKTogUmVxdWlyZWQ8TG9SQUNvbmZpZz4ge1xuICAgIHJldHVybiB7IC4uLnRoaXMuY29uZmlnIH07XG4gIH1cblxuICAvKipcbiAgICogR2V0IGFkYXB0ZXIgd2VpZ2h0c1xuICAgKi9cbiAgZ2V0V2VpZ2h0cygpOiBMb3JhV2VpZ2h0cyB7XG4gICAgcmV0dXJuIHtcbiAgICAgIGxvcmFBOiB0aGlzLndlaWdodHMubG9yYUEubWFwKHJvdyA9PiBbLi4ucm93XSksXG4gICAgICBsb3JhQjogdGhpcy53ZWlnaHRzLmxvcmFCLm1hcChyb3cgPT4gWy4uLnJvd10pLFxuICAgICAgc2NhbGluZzogdGhpcy53ZWlnaHRzLnNjYWxpbmcsXG4gICAgfTtcbiAgfVxuXG4gIC8qKlxuICAgKiBTZXQgYWRhcHRlciB3ZWlnaHRzXG4gICAqL1xuICBzZXRXZWlnaHRzKHdlaWdodHM6IExvcmFXZWlnaHRzKTogdm9pZCB7XG4gICAgdGhpcy53ZWlnaHRzID0ge1xuICAgICAgbG9yYUE6IHdlaWdodHMubG9yYUEubWFwKHJvdyA9PiBbLi4ucm93XSksXG4gICAgICBsb3JhQjogd2VpZ2h0cy5sb3JhQi5tYXAocm93ID0+IFsuLi5yb3ddKSxcbiAgICAgIHNjYWxpbmc6IHdlaWdodHMuc2NhbGluZyxcbiAgICB9O1xuICB9XG5cbiAgLyoqXG4gICAqIE1lcmdlIGFkYXB0ZXIgaW50byBiYXNlIHdlaWdodHNcbiAgICpcbiAgICogUmV0dXJucyBkZWx0YSB0byBhZGQgdG8gYmFzZSBtb2RlbCB3ZWlnaHRzXG4gICAqL1xuICBtZXJnZSgpOiBudW1iZXJbXVtdIHtcbiAgICBjb25zdCBkZWx0YTogbnVtYmVyW11bXSA9IEFycmF5KHRoaXMuaW5wdXREaW0pXG4gICAgICAuZmlsbChudWxsKVxuICAgICAgLm1hcCgoKSA9PiBBcnJheSh0aGlzLm91dHB1dERpbSkuZmlsbCgwKSk7XG5cbiAgICBjb25zdCByYW5rID0gdGhpcy5jb25maWcucmFuaztcbiAgICBmb3IgKGxldCBpID0gMDsgaSA8IHRoaXMuaW5wdXREaW07IGkrKykge1xuICAgICAgZm9yIChsZXQgaiA9IDA7IGogPCB0aGlzLm91dHB1dERpbTsgaisrKSB7XG4gICAgICAgIGZvciAobGV0IHIgPSAwOyByIDwgcmFuazsgcisrKSB7XG4gICAgICAgICAgZGVsdGFbaV1bal0gKz0gdGhpcy53ZWlnaHRzLmxvcmFBW2ldW3JdICogdGhpcy53ZWlnaHRzLmxvcmFCW3JdW2pdO1xuICAgICAgICB9XG4gICAgICAgIGRlbHRhW2ldW2pdICo9IHRoaXMud2VpZ2h0cy5zY2FsaW5nO1xuICAgICAgfVxuICAgIH1cblxuICAgIHJldHVybiBkZWx0YTtcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgbnVtYmVyIG9mIHRyYWluYWJsZSBwYXJhbWV0ZXJzXG4gICAqL1xuICBudW1QYXJhbWV0ZXJzKCk6IG51bWJlciB7XG4gICAgcmV0dXJuICh0aGlzLmlucHV0RGltICogdGhpcy5jb25maWcucmFuaykgKyAodGhpcy5jb25maWcucmFuayAqIHRoaXMub3V0cHV0RGltKTtcbiAgfVxuXG4gIC8qKlxuICAgKiBSZXNldCB0byBpbml0aWFsIHdlaWdodHNcbiAgICovXG4gIHJlc2V0KCk6IHZvaWQge1xuICAgIHRoaXMud2VpZ2h0cyA9IHRoaXMuaW5pdGlhbGl6ZVdlaWdodHMoKTtcbiAgICB0aGlzLnRyYWluaW5nU3RhdGUgPSBudWxsO1xuICAgIHRoaXMuZnJvemVuID0gZmFsc2U7XG4gIH1cblxuICAvKipcbiAgICogQ2xvbmUgYWRhcHRlclxuICAgKi9cbiAgY2xvbmUoKTogTG9yYUFkYXB0ZXIge1xuICAgIGNvbnN0IGFkYXB0ZXIgPSBuZXcgTG9yYUFkYXB0ZXIodGhpcy5jb25maWcsIHRoaXMuaW5wdXREaW0sIHRoaXMub3V0cHV0RGltKTtcbiAgICBhZGFwdGVyLnNldFdlaWdodHModGhpcy5nZXRXZWlnaHRzKCkpO1xuICAgIHJldHVybiBhZGFwdGVyO1xuICB9XG5cbiAgLyoqXG4gICAqIFNlcmlhbGl6ZSB0byBKU09OXG4gICAqL1xuICB0b0pTT04oKTogc3RyaW5nIHtcbiAgICByZXR1cm4gSlNPTi5zdHJpbmdpZnkoe1xuICAgICAgY29uZmlnOiB0aGlzLmNvbmZpZyxcbiAgICAgIGlucHV0RGltOiB0aGlzLmlucHV0RGltLFxuICAgICAgb3V0cHV0RGltOiB0aGlzLm91dHB1dERpbSxcbiAgICAgIHdlaWdodHM6IHRoaXMud2VpZ2h0cyxcbiAgICAgIGZyb3plbjogdGhpcy5mcm96ZW4sXG4gICAgfSk7XG4gIH1cblxuICAvKipcbiAgICogRGVzZXJpYWxpemUgZnJvbSBKU09OXG4gICAqL1xuICBzdGF0aWMgZnJvbUpTT04oanNvbjogc3RyaW5nKTogTG9yYUFkYXB0ZXIge1xuICAgIGNvbnN0IGRhdGEgPSBKU09OLnBhcnNlKGpzb24pO1xuICAgIGNvbnN0IGFkYXB0ZXIgPSBuZXcgTG9yYUFkYXB0ZXIoZGF0YS5jb25maWcsIGRhdGEuaW5wdXREaW0sIGRhdGEub3V0cHV0RGltKTtcbiAgICBhZGFwdGVyLnNldFdlaWdodHMoZGF0YS53ZWlnaHRzKTtcbiAgICBpZiAoZGF0YS5mcm96ZW4pIGFkYXB0ZXIuZnJlZXplKCk7XG4gICAgcmV0dXJuIGFkYXB0ZXI7XG4gIH1cblxuICBwcml2YXRlIGluaXRpYWxpemVXZWlnaHRzKCk6IExvcmFXZWlnaHRzIHtcbiAgICBjb25zdCByYW5rID0gdGhpcy5jb25maWcucmFuaztcblxuICAgIC8vIEthaW1pbmcgaW5pdGlhbGl6YXRpb24gZm9yIEEsIHplcm8gaW5pdGlhbGl6YXRpb24gZm9yIEJcbiAgICBjb25zdCBsb3JhQTogbnVtYmVyW11bXSA9IEFycmF5KHRoaXMuaW5wdXREaW0pXG4gICAgICAuZmlsbChudWxsKVxuICAgICAgLm1hcCgoKSA9PlxuICAgICAgICBBcnJheShyYW5rKVxuICAgICAgICAgIC5maWxsKDApXG4gICAgICAgICAgLm1hcCgoKSA9PiAoTWF0aC5yYW5kb20oKSAtIDAuNSkgKiBNYXRoLnNxcnQoMiAvIHRoaXMuaW5wdXREaW0pKVxuICAgICAgKTtcblxuICAgIGNvbnN0IGxvcmFCOiBudW1iZXJbXVtdID0gQXJyYXkocmFuaylcbiAgICAgIC5maWxsKG51bGwpXG4gICAgICAubWFwKCgpID0+IEFycmF5KHRoaXMub3V0cHV0RGltKS5maWxsKDApKTtcblxuICAgIHJldHVybiB7XG4gICAgICBsb3JhQSxcbiAgICAgIGxvcmFCLFxuICAgICAgc2NhbGluZzogdGhpcy5jb25maWcuYWxwaGEgLyB0aGlzLmNvbmZpZy5yYW5rLFxuICAgIH07XG4gIH1cbn1cblxuLyoqXG4gKiBMb1JBIE1hbmFnZXIgZm9yIG11bHRpcGxlIGFkYXB0ZXJzXG4gKlxuICogTWFuYWdlcyBhIGNvbGxlY3Rpb24gb2YgTG9SQSBhZGFwdGVycyBmb3IgZGlmZmVyZW50IHRhc2tzL2RvbWFpbnMuXG4gKi9cbmV4cG9ydCBjbGFzcyBMb3JhTWFuYWdlciB7XG4gIHByaXZhdGUgYWRhcHRlcnM6IE1hcDxzdHJpbmcsIExvcmFBZGFwdGVyPiA9IG5ldyBNYXAoKTtcbiAgcHJpdmF0ZSBhY3RpdmVBZGFwdGVySWQ6IHN0cmluZyB8IG51bGwgPSBudWxsO1xuICBwcml2YXRlIGRlZmF1bHRDb25maWc6IFJlcXVpcmVkPExvUkFDb25maWc+O1xuXG4gIGNvbnN0cnVjdG9yKGRlZmF1bHRDb25maWc/OiBQYXJ0aWFsPExvUkFDb25maWc+KSB7XG4gICAgdGhpcy5kZWZhdWx0Q29uZmlnID0geyAuLi5ERUZBVUxUX0xPUkFfQ09ORklHLCAuLi5kZWZhdWx0Q29uZmlnIH07XG4gIH1cblxuICAvKipcbiAgICogUmVnaXN0ZXIgYSBuZXcgYWRhcHRlclxuICAgKi9cbiAgcmVnaXN0ZXIoaWQ6IHN0cmluZywgYWRhcHRlcjogTG9yYUFkYXB0ZXIpOiB2b2lkIHtcbiAgICB0aGlzLmFkYXB0ZXJzLnNldChpZCwgYWRhcHRlcik7XG4gIH1cblxuICAvKipcbiAgICogQ3JlYXRlIGFuZCByZWdpc3RlciBhIG5ldyBhZGFwdGVyXG4gICAqL1xuICBjcmVhdGUoaWQ6IHN0cmluZywgY29uZmlnPzogUGFydGlhbDxMb1JBQ29uZmlnPiwgaW5wdXREaW0/OiBudW1iZXIsIG91dHB1dERpbT86IG51bWJlcik6IExvcmFBZGFwdGVyIHtcbiAgICBjb25zdCBtZXJnZWRDb25maWcgPSB7IC4uLnRoaXMuZGVmYXVsdENvbmZpZywgLi4uY29uZmlnIH07XG4gICAgY29uc3QgYWRhcHRlciA9IG5ldyBMb3JhQWRhcHRlcihtZXJnZWRDb25maWcsIGlucHV0RGltLCBvdXRwdXREaW0pO1xuICAgIHRoaXMucmVnaXN0ZXIoaWQsIGFkYXB0ZXIpO1xuICAgIHJldHVybiBhZGFwdGVyO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCBhZGFwdGVyIGJ5IElEXG4gICAqL1xuICBnZXQoaWQ6IHN0cmluZyk6IExvcmFBZGFwdGVyIHwgdW5kZWZpbmVkIHtcbiAgICByZXR1cm4gdGhpcy5hZGFwdGVycy5nZXQoaWQpO1xuICB9XG5cbiAgLyoqXG4gICAqIFJlbW92ZSBhZGFwdGVyXG4gICAqL1xuICByZW1vdmUoaWQ6IHN0cmluZyk6IGJvb2xlYW4ge1xuICAgIGlmICh0aGlzLmFjdGl2ZUFkYXB0ZXJJZCA9PT0gaWQpIHtcbiAgICAgIHRoaXMuYWN0aXZlQWRhcHRlcklkID0gbnVsbDtcbiAgICB9XG4gICAgcmV0dXJuIHRoaXMuYWRhcHRlcnMuZGVsZXRlKGlkKTtcbiAgfVxuXG4gIC8qKlxuICAgKiBBY3RpdmF0ZSBhbiBhZGFwdGVyXG4gICAqL1xuICBhY3RpdmF0ZShpZDogc3RyaW5nKTogYm9vbGVhbiB7XG4gICAgaWYgKHRoaXMuYWRhcHRlcnMuaGFzKGlkKSkge1xuICAgICAgdGhpcy5hY3RpdmVBZGFwdGVySWQgPSBpZDtcbiAgICAgIHJldHVybiB0cnVlO1xuICAgIH1cbiAgICByZXR1cm4gZmFsc2U7XG4gIH1cblxuICAvKipcbiAgICogRGVhY3RpdmF0ZSBjdXJyZW50IGFkYXB0ZXJcbiAgICovXG4gIGRlYWN0aXZhdGUoKTogdm9pZCB7XG4gICAgdGhpcy5hY3RpdmVBZGFwdGVySWQgPSBudWxsO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCBhY3RpdmUgYWRhcHRlclxuICAgKi9cbiAgZ2V0QWN0aXZlKCk6IExvcmFBZGFwdGVyIHwgbnVsbCB7XG4gICAgcmV0dXJuIHRoaXMuYWN0aXZlQWRhcHRlcklkID8gdGhpcy5hZGFwdGVycy5nZXQodGhpcy5hY3RpdmVBZGFwdGVySWQpIHx8IG51bGwgOiBudWxsO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCBhY3RpdmUgYWRhcHRlciBJRFxuICAgKi9cbiAgZ2V0QWN0aXZlSWQoKTogc3RyaW5nIHwgbnVsbCB7XG4gICAgcmV0dXJuIHRoaXMuYWN0aXZlQWRhcHRlcklkO1xuICB9XG5cbiAgLyoqXG4gICAqIEFwcGx5IGFjdGl2ZSBhZGFwdGVyXG4gICAqL1xuICBmb3J3YXJkKGlucHV0OiBudW1iZXJbXSk6IG51bWJlcltdIHtcbiAgICBjb25zdCBhY3RpdmUgPSB0aGlzLmdldEFjdGl2ZSgpO1xuICAgIHJldHVybiBhY3RpdmUgPyBhY3RpdmUuZm9yd2FyZChpbnB1dCkgOiBbLi4uaW5wdXRdO1xuICB9XG5cbiAgLyoqXG4gICAqIExpc3QgYWxsIGFkYXB0ZXIgSURzXG4gICAqL1xuICBsaXN0KCk6IHN0cmluZ1tdIHtcbiAgICByZXR1cm4gQXJyYXkuZnJvbSh0aGlzLmFkYXB0ZXJzLmtleXMoKSk7XG4gIH1cblxuICAvKipcbiAgICogR2V0IGFkYXB0ZXIgY291bnRcbiAgICovXG4gIGNvdW50KCk6IG51bWJlciB7XG4gICAgcmV0dXJuIHRoaXMuYWRhcHRlcnMuc2l6ZTtcbiAgfVxuXG4gIC8qKlxuICAgKiBGcmVlemUgYWxsIGFkYXB0ZXJzXG4gICAqL1xuICBmcmVlemVBbGwoKTogdm9pZCB7XG4gICAgZm9yIChjb25zdCBhZGFwdGVyIG9mIHRoaXMuYWRhcHRlcnMudmFsdWVzKCkpIHtcbiAgICAgIGFkYXB0ZXIuZnJlZXplKCk7XG4gICAgfVxuICB9XG5cbiAgLyoqXG4gICAqIFVuZnJlZXplIGFsbCBhZGFwdGVyc1xuICAgKi9cbiAgdW5mcmVlemVBbGwoKTogdm9pZCB7XG4gICAgZm9yIChjb25zdCBhZGFwdGVyIG9mIHRoaXMuYWRhcHRlcnMudmFsdWVzKCkpIHtcbiAgICAgIGFkYXB0ZXIudW5mcmVlemUoKTtcbiAgICB9XG4gIH1cblxuICAvKipcbiAgICogTWVyZ2UgbXVsdGlwbGUgYWRhcHRlcnMgaW50byBvbmVcbiAgICovXG4gIG1lcmdlQWRhcHRlcnMoaWRzOiBzdHJpbmdbXSwgb3V0cHV0SWQ6IHN0cmluZyk6IExvcmFBZGFwdGVyIHwgbnVsbCB7XG4gICAgY29uc3QgYWRhcHRlcnMgPSBpZHMubWFwKGlkID0+IHRoaXMuYWRhcHRlcnMuZ2V0KGlkKSkuZmlsdGVyKEJvb2xlYW4pIGFzIExvcmFBZGFwdGVyW107XG4gICAgaWYgKGFkYXB0ZXJzLmxlbmd0aCA9PT0gMCkgcmV0dXJuIG51bGw7XG5cbiAgICAvLyBVc2UgZmlyc3QgYWRhcHRlciBhcyBiYXNlXG4gICAgY29uc3QgbWVyZ2VkID0gYWRhcHRlcnNbMF0uY2xvbmUoKTtcbiAgICBjb25zdCB3ZWlnaHRzID0gbWVyZ2VkLmdldFdlaWdodHMoKTtcblxuICAgIC8vIEF2ZXJhZ2Ugd2VpZ2h0cyBmcm9tIG90aGVyIGFkYXB0ZXJzXG4gICAgZm9yIChsZXQgaSA9IDE7IGkgPCBhZGFwdGVycy5sZW5ndGg7IGkrKykge1xuICAgICAgY29uc3Qgb3RoZXJXZWlnaHRzID0gYWRhcHRlcnNbaV0uZ2V0V2VpZ2h0cygpO1xuXG4gICAgICBmb3IgKGxldCByb3cgPSAwOyByb3cgPCB3ZWlnaHRzLmxvcmFBLmxlbmd0aCAmJiByb3cgPCBvdGhlcldlaWdodHMubG9yYUEubGVuZ3RoOyByb3crKykge1xuICAgICAgICBmb3IgKGxldCBjb2wgPSAwOyBjb2wgPCB3ZWlnaHRzLmxvcmFBW3Jvd10ubGVuZ3RoICYmIGNvbCA8IG90aGVyV2VpZ2h0cy5sb3JhQVtyb3ddLmxlbmd0aDsgY29sKyspIHtcbiAgICAgICAgICB3ZWlnaHRzLmxvcmFBW3Jvd11bY29sXSA9ICh3ZWlnaHRzLmxvcmFBW3Jvd11bY29sXSArIG90aGVyV2VpZ2h0cy5sb3JhQVtyb3ddW2NvbF0pIC8gMjtcbiAgICAgICAgfVxuICAgICAgfVxuICAgICAgZm9yIChsZXQgcm93ID0gMDsgcm93IDwgd2VpZ2h0cy5sb3JhQi5sZW5ndGggJiYgcm93IDwgb3RoZXJXZWlnaHRzLmxvcmFCLmxlbmd0aDsgcm93KyspIHtcbiAgICAgICAgZm9yIChsZXQgY29sID0gMDsgY29sIDwgd2VpZ2h0cy5sb3JhQltyb3ddLmxlbmd0aCAmJiBjb2wgPCBvdGhlcldlaWdodHMubG9yYUJbcm93XS5sZW5ndGg7IGNvbCsrKSB7XG4gICAgICAgICAgd2VpZ2h0cy5sb3JhQltyb3ddW2NvbF0gPSAod2VpZ2h0cy5sb3JhQltyb3ddW2NvbF0gKyBvdGhlcldlaWdodHMubG9yYUJbcm93XVtjb2xdKSAvIDI7XG4gICAgICAgIH1cbiAgICAgIH1cbiAgICB9XG5cbiAgICBtZXJnZWQuc2V0V2VpZ2h0cyh3ZWlnaHRzKTtcbiAgICB0aGlzLnJlZ2lzdGVyKG91dHB1dElkLCBtZXJnZWQpO1xuICAgIHJldHVybiBtZXJnZWQ7XG4gIH1cblxuICAvKipcbiAgICogR2V0IHN0YXRpc3RpY3NcbiAgICovXG4gIHN0YXRzKCk6IHtcbiAgICB0b3RhbEFkYXB0ZXJzOiBudW1iZXI7XG4gICAgYWN0aXZlQWRhcHRlcjogc3RyaW5nIHwgbnVsbDtcbiAgICB0b3RhbFBhcmFtZXRlcnM6IG51bWJlcjtcbiAgICBmcm96ZW5Db3VudDogbnVtYmVyO1xuICB9IHtcbiAgICBsZXQgdG90YWxQYXJhbXMgPSAwO1xuICAgIGxldCBmcm96ZW5Db3VudCA9IDA7XG5cbiAgICBmb3IgKGNvbnN0IGFkYXB0ZXIgb2YgdGhpcy5hZGFwdGVycy52YWx1ZXMoKSkge1xuICAgICAgdG90YWxQYXJhbXMgKz0gYWRhcHRlci5udW1QYXJhbWV0ZXJzKCk7XG4gICAgICBpZiAoYWRhcHRlci5pc0Zyb3plbigpKSBmcm96ZW5Db3VudCsrO1xuICAgIH1cblxuICAgIHJldHVybiB7XG4gICAgICB0b3RhbEFkYXB0ZXJzOiB0aGlzLmFkYXB0ZXJzLnNpemUsXG4gICAgICBhY3RpdmVBZGFwdGVyOiB0aGlzLmFjdGl2ZUFkYXB0ZXJJZCxcbiAgICAgIHRvdGFsUGFyYW1ldGVyczogdG90YWxQYXJhbXMsXG4gICAgICBmcm96ZW5Db3VudCxcbiAgICB9O1xuICB9XG5cbiAgLyoqXG4gICAqIENsZWFyIGFsbCBhZGFwdGVyc1xuICAgKi9cbiAgY2xlYXIoKTogdm9pZCB7XG4gICAgdGhpcy5hZGFwdGVycy5jbGVhcigpO1xuICAgIHRoaXMuYWN0aXZlQWRhcHRlcklkID0gbnVsbDtcbiAgfVxufVxuIl19