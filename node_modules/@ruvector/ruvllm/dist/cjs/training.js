"use strict";
/**
 * Training Pipeline for SONA
 *
 * Comprehensive training infrastructure with metrics tracking,
 * learning rate scheduling, and checkpoint management.
 *
 * @example
 * ```typescript
 * import { TrainingPipeline, TrainingConfig } from '@ruvector/ruvllm';
 *
 * const pipeline = new TrainingPipeline({
 *   learningRate: 0.001,
 *   batchSize: 32,
 *   epochs: 10,
 * });
 *
 * // Add training data
 * pipeline.addBatch(inputs, targets, qualities);
 *
 * // Run training
 * const result = pipeline.train();
 * console.log(`Final loss: ${result.finalLoss}`);
 * ```
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.TrainingFactory = exports.TrainingPipeline = exports.MetricsTracker = exports.LRScheduler = void 0;
const lora_1 = require("./lora");
const sona_1 = require("./sona");
/**
 * Default training config
 */
const DEFAULT_TRAINING_CONFIG = {
    learningRate: 0.001,
    batchSize: 32,
    epochs: 10,
    scheduler: 'cosine',
    warmupSteps: 100,
    weightDecay: 0.01,
    gradientClip: 1.0,
    earlyStoppingPatience: 3,
    checkpointInterval: 1,
    ewcLambda: 2000,
    validationSplit: 0.1,
};
/**
 * Learning Rate Scheduler
 */
class LRScheduler {
    constructor(config, totalSteps) {
        this.currentStep = 0;
        this.config = config;
        this.initialLR = config.learningRate;
        this.totalSteps = totalSteps;
    }
    /**
     * Get learning rate for current step
     */
    getLR() {
        switch (this.config.scheduler) {
            case 'constant':
                return this.initialLR;
            case 'linear':
                return this.initialLR * (1 - this.currentStep / this.totalSteps);
            case 'cosine':
                return this.initialLR * 0.5 * (1 + Math.cos(Math.PI * this.currentStep / this.totalSteps));
            case 'warmup':
                if (this.currentStep < this.config.warmupSteps) {
                    return this.initialLR * (this.currentStep / this.config.warmupSteps);
                }
                // Cosine decay after warmup
                const decaySteps = this.totalSteps - this.config.warmupSteps;
                const decayProgress = (this.currentStep - this.config.warmupSteps) / decaySteps;
                return this.initialLR * 0.5 * (1 + Math.cos(Math.PI * decayProgress));
            default:
                return this.initialLR;
        }
    }
    /**
     * Step the scheduler
     */
    step() {
        this.currentStep++;
    }
    /**
     * Reset scheduler
     */
    reset() {
        this.currentStep = 0;
    }
}
exports.LRScheduler = LRScheduler;
/**
 * Training Metrics Tracker
 */
class MetricsTracker {
    constructor() {
        this.lossHistory = [];
        this.valLossHistory = [];
        this.gradNormHistory = [];
        this.startTime = Date.now();
        this.stepTimes = [];
    }
    /**
     * Record training loss
     */
    recordLoss(loss) {
        this.lossHistory.push(loss);
    }
    /**
     * Record validation loss
     */
    recordValLoss(loss) {
        this.valLossHistory.push(loss);
    }
    /**
     * Record gradient norm
     */
    recordGradNorm(norm) {
        this.gradNormHistory.push(norm);
    }
    /**
     * Record step time
     */
    recordStepTime(ms) {
        this.stepTimes.push(ms);
    }
    /**
     * Get average loss over last N steps
     */
    avgLoss(n = 100) {
        const recent = this.lossHistory.slice(-n);
        return recent.length > 0 ? recent.reduce((a, b) => a + b, 0) / recent.length : 0;
    }
    /**
     * Get average validation loss
     */
    avgValLoss(n = 10) {
        const recent = this.valLossHistory.slice(-n);
        return recent.length > 0 ? recent.reduce((a, b) => a + b, 0) / recent.length : 0;
    }
    /**
     * Get steps per second
     */
    stepsPerSecond() {
        if (this.stepTimes.length === 0)
            return 0;
        const avgStepTime = this.stepTimes.slice(-100).reduce((a, b) => a + b, 0) / Math.min(this.stepTimes.length, 100);
        return avgStepTime > 0 ? 1000 / avgStepTime : 0;
    }
    /**
     * Get ETA in seconds
     */
    eta(remainingSteps) {
        const sps = this.stepsPerSecond();
        return sps > 0 ? remainingSteps / sps : 0;
    }
    /**
     * Get best validation loss
     */
    bestValLoss() {
        return this.valLossHistory.length > 0 ? Math.min(...this.valLossHistory) : Infinity;
    }
    /**
     * Get total duration
     */
    duration() {
        return Date.now() - this.startTime;
    }
    /**
     * Get all loss history
     */
    getLossHistory() {
        return [...this.lossHistory];
    }
    /**
     * Get all validation loss history
     */
    getValLossHistory() {
        return [...this.valLossHistory];
    }
    /**
     * Reset tracker
     */
    reset() {
        this.lossHistory = [];
        this.valLossHistory = [];
        this.gradNormHistory = [];
        this.stepTimes = [];
        this.startTime = Date.now();
    }
}
exports.MetricsTracker = MetricsTracker;
/**
 * Training Pipeline
 *
 * Full training infrastructure for SONA models.
 */
