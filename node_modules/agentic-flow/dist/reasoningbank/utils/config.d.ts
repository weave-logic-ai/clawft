export interface ReasoningBankConfig {
    retrieve: {
        k: number;
        alpha: number;
        beta: number;
        gamma: number;
        delta: number;
        recency_half_life_days: number;
        min_score: number;
    };
    judge: {
        model: string;
        max_tokens: number;
        temperature: number;
        confidence_threshold?: number;
    };
    distill: {
        model?: string;
        max_tokens?: number;
        temperature?: number;
        max_items_success: number;
        max_items_failure: number;
        confidence_prior_success: number;
        confidence_prior_failure: number;
    };
    consolidate: {
        duplicate_threshold: number;
        contradiction_threshold: number;
        trigger_threshold: number;
        prune_age_days: number;
        prune_min_confidence: number;
        min_confidence_keep: number;
    };
    matts: {
        parallel_k: number;
        sequential_k: number;
        sequential_r: number;
        sequential_stop_on_success: boolean;
        confidence_boost: number;
    };
    embeddings: {
        provider: 'claude' | 'openai';
        model: string;
        dims: number;
        dimensions: number;
        cache_ttl_seconds: number;
    };
    governance: {
        scrub_pii?: boolean;
        pii_scrubber: boolean;
        tenant_scoped?: boolean;
    };
    features?: {
        enable_pre_task_hook?: boolean;
        enable_post_task_hook?: boolean;
        enable_matts_parallel?: boolean;
    };
}
export declare function loadConfig(): ReasoningBankConfig;
export declare function clearConfigCache(): void;
//# sourceMappingURL=config.d.ts.map