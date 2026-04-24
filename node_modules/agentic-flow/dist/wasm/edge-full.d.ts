/**
 * RuVector Edge-Full Integration
 *
 * Complete WASM toolkit for edge AI providing:
 *
 * Modules:
 * - edge: Core WASM primitives (HNSW, Identity, Crypto)
 * - graph: Graph database with Cypher/SPARQL/SQL support
 * - rvlite: Lightweight vector operations
 * - sona: Self-learning neural adaptation (SONA)
 * - dag: DAG workflow orchestration
 * - onnx: ONNX embeddings inference
 *
 * Features:
 * - Pure WASM: Runs in browser, Node.js, Cloudflare Workers, Deno
 * - Post-quantum cryptography
 * - P2P swarm coordination
 * - Raft consensus
 * - SIMD acceleration
 *
 * @see https://github.com/ruvnet/ruvector/tree/main/examples/edge-full
 */
/**
 * Main EdgeFull module interface
 */
interface EdgeFullModule {
    edge: EdgeModule;
    graph: GraphModule;
    rvlite: RvLiteModule;
    sona: SonaModule;
    dag: DagModule;
    onnx: OnnxModule;
    isReady: () => boolean;
    getVersion: () => string;
}
/**
 * Edge module - Core WASM primitives
 */
interface EdgeModule {
    default: () => Promise<void>;
    init: () => Promise<void>;
    initSync?: () => void;
    version?: string;
    WasmHnswIndex: new (dim: number, m?: number, efConstruction?: number) => HnswIndexInstance;
    WasmIdentity: {
        generate: () => IdentityInstance;
        fromSecretKey: (key: Uint8Array) => IdentityInstance;
    };
    WasmCrypto?: {
        hash: (data: Uint8Array) => Uint8Array;
        encrypt: (key: Uint8Array, data: Uint8Array) => Uint8Array;
        decrypt: (key: Uint8Array, ciphertext: Uint8Array) => Uint8Array;
    };
    WasmHybridKeyPair?: {
        generate: () => {
            publicKey: Uint8Array;
            secretKey: Uint8Array;
        };
        encrypt: (publicKey: Uint8Array, data: Uint8Array) => Uint8Array;
        decrypt: (secretKey: Uint8Array, ciphertext: Uint8Array) => Uint8Array;
    };
    WasmSemanticMatcher: new () => SemanticMatcherInstance;
    WasmAdaptiveCompressor?: new () => unknown;
    WasmQuantizer?: new () => unknown;
    WasmRaftNode?: new () => unknown;
    WasmSpikingNetwork?: new () => unknown;
    HnswIndex?: new (dim: number, m?: number, efConstruction?: number) => HnswIndexInstance;
    Identity?: {
        generate: () => IdentityInstance;
        fromSecretKey: (key: Uint8Array) => IdentityInstance;
    };
    SemanticMatcher?: new () => SemanticMatcherInstance;
    isReady?: () => boolean;
}
interface HnswIndexInstance {
    add: (vector: Float32Array) => number;
    search: (query: Float32Array, k: number) => Array<{
        index: number;
        distance: number;
    }>;
    len: () => number;
    serialize: () => Uint8Array;
    deserialize: (data: Uint8Array) => void;
}
interface IdentityInstance {
    publicKey: () => Uint8Array;
    secretKey: () => Uint8Array;
    sign: (data: Uint8Array) => Uint8Array;
    verify: (data: Uint8Array, signature: Uint8Array) => boolean;
}
interface SemanticMatcherInstance {
    registerAgent: (id: string, embedding: Float32Array, capabilities: string[]) => void;
    matchTask: (embedding: Float32Array, topK: number) => Array<{
        agentId: string;
        score: number;
        capabilities: string[];
    }>;
}
/**
 * Graph module - Cypher/SPARQL/SQL
 */
