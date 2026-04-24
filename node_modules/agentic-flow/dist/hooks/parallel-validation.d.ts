/**
 * Validation hooks to ensure agents follow parallel execution best practices
 */
export interface AgentResponse {
    code?: string;
    output?: string;
    subprocesses?: Array<{
        command: string;
        result: any;
    }>;
    toolCalls?: Array<{
        name: string;
        args: any;
    }>;
}
export interface ExecutionMetrics {
    avgBatchSize: number;
    subprocessesSpawned: number;
    reasoningBankUsage: number;
    parallelOpsCount?: number;
    sequentialOpsCount?: number;
}
export interface ParallelValidationResult {
    score: number;
    issues: string[];
    recommendations: string[];
    metrics: {
        parallelOpsCount: number;
        sequentialOpsCount: number;
        avgBatchSize: number;
        subprocessesSpawned: number;
        reasoningBankUsage: number;
    };
}
/**
 * Validate agent's parallel execution patterns
 */
export declare function validateParallelExecution(response: AgentResponse, metrics: ExecutionMetrics): ParallelValidationResult;
/**
 * Post-execution hook: Log validation results and suggestions
 */
export declare function postExecutionValidation(response: AgentResponse, metrics: ExecutionMetrics): Promise<ParallelValidationResult>;
/**
 * Grade parallel execution quality
 */
export declare function gradeParallelExecution(score: number): {
    grade: string;
    description: string;
    color: string;
};
//# sourceMappingURL=parallel-validation.d.ts.map