/**
 * TypeScript types for ReasoningBank database schema
 */
export interface ReasoningMemory {
    id: string;
    type: 'reasoning_memory';
    pattern_data: {
        title: string;
        description: string;
        content: string;
        source: {
            task_id: string;
            agent_id: string;
            outcome: 'Success' | 'Failure';
            evidence: string[];
        };
        tags: string[];
        domain?: string;
        created_at: string;
        confidence: number;
        n_uses: number;
    };
    confidence: number;
    usage_count: number;
    created_at: string;
    last_used?: string;
}
export interface PatternEmbedding {
    id: string;
    model: string;
    dims: number;
    vector: Float32Array;
    created_at: string;
}
export interface PatternLink {
    src_id: string;
    dst_id: string;
    relation: 'entails' | 'contradicts' | 'refines' | 'duplicate_of';
    weight: number;
    created_at: string;
}
export interface TaskTrajectory {
    task_id: string;
    agent_id: string;
    query: string;
    trajectory_json: string;
    started_at?: string;
    ended_at?: string;
    judge_label?: 'Success' | 'Failure';
    judge_conf?: number;
    judge_reasons?: string;
    matts_run_id?: string;
    created_at: string;
}
export interface MattsRun {
    run_id: string;
    task_id: string;
    mode: 'parallel' | 'sequential';
    k: number;
    status: 'pending' | 'running' | 'completed' | 'failed';
    summary?: string;
    created_at: string;
}
export interface ConsolidationRun {
    run_id: string;
    items_processed: number;
    duplicates_found: number;
    contradictions_found: number;
    items_pruned: number;
    duration_ms: number;
    created_at: string;
}
export interface Trajectory {
    steps: TrajectoryStep[];
    metadata?: Record<string, any>;
}
export interface TrajectoryStep {
    action: string;
    [key: string]: any;
}
//# sourceMappingURL=schema.d.ts.map