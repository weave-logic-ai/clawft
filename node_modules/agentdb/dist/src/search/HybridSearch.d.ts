/**
 * HybridSearch - Combined Vector + Keyword Search for AgentDB
 *
 * Features:
 * - BM25 keyword search with inverted index
 * - Vector similarity search integration
 * - Reciprocal Rank Fusion (RRF) for result combination
 * - Linear and max fusion methods
 * - Configurable weights and fusion strategies
 * - Metadata filtering support
 *
 * @module search/HybridSearch
 */
import type { VectorBackend } from '../backends/VectorBackend.js';
/**
 * Options for hybrid search operations
 */
export interface HybridSearchOptions {
    /** Maximum number of results to return */
    limit: number;
    /** Weight for vector search results (default: 0.7) */
    vectorWeight?: number;
    /** Weight for keyword search results (default: 0.3) */
    keywordWeight?: number;
    /** Metadata filter to apply to results */
    filter?: Record<string, any>;
    /** Fusion method for combining results */
    fusionMethod?: 'rrf' | 'linear' | 'max';
    /** RRF k parameter (default: 60) - higher values reduce impact of top ranks */
    rrfK?: number;
    /** Minimum score threshold (0-1) */
    threshold?: number;
    /** Override efSearch for vector queries */
    efSearch?: number;
}
/**
 * Result from hybrid search
 */
export interface HybridSearchResult {
    /** Document/vector ID */
    id: string;
    /** Combined score (0-1, higher is better) */
    score: number;
    /** Vector similarity score (if vector search was performed) */
    vectorScore?: number;
    /** Keyword relevance score (if keyword search was performed) */
    keywordScore?: number;
    /** Original metadata from vector store */
    metadata?: Record<string, any>;
    /** Source of the result */
    source: 'vector' | 'keyword' | 'both';
}
/**
 * Query input for hybrid search
 */
export interface HybridQuery {
    /** Text query for keyword search */
    text?: string;
    /** Vector embedding for similarity search */
    vector?: Float32Array;
}
/**
 * Configuration for BM25 scoring
 */
export interface BM25Config {
    /** Term frequency saturation parameter (default: 1.2) */
    k1: number;
    /** Document length normalization (default: 0.75) */
    b: number;
}
/**
 * KeywordIndex - Inverted index with BM25 scoring for text search
 *
 * Implements the BM25 ranking function for document retrieval:
 * score(D,Q) = SUM(IDF(qi) * (f(qi,D) * (k1 + 1)) / (f(qi,D) + k1 * (1 - b + b * |D|/avgdl)))
 *
 * Where:
 * - IDF(qi) = log((N - n(qi) + 0.5) / (n(qi) + 0.5) + 1)
 * - f(qi,D) = term frequency of qi in document D
 * - |D| = document length
 * - avgdl = average document length
 * - k1, b = tuning parameters
 */
