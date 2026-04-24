/**
 * MMR (Maximal Marginal Relevance) Diversity Ranking
 *
 * Implements MMR algorithm to select diverse results that balance
 * relevance to query with diversity from already-selected results.
 *
 * Formula: MMR = argmax [λ × Sim(Di, Q) - (1-λ) × max Sim(Di, Dj)]
 *                Di∈R\S              Dj∈S
 *
 * Where:
 * - Di = candidate document
 * - Q = query
 * - S = already selected documents
 * - λ = balance parameter (0 = max diversity, 1 = max relevance)
 */
export interface MMROptions {
    lambda?: number;
    k?: number;
    metric?: 'cosine' | 'euclidean' | 'dot';
}
export interface MMRCandidate {
    id: number;
    embedding: number[];
    similarity: number;
    [key: string]: any;
}
export declare class MMRDiversityRanker {
    /**
     * Select diverse results using MMR algorithm
     *
     * @param candidates - All candidate results with embeddings
     * @param queryEmbedding - Query vector
     * @param options - MMR configuration
     * @returns Diverse subset of candidates
     */
    static selectDiverse(candidates: MMRCandidate[], queryEmbedding: number[], options?: MMROptions): MMRCandidate[];
    /**
     * Calculate similarity between two vectors
     */
    private static calculateSimilarity;
    /**
     * Calculate diversity score for a set of results
     *
     * @param results - Results to analyze
     * @param metric - Similarity metric
     * @returns Average pairwise distance (higher = more diverse)
     */
    static calculateDiversityScore(results: MMRCandidate[], metric?: 'cosine' | 'euclidean' | 'dot'): number;
}
//# sourceMappingURL=MMRDiversityRanker.d.ts.map