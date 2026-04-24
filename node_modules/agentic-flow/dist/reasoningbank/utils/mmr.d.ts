/**
 * Maximal Marginal Relevance (MMR) for diversity in retrieval
 * Balances relevance and diversity in top-k selection
 */
/**
 * Cosine similarity between two vectors
 */
declare function cosineSimilarity(a: Float32Array, b: Float32Array): number;
/**
 * MMR selection for diversity-aware top-k retrieval
 *
 * @param candidates - Scored candidates with embeddings
 * @param queryEmbed - Query embedding for relevance scoring
 * @param k - Number of items to select
 * @param lambda - Balance between relevance (1.0) and diversity (0.0)
 * @returns Selected items with maximum marginal relevance
 */
export declare function mmrSelection<T extends {
    score: number;
    embedding: Float32Array;
}>(candidates: T[], queryEmbed: Float32Array, k: number, lambda?: number): T[];
export { cosineSimilarity };
//# sourceMappingURL=mmr.d.ts.map