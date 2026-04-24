/**
 * Memory Retrieval with MMR diversity
 * Algorithm 1 from ReasoningBank paper
 */
export interface RetrievedMemory {
    id: string;
    title: string;
    description: string;
    content: string;
    score: number;
    components: {
        similarity: number;
        recency: number;
        reliability: number;
    };
}
/**
 * Retrieve top-k memories with MMR diversity
 *
 * Scoring formula: score = α·sim + β·recency + γ·reliability
 * Where:
 * - sim: cosine similarity to query
 * - recency: exp(-age_days / half_life)
 * - reliability: min(confidence, 1.0)
 */
export declare function retrieveMemories(query: string, options?: {
    k?: number;
    domain?: string;
    agent?: string;
}): Promise<RetrievedMemory[]>;
/**
 * Format memories for injection into system prompt
 */
export declare function formatMemoriesForPrompt(memories: RetrievedMemory[]): string;
//# sourceMappingURL=retrieve.d.ts.map