/**
 * Enhanced EmbeddingService with WASM Acceleration
 *
 * Extends the base EmbeddingService with WASM-accelerated batch operations
 * and improved performance for large-scale embedding generation.
 */
import { EmbeddingService } from './EmbeddingService.js';
import { WASMVectorSearch } from './WASMVectorSearch.js';
export class EnhancedEmbeddingService extends EmbeddingService {
    wasmSearch = null;
    enhancedConfig;
    constructor(config) {
        super(config);
        this.enhancedConfig = {
            enableWASM: true,
            enableBatchProcessing: true,
            batchSize: 100,
            ...config,
        };
        if (this.enhancedConfig.enableWASM) {
            this.initializeWASM();
        }
    }
    /**
     * Initialize WASM acceleration
     */
    initializeWASM() {
        const mockDb = {
            prepare: () => ({ all: () => [], get: () => null, run: () => ({}) }),
            exec: () => { },
        };
        this.wasmSearch = new WASMVectorSearch(mockDb, {
            enableWASM: true,
            batchSize: this.enhancedConfig.batchSize || 100,
        });
    }
    /**
     * Enhanced batch embedding with parallel processing
     */
    async embedBatch(texts) {
        if (!this.enhancedConfig.enableBatchProcessing || texts.length < 10) {
            return super.embedBatch(texts);
        }
        const batchSize = this.enhancedConfig.batchSize || 100;
        const batches = [];
        // Split into batches
        for (let i = 0; i < texts.length; i += batchSize) {
            batches.push(texts.slice(i, i + batchSize));
        }
        // Process batches in parallel
        const results = await Promise.all(batches.map(batch => super.embedBatch(batch)));
        // Flatten results
        return results.flat();
    }
    /**
     * Calculate similarity between two texts using WASM acceleration
     */
    async similarity(textA, textB) {
        const [embeddingA, embeddingB] = await Promise.all([
            this.embed(textA),
            this.embed(textB),
        ]);
        if (this.wasmSearch) {
            return this.wasmSearch.cosineSimilarity(embeddingA, embeddingB);
        }
        // Fallback to manual calculation
        return this.cosineSimilarity(embeddingA, embeddingB);
    }
    /**
     * Find most similar texts from a corpus
     */
    async findMostSimilar(query, corpus, k = 5) {
        const queryEmbedding = await this.embed(query);
        const corpusEmbeddings = await this.embedBatch(corpus);
        let similarities;
        if (this.wasmSearch) {
            similarities = this.wasmSearch.batchSimilarity(queryEmbedding, corpusEmbeddings);
        }
        else {
            similarities = corpusEmbeddings.map(emb => this.cosineSimilarity(queryEmbedding, emb));
        }
        // Create results with indices
        const results = corpus.map((text, index) => ({
            text,
            similarity: similarities[index],
            index,
        }));
        // Sort by similarity and take top k
        results.sort((a, b) => b.similarity - a.similarity);
        return results.slice(0, k);
    }
    /**
     * Get service statistics
     */
    getStats() {
        const wasmStats = this.wasmSearch?.getStats();
        return {
            cacheSize: this.cache.size,
            wasmEnabled: wasmStats?.wasmAvailable ?? false,
            simdEnabled: wasmStats?.simdAvailable ?? false,
        };
    }
    /**
     * Cosine similarity fallback
     */
    cosineSimilarity(a, b) {
        let dotProduct = 0;
        let normA = 0;
        let normB = 0;
        for (let i = 0; i < a.length; i++) {
            dotProduct += a[i] * b[i];
            normA += a[i] * a[i];
            normB += b[i] * b[i];
        }
        const denom = Math.sqrt(normA) * Math.sqrt(normB);
        return denom === 0 ? 0 : dotProduct / denom;
    }
}
//# sourceMappingURL=EnhancedEmbeddingService.js.map