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
export { RuVectorIntelligence, createIntelligenceLayer, IntelligencePresets, } from './RuVectorIntelligence.js';
// Re-export attention types
export { AttentionType, } from '@ruvector/attention';
// Enhanced Agent Booster v2 with full RuVector intelligence
export { EnhancedAgentBooster, getEnhancedBooster, enhancedApply, benchmark as benchmarkEnhancedBooster, } from './agent-booster-enhanced.js';
// WASM Acceleration - 150x faster pattern search
export { WasmPatternIndex, WasmAgentRouter, getWasmPatternIndex, getWasmAgentRouter, initWasmAcceleration, getWasmAccelerationStatus, } from './wasm-acceleration.js';
// TinyDancer - FastGRNN Neural Routing
export { TinyDancerRouter, getTinyDancerRouter, initTinyDancer, isTinyDancerAvailable, } from '../routing/TinyDancerRouter.js';
// ONNX Embeddings WASM - Browser-compatible embeddings
export { OnnxEmbeddingsWasm, getOnnxEmbeddingsWasm, initOnnxEmbeddingsWasm, isOnnxWasmAvailable, isSIMDEnabled, embed as onnxEmbed, embedBatch as onnxEmbedBatch, cosineSimilarity as onnxCosineSimilarity, } from '../wasm/onnx-embeddings-wasm.js';
// RuVector Edge - WASM-accelerated primitives
export { initRuVectorWasm, isWasmInitialized, isWasmSupported, RuVectorHnswIndex, RuVectorSemanticMatcher, generateIdentity, signData, verifySignature, } from '../wasm/ruvector-edge.js';
// RuVector Edge-Full - Complete WASM toolkit
export { initEdgeFull, isEdgeFullAvailable, getEdgeFull, getEdgeFullStats, EdgeFullHnswIndex, EdgeFullGraphDB, EdgeFullSonaEngine, EdgeFullOnnxEmbeddings, EdgeFullDagWorkflow, cosineSimilarity as edgeFullCosineSimilarity, dotProduct, normalize, isSIMDEnabled as edgeFullSIMDEnabled, } from '../wasm/edge-full.js';
// Embedding Service - Unified embedding interface
export { EmbeddingService, getEmbeddingService, embed, embedBatch, textSimilarity, simpleEmbed, semanticSearch, findDuplicates, clusterTexts, pretrainCodePatterns, pretrainFromRepo, } from './EmbeddingService.js';
//# sourceMappingURL=index.js.map