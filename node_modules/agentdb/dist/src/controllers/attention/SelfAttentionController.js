/**
 * SelfAttentionController - Self-Attention Mechanism for Memory Systems
 *
 * Implements self-attention over stored memory entries, allowing the system
 * to compute attention scores that determine which memories are most relevant
 * to a given query vector.
 *
 * Features:
 * - Scaled dot-product attention
 * - Softmax normalization
 * - Top-k filtering with minimum score threshold
 * - Efficient batch processing for large memory sets
 */
/**
 * Self-attention controller for computing attention over memory entries
 */
export class SelfAttentionController {
    vectorBackend;
    memoryStore;
    dimension;
    config;
    constructor(vectorBackend = null, config = {}) {
        this.vectorBackend = vectorBackend;
        this.memoryStore = new Map();
        this.dimension = 0;
        this.config = {
            topK: 10,
            minScore: 0.0,
            temperature: 1.0,
            returnWeights: false,
            ...config
        };
    }
    /**
     * Add a memory entry to the attention context
     */
    addMemory(entry) {
        if (!entry.id || !entry.embedding || entry.embedding.length === 0) {
            throw new Error('Memory entry must have id and non-empty embedding');
        }
        if (this.dimension === 0) {
            this.dimension = entry.embedding.length;
        }
        else if (entry.embedding.length !== this.dimension) {
            throw new Error(`Embedding dimension mismatch: expected ${this.dimension}, got ${entry.embedding.length}`);
        }
        this.memoryStore.set(entry.id, entry);
        // Also add to vector backend if available
        if (this.vectorBackend) {
            const embedding = new Float32Array(entry.embedding);
            this.vectorBackend.insert(entry.id, embedding, entry.metadata);
        }
    }
    /**
     * Remove a memory entry from the attention context
     */
    removeMemory(id) {
        return this.memoryStore.delete(id);
    }
    /**
     * Clear all memory entries
     */
    clearMemories() {
        this.memoryStore.clear();
        this.dimension = 0;
    }
    /**
     * Compute self-attention for a query vector
     *
     * @param query - Query vector for attention computation
     * @param options - Override default configuration options
     * @returns Self-attention result with scores and attended output
     */
    async computeAttention(query, options = {}) {
        const startTime = performance.now();
        // Validate input
        if (!query || !Array.isArray(query)) {
            throw new Error('Query must be a non-null array');
        }
        const config = { ...this.config, ...options };
        const { topK = 10, minScore = 0.0, temperature = 1.0 } = config;
        // Handle empty memory case
        if (this.memoryStore.size === 0) {
            return {
                scores: [],
                attended: [...query],
                executionTimeMs: performance.now() - startTime
            };
        }
        // Validate query dimension
        if (this.dimension > 0 && query.length !== this.dimension) {
            throw new Error(`Query dimension mismatch: expected ${this.dimension}, got ${query.length}`);
        }
        // Compute raw attention scores using scaled dot-product
        const rawScores = [];
        const scale = 1.0 / Math.sqrt(query.length);
        for (const [id, entry] of this.memoryStore.entries()) {
            const dotProduct = this.computeDotProduct(query, entry.embedding);
            const scaledScore = dotProduct * scale / temperature;
            rawScores.push({ id, score: scaledScore, embedding: entry.embedding });
        }
        // Apply softmax normalization
        const normalizedScores = this.applySoftmax(rawScores);
        // Filter by minimum score and take top-k
        const filteredScores = normalizedScores
            .filter(item => item.score >= minScore)
            .sort((a, b) => b.score - a.score)
            .slice(0, topK);
        // Compute attended output (weighted sum of values)
        const attended = this.computeAttendedOutput(query, normalizedScores);
        const executionTimeMs = performance.now() - startTime;
        return {
            scores: filteredScores.map(item => ({
                id: item.id,
                score: item.score,
                rawScore: item.rawScore
            })),
            attended,
            executionTimeMs
        };
    }
    /**
     * Compute dot product between two vectors
     */
    computeDotProduct(a, b) {
        let sum = 0;
        const len = Math.min(a.length, b.length);
        for (let i = 0; i < len; i++) {
            sum += a[i] * b[i];
        }
        return sum;
    }
    /**
     * Apply softmax normalization to scores
     */
    applySoftmax(items) {
        if (items.length === 0)
            return [];
        // Find max for numerical stability
        const maxScore = Math.max(...items.map(item => item.score));
        // Compute exp(score - max) for each item
        const expScores = items.map(item => ({
            ...item,
            rawScore: item.score,
            expScore: Math.exp(item.score - maxScore)
        }));
        // Compute sum for normalization
        const sumExp = expScores.reduce((sum, item) => sum + item.expScore, 0);
        // Handle edge case where sum is 0 or very small
        if (sumExp === 0 || !isFinite(sumExp)) {
            return items.map(item => ({
                ...item,
                rawScore: item.score,
                score: 1 / items.length
            }));
        }
        // Normalize to get probabilities
        return expScores.map(item => ({
            id: item.id,
            score: item.expScore / sumExp,
            rawScore: item.rawScore,
            embedding: item.embedding
        }));
    }
    /**
     * Compute attended output as weighted sum of memory embeddings
     */
    computeAttendedOutput(query, normalizedScores) {
        if (normalizedScores.length === 0) {
            return [...query];
        }
        const dimension = query.length;
        const attended = new Array(dimension).fill(0);
        for (const item of normalizedScores) {
            const weight = item.score;
            for (let i = 0; i < dimension; i++) {
                attended[i] += weight * item.embedding[i];
            }
        }
        return attended;
    }
    /**
     * Get the number of memories in the attention context
     */
    get memoryCount() {
        return this.memoryStore.size;
    }
    /**
     * Get the embedding dimension
     */
    get embeddingDimension() {
        return this.dimension;
    }
    /**
     * Get statistics about the attention controller
     */
    getStats() {
        return {
            memoryCount: this.memoryStore.size,
            dimension: this.dimension,
            hasVectorBackend: this.vectorBackend !== null
        };
    }
}
//# sourceMappingURL=SelfAttentionController.js.map