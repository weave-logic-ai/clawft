/**
 * AgentDB WASM-Only Entry Point
 *
 * Use this entry point on systems without C++ build tools.
 * Excludes hnswlib-node and provides pure WASM vector backends.
 *
 * Usage:
 *   import { AgentDB } from 'agentdb/wasm';
 *   const db = new AgentDB({ forceWasm: true });
 *
 * This entry point:
 * - Uses RuVector (WASM-based) for vector search
 * - Uses sql.js (WASM-based) for SQLite
 * - Does NOT require any native module compilation
 * - Works on Windows without Visual Studio Build Tools
 * - Works in browsers and edge runtimes
 */
// Main AgentDB class (uses RuVector/WASM backends when forceWasm: true)
export { AgentDB } from './core/AgentDB.js';
import { AgentDB as AgentDBClass } from './core/AgentDB.js';
export default AgentDBClass;
// Core controllers (WASM-compatible)
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
// WASM-only vector search (no hnswlib-node)
export { WASMVectorSearch } from './controllers/WASMVectorSearch.js';
// Note: HNSWIndex is NOT exported here - use the main entry point if you need it
// Attention mechanisms
export { AttentionService } from './controllers/AttentionService.js';
// Memory Controller with Attention Integration
export { MemoryController } from './controllers/MemoryController.js';
// Attention Controllers
export { SelfAttentionController } from './controllers/attention/SelfAttentionController.js';
export { CrossAttentionController } from './controllers/attention/CrossAttentionController.js';
export { MultiHeadAttentionController } from './controllers/attention/MultiHeadAttentionController.js';
// Database utilities (uses sql.js WASM fallback)
export { createDatabase } from './db-fallback.js';
// Optimizations
export { BatchOperations } from './optimizations/BatchOperations.js';
export { QueryOptimizer } from './optimizations/QueryOptimizer.js';
export { QueryCache } from './core/QueryCache.js';
// Security
export { validateTableName, validateColumnName, validatePragmaCommand, buildSafeWhereClause, buildSafeSetClause, ValidationError, } from './security/input-validation.js';
// Vector Quantization
export { quantize8bit, quantize4bit, dequantize8bit, dequantize4bit, calculateQuantizationError, getQuantizationStats, ProductQuantizer, QuantizedVectorStore, createScalar8BitStore, createScalar4BitStore, createProductQuantizedStore, } from './quantization/index.js';
// Hybrid Search (Vector + Keyword)
export { KeywordIndex, HybridSearch, createKeywordIndex, createHybridSearch, } from './search/index.js';
// Benchmarking Suite
export { BenchmarkSuite, Benchmark, VectorInsertBenchmark, VectorSearchBenchmark, MemoryUsageBenchmark, ConcurrencyBenchmark, QuantizationBenchmark, runBenchmarks, runSelectedBenchmarks, formatReportAsMarkdown, formatComparisonAsMarkdown, } from './benchmark/index.js';
// Coordination - Multi-database synchronization
export { MultiDatabaseCoordinator, } from './coordination/index.js';
export { RuVectorBackend } from './backends/ruvector/RuVectorBackend.js';
export { RuVectorLearning } from './backends/ruvector/RuVectorLearning.js';
// Note: HNSWLibBackend is NOT exported - use the main entry point if you need it
export { createBackend, detectBackends, getRecommendedBackend, isBackendAvailable, getInstallCommand } from './backends/factory.js';
//# sourceMappingURL=wasm-loader.js.map