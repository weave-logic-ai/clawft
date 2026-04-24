/**
 * Attention Type Definitions for AgentDB v2
 *
 * Shared types for @ruvector/attention integration across memory controllers.
 * These types are used by AttentionService and all enhanced controllers.
 *
 * @module types/attention
 * @see controllers/AttentionService
 * @see docs/integration/ARCHITECTURE.md
 */

// ============================================================================
// Core Types
// ============================================================================

/**
 * Available attention mechanisms from @ruvector/attention
 */
export type AttentionMechanism =
  | 'multihead' // Standard multi-head attention (Vaswani 2017)
  | 'flash' // FlashAttention with block-wise tiling (Dao 2022)
  | 'hyperbolic' // Hyperbolic attention using Poincaré distance
  | 'graphrope' // Graph-aware RoPE with hop distances
  | 'moe' // Mixture of Experts with sparse gating
  | 'dualspace' // Hybrid Euclidean + hyperbolic fusion
  | 'linear'; // Linear attention with kernel approximation

/**
 * Runtime environment detection
 */
export type AttentionRuntime = 'node' | 'browser' | 'auto';

/**
 * Backend implementation (NAPI for Node.js, WASM for browser)
 */
export type AttentionBackend = 'napi' | 'wasm';

// ============================================================================
// Memory Controller Enhancement Configurations
// ============================================================================

/**
 * Configuration for memory controllers with attention enhancements
 */
export interface MemoryControllerAttentionConfig {
  /** Enable hyperbolic attention for CausalMemoryGraph */
  enableHyperbolicAttention?: boolean;

  /** Enable FlashAttention for large memory consolidation */
  enableFlashAttention?: boolean;

  /** Enable GraphRoPE for graph-aware retrieval */
  enableGraphRoPE?: boolean;

  /** Enable MoE for expert routing */
  enableMoERouting?: boolean;

  /** Enable DualSpace for hybrid retrieval */
  enableDualSpace?: boolean;

  /** Hyperbolic attention curvature (default: -1.0) */
  hyperbolicCurvature?: number;

  /** FlashAttention block size (default: 256) */
  flashBlockSize?: number;

  /** GraphRoPE maximum hops (default: 32) */
  maxGraphHops?: number;

  /** MoE number of experts (default: 4) */
  numExperts?: number;

  /** MoE top-k expert selection (default: 1) */
  expertTopK?: number;

  /** Fallback to vector search on errors (default: true) */
  fallbackToVector?: boolean;
}

// ============================================================================
// Enhanced Search Results
// ============================================================================

/**
 * Search result with attention scores (extends standard SearchResult)
 */
export interface AttentionEnhancedResult {
  /** Original result ID */
  id: string | number;

  /** Original similarity/distance */
  similarity: number;

  /** Attention weight (0-1, higher = more relevant) */
  attentionScore?: number;

  /** Mechanism used for re-ranking */
  mechanism?: AttentionMechanism;

  /** Attention-weighted final score */
  finalScore?: number;

  /** Optional metadata */
  metadata?: Record<string, any>;
}

// ============================================================================
// CausalMemoryGraph Enhancements
// ============================================================================

/**
 * Causal query with hyperbolic attention options
 */
export interface CausalQueryWithAttention {
  /** Base causal query parameters */
  interventionMemoryId: number;
  interventionMemoryType: string;
  outcomeMemoryId?: number;
  minConfidence?: number;
  minUplift?: number;

  /** Hyperbolic attention options */
  useHyperbolicAttention?: boolean;
  curvature?: number;
  temperature?: number;

  /** Return attention weights for visualization */
  returnWeights?: boolean;
}

/**
 * Causal edge with hyperbolic attention metadata
 */
export interface CausalEdgeWithAttention {
  /** Standard causal edge fields */
  id?: number;
  fromMemoryId: number;
  fromMemoryType: string;
  toMemoryId: number;
  toMemoryType: string;
  similarity: number;
  uplift?: number;
  confidence: number;

  /** Hyperbolic attention enhancements */
  hyperbolicScore?: number;
  attentionWeight?: number;
  mechanism?: 'hyperbolic' | 'vector';

  /** Tree distance in Poincaré ball */
  poincareDistance?: number;
}

// ============================================================================
// ReasoningBank Enhancements
// ============================================================================

/**
 * Pattern search query with Flash/MoE attention options
 */
export interface PatternSearchWithAttention {
  /** Base pattern search parameters */
  task?: string;
  taskEmbedding?: Float32Array;
  k?: number;
  threshold?: number;

  /** Flash attention for large libraries */
  useFlashAttention?: boolean;
  blockSize?: number;

  /** MoE routing to specialized experts */
  useMoERouting?: boolean;
  numExperts?: number;
  expertTopK?: number;

