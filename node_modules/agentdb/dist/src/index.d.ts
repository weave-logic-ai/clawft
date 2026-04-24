/**
 * AgentDB - Main Entry Point
 *
 * Frontier Memory Features with MCP Integration:
 * - Causal reasoning and memory graphs
 * - Reflexion memory with self-critique
 * - Skill library with automated learning
 * - Vector search with embeddings
 * - Reinforcement learning (9 algorithms)
 */
export { AgentDB } from './core/AgentDB.js';
import { AgentDB as AgentDBClass } from './core/AgentDB.js';
export default AgentDBClass;
export { CausalMemoryGraph } from './controllers/CausalMemoryGraph.js';
export { CausalRecall } from './controllers/CausalRecall.js';
export { ExplainableRecall } from './controllers/ExplainableRecall.js';
export { NightlyLearner } from './controllers/NightlyLearner.js';
export { ReflexionMemory } from './controllers/ReflexionMemory.js';
export { SkillLibrary } from './controllers/SkillLibrary.js';
export { LearningSystem } from './controllers/LearningSystem.js';
export { ReasoningBank } from './controllers/ReasoningBank.js';
export { EmbeddingService } from './controllers/EmbeddingService.js';
export { EnhancedEmbeddingService } from './controllers/EnhancedEmbeddingService.js';
export { WASMVectorSearch } from './controllers/WASMVectorSearch.js';
export { HNSWIndex, isHnswlibAvailable } from './controllers/HNSWIndex.js';
export { AttentionService } from './controllers/AttentionService.js';
export { MemoryController } from './controllers/MemoryController.js';
export { SelfAttentionController } from './controllers/attention/SelfAttentionController.js';
export { CrossAttentionController } from './controllers/attention/CrossAttentionController.js';
export { MultiHeadAttentionController } from './controllers/attention/MultiHeadAttentionController.js';
export { createDatabase } from './db-fallback.js';
export { BatchOperations } from './optimizations/BatchOperations.js';
export { QueryOptimizer } from './optimizations/QueryOptimizer.js';
export { QueryCache } from './core/QueryCache.js';
export type { QueryCacheConfig, CacheEntry, CacheStatistics } from './core/QueryCache.js';
export { validateTableName, validateColumnName, validatePragmaCommand, buildSafeWhereClause, buildSafeSetClause, ValidationError, } from './security/input-validation.js';
export { type QuantizationStats, type QuantizedVector, type ProductQuantizerConfig, type PQEncodedVector, type QuantizedVectorStoreConfig, type QuantizedSearchResult, quantize8bit, quantize4bit, dequantize8bit, dequantize4bit, calculateQuantizationError, getQuantizationStats, ProductQuantizer, QuantizedVectorStore, createScalar8BitStore, createScalar4BitStore, createProductQuantizedStore, } from './quantization/index.js';
export { KeywordIndex, HybridSearch, createKeywordIndex, createHybridSearch, type HybridSearchOptions, type HybridSearchResult, type HybridQuery, type BM25Config, } from './search/index.js';
export { BenchmarkSuite, Benchmark, VectorInsertBenchmark, VectorSearchBenchmark, MemoryUsageBenchmark, ConcurrencyBenchmark, QuantizationBenchmark, runBenchmarks, runSelectedBenchmarks, formatReportAsMarkdown, formatComparisonAsMarkdown, type LatencyStats, type BenchmarkResult, type BenchmarkReport, type ComparisonReport, type BenchmarkConfig, } from './benchmark/index.js';
export * from './controllers/index.js';
export { MultiDatabaseCoordinator, type DatabaseInstance, type InstanceStatus, type ConflictResolutionStrategy, type SyncOptions, type SyncProgress, type SyncResult, type ConflictInfo, type VectorData, type MultiDatabaseCoordinatorConfig, type StatusChangeCallback, type DistributedOperationResult, } from './coordination/index.js';
export { LLMRouter, isRuvLLMInstalled, type LLMConfig, type LLMResponse, } from './services/LLMRouter.js';
//# sourceMappingURL=index.d.ts.map