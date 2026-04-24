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
// ============================================================================
// Stopwords
// ============================================================================
/**
 * Common English stopwords to filter out during tokenization
 */
const STOPWORDS = new Set([
    'a', 'an', 'the', 'and', 'or', 'but', 'in', 'on', 'at', 'to', 'for',
    'of', 'with', 'by', 'from', 'as', 'is', 'was', 'are', 'were', 'been',
    'be', 'have', 'has', 'had', 'do', 'does', 'did', 'will', 'would', 'could',
    'should', 'may', 'might', 'must', 'shall', 'can', 'need', 'dare', 'ought',
    'used', 'it', 'its', 'this', 'that', 'these', 'those', 'i', 'you', 'he',
    'she', 'we', 'they', 'me', 'him', 'her', 'us', 'them', 'my', 'your',
    'his', 'our', 'their', 'mine', 'yours', 'hers', 'ours', 'theirs',
    'what', 'which', 'who', 'whom', 'when', 'where', 'why', 'how',
    'all', 'each', 'every', 'both', 'few', 'more', 'most', 'other', 'some',
    'such', 'no', 'nor', 'not', 'only', 'own', 'same', 'so', 'than', 'too',
    'very', 'just', 'also', 'now', 'here', 'there', 'then', 'once',
    'if', 'else', 'because', 'although', 'though', 'while', 'unless',
    'until', 'after', 'before', 'since', 'during', 'about', 'into',
    'through', 'between', 'under', 'over', 'above', 'below', 'up', 'down',
    'out', 'off', 'again', 'further', 'any', 'even', 'still',
]);
// ============================================================================
// KeywordIndex Class - BM25 Inverted Index
// ============================================================================
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
export class KeywordIndex {
    /** Inverted index: term -> posting list */
    invertedIndex = new Map();
    /** Document store: id -> document entry */
    documents = new Map();
    /** Total number of documents */
    documentCount = 0;
    /** Sum of all document lengths (for average calculation) */
    totalDocumentLength = 0;
    /** BM25 configuration */
    config;
    /** Custom stopwords (merged with defaults) */
    stopwords;
    constructor(config, customStopwords) {
        this.config = {
            k1: 1.2,
            b: 0.75,
            ...config,
        };
        this.stopwords = new Set([...STOPWORDS]);
        if (customStopwords) {
            customStopwords.forEach((word) => this.stopwords.add(word.toLowerCase()));
        }
    }
    /**
     * Tokenize text into terms
     * - Lowercases text
     * - Splits on non-alphanumeric characters
     * - Removes stopwords
     * - Filters short tokens (< 2 chars)
     */
    tokenize(text) {
        if (!text || typeof text !== 'string') {
            return [];
        }
        return text
            .toLowerCase()
            .split(/[^a-z0-9]+/)
            .filter((token) => token.length >= 2 && !this.stopwords.has(token));
    }
    /**
     * Calculate term frequencies for a list of tokens
     */
    calculateTermFrequencies(tokens) {
        const frequencies = new Map();
        for (const token of tokens) {
            frequencies.set(token, (frequencies.get(token) || 0) + 1);
        }
        return frequencies;
    }
    /**
     * Add a document to the index
     *
     * @param id - Unique document identifier
     * @param text - Document text content
     * @param metadata - Optional metadata to associate with the document
     */
    add(id, text, metadata) {
        // Remove existing document if present (update case)
        if (this.documents.has(id)) {
            this.remove(id);
        }
        const tokens = this.tokenize(text);
        const termFrequencies = this.calculateTermFrequencies(tokens);
        // Create document entry
        const docEntry = {
            id,
            termFrequencies,
            length: tokens.length,
            metadata,
        };
        this.documents.set(id, docEntry);
        this.documentCount++;
        this.totalDocumentLength += tokens.length;
        // Update inverted index
        for (const term of termFrequencies.keys()) {
            let termEntry = this.invertedIndex.get(term);
            if (!termEntry) {
                termEntry = {
                    documentFrequency: 0,
                    postings: new Set(),
                };
                this.invertedIndex.set(term, termEntry);
            }
            if (!termEntry.postings.has(id)) {
                termEntry.documentFrequency++;
                termEntry.postings.add(id);
            }
        }
    }
    /**
     * Remove a document from the index
     *
     * @param id - Document ID to remove
     * @returns true if document was removed, false if not found
     */
    remove(id) {
        const docEntry = this.documents.get(id);
        if (!docEntry) {
            return false;
        }
        // Update inverted index
        for (const term of docEntry.termFrequencies.keys()) {
            const termEntry = this.invertedIndex.get(term);
            if (termEntry) {
                termEntry.postings.delete(id);
                termEntry.documentFrequency--;
                // Remove term if no documents contain it
                if (termEntry.documentFrequency === 0) {
                    this.invertedIndex.delete(term);
                }
            }
        }
        // Update statistics
        this.totalDocumentLength -= docEntry.length;
        this.documentCount--;
        // Remove document
        this.documents.delete(id);
        return true;
    }
    /**
     * Calculate IDF (Inverse Document Frequency) for a term
     * Using BM25 IDF formula: log((N - n(q) + 0.5) / (n(q) + 0.5) + 1)
     */
    calculateIDF(documentFrequency) {
        const N = this.documentCount;
        const n = documentFrequency;
        // BM25 IDF with smoothing
        return Math.log((N - n + 0.5) / (n + 0.5) + 1);
    }
    /**
     * Calculate BM25 score for a document given query terms
     */
    calculateBM25Score(docEntry, queryTerms) {
        const { k1, b } = this.config;
        const avgdl = this.totalDocumentLength / Math.max(this.documentCount, 1);
        const docLength = docEntry.length;
        let score = 0;
        for (const term of queryTerms) {
            const termEntry = this.invertedIndex.get(term);
            if (!termEntry)
                continue;
            const tf = docEntry.termFrequencies.get(term) || 0;
            if (tf === 0)
                continue;
            const idf = this.calculateIDF(termEntry.documentFrequency);
            // BM25 scoring formula
            const numerator = tf * (k1 + 1);
            const denominator = tf + k1 * (1 - b + b * (docLength / avgdl));
            const termScore = idf * (numerator / denominator);
            score += termScore;
        }
        return score;
    }
    /**
     * Search the index for documents matching the query
     *
     * @param query - Search query text
     * @param limit - Maximum number of results
     * @param filter - Optional metadata filter
     * @returns Array of search results sorted by score (descending)
     */
    search(query, limit, filter) {
        const queryTerms = this.tokenize(query);
        if (queryTerms.length === 0) {
            return [];
        }
        // Find candidate documents (union of postings lists)
        const candidateIds = new Set();
        for (const term of queryTerms) {
            const termEntry = this.invertedIndex.get(term);
            if (termEntry) {
                for (const docId of termEntry.postings) {
                    candidateIds.add(docId);
                }
            }
        }
        // Score candidates
        const results = [];
        for (const docId of candidateIds) {
            const docEntry = this.documents.get(docId);
            if (!docEntry)
                continue;
            // Apply metadata filter
            if (filter && !this.matchesFilter(docEntry.metadata, filter)) {
                continue;
            }
            const score = this.calculateBM25Score(docEntry, queryTerms);
            results.push({
                id: docId,
                score,
                metadata: docEntry.metadata,
            });
        }
        // Sort by score (descending) and limit
        results.sort((a, b) => b.score - a.score);
        return results.slice(0, limit);
    }
    /**
     * Check if document metadata matches the filter criteria
     */
    matchesFilter(metadata, filter) {
        if (!metadata)
            return false;
        for (const [key, value] of Object.entries(filter)) {
            if (metadata[key] !== value) {
                return false;
            }
        }
        return true;
    }
    /**
     * Get index statistics
     */
    getStats() {
        return {
            documentCount: this.documentCount,
            termCount: this.invertedIndex.size,
            avgDocumentLength: this.documentCount > 0
                ? this.totalDocumentLength / this.documentCount
                : 0,
            k1: this.config.k1,
            b: this.config.b,
        };
    }
    /**
     * Update BM25 parameters
     */
    setConfig(config) {
        this.config = { ...this.config, ...config };
    }
    /**
     * Clear the entire index
     */
    clear() {
        this.invertedIndex.clear();
        this.documents.clear();
        this.documentCount = 0;
        this.totalDocumentLength = 0;
    }
    /**
     * Check if a document exists in the index
     */
    has(id) {
        return this.documents.has(id);
    }
    /**
     * Get document count
     */
    size() {
        return this.documentCount;
    }
    /**
     * Add stopwords dynamically
     */
    addStopwords(words) {
        words.forEach((word) => this.stopwords.add(word.toLowerCase()));
    }
    /**
     * Remove stopwords dynamically
     */
    removeStopwords(words) {
        words.forEach((word) => this.stopwords.delete(word.toLowerCase()));
    }
    /**
     * Get all indexed terms (for debugging/analysis)
     */
    getTerms() {
        return Array.from(this.invertedIndex.keys());
    }
    /**
     * Get document frequency for a term
     */
    getDocumentFrequency(term) {
        const termEntry = this.invertedIndex.get(term.toLowerCase());
        return termEntry?.documentFrequency || 0;
    }
}
// ============================================================================
// HybridSearch Class
// ============================================================================
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
export class HybridSearch {
    vectorBackend;
    keywordIndex;
    constructor(vectorBackend, keywordIndex) {
        this.vectorBackend = vectorBackend;
        this.keywordIndex = keywordIndex;
    }
    /**
     * Perform hybrid search combining vector and keyword results
     *
     * @param query - Query object containing text and/or vector
     * @param options - Search options including weights and fusion method
     * @returns Combined search results
     */
    async search(query, options) {
        const { limit, vectorWeight = 0.7, keywordWeight = 0.3, filter, fusionMethod = 'rrf', rrfK = 60, threshold, efSearch, } = options;
        // Validate weights
        if (vectorWeight < 0 || keywordWeight < 0) {
            throw new Error('Weights must be non-negative');
        }
        // Normalize weights
        const totalWeight = vectorWeight + keywordWeight;
        const normalizedVectorWeight = vectorWeight / totalWeight;
        const normalizedKeywordWeight = keywordWeight / totalWeight;
        // Collect results from both sources
        let vectorResults = [];
        let keywordResults = [];
        // Perform vector search if vector provided
        if (query.vector) {
            const searchOptions = {
                filter,
                efSearch,
            };
            // Request more results than limit for better fusion
            vectorResults = this.vectorBackend.search(query.vector, Math.min(limit * 3, 1000), searchOptions);
        }
        // Perform keyword search if text provided
        if (query.text) {
            // Request more results than limit for better fusion
            keywordResults = this.keywordIndex.search(query.text, Math.min(limit * 3, 1000), filter);
        }
        // If only one type of search was performed, return those results
        if (!query.vector && query.text) {
            return this.normalizeKeywordResults(keywordResults, limit, threshold);
        }
        if (query.vector && !query.text) {
            return this.normalizeVectorResults(vectorResults, limit, threshold);
        }
        // Combine results using fusion method
        let fusedResults;
        switch (fusionMethod) {
            case 'rrf':
                fusedResults = this.reciprocalRankFusion(vectorResults, keywordResults, normalizedVectorWeight, normalizedKeywordWeight, rrfK);
                break;
            case 'linear':
                fusedResults = this.linearFusion(vectorResults, keywordResults, normalizedVectorWeight, normalizedKeywordWeight);
                break;
            case 'max':
                fusedResults = this.maxFusion(vectorResults, keywordResults);
                break;
            default:
                throw new Error(`Unknown fusion method: ${fusionMethod}`);
        }
        // Apply threshold and limit
        if (threshold !== undefined) {
            fusedResults = fusedResults.filter((r) => r.score >= threshold);
        }
        return fusedResults.slice(0, limit);
    }
    /**
     * Reciprocal Rank Fusion (RRF)
     *
     * Combines results based on their ranks rather than scores.
     * RRF(d) = SUM(1 / (k + rank(d)))
     *
     * This is robust to different score distributions between sources.
     */
    reciprocalRankFusion(vectorResults, keywordResults, vectorWeight, keywordWeight, k) {
        const scoreMap = new Map();
        // Process vector results
        vectorResults.forEach((result, index) => {
            const rank = index + 1;
            const rrfContribution = vectorWeight / (k + rank);
            scoreMap.set(result.id, {
                rrfScore: rrfContribution,
                vectorScore: result.similarity,
                vectorRank: rank,
                metadata: result.metadata,
            });
        });
        // Process keyword results
        keywordResults.forEach((result, index) => {
            const rank = index + 1;
            const rrfContribution = keywordWeight / (k + rank);
            const existing = scoreMap.get(result.id);
            if (existing) {
                existing.rrfScore += rrfContribution;
                existing.keywordScore = result.score;
                existing.keywordRank = rank;
                // Merge metadata if not already present
                if (!existing.metadata && result.metadata) {
                    existing.metadata = result.metadata;
                }
            }
            else {
                scoreMap.set(result.id, {
                    rrfScore: rrfContribution,
                    keywordScore: result.score,
                    keywordRank: rank,
                    metadata: result.metadata,
                });
            }
        });
        // Convert to results array
        const results = [];
        for (const [id, data] of scoreMap) {
            const hasVector = data.vectorScore !== undefined;
            const hasKeyword = data.keywordScore !== undefined;
            results.push({
                id,
                score: data.rrfScore,
                vectorScore: data.vectorScore,
                keywordScore: data.keywordScore,
                metadata: data.metadata,
                source: hasVector && hasKeyword ? 'both' : hasVector ? 'vector' : 'keyword',
            });
        }
        // Sort by RRF score (descending)
        results.sort((a, b) => b.score - a.score);
        // Normalize scores to 0-1 range
        if (results.length > 0) {
            const maxScore = results[0].score;
            for (const result of results) {
                result.score = result.score / maxScore;
            }
        }
        return results;
    }
    /**
     * Linear Fusion
     *
     * Combines normalized scores linearly:
     * score(d) = vectorWeight * vectorScore(d) + keywordWeight * keywordScore(d)
     */
    linearFusion(vectorResults, keywordResults, vectorWeight, keywordWeight) {
        // Normalize keyword scores to 0-1
        const maxKeywordScore = keywordResults.length > 0
            ? Math.max(...keywordResults.map((r) => r.score))
            : 1;
        const normalizedKeywordScores = new Map();
        for (const result of keywordResults) {
            normalizedKeywordScores.set(result.id, {
                score: maxKeywordScore > 0 ? result.score / maxKeywordScore : 0,
                metadata: result.metadata,
            });
        }
        // Create vector score map
        const vectorScores = new Map();
        for (const result of vectorResults) {
            vectorScores.set(result.id, {
                score: result.similarity,
                metadata: result.metadata,
            });
        }
        // Combine all unique IDs
        const allIds = new Set([
            ...vectorScores.keys(),
            ...normalizedKeywordScores.keys(),
        ]);
        // Calculate combined scores
        const results = [];
        for (const id of allIds) {
            const vectorData = vectorScores.get(id);
            const keywordData = normalizedKeywordScores.get(id);
            const vScore = vectorData?.score || 0;
            const kScore = keywordData?.score || 0;
            const combinedScore = vectorWeight * vScore + keywordWeight * kScore;
            const hasVector = vectorData !== undefined;
            const hasKeyword = keywordData !== undefined;
            results.push({
                id,
                score: combinedScore,
                vectorScore: vectorData?.score,
                keywordScore: keywordData?.score,
                metadata: vectorData?.metadata || keywordData?.metadata,
                source: hasVector && hasKeyword ? 'both' : hasVector ? 'vector' : 'keyword',
            });
        }
        // Sort by combined score (descending)
        results.sort((a, b) => b.score - a.score);
        return results;
    }
    /**
     * Max Fusion
     *
     * Takes the maximum normalized score from either source.
     * score(d) = max(vectorScore(d), keywordScore(d))
     */
    maxFusion(vectorResults, keywordResults) {
        // Normalize keyword scores to 0-1
        const maxKeywordScore = keywordResults.length > 0
            ? Math.max(...keywordResults.map((r) => r.score))
            : 1;
        const scoreMap = new Map();
        // Process vector results
        for (const result of vectorResults) {
            scoreMap.set(result.id, {
                maxScore: result.similarity,
                vectorScore: result.similarity,
                metadata: result.metadata,
            });
        }
        // Process keyword results
        for (const result of keywordResults) {
            const normalizedScore = maxKeywordScore > 0 ? result.score / maxKeywordScore : 0;
            const existing = scoreMap.get(result.id);
            if (existing) {
                existing.keywordScore = normalizedScore;
                existing.maxScore = Math.max(existing.maxScore, normalizedScore);
                if (!existing.metadata && result.metadata) {
                    existing.metadata = result.metadata;
                }
            }
            else {
                scoreMap.set(result.id, {
                    maxScore: normalizedScore,
                    keywordScore: normalizedScore,
                    metadata: result.metadata,
                });
            }
        }
        // Convert to results array
        const results = [];
        for (const [id, data] of scoreMap) {
            const hasVector = data.vectorScore !== undefined;
            const hasKeyword = data.keywordScore !== undefined;
            results.push({
                id,
                score: data.maxScore,
                vectorScore: data.vectorScore,
                keywordScore: data.keywordScore,
                metadata: data.metadata,
                source: hasVector && hasKeyword ? 'both' : hasVector ? 'vector' : 'keyword',
            });
        }
        // Sort by max score (descending)
        results.sort((a, b) => b.score - a.score);
        return results;
    }
    /**
     * Normalize vector-only results to HybridSearchResult format
     */
    normalizeVectorResults(results, limit, threshold) {
        let normalized = results.map((r) => ({
            id: r.id,
            score: r.similarity,
            vectorScore: r.similarity,
            metadata: r.metadata,
            source: 'vector',
        }));
        if (threshold !== undefined) {
            normalized = normalized.filter((r) => r.score >= threshold);
        }
        return normalized.slice(0, limit);
    }
    /**
     * Normalize keyword-only results to HybridSearchResult format
     */
    normalizeKeywordResults(results, limit, threshold) {
        // Normalize scores to 0-1
        const maxScore = results.length > 0
            ? Math.max(...results.map((r) => r.score))
            : 1;
        let normalized = results.map((r) => ({
            id: r.id,
            score: maxScore > 0 ? r.score / maxScore : 0,
            keywordScore: maxScore > 0 ? r.score / maxScore : 0,
            metadata: r.metadata,
            source: 'keyword',
        }));
        if (threshold !== undefined) {
            normalized = normalized.filter((r) => r.score >= threshold);
        }
        return normalized.slice(0, limit);
    }
    /**
     * Add a document to the keyword index
     * (Convenience method - delegates to KeywordIndex)
     */
    addDocument(id, text, metadata) {
        this.keywordIndex.add(id, text, metadata);
    }
    /**
     * Remove a document from the keyword index
     * (Convenience method - delegates to KeywordIndex)
     */
    removeDocument(id) {
        return this.keywordIndex.remove(id);
    }
    /**
     * Get statistics about the hybrid search system
     */
    getStats() {
        const vectorStats = this.vectorBackend.getStats();
        const keywordStats = this.keywordIndex.getStats();
        return {
            vector: {
                count: vectorStats.count,
                dimension: vectorStats.dimension,
                backend: vectorStats.backend,
            },
            keyword: {
                documentCount: keywordStats.documentCount,
                termCount: keywordStats.termCount,
                avgDocumentLength: keywordStats.avgDocumentLength,
            },
        };
    }
    /**
     * Get the underlying vector backend
     */
    getVectorBackend() {
        return this.vectorBackend;
    }
    /**
     * Get the underlying keyword index
     */
    getKeywordIndex() {
        return this.keywordIndex;
    }
}
// ============================================================================
// Factory Functions
// ============================================================================
/**
 * Create a new KeywordIndex with default configuration
 */
export function createKeywordIndex(config, customStopwords) {
    return new KeywordIndex(config, customStopwords);
}
/**
 * Create a new HybridSearch instance
 */
export function createHybridSearch(vectorBackend, keywordIndex) {
    const kwIndex = keywordIndex || new KeywordIndex();
    return new HybridSearch(vectorBackend, kwIndex);
}
//# sourceMappingURL=HybridSearch.js.map