/**
 * ReasoningBank Type Definitions
 * Based on arXiv:2509.25140 (Google DeepMind)
 */
export interface Memory {
    id: string;
    title: string;
    description: string;
    content: string;
    confidence: number;
    usage_count: number;
    created_at: string;
    pattern_data: PatternData;
}
export interface PatternData {
    domain: string;
    error_type?: string;
    success_pattern?: boolean;
    failure_guardrail?: boolean;
    [key: string]: any;
}
export interface PatternEmbedding {
    pattern_id: string;
    embedding: number[];
}
export interface TaskTrajectory {
    id: string;
    task_id: string;
    trajectory: string;
    verdict: 'Success' | 'Failure';
    confidence: number;
    created_at: string;
}
export interface MattsRun {
    id: string;
    task_id: string;
    run_index: number;
    result: string;
    verdict: 'Success' | 'Failure';
    confidence: number;
    created_at: string;
}
export interface RetrievalOptions {
    query: string;
    domain?: string;
    k?: number;
    minConfidence?: number;
    lambda?: number;
}
export interface ScoringWeights {
    alpha: number;
    beta: number;
    gamma: number;
    delta: number;
}
export interface JudgmentResult {
    label: 'Success' | 'Failure';
    confidence: number;
    rationale?: string;
}
export interface ConsolidationOptions {
    dedupeThreshold?: number;
    prune?: {
        maxAgeDays?: number;
        minConfidence?: number;
        unusedDays?: number;
    };
}
export interface ConsolidationStats {
    processed: number;
    duplicates: number;
    contradictions: number;
    pruned: number;
    durationMs: number;
}
export interface ReasoningBankConfig {
    dbPath: string;
    embeddings?: {
        provider?: 'openai' | 'anthropic' | 'hash';
        model?: string;
        cache?: {
            l1?: boolean;
            l2?: boolean;
            redisUrl?: string;
        };
    };
    retrieval?: {
        k?: number;
        minConfidence?: number;
        weights?: Partial<ScoringWeights>;
    };
    consolidation?: {
        scheduleEvery?: number;
        autoRun?: boolean;
    };
    piiScrub?: {
        enabled?: boolean;
    };
}
export interface TaskExecutionOptions {
    taskId: string;
    agentId: string;
    query: string;
    domain: string;
    executeFn: (memories: Memory[]) => Promise<{
        success: boolean;
        log: string;
    }>;
}
export interface TaskResult {
    success: boolean;
    summary: string;
    memories: Memory[];
    verdict: JudgmentResult;
}
export interface MemoryCandidate extends Memory {
    score: number;
    similarity: number;
    recency: number;
    reliability: number;
    diversityPenalty: number;
}
//# sourceMappingURL=index.d.ts.map