export declare class KeywordIndex {
    /** Inverted index: term -> posting list */
    private invertedIndex;
    /** Document store: id -> document entry */
    private documents;
    /** Total number of documents */
    private documentCount;
    /** Sum of all document lengths (for average calculation) */
    private totalDocumentLength;
    /** BM25 configuration */
    private config;
    /** Custom stopwords (merged with defaults) */
    private stopwords;
    constructor(config?: Partial<BM25Config>, customStopwords?: string[]);
    /**
     * Tokenize text into terms
     * - Lowercases text
     * - Splits on non-alphanumeric characters
     * - Removes stopwords
     * - Filters short tokens (< 2 chars)
     */
    private tokenize;
    /**
     * Calculate term frequencies for a list of tokens
     */
    private calculateTermFrequencies;
    /**
     * Add a document to the index
     *
     * @param id - Unique document identifier
     * @param text - Document text content
     * @param metadata - Optional metadata to associate with the document
     */
    add(id: string, text: string, metadata?: Record<string, any>): void;
    /**
     * Remove a document from the index
     *
     * @param id - Document ID to remove
     * @returns true if document was removed, false if not found
     */
    remove(id: string): boolean;
    /**
     * Calculate IDF (Inverse Document Frequency) for a term
     * Using BM25 IDF formula: log((N - n(q) + 0.5) / (n(q) + 0.5) + 1)
     */
    private calculateIDF;
    /**
     * Calculate BM25 score for a document given query terms
     */
    private calculateBM25Score;
    /**
     * Search the index for documents matching the query
     *
     * @param query - Search query text
     * @param limit - Maximum number of results
     * @param filter - Optional metadata filter
     * @returns Array of search results sorted by score (descending)
     */
    search(query: string, limit: number, filter?: Record<string, any>): {
        id: string;
        score: number;
        metadata?: Record<string, any>;
    }[];
    /**
     * Check if document metadata matches the filter criteria
     */
    private matchesFilter;
    /**
     * Get index statistics
     */
    getStats(): {
        documentCount: number;
        termCount: number;
        avgDocumentLength: number;
        k1: number;
        b: number;
    };
    /**
     * Update BM25 parameters
     */
    setConfig(config: Partial<BM25Config>): void;
    /**
     * Clear the entire index
     */
    clear(): void;
    /**
     * Check if a document exists in the index
     */
    has(id: string): boolean;
    /**
     * Get document count
     */
    size(): number;
    /**
     * Add stopwords dynamically
     */
    addStopwords(words: string[]): void;
    /**
     * Remove stopwords dynamically
     */
    removeStopwords(words: string[]): void;
    /**
     * Get all indexed terms (for debugging/analysis)
     */
    getTerms(): string[];
    /**
     * Get document frequency for a term
     */
    getDocumentFrequency(term: string): number;
}
/**
 * HybridSearch - Combines vector similarity and keyword search
 *
 * Supports multiple fusion strategies:
 * - RRF (Reciprocal Rank Fusion): Robust rank-based fusion
 * - Linear: Weighted score combination
 * - Max: Take maximum score from either source
 *
 * Usage:
 * ```typescript
 * const hybrid = new HybridSearch(vectorBackend, keywordIndex);
 *
 * // Combined search
 * const results = await hybrid.search(
 *   { text: "machine learning", vector: embedding },
 *   { limit: 10, vectorWeight: 0.7, keywordWeight: 0.3 }
 * );
 *
 * // Vector-only search
 * const vectorResults = await hybrid.search(
 *   { vector: embedding },
 *   { limit: 10 }
 * );
 *
 * // Keyword-only search
 * const keywordResults = await hybrid.search(
 *   { text: "neural networks" },
 *   { limit: 10 }
 * );
 * ```
 */
export declare class HybridSearch {
    private vectorBackend;
    private keywordIndex;
    constructor(vectorBackend: VectorBackend, keywordIndex: KeywordIndex);
    /**
     * Perform hybrid search combining vector and keyword results
     *
     * @param query - Query object containing text and/or vector
     * @param options - Search options including weights and fusion method
     * @returns Combined search results
     */
    search(query: HybridQuery, options: HybridSearchOptions): Promise<HybridSearchResult[]>;
    /**
     * Reciprocal Rank Fusion (RRF)
     *
     * Combines results based on their ranks rather than scores.
     * RRF(d) = SUM(1 / (k + rank(d)))
     *
     * This is robust to different score distributions between sources.
     */
    private reciprocalRankFusion;
    /**
     * Linear Fusion
     *
     * Combines normalized scores linearly:
     * score(d) = vectorWeight * vectorScore(d) + keywordWeight * keywordScore(d)
     */
    private linearFusion;
    /**
     * Max Fusion
     *
     * Takes the maximum normalized score from either source.
     * score(d) = max(vectorScore(d), keywordScore(d))
     */
    private maxFusion;
    /**
     * Normalize vector-only results to HybridSearchResult format
     */
    private normalizeVectorResults;
    /**
     * Normalize keyword-only results to HybridSearchResult format
     */
    private normalizeKeywordResults;
    /**
     * Add a document to the keyword index
     * (Convenience method - delegates to KeywordIndex)
     */
    addDocument(id: string, text: string, metadata?: Record<string, any>): void;
    /**
     * Remove a document from the keyword index
     * (Convenience method - delegates to KeywordIndex)
     */
    removeDocument(id: string): boolean;
    /**
     * Get statistics about the hybrid search system
     */
    getStats(): {
        vector: {
            count: number;
            dimension: number;
            backend: string;
        };
        keyword: {
            documentCount: number;
            termCount: number;
            avgDocumentLength: number;
        };
    };
    /**
     * Get the underlying vector backend
     */
    getVectorBackend(): VectorBackend;
    /**
     * Get the underlying keyword index
     */
    getKeywordIndex(): KeywordIndex;
}
/**
 * Create a new KeywordIndex with default configuration
 */
export declare function createKeywordIndex(config?: Partial<BM25Config>, customStopwords?: string[]): KeywordIndex;
/**
 * Create a new HybridSearch instance
 */
export declare function createHybridSearch(vectorBackend: VectorBackend, keywordIndex?: KeywordIndex): HybridSearch;
export type { VectorBackend, SearchResult, SearchOptions, } from '../backends/VectorBackend.js';
//# sourceMappingURL=HybridSearch.d.ts.map