interface GraphModule {
    default: () => Promise<void>;
    init: () => Promise<void>;
    initSync?: () => void;
    GraphDB: new () => GraphDBInstance;
    isReady?: () => boolean;
}
interface GraphDBInstance {
    cypher: (query: string) => Promise<any>;
    sparql: (query: string) => Promise<any>;
    sql: (query: string) => Promise<any>;
    createNode: (labels: string[], properties: Record<string, any>) => string;
    createEdge: (from: string, to: string, type: string, properties?: Record<string, any>) => string;
    getNode: (id: string) => any;
    deleteNode: (id: string) => boolean;
    vectorSearch: (embedding: Float32Array, k: number) => Array<{
        nodeId: string;
        distance: number;
    }>;
    export: () => Uint8Array;
    import: (data: Uint8Array) => void;
}
/**
 * RvLite module - Lightweight vector ops
 */
interface RvLiteModule {
    default: () => Promise<void>;
    init: () => Promise<void>;
    initSync?: () => void;
    cosineSimilarity?: (a: Float32Array, b: Float32Array) => number;
    dotProduct?: (a: Float32Array, b: Float32Array) => number;
    euclideanDistance?: (a: Float32Array, b: Float32Array) => number;
    normalize?: (v: Float32Array) => Float32Array;
    batchCosineSimilarity?: (queries: Float32Array[], corpus: Float32Array[]) => number[][];
    isSIMDEnabled?: () => boolean;
    isReady?: () => boolean;
}
/**
 * SONA module - Self-learning
 */
interface SonaModule {
    default: () => Promise<void>;
    init: () => Promise<void>;
    initSync?: () => void;
    SonaEngine?: new (config?: SonaConfig) => SonaEngineInstance;
    isReady?: () => boolean;
}
interface SonaConfig {
    embeddingDim?: number;
    hiddenDim?: number;
    microLoraRank?: number;
    baseLoraRank?: number;
    ewcLambda?: number;
    learningRate?: number;
}
interface SonaEngineInstance {
    route: (taskEmbedding: Float32Array) => {
        agentId: string;
        confidence: number;
    };
    learn: (taskEmbedding: Float32Array, agentId: string, reward: number) => void;
    addPattern: (pattern: {
        task: string;
        agent: string;
        success: boolean;
    }) => void;
    findPatterns: (query: Float32Array, k: number) => Array<{
        task: string;
        agent: string;
        similarity: number;
    }>;
    getStats: () => {
        patterns: number;
        trajectories: number;
        avgReward: number;
    };
    serialize: () => Uint8Array;
    deserialize: (data: Uint8Array) => void;
}
/**
 * DAG module - Workflow orchestration
 */
interface DagModule {
    default: () => Promise<void>;
    init: () => Promise<void>;
    initSync?: () => void;
    DagWorkflow?: new () => DagWorkflowInstance;
    isReady?: () => boolean;
}
interface DagWorkflowInstance {
    addNode: (id: string, handler: (input: any) => Promise<any>) => void;
    addEdge: (from: string, to: string) => void;
    execute: (input: any) => Promise<any>;
    executeParallel: (inputs: any[]) => Promise<any[]>;
    getTopologicalOrder: () => string[];
    validate: () => {
        valid: boolean;
        errors: string[];
    };
}
/**
 * ONNX module - Embeddings
 */
interface OnnxModule {
    default: () => Promise<void>;
    init: () => Promise<void>;
    initSync?: () => void;
    embed?: (text: string) => Promise<Float32Array>;
    embedBatch?: (texts: string[]) => Promise<Float32Array[]>;
    loadModel?: (modelData?: Uint8Array) => Promise<void>;
    isModelLoaded?: () => boolean;
    getModelInfo?: () => {
        name: string;
        dimension: number;
    };
    similarity?: (text1: string, text2: string) => Promise<number>;
    isReady?: () => boolean;
}
/**
 * Initialize EdgeFull WASM module
 */
export declare function initEdgeFull(): Promise<boolean>;
/**
 * Check if EdgeFull is available
 */
export declare function isEdgeFullAvailable(): boolean;
/**
 * Get EdgeFull module (initialize if needed)
 */
export declare function getEdgeFull(): Promise<EdgeFullModule | null>;
/**
 * EdgeFull HNSW Index with JS fallback
 */