class TrainingPipeline {
    constructor(config, adapter) {
        this.scheduler = null;
        this.batches = [];
        this.checkpoints = [];
        this.currentEpoch = 0;
        this.currentStep = 0;
        this.bestValLoss = Infinity;
        this.patienceCounter = 0;
        this.config = { ...DEFAULT_TRAINING_CONFIG, ...config };
        this.adapter = adapter || new lora_1.LoraAdapter({ rank: 8 });
        this.ewcManager = new sona_1.EwcManager(this.config.ewcLambda);
        this.metrics = new MetricsTracker();
    }
    /**
     * Add training batch
     */
    addBatch(inputs, targets, qualities) {
        this.batches.push({ inputs, targets, qualities });
    }
    /**
     * Add training data
     */
    addData(data) {
        // Group into batches
        for (let i = 0; i < data.length; i += this.config.batchSize) {
            const batch = data.slice(i, i + this.config.batchSize);
            this.addBatch(batch.map(d => d.input), batch.map(d => d.target), batch.map(d => d.quality));
        }
    }
    /**
     * Run training
     */
    train() {
        const totalSteps = this.batches.length * this.config.epochs;
        this.scheduler = new LRScheduler(this.config, totalSteps);
        this.metrics.reset();
        this.adapter.startTraining(this.config.learningRate);
        let earlyStopped = false;
        for (let epoch = 0; epoch < this.config.epochs; epoch++) {
            this.currentEpoch = epoch;
            // Shuffle batches
            const shuffledBatches = this.shuffleBatches();
            // Split into train/val
            const valSize = Math.floor(shuffledBatches.length * this.config.validationSplit);
            const trainBatches = shuffledBatches.slice(valSize);
            const valBatches = shuffledBatches.slice(0, valSize);
            // Training epoch
            for (const batch of trainBatches) {
                const stepStart = Date.now();
                const loss = this.trainStep(batch);
                this.metrics.recordLoss(loss);
                this.metrics.recordStepTime(Date.now() - stepStart);
                this.scheduler.step();
                this.currentStep++;
            }
            // Validation
            if (valBatches.length > 0) {
                const valLoss = this.validate(valBatches);
                this.metrics.recordValLoss(valLoss);
                // Early stopping
                if (valLoss < this.bestValLoss) {
                    this.bestValLoss = valLoss;
                    this.patienceCounter = 0;
                }
                else {
                    this.patienceCounter++;
                    if (this.patienceCounter >= this.config.earlyStoppingPatience) {
                        earlyStopped = true;
                        break;
                    }
                }
            }
            // Checkpoint
            if ((epoch + 1) % this.config.checkpointInterval === 0) {
                this.saveCheckpoint();
            }
        }
        this.adapter.endTraining();
        // Register with EWC for continual learning
        const weights = this.adapter.merge().flat();
        this.ewcManager.registerTask(`task-${Date.now()}`, weights);
        return {
            epochs: this.currentEpoch + 1,
            steps: this.currentStep,
            finalLoss: this.metrics.avgLoss(100),
            bestValLoss: this.bestValLoss,
            durationMs: this.metrics.duration(),
            lossHistory: this.metrics.getLossHistory(),
            valLossHistory: this.metrics.getValLossHistory(),
            earlyStopped,
        };
    }
    /**
     * Single training step
     */
    trainStep(batch) {
        let totalLoss = 0;
        const lr = this.scheduler?.getLR() || this.config.learningRate;
        for (let i = 0; i < batch.inputs.length; i++) {
            const input = batch.inputs[i];
            const target = batch.targets[i];
            const quality = batch.qualities[i];
            // Forward pass
            const output = this.adapter.forward(input);
            // Compute loss (MSE weighted by quality)
            const gradOutput = [];
            let loss = 0;
            for (let j = 0; j < output.length; j++) {
                const diff = output[j] - (target[j] || 0);
                loss += diff * diff;
                gradOutput.push(2 * diff * quality); // Quality-weighted gradient
            }
            loss = (loss / output.length) * quality;
            // Add EWC penalty
            const ewcPenalty = this.ewcManager.computePenalty(this.adapter.merge().flat());
            loss += ewcPenalty * 0.001;
            // Backward pass
            this.adapter.backward(input, gradOutput, lr);
            totalLoss += loss;
        }
        return totalLoss / batch.inputs.length;
    }
    /**
     * Validation pass
     */
    validate(batches) {
        let totalLoss = 0;
        let count = 0;
        for (const batch of batches) {
            for (let i = 0; i < batch.inputs.length; i++) {
                const output = this.adapter.forward(batch.inputs[i]);
                const target = batch.targets[i];
                let loss = 0;
                for (let j = 0; j < output.length; j++) {
                    const diff = output[j] - (target[j] || 0);
                    loss += diff * diff;
                }
                totalLoss += loss / output.length;
                count++;
            }
        }
        return count > 0 ? totalLoss / count : 0;
    }
    /**
     * Save checkpoint
     */
    saveCheckpoint() {
        this.checkpoints.push({
            epoch: this.currentEpoch,
            step: this.currentStep,
            loss: this.metrics.avgLoss(100),
            weights: this.adapter.toJSON(),
            timestamp: Date.now(),
        });
    }
    /**
     * Load checkpoint
     */
    loadCheckpoint(index) {
        const checkpoint = this.checkpoints[index];
        if (!checkpoint)
            return false;
        this.adapter = lora_1.LoraAdapter.fromJSON(checkpoint.weights);
        this.currentEpoch = checkpoint.epoch;
        this.currentStep = checkpoint.step;
        return true;
    }
    /**
     * Get current metrics
     */
    getMetrics() {
        return {
            epoch: this.currentEpoch,
            step: this.currentStep,
            trainLoss: this.metrics.avgLoss(100),
            valLoss: this.metrics.avgValLoss(10),
            learningRate: this.scheduler?.getLR() || this.config.learningRate,
            gradNorm: 0,
            stepsPerSecond: this.metrics.stepsPerSecond(),
            etaSeconds: this.metrics.eta((this.config.epochs - this.currentEpoch) * this.batches.length),
        };
    }
    /**
     * Get adapter
     */
    getAdapter() {
        return this.adapter;
    }
    /**
     * Get EWC manager
     */
    getEwcManager() {
        return this.ewcManager;
    }
    /**
     * Get checkpoints
     */
    getCheckpoints() {
        return [...this.checkpoints];
    }
    /**
     * Reset pipeline
     */
    reset() {
        this.batches = [];
        this.checkpoints = [];
        this.currentEpoch = 0;
        this.currentStep = 0;
        this.bestValLoss = Infinity;
        this.patienceCounter = 0;
        this.metrics.reset();
        this.adapter.reset();
    }
    shuffleBatches() {
        const shuffled = [...this.batches];
        for (let i = shuffled.length - 1; i > 0; i--) {
            const j = Math.floor(Math.random() * (i + 1));
            [shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]];
        }
        return shuffled;
    }
}
exports.TrainingPipeline = TrainingPipeline;
/**
 * Training Factory
 *
 * Create pre-configured training pipelines for common scenarios.
 */
