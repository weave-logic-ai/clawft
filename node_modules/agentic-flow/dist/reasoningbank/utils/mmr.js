/**
 * Maximal Marginal Relevance (MMR) for diversity in retrieval
 * Balances relevance and diversity in top-k selection
 */
/**
 * Cosine similarity between two vectors
 */
function cosineSimilarity(a, b) {
    if (a.length !== b.length) {
        throw new Error(`Vector dimension mismatch: ${a.length} vs ${b.length}`);
    }
    let dot = 0, magA = 0, magB = 0;
    for (let i = 0; i < a.length; i++) {
        dot += a[i] * b[i];
        magA += a[i] * a[i];
        magB += b[i] * b[i];
    }
    const denominator = Math.sqrt(magA) * Math.sqrt(magB);
    if (denominator === 0)
        return 0;
    return dot / denominator;
}
/**
 * MMR selection for diversity-aware top-k retrieval
 *
 * @param candidates - Scored candidates with embeddings
 * @param queryEmbed - Query embedding for relevance scoring
 * @param k - Number of items to select
 * @param lambda - Balance between relevance (1.0) and diversity (0.0)
 * @returns Selected items with maximum marginal relevance
 */
export function mmrSelection(candidates, queryEmbed, k, lambda = 0.5) {
    if (candidates.length === 0)
        return [];
    if (k <= 0)
        return [];
    if (k >= candidates.length)
        return [...candidates];
    const selected = [];
    const remaining = [...candidates].sort((a, b) => b.score - a.score);
    while (selected.length < k && remaining.length > 0) {
        let bestIdx = 0;
        let bestScore = -Infinity;
        for (let i = 0; i < remaining.length; i++) {
            const item = remaining[i];
            // Calculate max similarity to already selected items
            let maxSimilarity = 0;
            for (const sel of selected) {
                const sim = cosineSimilarity(item.embedding, sel.embedding);
                maxSimilarity = Math.max(maxSimilarity, sim);
            }
            // MMR score balances relevance and diversity
            const mmrScore = lambda * item.score - (1 - lambda) * maxSimilarity;
            if (mmrScore > bestScore) {
                bestScore = mmrScore;
                bestIdx = i;
            }
        }
        selected.push(remaining[bestIdx]);
        remaining.splice(bestIdx, 1);
    }
    return selected;
}
export { cosineSimilarity };
//# sourceMappingURL=mmr.js.map