export declare class EdgeFullHnswIndex {
    private wasmIndex;
    private jsVectors;
    private dimensions;
    constructor(dimensions: number, m?: number, efConstruction?: number);
    add(vector: Float32Array | number[]): number;
    search(query: Float32Array | number[], k?: number): Array<{
        index: number;
        distance: number;
    }>;
    size(): number;
    isWasmAccelerated(): boolean;
    private euclideanDistance;
}
/**
 * EdgeFull Graph Database with Cypher support
 */
export declare class EdgeFullGraphDB {
    private wasmGraph;
    private jsNodes;
    private jsEdges;
    private nodeCounter;
    private edgeCounter;
    constructor();
    cypher(query: string): Promise<any>;
    createNode(labels: string[], properties?: Record<string, any>): string;
    createEdge(from: string, to: string, type: string, properties?: Record<string, any>): string;
    getNode(id: string): any;
    isWasmAccelerated(): boolean;
}
/**
 * EdgeFull SONA Engine with JS fallback
 */
export declare class EdgeFullSonaEngine {
    private wasmSona;
    private jsPatterns;
    private agentScores;
    constructor(config?: SonaConfig);
    route(taskEmbedding: Float32Array): {
        agentId: string;
        confidence: number;
    };
    learn(taskEmbedding: Float32Array, agentId: string, reward: number): void;
    addPattern(pattern: {
        task: string;
        agent: string;
        success: boolean;
    }): void;
    getStats(): {
        patterns: number;
        trajectories: number;
        avgReward: number;
    };
    isWasmAccelerated(): boolean;
}
/**
 * EdgeFull ONNX Embeddings
 */
export declare class EdgeFullOnnxEmbeddings {
    private initialized;
    init(): Promise<boolean>;
    embed(text: string): Promise<Float32Array>;
    embedBatch(texts: string[]): Promise<Float32Array[]>;
    similarity(text1: string, text2: string): Promise<number>;
    isAvailable(): boolean;
    private simpleEmbed;
    private cosineSimilarity;
}
/**
 * EdgeFull DAG Workflow
 */
export declare class EdgeFullDagWorkflow {
    private wasmDag;
    private nodes;
    private edges;
    constructor();
    addNode(id: string, handler: (input: any) => Promise<any>): void;
    addEdge(from: string, to: string): void;
    execute(input: any): Promise<any>;
    getTopologicalOrder(): string[];
    isWasmAccelerated(): boolean;
}
/**
 * Cosine similarity (SIMD-accelerated when available)
 */
export declare function cosineSimilarity(a: Float32Array, b: Float32Array): number;
/**
 * Dot product (SIMD-accelerated when available)
 */
export declare function dotProduct(a: Float32Array, b: Float32Array): number;
/**
 * Normalize vector (SIMD-accelerated when available)
 */
export declare function normalize(v: Float32Array): Float32Array;
/**
 * Check if SIMD is enabled
 */
export declare function isSIMDEnabled(): boolean;
export interface EdgeFullStats {
    available: boolean;
    version: string;
    modules: {
        edge: boolean;
        graph: boolean;
        rvlite: boolean;
        sona: boolean;
        dag: boolean;
        onnx: boolean;
    };
    simdEnabled: boolean;
}
/**
 * Get EdgeFull statistics
 */
export declare function getEdgeFullStats(): EdgeFullStats;
declare const _default: {
    initEdgeFull: typeof initEdgeFull;
    isEdgeFullAvailable: typeof isEdgeFullAvailable;
    getEdgeFull: typeof getEdgeFull;
    getEdgeFullStats: typeof getEdgeFullStats;
    EdgeFullHnswIndex: typeof EdgeFullHnswIndex;
    EdgeFullGraphDB: typeof EdgeFullGraphDB;
    EdgeFullSonaEngine: typeof EdgeFullSonaEngine;
    EdgeFullOnnxEmbeddings: typeof EdgeFullOnnxEmbeddings;
    EdgeFullDagWorkflow: typeof EdgeFullDagWorkflow;
    cosineSimilarity: typeof cosineSimilarity;
    dotProduct: typeof dotProduct;
    normalize: typeof normalize;
    isSIMDEnabled: typeof isSIMDEnabled;
};
export default _default;
//# sourceMappingURL=edge-full.d.ts.map