  /** Return routing decisions */
  returnExpertRouting?: boolean;
}

/**
 * Reasoning pattern with attention metadata
 */
export interface ReasoningPatternWithAttention {
  /** Standard pattern fields */
  id?: number;
  taskType: string;
  approach: string;
  successRate: number;
  uses?: number;
  avgReward?: number;

  /** Attention enhancements */
  flashScore?: number;
  expertId?: number;
  expertScore?: number;
  routingWeight?: number;
}

// ============================================================================
// ExplainableRecall Enhancements
// ============================================================================

/**
 * Explanation query with GraphRoPE options
 */
export interface ExplanationQueryWithAttention {
  /** Base explanation parameters */
  query: string;
  maxDepth?: number;
  minConfidence?: number;

  /** GraphRoPE for hop-aware ranking */
  useGraphRoPE?: boolean;
  maxHops?: number;
  hopDecayFactor?: number;

  /** Return attention heatmap for visualization */
  returnAttentionHeatmap?: boolean;
}

/**
 * Explanation chain with graph attention metadata
 */
export interface ExplanationChainWithAttention {
  /** Standard explanation fields */
  nodes: Array<{
    id: number;
    type: string;
    content: string;
  }>;
  edges: Array<{
    from: number;
    to: number;
    mechanism: string;
  }>;
  confidence: number;

  /** GraphRoPE enhancements */
  hopDistances?: number[];
  graphRoPEScores?: number[];
  positionalBias?: number[];
}

// ============================================================================
// Attention Visualization
// ============================================================================

/**
 * Attention heatmap data for visualization
 */
export interface AttentionHeatmap {
  /** Query identifier */
  queryId: string;

  /** Key identifiers */
  keyIds: string[];

  /** Attention weights matrix (queries × keys) */
  weights: number[][];

  /** Mechanism used */
  mechanism: AttentionMechanism;

  /** Timestamp */
  timestamp: number;
}

/**
 * Attention weight distribution stats
 */
export interface AttentionWeightStats {
  /** Mean attention weight */
  mean: number;

  /** Standard deviation */
  stdDev: number;

  /** Entropy (measure of distribution uniformity) */
  entropy: number;

  /** Maximum weight */
  max: number;

  /** Minimum weight */
  min: number;

  /** Top-k indices */
  topKIndices: number[];

  /** Top-k weights */
  topKWeights: number[];
}

// ============================================================================
// Utility Types
// ============================================================================

/**
 * Attention performance metrics (subset for memory controllers)
 */
export interface AttentionPerformanceMetrics {
  /** Mechanism used */
  mechanism: AttentionMechanism;

  /** Latency in milliseconds */
  latencyMs: number;

  /** Memory used in bytes */
  memoryUsed: number;

  /** Number of keys processed */
  numKeys: number;

  /** Backend used */
  backend: AttentionBackend;
}

/**
 * Feature flag utility type for controllers
 */
export interface AttentionFeatureFlags {
  hyperbolic: boolean;
  flash: boolean;
  graphrope: boolean;
  moe: boolean;
  dualspace: boolean;
  linear: boolean;
}

// ============================================================================
// Re-exports from AttentionService (for convenience)
// ============================================================================

// Note: These types are also defined in AttentionService.ts
// Re-exported here for convenience so memory controllers don't need
// to import from AttentionService directly

export type {
  AttentionConfig,
  AttentionOptions,
  AttentionResult,
  AttentionMetrics,
} from '../controllers/AttentionService.js';

// ============================================================================
// Type Guards
// ============================================================================

/**
 * Type guard for attention-enhanced causal edge
 */
export function isCausalEdgeWithAttention(
  edge: any
): edge is CausalEdgeWithAttention {
  return (
    typeof edge === 'object' &&
    edge !== null &&
    'fromMemoryId' in edge &&
    'toMemoryId' in edge &&
    ('hyperbolicScore' in edge || 'attentionWeight' in edge)
  );
}

/**
 * Type guard for attention-enhanced pattern
 */
export function isPatternWithAttention(
  pattern: any
): pattern is ReasoningPatternWithAttention {
  return (
    typeof pattern === 'object' &&
    pattern !== null &&
    'taskType' in pattern &&
    ('flashScore' in pattern || 'expertId' in pattern)
  );
}

/**
 * Type guard for attention-enhanced explanation
 */
export function isExplanationWithAttention(
  explanation: any
): explanation is ExplanationChainWithAttention {
  return (
    typeof explanation === 'object' &&
    explanation !== null &&
    'nodes' in explanation &&
    'edges' in explanation &&
    ('hopDistances' in explanation || 'graphRoPEScores' in explanation)
  );
}