class TrainingFactory {
    /**
     * Create pipeline for quick fine-tuning
     */
    static quickFinetune() {
        return new TrainingPipeline({
            learningRate: 0.01,
            epochs: 3,
            batchSize: 16,
            scheduler: 'constant',
        });
    }
    /**
     * Create pipeline for deep training
     */
    static deepTraining() {
        return new TrainingPipeline({
            learningRate: 0.001,
            epochs: 50,
            batchSize: 32,
            scheduler: 'warmup',
            warmupSteps: 500,
            earlyStoppingPatience: 5,
        });
    }
    /**
     * Create pipeline for continual learning
     */
    static continualLearning(ewcLambda = 5000) {
        return new TrainingPipeline({
            learningRate: 0.0005,
            epochs: 10,
            batchSize: 16,
            scheduler: 'cosine',
            ewcLambda,
            earlyStoppingPatience: 10,
        });
    }
    /**
     * Create pipeline for federated aggregation
     */
    static federatedAggregation() {
        return new TrainingPipeline({
            learningRate: 0.0001,
            epochs: 5,
            batchSize: 64,
            scheduler: 'linear',
            ewcLambda: 2000,
        });
    }
}
exports.TrainingFactory = TrainingFactory;
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoidHJhaW5pbmcuanMiLCJzb3VyY2VSb290IjoiIiwic291cmNlcyI6WyIuLi8uLi9zcmMvdHJhaW5pbmcudHMiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6IjtBQUFBOzs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7OztHQXVCRzs7O0FBR0gsaUNBQXFDO0FBQ3JDLGlDQUFvQztBQUVwQzs7R0FFRztBQUNILE1BQU0sdUJBQXVCLEdBQTZCO0lBQ3hELFlBQVksRUFBRSxLQUFLO0lBQ25CLFNBQVMsRUFBRSxFQUFFO0lBQ2IsTUFBTSxFQUFFLEVBQUU7SUFDVixTQUFTLEVBQUUsUUFBUTtJQUNuQixXQUFXLEVBQUUsR0FBRztJQUNoQixXQUFXLEVBQUUsSUFBSTtJQUNqQixZQUFZLEVBQUUsR0FBRztJQUNqQixxQkFBcUIsRUFBRSxDQUFDO0lBQ3hCLGtCQUFrQixFQUFFLENBQUM7SUFDckIsU0FBUyxFQUFFLElBQUk7SUFDZixlQUFlLEVBQUUsR0FBRztDQUNyQixDQUFDO0FBb0RGOztHQUVHO0FBQ0gsTUFBYSxXQUFXO0lBTXRCLFlBQVksTUFBZ0MsRUFBRSxVQUFrQjtRQUh4RCxnQkFBVyxHQUFXLENBQUMsQ0FBQztRQUk5QixJQUFJLENBQUMsTUFBTSxHQUFHLE1BQU0sQ0FBQztRQUNyQixJQUFJLENBQUMsU0FBUyxHQUFHLE1BQU0sQ0FBQyxZQUFZLENBQUM7UUFDckMsSUFBSSxDQUFDLFVBQVUsR0FBRyxVQUFVLENBQUM7SUFDL0IsQ0FBQztJQUVEOztPQUVHO0lBQ0gsS0FBSztRQUNILFFBQVEsSUFBSSxDQUFDLE1BQU0sQ0FBQyxTQUFTLEVBQUUsQ0FBQztZQUM5QixLQUFLLFVBQVU7Z0JBQ2IsT0FBTyxJQUFJLENBQUMsU0FBUyxDQUFDO1lBRXhCLEtBQUssUUFBUTtnQkFDWCxPQUFPLElBQUksQ0FBQyxTQUFTLEdBQUcsQ0FBQyxDQUFDLEdBQUcsSUFBSSxDQUFDLFdBQVcsR0FBRyxJQUFJLENBQUMsVUFBVSxDQUFDLENBQUM7WUFFbkUsS0FBSyxRQUFRO2dCQUNY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dXJjZXNDb250ZW50IjpbIi8qKlxuICogVHJhaW5pbmcgUGlwZWxpbmUgZm9yIFNPTkFcbiAqXG4gKiBDb21wcmVoZW5zaXZlIHRyYWluaW5nIGluZnJhc3RydWN0dXJlIHdpdGggbWV0cmljcyB0cmFja2luZyxcbiAqIGxlYXJuaW5nIHJhdGUgc2NoZWR1bGluZywgYW5kIGNoZWNrcG9pbnQgbWFuYWdlbWVudC5cbiAqXG4gKiBAZXhhbXBsZVxuICogYGBgdHlwZXNjcmlwdFxuICogaW1wb3J0IHsgVHJhaW5pbmdQaXBlbGluZSwgVHJhaW5pbmdDb25maWcgfSBmcm9tICdAcnV2ZWN0b3IvcnV2bGxtJztcbiAqXG4gKiBjb25zdCBwaXBlbGluZSA9IG5ldyBUcmFpbmluZ1BpcGVsaW5lKHtcbiAqICAgbGVhcm5pbmdSYXRlOiAwLjAwMSxcbiAqICAgYmF0Y2hTaXplOiAzMixcbiAqICAgZXBvY2hzOiAxMCxcbiAqIH0pO1xuICpcbiAqIC8vIEFkZCB0cmFpbmluZyBkYXRhXG4gKiBwaXBlbGluZS5hZGRCYXRjaChpbnB1dHMsIHRhcmdldHMsIHF1YWxpdGllcyk7XG4gKlxuICogLy8gUnVuIHRyYWluaW5nXG4gKiBjb25zdCByZXN1bHQgPSBwaXBlbGluZS50cmFpbigpO1xuICogY29uc29sZS5sb2coYEZpbmFsIGxvc3M6ICR7cmVzdWx0LmZpbmFsTG9zc31gKTtcbiAqIGBgYFxuICovXG5cbmltcG9ydCB7IEVtYmVkZGluZywgVHJhaW5pbmdDb25maWcsIFRyYWluaW5nUmVzdWx0IH0gZnJvbSAnLi90eXBlcyc7XG5pbXBvcnQgeyBMb3JhQWRhcHRlciB9IGZyb20gJy4vbG9yYSc7XG5pbXBvcnQgeyBFd2NNYW5hZ2VyIH0gZnJvbSAnLi9zb25hJztcblxuLyoqXG4gKiBEZWZhdWx0IHRyYWluaW5nIGNvbmZpZ1xuICovXG5jb25zdCBERUZBVUxUX1RSQUlOSU5HX0NPTkZJRzogUmVxdWlyZWQ8VHJhaW5pbmdDb25maWc+ID0ge1xuICBsZWFybmluZ1JhdGU6IDAuMDAxLFxuICBiYXRjaFNpemU6IDMyLFxuICBlcG9jaHM6IDEwLFxuICBzY2hlZHVsZXI6ICdjb3NpbmUnLFxuICB3YXJtdXBTdGVwczogMTAwLFxuICB3ZWlnaHREZWNheTogMC4wMSxcbiAgZ3JhZGllbnRDbGlwOiAxLjAsXG4gIGVhcmx5U3RvcHBpbmdQYXRpZW5jZTogMyxcbiAgY2hlY2twb2ludEludGVydmFsOiAxLFxuICBld2NMYW1iZGE6IDIwMDAsXG4gIHZhbGlkYXRpb25TcGxpdDogMC4xLFxufTtcblxuLyoqXG4gKiBUcmFpbmluZyBtZXRyaWNzXG4gKi9cbmV4cG9ydCBpbnRlcmZhY2UgVHJhaW5pbmdNZXRyaWNzIHtcbiAgLyoqIEN1cnJlbnQgZXBvY2ggKi9cbiAgZXBvY2g6IG51bWJlcjtcbiAgLyoqIEN1cnJlbnQgc3RlcCAqL1xuICBzdGVwOiBudW1iZXI7XG4gIC8qKiBUcmFpbmluZyBsb3NzICovXG4gIHRyYWluTG9zczogbnVtYmVyO1xuICAvKiogVmFsaWRhdGlvbiBsb3NzICovXG4gIHZhbExvc3M6IG51bWJlcjtcbiAgLyoqIExlYXJuaW5nIHJhdGUgKi9cbiAgbGVhcm5pbmdSYXRlOiBudW1iZXI7XG4gIC8qKiBHcmFkaWVudCBub3JtICovXG4gIGdyYWROb3JtOiBudW1iZXI7XG4gIC8qKiBTdGVwcyBwZXIgc2Vjb25kICovXG4gIHN0ZXBzUGVyU2Vjb25kOiBudW1iZXI7XG4gIC8qKiBFVEEgaW4gc2Vjb25kcyAqL1xuICBldGFTZWNvbmRzOiBudW1iZXI7XG59XG5cbi8qKlxuICogVHJhaW5pbmcgZGF0YSBiYXRjaFxuICovXG5leHBvcnQgaW50ZXJmYWNlIFRyYWluaW5nQmF0Y2gge1xuICAvKiogSW5wdXQgZW1iZWRkaW5ncyAqL1xuICBpbnB1dHM6IEVtYmVkZGluZ1tdO1xuICAvKiogVGFyZ2V0IG91dHB1dHMgKi9cbiAgdGFyZ2V0czogRW1iZWRkaW5nW107XG4gIC8qKiBRdWFsaXR5IHNjb3JlcyAqL1xuICBxdWFsaXRpZXM6IG51bWJlcltdO1xufVxuXG4vKipcbiAqIENoZWNrcG9pbnQgZGF0YVxuICovXG5leHBvcnQgaW50ZXJmYWNlIENoZWNrcG9pbnQge1xuICAvKiogRXBvY2ggbnVtYmVyICovXG4gIGVwb2NoOiBudW1iZXI7XG4gIC8qKiBTdGVwIG51bWJlciAqL1xuICBzdGVwOiBudW1iZXI7XG4gIC8qKiBUcmFpbmluZyBsb3NzIGF0IGNoZWNrcG9pbnQgKi9cbiAgbG9zczogbnVtYmVyO1xuICAvKiogTW9kZWwgd2VpZ2h0cyAoc2VyaWFsaXplZCkgKi9cbiAgd2VpZ2h0czogc3RyaW5nO1xuICAvKiogVGltZXN0YW1wICovXG4gIHRpbWVzdGFtcDogbnVtYmVyO1xufVxuXG4vKipcbiAqIExlYXJuaW5nIFJhdGUgU2NoZWR1bGVyXG4gKi9cbmV4cG9ydCBjbGFzcyBMUlNjaGVkdWxlciB7XG4gIHByaXZhdGUgY29uZmlnOiBSZXF1aXJlZDxUcmFpbmluZ0NvbmZpZz47XG4gIHByaXZhdGUgaW5pdGlhbExSOiBudW1iZXI7XG4gIHByaXZhdGUgY3VycmVudFN0ZXA6IG51bWJlciA9IDA7XG4gIHByaXZhdGUgdG90YWxTdGVwczogbnVtYmVyO1xuXG4gIGNvbnN0cnVjdG9yKGNvbmZpZzogUmVxdWlyZWQ8VHJhaW5pbmdDb25maWc+LCB0b3RhbFN0ZXBzOiBudW1iZXIpIHtcbiAgICB0aGlzLmNvbmZpZyA9IGNvbmZpZztcbiAgICB0aGlzLmluaXRpYWxMUiA9IGNvbmZpZy5sZWFybmluZ1JhdGU7XG4gICAgdGhpcy50b3RhbFN0ZXBzID0gdG90YWxTdGVwcztcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgbGVhcm5pbmcgcmF0ZSBmb3IgY3VycmVudCBzdGVwXG4gICAqL1xuICBnZXRMUigpOiBudW1iZXIge1xuICAgIHN3aXRjaCAodGhpcy5jb25maWcuc2NoZWR1bGVyKSB7XG4gICAgICBjYXNlICdjb25zdGFudCc6XG4gICAgICAgIHJldHVybiB0aGlzLmluaXRpYWxMUjtcblxuICAgICAgY2FzZSAnbGluZWFyJzpcbiAgICAgICAgcmV0dXJuIHRoaXMuaW5pdGlhbExSICogKDEgLSB0aGlzLmN1cnJlbnRTdGVwIC8gdGhpcy50b3RhbFN0ZXBzKTtcblxuICAgICAgY2FzZSAnY29zaW5lJzpcbiAgICAgICAgcmV0dXJuIHRoaXMuaW5pdGlhbExSICogMC41ICogKDEgKyBNYXRoLmNvcyhNYXRoLlBJICogdGhpcy5jdXJyZW50U3RlcCAvIHRoaXMudG90YWxTdGVwcykpO1xuXG4gICAgICBjYXNlICd3YXJtdXAnOlxuICAgICAgICBpZiAodGhpcy5jdXJyZW50U3RlcCA8IHRoaXMuY29uZmlnLndhcm11cFN0ZXBzKSB7XG4gICAgICAgICAgcmV0dXJuIHRoaXMuaW5pdGlhbExSICogKHRoaXMuY3VycmVudFN0ZXAgLyB0aGlzLmNvbmZpZy53YXJtdXBTdGVwcyk7XG4gICAgICAgIH1cbiAgICAgICAgLy8gQ29zaW5lIGRlY2F5IGFmdGVyIHdhcm11cFxuICAgICAgICBjb25zdCBkZWNheVN0ZXBzID0gdGhpcy50b3RhbFN0ZXBzIC0gdGhpcy5jb25maWcud2FybXVwU3RlcHM7XG4gICAgICAgIGNvbnN0IGRlY2F5UHJvZ3Jlc3MgPSAodGhpcy5jdXJyZW50U3RlcCAtIHRoaXMuY29uZmlnLndhcm11cFN0ZXBzKSAvIGRlY2F5U3RlcHM7XG4gICAgICAgIHJldHVybiB0aGlzLmluaXRpYWxMUiAqIDAuNSAqICgxICsgTWF0aC5jb3MoTWF0aC5QSSAqIGRlY2F5UHJvZ3Jlc3MpKTtcblxuICAgICAgZGVmYXVsdDpcbiAgICAgICAgcmV0dXJuIHRoaXMuaW5pdGlhbExSO1xuICAgIH1cbiAgfVxuXG4gIC8qKlxuICAgKiBTdGVwIHRoZSBzY2hlZHVsZXJcbiAgICovXG4gIHN0ZXAoKTogdm9pZCB7XG4gICAgdGhpcy5jdXJyZW50U3RlcCsrO1xuICB9XG5cbiAgLyoqXG4gICAqIFJlc2V0IHNjaGVkdWxlclxuICAgKi9cbiAgcmVzZXQoKTogdm9pZCB7XG4gICAgdGhpcy5jdXJyZW50U3RlcCA9IDA7XG4gIH1cbn1cblxuLyoqXG4gKiBUcmFpbmluZyBNZXRyaWNzIFRyYWNrZXJcbiAqL1xuZXhwb3J0IGNsYXNzIE1ldHJpY3NUcmFja2VyIHtcbiAgcHJpdmF0ZSBsb3NzSGlzdG9yeTogbnVtYmVyW10gPSBbXTtcbiAgcHJpdmF0ZSB2YWxMb3NzSGlzdG9yeTogbnVtYmVyW10gPSBbXTtcbiAgcHJpdmF0ZSBncmFkTm9ybUhpc3Rvcnk6IG51bWJlcltdID0gW107XG4gIHByaXZhdGUgc3RhcnRUaW1lOiBudW1iZXIgPSBEYXRlLm5vdygpO1xuICBwcml2YXRlIHN0ZXBUaW1lczogbnVtYmVyW10gPSBbXTtcblxuICAvKipcbiAgICogUmVjb3JkIHRyYWluaW5nIGxvc3NcbiAgICovXG4gIHJlY29yZExvc3MobG9zczogbnVtYmVyKTogdm9pZCB7XG4gICAgdGhpcy5sb3NzSGlzdG9yeS5wdXNoKGxvc3MpO1xuICB9XG5cbiAgLyoqXG4gICAqIFJlY29yZCB2YWxpZGF0aW9uIGxvc3NcbiAgICovXG4gIHJlY29yZFZhbExvc3MobG9zczogbnVtYmVyKTogdm9pZCB7XG4gICAgdGhpcy52YWxMb3NzSGlzdG9yeS5wdXNoKGxvc3MpO1xuICB9XG5cbiAgLyoqXG4gICAqIFJlY29yZCBncmFkaWVudCBub3JtXG4gICAqL1xuICByZWNvcmRHcmFkTm9ybShub3JtOiBudW1iZXIpOiB2b2lkIHtcbiAgICB0aGlzLmdyYWROb3JtSGlzdG9yeS5wdXNoKG5vcm0pO1xuICB9XG5cbiAgLyoqXG4gICAqIFJlY29yZCBzdGVwIHRpbWVcbiAgICovXG4gIHJlY29yZFN0ZXBUaW1lKG1zOiBudW1iZXIpOiB2b2lkIHtcbiAgICB0aGlzLnN0ZXBUaW1lcy5wdXNoKG1zKTtcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgYXZlcmFnZSBsb3NzIG92ZXIgbGFzdCBOIHN0ZXBzXG4gICAqL1xuICBhdmdMb3NzKG46IG51bWJlciA9IDEwMCk6IG51bWJlciB7XG4gICAgY29uc3QgcmVjZW50ID0gdGhpcy5sb3NzSGlzdG9yeS5zbGljZSgtbik7XG4gICAgcmV0dXJuIHJlY2VudC5sZW5ndGggPiAwID8gcmVjZW50LnJlZHVjZSgoYSwgYikgPT4gYSArIGIsIDApIC8gcmVjZW50Lmxlbmd0aCA6IDA7XG4gIH1cblxuICAvKipcbiAgICogR2V0IGF2ZXJhZ2UgdmFsaWRhdGlvbiBsb3NzXG4gICAqL1xuICBhdmdWYWxMb3NzKG46IG51bWJlciA9IDEwKTogbnVtYmVyIHtcbiAgICBjb25zdCByZWNlbnQgPSB0aGlzLnZhbExvc3NIaXN0b3J5LnNsaWNlKC1uKTtcbiAgICByZXR1cm4gcmVjZW50Lmxlbmd0aCA+IDAgPyByZWNlbnQucmVkdWNlKChhLCBiKSA9PiBhICsgYiwgMCkgLyByZWNlbnQubGVuZ3RoIDogMDtcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgc3RlcHMgcGVyIHNlY29uZFxuICAgKi9cbiAgc3RlcHNQZXJTZWNvbmQoKTogbnVtYmVyIHtcbiAgICBpZiAodGhpcy5zdGVwVGltZXMubGVuZ3RoID09PSAwKSByZXR1cm4gMDtcbiAgICBjb25zdCBhdmdTdGVwVGltZSA9IHRoaXMuc3RlcFRpbWVzLnNsaWNlKC0xMDApLnJlZHVjZSgoYSwgYikgPT4gYSArIGIsIDApIC8gTWF0aC5taW4odGhpcy5zdGVwVGltZXMubGVuZ3RoLCAxMDApO1xuICAgIHJldHVybiBhdmdTdGVwVGltZSA+IDAgPyAxMDAwIC8gYXZnU3RlcFRpbWUgOiAwO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCBFVEEgaW4gc2Vjb25kc1xuICAgKi9cbiAgZXRhKHJlbWFpbmluZ1N0ZXBzOiBudW1iZXIpOiBudW1iZXIge1xuICAgIGNvbnN0IHNwcyA9IHRoaXMuc3RlcHNQZXJTZWNvbmQoKTtcbiAgICByZXR1cm4gc3BzID4gMCA/IHJlbWFpbmluZ1N0ZXBzIC8gc3BzIDogMDtcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgYmVzdCB2YWxpZGF0aW9uIGxvc3NcbiAgICovXG4gIGJlc3RWYWxMb3NzKCk6IG51bWJlciB7XG4gICAgcmV0dXJuIHRoaXMudmFsTG9zc0hpc3RvcnkubGVuZ3RoID4gMCA/IE1hdGgubWluKC4uLnRoaXMudmFsTG9zc0hpc3RvcnkpIDogSW5maW5pdHk7XG4gIH1cblxuICAvKipcbiAgICogR2V0IHRvdGFsIGR1cmF0aW9uXG4gICAqL1xuICBkdXJhdGlvbigpOiBudW1iZXIge1xuICAgIHJldHVybiBEYXRlLm5vdygpIC0gdGhpcy5zdGFydFRpbWU7XG4gIH1cblxuICAvKipcbiAgICogR2V0IGFsbCBsb3NzIGhpc3RvcnlcbiAgICovXG4gIGdldExvc3NIaXN0b3J5KCk6IG51bWJlcltdIHtcbiAgICByZXR1cm4gWy4uLnRoaXMubG9zc0hpc3RvcnldO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCBhbGwgdmFsaWRhdGlvbiBsb3NzIGhpc3RvcnlcbiAgICovXG4gIGdldFZhbExvc3NIaXN0b3J5KCk6IG51bWJlcltdIHtcbiAgICByZXR1cm4gWy4uLnRoaXMudmFsTG9zc0hpc3RvcnldO1xuICB9XG5cbiAgLyoqXG4gICAqIFJlc2V0IHRyYWNrZXJcbiAgICovXG4gIHJlc2V0KCk6IHZvaWQge1xuICAgIHRoaXMubG9zc0hpc3RvcnkgPSBbXTtcbiAgICB0aGlzLnZhbExvc3NIaXN0b3J5ID0gW107XG4gICAgdGhpcy5ncmFkTm9ybUhpc3RvcnkgPSBbXTtcbiAgICB0aGlzLnN0ZXBUaW1lcyA9IFtdO1xuICAgIHRoaXMuc3RhcnRUaW1lID0gRGF0ZS5ub3coKTtcbiAgfVxufVxuXG4vKipcbiAqIFRyYWluaW5nIFBpcGVsaW5lXG4gKlxuICogRnVsbCB0cmFpbmluZyBpbmZyYXN0cnVjdHVyZSBmb3IgU09OQSBtb2RlbHMuXG4gKi9cbmV4cG9ydCBjbGFzcyBUcmFpbmluZ1BpcGVsaW5lIHtcbiAgcHJpdmF0ZSBjb25maWc6IFJlcXVpcmVkPFRyYWluaW5nQ29uZmlnPjtcbiAgcHJpdmF0ZSBhZGFwdGVyOiBMb3JhQWRhcHRlcjtcbiAgcHJpdmF0ZSBld2NNYW5hZ2VyOiBFd2NNYW5hZ2VyO1xuICBwcml2YXRlIG1ldHJpY3M6IE1ldHJpY3NUcmFja2VyO1xuICBwcml2YXRlIHNjaGVkdWxlcjogTFJTY2hlZHVsZXIgfCBudWxsID0gbnVsbDtcbiAgcHJpdmF0ZSBiYXRjaGVzOiBUcmFpbmluZ0JhdGNoW10gPSBbXTtcbiAgcHJpdmF0ZSBjaGVja3BvaW50czogQ2hlY2twb2ludFtdID0gW107XG4gIHByaXZhdGUgY3VycmVudEVwb2NoOiBudW1iZXIgPSAwO1xuICBwcml2YXRlIGN1cnJlbnRTdGVwOiBudW1iZXIgPSAwO1xuICBwcml2YXRlIGJlc3RWYWxMb3NzOiBudW1iZXIgPSBJbmZpbml0eTtcbiAgcHJpdmF0ZSBwYXRpZW5jZUNvdW50ZXI6IG51bWJlciA9IDA7XG5cbiAgY29uc3RydWN0b3IoY29uZmlnPzogVHJhaW5pbmdDb25maWcsIGFkYXB0ZXI/OiBMb3JhQWRhcHRlcikge1xuICAgIHRoaXMuY29uZmlnID0geyAuLi5ERUZBVUxUX1RSQUlOSU5HX0NPTkZJRywgLi4uY29uZmlnIH07XG4gICAgdGhpcy5hZGFwdGVyID0gYWRhcHRlciB8fCBuZXcgTG9yYUFkYXB0ZXIoeyByYW5rOiA4IH0pO1xuICAgIHRoaXMuZXdjTWFuYWdlciA9IG5ldyBFd2NNYW5hZ2VyKHRoaXMuY29uZmlnLmV3Y0xhbWJkYSk7XG4gICAgdGhpcy5tZXRyaWNzID0gbmV3IE1ldHJpY3NUcmFja2VyKCk7XG4gIH1cblxuICAvKipcbiAgICogQWRkIHRyYWluaW5nIGJhdGNoXG4gICAqL1xuICBhZGRCYXRjaChpbnB1dHM6IEVtYmVkZGluZ1tdLCB0YXJnZXRzOiBFbWJlZGRpbmdbXSwgcXVhbGl0aWVzOiBudW1iZXJbXSk6IHZvaWQge1xuICAgIHRoaXMuYmF0Y2hlcy5wdXNoKHsgaW5wdXRzLCB0YXJnZXRzLCBxdWFsaXRpZXMgfSk7XG4gIH1cblxuICAvKipcbiAgICogQWRkIHRyYWluaW5nIGRhdGFcbiAgICovXG4gIGFkZERhdGEoZGF0YTogQXJyYXk8eyBpbnB1dDogRW1iZWRkaW5nOyB0YXJnZXQ6IEVtYmVkZGluZzsgcXVhbGl0eTogbnVtYmVyIH0+KTogdm9pZCB7XG4gICAgLy8gR3JvdXAgaW50byBiYXRjaGVzXG4gICAgZm9yIChsZXQgaSA9IDA7IGkgPCBkYXRhLmxlbmd0aDsgaSArPSB0aGlzLmNvbmZpZy5iYXRjaFNpemUpIHtcbiAgICAgIGNvbnN0IGJhdGNoID0gZGF0YS5zbGljZShpLCBpICsgdGhpcy5jb25maWcuYmF0Y2hTaXplKTtcbiAgICAgIHRoaXMuYWRkQmF0Y2goXG4gICAgICAgIGJhdGNoLm1hcChkID0+IGQuaW5wdXQpLFxuICAgICAgICBiYXRjaC5tYXAoZCA9PiBkLnRhcmdldCksXG4gICAgICAgIGJhdGNoLm1hcChkID0+IGQucXVhbGl0eSlcbiAgICAgICk7XG4gICAgfVxuICB9XG5cbiAgLyoqXG4gICAqIFJ1biB0cmFpbmluZ1xuICAgKi9cbiAgdHJhaW4oKTogVHJhaW5pbmdSZXN1bHQge1xuICAgIGNvbnN0IHRvdGFsU3RlcHMgPSB0aGlzLmJhdGNoZXMubGVuZ3RoICogdGhpcy5jb25maWcuZXBvY2hzO1xuICAgIHRoaXMuc2NoZWR1bGVyID0gbmV3IExSU2NoZWR1bGVyKHRoaXMuY29uZmlnLCB0b3RhbFN0ZXBzKTtcbiAgICB0aGlzLm1ldHJpY3MucmVzZXQoKTtcbiAgICB0aGlzLmFkYXB0ZXIuc3RhcnRUcmFpbmluZyh0aGlzLmNvbmZpZy5sZWFybmluZ1JhdGUpO1xuXG4gICAgbGV0IGVhcmx5U3RvcHBlZCA9IGZhbHNlO1xuXG4gICAgZm9yIChsZXQgZXBvY2ggPSAwOyBlcG9jaCA8IHRoaXMuY29uZmlnLmVwb2NoczsgZXBvY2grKykge1xuICAgICAgdGhpcy5jdXJyZW50RXBvY2ggPSBlcG9jaDtcblxuICAgICAgLy8gU2h1ZmZsZSBiYXRjaGVzXG4gICAgICBjb25zdCBzaHVmZmxlZEJhdGNoZXMgPSB0aGlzLnNodWZmbGVCYXRjaGVzKCk7XG5cbiAgICAgIC8vIFNwbGl0IGludG8gdHJhaW4vdmFsXG4gICAgICBjb25zdCB2YWxTaXplID0gTWF0aC5mbG9vcihzaHVmZmxlZEJhdGNoZXMubGVuZ3RoICogdGhpcy5jb25maWcudmFsaWRhdGlvblNwbGl0KTtcbiAgICAgIGNvbnN0IHRyYWluQmF0Y2hlcyA9IHNodWZmbGVkQmF0Y2hlcy5zbGljZSh2YWxTaXplKTtcbiAgICAgIGNvbnN0IHZhbEJhdGNoZXMgPSBzaHVmZmxlZEJhdGNoZXMuc2xpY2UoMCwgdmFsU2l6ZSk7XG5cbiAgICAgIC8vIFRyYWluaW5nIGVwb2NoXG4gICAgICBmb3IgKGNvbnN0IGJhdGNoIG9mIHRyYWluQmF0Y2hlcykge1xuICAgICAgICBjb25zdCBzdGVwU3RhcnQgPSBEYXRlLm5vdygpO1xuICAgICAgICBjb25zdCBsb3NzID0gdGhpcy50cmFpblN0ZXAoYmF0Y2gpO1xuICAgICAgICB0aGlzLm1ldHJpY3MucmVjb3JkTG9zcyhsb3NzKTtcbiAgICAgICAgdGhpcy5tZXRyaWNzLnJlY29yZFN0ZXBUaW1lKERhdGUubm93KCkgLSBzdGVwU3RhcnQpO1xuICAgICAgICB0aGlzLnNjaGVkdWxlci5zdGVwKCk7XG4gICAgICAgIHRoaXMuY3VycmVudFN0ZXArKztcbiAgICAgIH1cblxuICAgICAgLy8gVmFsaWRhdGlvblxuICAgICAgaWYgKHZhbEJhdGNoZXMubGVuZ3RoID4gMCkge1xuICAgICAgICBjb25zdCB2YWxMb3NzID0gdGhpcy52YWxpZGF0ZSh2YWxCYXRjaGVzKTtcbiAgICAgICAgdGhpcy5tZXRyaWNzLnJlY29yZFZhbExvc3ModmFsTG9zcyk7XG5cbiAgICAgICAgLy8gRWFybHkgc3RvcHBpbmdcbiAgICAgICAgaWYgKHZhbExvc3MgPCB0aGlzLmJlc3RWYWxMb3NzKSB7XG4gICAgICAgICAgdGhpcy5iZXN0VmFsTG9zcyA9IHZhbExvc3M7XG4gICAgICAgICAgdGhpcy5wYXRpZW5jZUNvdW50ZXIgPSAwO1xuICAgICAgICB9IGVsc2Uge1xuICAgICAgICAgIHRoaXMucGF0aWVuY2VDb3VudGVyKys7XG4gICAgICAgICAgaWYgKHRoaXMucGF0aWVuY2VDb3VudGVyID49IHRoaXMuY29uZmlnLmVhcmx5U3RvcHBpbmdQYXRpZW5jZSkge1xuICAgICAgICAgICAgZWFybHlTdG9wcGVkID0gdHJ1ZTtcbiAgICAgICAgICAgIGJyZWFrO1xuICAgICAgICAgIH1cbiAgICAgICAgfVxuICAgICAgfVxuXG4gICAgICAvLyBDaGVja3BvaW50XG4gICAgICBpZiAoKGVwb2NoICsgMSkgJSB0aGlzLmNvbmZpZy5jaGVja3BvaW50SW50ZXJ2YWwgPT09IDApIHtcbiAgICAgICAgdGhpcy5zYXZlQ2hlY2twb2ludCgpO1xuICAgICAgfVxuICAgIH1cblxuICAgIHRoaXMuYWRhcHRlci5lbmRUcmFpbmluZygpO1xuXG4gICAgLy8gUmVnaXN0ZXIgd2l0aCBFV0MgZm9yIGNvbnRpbnVhbCBsZWFybmluZ1xuICAgIGNvbnN0IHdlaWdodHMgPSB0aGlzLmFkYXB0ZXIubWVyZ2UoKS5mbGF0KCk7XG4gICAgdGhpcy5ld2NNYW5hZ2VyLnJlZ2lzdGVyVGFzayhgdGFzay0ke0RhdGUubm93KCl9YCwgd2VpZ2h0cyk7XG5cbiAgICByZXR1cm4ge1xuICAgICAgZXBvY2hzOiB0aGlzLmN1cnJlbnRFcG9jaCArIDEsXG4gICAgICBzdGVwczogdGhpcy5jdXJyZW50U3RlcCxcbiAgICAgIGZpbmFsTG9zczogdGhpcy5tZXRyaWNzLmF2Z0xvc3MoMTAwKSxcbiAgICAgIGJlc3RWYWxMb3NzOiB0aGlzLmJlc3RWYWxMb3NzLFxuICAgICAgZHVyYXRpb25NczogdGhpcy5tZXRyaWNzLmR1cmF0aW9uKCksXG4gICAgICBsb3NzSGlzdG9yeTogdGhpcy5tZXRyaWNzLmdldExvc3NIaXN0b3J5KCksXG4gICAgICB2YWxMb3NzSGlzdG9yeTogdGhpcy5tZXRyaWNzLmdldFZhbExvc3NIaXN0b3J5KCksXG4gICAgICBlYXJseVN0b3BwZWQsXG4gICAgfTtcbiAgfVxuXG4gIC8qKlxuICAgKiBTaW5nbGUgdHJhaW5pbmcgc3RlcFxuICAgKi9cbiAgcHJpdmF0ZSB0cmFpblN0ZXAoYmF0Y2g6IFRyYWluaW5nQmF0Y2gpOiBudW1iZXIge1xuICAgIGxldCB0b3RhbExvc3MgPSAwO1xuICAgIGNvbnN0IGxyID0gdGhpcy5zY2hlZHVsZXI/LmdldExSKCkgfHwgdGhpcy5jb25maWcubGVhcm5pbmdSYXRlO1xuXG4gICAgZm9yIChsZXQgaSA9IDA7IGkgPCBiYXRjaC5pbnB1dHMubGVuZ3RoOyBpKyspIHtcbiAgICAgIGNvbnN0IGlucHV0ID0gYmF0Y2guaW5wdXRzW2ldO1xuICAgICAgY29uc3QgdGFyZ2V0ID0gYmF0Y2gudGFyZ2V0c1tpXTtcbiAgICAgIGNvbnN0IHF1YWxpdHkgPSBiYXRjaC5xdWFsaXRpZXNbaV07XG5cbiAgICAgIC8vIEZvcndhcmQgcGFzc1xuICAgICAgY29uc3Qgb3V0cHV0ID0gdGhpcy5hZGFwdGVyLmZvcndhcmQoaW5wdXQpO1xuXG4gICAgICAvLyBDb21wdXRlIGxvc3MgKE1TRSB3ZWlnaHRlZCBieSBxdWFsaXR5KVxuICAgICAgY29uc3QgZ3JhZE91dHB1dDogbnVtYmVyW10gPSBbXTtcbiAgICAgIGxldCBsb3NzID0gMDtcbiAgICAgIGZvciAobGV0IGogPSAwOyBqIDwgb3V0cHV0Lmxlbmd0aDsgaisrKSB7XG4gICAgICAgIGNvbnN0IGRpZmYgPSBvdXRwdXRbal0gLSAodGFyZ2V0W2pdIHx8IDApO1xuICAgICAgICBsb3NzICs9IGRpZmYgKiBkaWZmO1xuICAgICAgICBncmFkT3V0cHV0LnB1c2goMiAqIGRpZmYgKiBxdWFsaXR5KTsgLy8gUXVhbGl0eS13ZWlnaHRlZCBncmFkaWVudFxuICAgICAgfVxuICAgICAgbG9zcyA9IChsb3NzIC8gb3V0cHV0Lmxlbmd0aCkgKiBxdWFsaXR5O1xuXG4gICAgICAvLyBBZGQgRVdDIHBlbmFsdHlcbiAgICAgIGNvbnN0IGV3Y1BlbmFsdHkgPSB0aGlzLmV3Y01hbmFnZXIuY29tcHV0ZVBlbmFsdHkodGhpcy5hZGFwdGVyLm1lcmdlKCkuZmxhdCgpKTtcbiAgICAgIGxvc3MgKz0gZXdjUGVuYWx0eSAqIDAuMDAxO1xuXG4gICAgICAvLyBCYWNrd2FyZCBwYXNzXG4gICAgICB0aGlzLmFkYXB0ZXIuYmFja3dhcmQoaW5wdXQsIGdyYWRPdXRwdXQsIGxyKTtcblxuICAgICAgdG90YWxMb3NzICs9IGxvc3M7XG4gICAgfVxuXG4gICAgcmV0dXJuIHRvdGFsTG9zcyAvIGJhdGNoLmlucHV0cy5sZW5ndGg7XG4gIH1cblxuICAvKipcbiAgICogVmFsaWRhdGlvbiBwYXNzXG4gICAqL1xuICBwcml2YXRlIHZhbGlkYXRlKGJhdGNoZXM6IFRyYWluaW5nQmF0Y2hbXSk6IG51bWJlciB7XG4gICAgbGV0IHRvdGFsTG9zcyA9IDA7XG4gICAgbGV0IGNvdW50ID0gMDtcblxuICAgIGZvciAoY29uc3QgYmF0Y2ggb2YgYmF0Y2hlcykge1xuICAgICAgZm9yIChsZXQgaSA9IDA7IGkgPCBiYXRjaC5pbnB1dHMubGVuZ3RoOyBpKyspIHtcbiAgICAgICAgY29uc3Qgb3V0cHV0ID0gdGhpcy5hZGFwdGVyLmZvcndhcmQoYmF0Y2guaW5wdXRzW2ldKTtcbiAgICAgICAgY29uc3QgdGFyZ2V0ID0gYmF0Y2gudGFyZ2V0c1tpXTtcblxuICAgICAgICBsZXQgbG9zcyA9IDA7XG4gICAgICAgIGZvciAobGV0IGogPSAwOyBqIDwgb3V0cHV0Lmxlbmd0aDsgaisrKSB7XG4gICAgICAgICAgY29uc3QgZGlmZiA9IG91dHB1dFtqXSAtICh0YXJnZXRbal0gfHwgMCk7XG4gICAgICAgICAgbG9zcyArPSBkaWZmICogZGlmZjtcbiAgICAgICAgfVxuICAgICAgICB0b3RhbExvc3MgKz0gbG9zcyAvIG91dHB1dC5sZW5ndGg7XG4gICAgICAgIGNvdW50Kys7XG4gICAgICB9XG4gICAgfVxuXG4gICAgcmV0dXJuIGNvdW50ID4gMCA/IHRvdGFsTG9zcyAvIGNvdW50IDogMDtcbiAgfVxuXG4gIC8qKlxuICAgKiBTYXZlIGNoZWNrcG9pbnRcbiAgICovXG4gIHByaXZhdGUgc2F2ZUNoZWNrcG9pbnQoKTogdm9pZCB7XG4gICAgdGhpcy5jaGVja3BvaW50cy5wdXNoKHtcbiAgICAgIGVwb2NoOiB0aGlzLmN1cnJlbnRFcG9jaCxcbiAgICAgIHN0ZXA6IHRoaXMuY3VycmVudFN0ZXAsXG4gICAgICBsb3NzOiB0aGlzLm1ldHJpY3MuYXZnTG9zcygxMDApLFxuICAgICAgd2VpZ2h0czogdGhpcy5hZGFwdGVyLnRvSlNPTigpLFxuICAgICAgdGltZXN0YW1wOiBEYXRlLm5vdygpLFxuICAgIH0pO1xuICB9XG5cbiAgLyoqXG4gICAqIExvYWQgY2hlY2twb2ludFxuICAgKi9cbiAgbG9hZENoZWNrcG9pbnQoaW5kZXg6IG51bWJlcik6IGJvb2xlYW4ge1xuICAgIGNvbnN0IGNoZWNrcG9pbnQgPSB0aGlzLmNoZWNrcG9pbnRzW2luZGV4XTtcbiAgICBpZiAoIWNoZWNrcG9pbnQpIHJldHVybiBmYWxzZTtcblxuICAgIHRoaXMuYWRhcHRlciA9IExvcmFBZGFwdGVyLmZyb21KU09OKGNoZWNrcG9pbnQud2VpZ2h0cyk7XG4gICAgdGhpcy5jdXJyZW50RXBvY2ggPSBjaGVja3BvaW50LmVwb2NoO1xuICAgIHRoaXMuY3VycmVudFN0ZXAgPSBjaGVja3BvaW50LnN0ZXA7XG4gICAgcmV0dXJuIHRydWU7XG4gIH1cblxuICAvKipcbiAgICogR2V0IGN1cnJlbnQgbWV0cmljc1xuICAgKi9cbiAgZ2V0TWV0cmljcygpOiBUcmFpbmluZ01ldHJpY3Mge1xuICAgIHJldHVybiB7XG4gICAgICBlcG9jaDogdGhpcy5jdXJyZW50RXBvY2gsXG4gICAgICBzdGVwOiB0aGlzLmN1cnJlbnRTdGVwLFxuICAgICAgdHJhaW5Mb3NzOiB0aGlzLm1ldHJpY3MuYXZnTG9zcygxMDApLFxuICAgICAgdmFsTG9zczogdGhpcy5tZXRyaWNzLmF2Z1ZhbExvc3MoMTApLFxuICAgICAgbGVhcm5pbmdSYXRlOiB0aGlzLnNjaGVkdWxlcj8uZ2V0TFIoKSB8fCB0aGlzLmNvbmZpZy5sZWFybmluZ1JhdGUsXG4gICAgICBncmFkTm9ybTogMCxcbiAgICAgIHN0ZXBzUGVyU2Vjb25kOiB0aGlzLm1ldHJpY3Muc3RlcHNQZXJTZWNvbmQoKSxcbiAgICAgIGV0YVNlY29uZHM6IHRoaXMubWV0cmljcy5ldGEoXG4gICAgICAgICh0aGlzLmNvbmZpZy5lcG9jaHMgLSB0aGlzLmN1cnJlbnRFcG9jaCkgKiB0aGlzLmJhdGNoZXMubGVuZ3RoXG4gICAgICApLFxuICAgIH07XG4gIH1cblxuICAvKipcbiAgICogR2V0IGFkYXB0ZXJcbiAgICovXG4gIGdldEFkYXB0ZXIoKTogTG9yYUFkYXB0ZXIge1xuICAgIHJldHVybiB0aGlzLmFkYXB0ZXI7XG4gIH1cblxuICAvKipcbiAgICogR2V0IEVXQyBtYW5hZ2VyXG4gICAqL1xuICBnZXRFd2NNYW5hZ2VyKCk6IEV3Y01hbmFnZXIge1xuICAgIHJldHVybiB0aGlzLmV3Y01hbmFnZXI7XG4gIH1cblxuICAvKipcbiAgICogR2V0IGNoZWNrcG9pbnRzXG4gICAqL1xuICBnZXRDaGVja3BvaW50cygpOiBDaGVja3BvaW50W10ge1xuICAgIHJldHVybiBbLi4udGhpcy5jaGVja3BvaW50c107XG4gIH1cblxuICAvKipcbiAgICogUmVzZXQgcGlwZWxpbmVcbiAgICovXG4gIHJlc2V0KCk6IHZvaWQge1xuICAgIHRoaXMuYmF0Y2hlcyA9IFtdO1xuICAgIHRoaXMuY2hlY2twb2ludHMgPSBbXTtcbiAgICB0aGlzLmN1cnJlbnRFcG9jaCA9IDA7XG4gICAgdGhpcy5jdXJyZW50U3RlcCA9IDA7XG4gICAgdGhpcy5iZXN0VmFsTG9zcyA9IEluZmluaXR5O1xuICAgIHRoaXMucGF0aWVuY2VDb3VudGVyID0gMDtcbiAgICB0aGlzLm1ldHJpY3MucmVzZXQoKTtcbiAgICB0aGlzLmFkYXB0ZXIucmVzZXQoKTtcbiAgfVxuXG4gIHByaXZhdGUgc2h1ZmZsZUJhdGNoZXMoKTogVHJhaW5pbmdCYXRjaFtdIHtcbiAgICBjb25zdCBzaHVmZmxlZCA9IFsuLi50aGlzLmJhdGNoZXNdO1xuICAgIGZvciAobGV0IGkgPSBzaHVmZmxlZC5sZW5ndGggLSAxOyBpID4gMDsgaS0tKSB7XG4gICAgICBjb25zdCBqID0gTWF0aC5mbG9vcihNYXRoLnJhbmRvbSgpICogKGkgKyAxKSk7XG4gICAgICBbc2h1ZmZsZWRbaV0sIHNodWZmbGVkW2pdXSA9IFtzaHVmZmxlZFtqXSwgc2h1ZmZsZWRbaV1dO1xuICAgIH1cbiAgICByZXR1cm4gc2h1ZmZsZWQ7XG4gIH1cbn1cblxuLyoqXG4gKiBUcmFpbmluZyBGYWN0b3J5XG4gKlxuICogQ3JlYXRlIHByZS1jb25maWd1cmVkIHRyYWluaW5nIHBpcGVsaW5lcyBmb3IgY29tbW9uIHNjZW5hcmlvcy5cbiAqL1xuZXhwb3J0IGNsYXNzIFRyYWluaW5nRmFjdG9yeSB7XG4gIC8qKlxuICAgKiBDcmVhdGUgcGlwZWxpbmUgZm9yIHF1aWNrIGZpbmUtdHVuaW5nXG4gICAqL1xuICBzdGF0aWMgcXVpY2tGaW5ldHVuZSgpOiBUcmFpbmluZ1BpcGVsaW5lIHtcbiAgICByZXR1cm4gbmV3IFRyYWluaW5nUGlwZWxpbmUoe1xuICAgICAgbGVhcm5pbmdSYXRlOiAwLjAxLFxuICAgICAgZXBvY2hzOiAzLFxuICAgICAgYmF0Y2hTaXplOiAxNixcbiAgICAgIHNjaGVkdWxlcjogJ2NvbnN0YW50JyxcbiAgICB9KTtcbiAgfVxuXG4gIC8qKlxuICAgKiBDcmVhdGUgcGlwZWxpbmUgZm9yIGRlZXAgdHJhaW5pbmdcbiAgICovXG4gIHN0YXRpYyBkZWVwVHJhaW5pbmcoKTogVHJhaW5pbmdQaXBlbGluZSB7XG4gICAgcmV0dXJuIG5ldyBUcmFpbmluZ1BpcGVsaW5lKHtcbiAgICAgIGxlYXJuaW5nUmF0ZTogMC4wMDEsXG4gICAgICBlcG9jaHM6IDUwLFxuICAgICAgYmF0Y2hTaXplOiAzMixcbiAgICAgIHNjaGVkdWxlcjogJ3dhcm11cCcsXG4gICAgICB3YXJtdXBTdGVwczogNTAwLFxuICAgICAgZWFybHlTdG9wcGluZ1BhdGllbmNlOiA1LFxuICAgIH0pO1xuICB9XG5cbiAgLyoqXG4gICAqIENyZWF0ZSBwaXBlbGluZSBmb3IgY29udGludWFsIGxlYXJuaW5nXG4gICAqL1xuICBzdGF0aWMgY29udGludWFsTGVhcm5pbmcoZXdjTGFtYmRhOiBudW1iZXIgPSA1MDAwKTogVHJhaW5pbmdQaXBlbGluZSB7XG4gICAgcmV0dXJuIG5ldyBUcmFpbmluZ1BpcGVsaW5lKHtcbiAgICAgIGxlYXJuaW5nUmF0ZTogMC4wMDA1LFxuICAgICAgZXBvY2hzOiAxMCxcbiAgICAgIGJhdGNoU2l6ZTogMTYsXG4gICAgICBzY2hlZHVsZXI6ICdjb3NpbmUnLFxuICAgICAgZXdjTGFtYmRhLFxuICAgICAgZWFybHlTdG9wcGluZ1BhdGllbmNlOiAxMCxcbiAgICB9KTtcbiAgfVxuXG4gIC8qKlxuICAgKiBDcmVhdGUgcGlwZWxpbmUgZm9yIGZlZGVyYXRlZCBhZ2dyZWdhdGlvblxuICAgKi9cbiAgc3RhdGljIGZlZGVyYXRlZEFnZ3JlZ2F0aW9uKCk6IFRyYWluaW5nUGlwZWxpbmUge1xuICAgIHJldHVybiBuZXcgVHJhaW5pbmdQaXBlbGluZSh7XG4gICAgICBsZWFybmluZ1JhdGU6IDAuMDAwMSxcbiAgICAgIGVwb2NoczogNSxcbiAgICAgIGJhdGNoU2l6ZTogNjQsXG4gICAgICBzY2hlZHVsZXI6ICdsaW5lYXInLFxuICAgICAgZXdjTGFtYmRhOiAyMDAwLFxuICAgIH0pO1xuICB9XG59XG4iXX0=