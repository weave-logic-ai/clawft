/**
 * Search Module - Hybrid Vector + Keyword Search
 *
 * Provides combined search capabilities for AgentDB:
 * - KeywordIndex: BM25-based inverted index for text search
 * - HybridSearch: Combines vector similarity with keyword relevance
 * - Multiple fusion strategies: RRF, Linear, Max
 *
 * @module search
 */
export { KeywordIndex, HybridSearch, createKeywordIndex, createHybridSearch, type HybridSearchOptions, type HybridSearchResult, type HybridQuery, type BM25Config, type VectorBackend, type SearchResult, type SearchOptions, } from './HybridSearch.js';
//# sourceMappingURL=index.d.ts.map