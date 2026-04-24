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
// Main AgentDB class
export { AgentDB } from './core/AgentDB.js';
import { AgentDB as AgentDBClass } from './core/AgentDB.js';
export default AgentDBClass;
// Core controllers
export { CausalMemoryGraph } from './controllers/CausalMemoryGraph.js';
export { CausalRecall } from './controllers/CausalRecall.js';
export { ExplainableRecall } from './controllers/ExplainableRecall.js';
export { NightlyLearner } from './controllers/NightlyLearner.js';
export { ReflexionMemory } from './controllers/ReflexionMemory.js';
export { SkillLibrary } from './controllers/SkillLibrary.js';
export { LearningSystem } from './controllers/LearningSystem.js';
export { ReasoningBank } from './controllers/ReasoningBank.js';
// Embedding services
export { EmbeddingService } from './controllers/EmbeddingService.js';
export { EnhancedEmbeddingService } from './controllers/EnhancedEmbeddingService.js';
// WASM acceleration and HNSW indexing
export { WASMVectorSearch } from './controllers/WASMVectorSearch.js';
export { HNSWIndex, isHnswlibAvailable } from './controllers/HNSWIndex.js';
// Attention mechanisms
export { AttentionService } from './controllers/AttentionService.js';
// Memory Controller with Attention Integration
export { MemoryController } from './controllers/MemoryController.js';
// Attention Controllers
export { SelfAttentionController } from './controllers/attention/SelfAttentionController.js';
export { CrossAttentionController } from './controllers/attention/CrossAttentionController.js';
export { MultiHeadAttentionController } from './controllers/attention/MultiHeadAttentionController.js';
// Database utilities
export { createDatabase } from './db-fallback.js';
// Optimizations
export { BatchOperations } from './optimizations/BatchOperations.js';
export { QueryOptimizer } from './optimizations/QueryOptimizer.js';
export { QueryCache } from './core/QueryCache.js';
// Security
export { validateTableName, validateColumnName, validatePragmaCommand, buildSafeWhereClause, buildSafeSetClause, ValidationError, } from './security/input-validation.js';
// Vector Quantization
export { 
// Scalar Quantization
quantize8bit, quantize4bit, dequantize8bit, dequantize4bit, calculateQuantizationError, getQuantizationStats, 
// Product Quantization
ProductQuantizer, 
// Quantized Vector Store
QuantizedVectorStore, 
// Factory Functions
createScalar8BitStore, createScalar4BitStore, createProductQuantizedStore, } from './quantization/index.js';
// Hybrid Search (Vector + Keyword)
export { KeywordIndex, HybridSearch, createKeywordIndex, createHybridSearch, } from './search/index.js';
// Benchmarking Suite
export { 
// Main Suite
BenchmarkSuite, 
// Base class for custom benchmarks
Benchmark, 
// Built-in benchmarks
VectorInsertBenchmark, VectorSearchBenchmark, MemoryUsageBenchmark, ConcurrencyBenchmark, QuantizationBenchmark, 
// CLI integration functions
runBenchmarks, runSelectedBenchmarks, 
// Formatting utilities
formatReportAsMarkdown, formatComparisonAsMarkdown, } from './benchmark/index.js';
// Re-export all controllers for convenience
export * from './controllers/index.js';
// Coordination - Multi-database synchronization
export { MultiDatabaseCoordinator, } from './coordination/index.js';
// LLM Router - Multi-provider LLM integration with RuvLLM support
export { LLMRouter, isRuvLLMInstalled, } from './services/LLMRouter.js';
//# sourceMappingURL=index.js.map