import type { TrainOptions, TrainResult, AcceptanceOptions, AcceptanceManifest, PolicyState } from './types';
/**
 * RVF Self-Learning Solver.
 *
 * Wraps the rvf-solver-wasm WASM module providing:
 * - PolicyKernel with Thompson Sampling (two-signal model)
 * - Context-bucketed bandit (18 buckets)
 * - KnowledgeCompiler with signature-based pattern cache
 * - Speculative dual-path execution
 * - Three-loop adaptive solver (fast/medium/slow)
 * - SHAKE-256 tamper-evident witness chain
 */
export declare class RvfSolver {
    private handle;
    private wasm;
    private constructor();
    /**
     * Create a new solver instance.
     * Initializes the WASM module on first call.
     */
    static create(): Promise<RvfSolver>;
    /**
     * Train the solver on generated puzzles.
     *
     * Uses the three-loop architecture:
     * - Fast loop: constraint propagation solver
     * - Medium loop: PolicyKernel skip-mode selection
     * - Slow loop: KnowledgeCompiler pattern distillation
     */
    train(options: TrainOptions): TrainResult;
    /**
     * Run the full acceptance test with training/holdout cycles.
     *
     * Runs all three ablation modes:
     * - Mode A: Fixed heuristic policy
     * - Mode B: Compiler-suggested policy
     * - Mode C: Learned Thompson Sampling policy
     *
     * Returns the full manifest with per-cycle metrics and witness chain.
     */
    acceptance(options?: AcceptanceOptions): AcceptanceManifest;
    /**
     * Get the current policy state (Thompson Sampling parameters,
     * context buckets, KnowledgeCompiler cache stats).
     */
    policy(): PolicyState | null;
    /**
     * Get the raw SHAKE-256 witness chain bytes.
     *
     * The witness chain is 73 bytes per entry and provides
     * tamper-evident proof of all training/acceptance operations.
     * Verifiable using `rvf_witness_verify` from `@ruvector/rvf-wasm`.
     */
    witnessChain(): Uint8Array | null;
    /**
     * Destroy the solver instance and free WASM resources.
     */
    destroy(): void;
}
