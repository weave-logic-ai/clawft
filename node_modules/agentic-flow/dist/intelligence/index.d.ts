/**
 * RuVector Intelligence Layer - Unified Exports
 *
 * Exposes the full power of RuVector ecosystem:
 *
 * Features Used:
 * - @ruvector/sona: Micro-LoRA, Base-LoRA, EWC++, ReasoningBank, Trajectories
 * - @ruvector/attention: Multi-head, Flash, Hyperbolic, MoE, Graph, DualSpace
 * - ruvector core: HNSW indexing, vector similarity search
 */
export { RuVectorIntelligence, createIntelligenceLayer, IntelligencePresets, type RuVectorIntelligenceConfig, type Trajectory, type TrajectoryStep, type AgentRoutingResult, type LearningOutcome, type OperationResult, } from './RuVectorIntelligence.js';
export type { JsSonaConfig as SonaConfig, JsLearnedPattern as LearnedPattern, } from '@ruvector/sona';
export { AttentionType, type MoEConfig, } from '@ruvector/attention';
export { EnhancedAgentBooster, getEnhancedBooster, enhancedApply, benchmark as benchmarkEnhancedBooster, type EnhancedEditRequest, type EnhancedEditResult, type LearnedPattern as BoosterPattern, type BoosterStats, type ErrorPattern, type PrefetchResult, } from './agent-booster-enhanced.js';
export { WasmPatternIndex, WasmAgentRouter, getWasmPatternIndex, getWasmAgentRouter, initWasmAcceleration, getWasmAccelerationStatus, type PatternEntry, type AgentProfile, } from './wasm-acceleration.js';
export { TinyDancerRouter, getTinyDancerRouter, initTinyDancer, isTinyDancerAvailable, type RouteResult, type RouterMetrics, type TinyDancerConfig, } from '../routing/TinyDancerRouter.js';
export { OnnxEmbeddingsWasm, getOnnxEmbeddingsWasm, initOnnxEmbeddingsWasm, isOnnxWasmAvailable, isSIMDEnabled, embed as onnxEmbed, embedBatch as onnxEmbedBatch, cosineSimilarity as onnxCosineSimilarity, type OnnxEmbeddingResult, type OnnxBatchResult, type OnnxEmbeddingsStats, } from '../wasm/onnx-embeddings-wasm.js';
export { initRuVectorWasm, isWasmInitialized, isWasmSupported, RuVectorHnswIndex, RuVectorSemanticMatcher, generateIdentity, signData, verifySignature, type WasmIdentityKeys, type HnswSearchResult, type SemanticMatch, } from '../wasm/ruvector-edge.js';
export { initEdgeFull, isEdgeFullAvailable, getEdgeFull, getEdgeFullStats, EdgeFullHnswIndex, EdgeFullGraphDB, EdgeFullSonaEngine, EdgeFullOnnxEmbeddings, EdgeFullDagWorkflow, cosineSimilarity as edgeFullCosineSimilarity, dotProduct, normalize, isSIMDEnabled as edgeFullSIMDEnabled, type EdgeFullStats, } from '../wasm/edge-full.js';
export { EmbeddingService, getEmbeddingService, embed, embedBatch, textSimilarity, simpleEmbed, semanticSearch, findDuplicates, clusterTexts, pretrainCodePatterns, pretrainFromRepo, type EmbeddingBackend, type EmbeddingStats, type SimilarityResult, type SearchResult, type DuplicateGroup, } from './EmbeddingService.js';
//# sourceMappingURL=index.d.ts.map