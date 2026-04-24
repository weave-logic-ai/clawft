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

export {
  // Classes
  KeywordIndex,
  HybridSearch,

  // Factory functions
  createKeywordIndex,
  createHybridSearch,

  // Types
  type HybridSearchOptions,
  type HybridSearchResult,
  type HybridQuery,
  type BM25Config,

  // Re-exports from VectorBackend
  type VectorBackend,
  type SearchResult,
  type SearchOptions,
} from './HybridSearch.js';
