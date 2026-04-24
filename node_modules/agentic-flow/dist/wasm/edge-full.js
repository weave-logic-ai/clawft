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
import { logger } from '../utils/logger.js';
// Module state
let edgeFullModule = null;
let initialized = false;
let initPromise = null;
// ============================================================================
// Initialization
// ============================================================================
/**
 * Initialize EdgeFull WASM module
 */
export async function initEdgeFull() {
    if (initialized)
        return edgeFullModule !== null;
    if (initPromise)
        return initPromise;
    initPromise = (async () => {
        try {
            // Import individual modules to avoid main module ONNX export bug
            const [edgeMod, graphMod, rvliteMod, sonaMod, dagMod] = await Promise.all([
                import('@ruvector/edge-full/edge').catch(() => null),
                import('@ruvector/edge-full/graph').catch(() => null),
                import('@ruvector/edge-full/rvlite').catch(() => null),
                import('@ruvector/edge-full/sona').catch(() => null),
                import('@ruvector/edge-full/dag').catch(() => null),
            ]);
            // Also try ONNX separately (may fail on some platforms)
            let onnxMod = null;
            try {
                onnxMod = await import('@ruvector/edge-full/onnx');
            }
            catch {
                logger.debug('ONNX module not available');
            }
            // Construct module interface
            edgeFullModule = {
                edge: edgeMod,
                graph: graphMod,
                rvlite: rvliteMod,
                sona: sonaMod,
                dag: dagMod,
                onnx: onnxMod,
                isReady: () => true,
                getVersion: () => '0.1.0',
            };
            // Initialize sub-modules in parallel
            const initPromises = [];
            if (edgeFullModule.edge?.default) {
                initPromises.push(edgeFullModule.edge.default());
            }
            if (edgeFullModule.graph?.default) {
                initPromises.push(edgeFullModule.graph.default());
            }
            if (edgeFullModule.rvlite?.default) {
                initPromises.push(edgeFullModule.rvlite.default());
            }
            if (edgeFullModule.sona?.default) {
                initPromises.push(edgeFullModule.sona.default());
            }
            if (edgeFullModule.dag?.default) {
                initPromises.push(edgeFullModule.dag.default());
            }
            if (edgeFullModule.onnx?.default) {
                initPromises.push(edgeFullModule.onnx.default());
            }
            await Promise.all(initPromises);
            initialized = true;
            logger.info('EdgeFull WASM initialized', {
                version: edgeFullModule.getVersion?.() || '0.1.0',
                modules: ['edge', 'graph', 'rvlite', 'sona', 'dag', 'onnx'],
                simd: edgeFullModule.rvlite?.isSIMDEnabled?.() ?? false,
            });
            return true;
        }
        catch (error) {
            logger.debug('EdgeFull WASM not available, using fallbacks', { error });
            initialized = true;
            return false;
        }
    })();
    return initPromise;
}
/**
 * Check if EdgeFull is available
 */
export function isEdgeFullAvailable() {
    return edgeFullModule !== null && (edgeFullModule.isReady?.() ?? false);
}
/**
 * Get EdgeFull module (initialize if needed)
 */
export async function getEdgeFull() {
    if (!initialized) {
        await initEdgeFull();
    }
    return edgeFullModule;
}
// ============================================================================
// High-Level API Wrappers
// ============================================================================
/**
 * EdgeFull HNSW Index with JS fallback
 */
export class EdgeFullHnswIndex {
    wasmIndex = null;
    jsVectors = [];
    dimensions;
    constructor(dimensions, m = 16, efConstruction = 200) {
        this.dimensions = dimensions;
        if (edgeFullModule?.edge?.HnswIndex) {
            try {
                this.wasmIndex = new edgeFullModule.edge.HnswIndex(dimensions, m, efConstruction);
            }
            catch (error) {
                logger.debug('Failed to create WASM HNSW, using JS fallback', { error });
            }
        }
    }
    add(vector) {
        const vec = vector instanceof Float32Array ? vector : new Float32Array(vector);
        if (this.wasmIndex) {
            try {
                return this.wasmIndex.add(vec);
            }
            catch {
                // Fall through
            }
        }
        const idx = this.jsVectors.length;
        this.jsVectors.push(vec);
        return idx;
    }
    search(query, k = 10) {
        const vec = query instanceof Float32Array ? query : new Float32Array(query);
        if (this.wasmIndex) {
            try {
                return this.wasmIndex.search(vec, k);
            }
            catch {
                // Fall through
            }
        }
        // JS fallback - brute force
        const distances = this.jsVectors.map((v, idx) => ({
            index: idx,
            distance: this.euclideanDistance(vec, v),
        }));
        return distances.sort((a, b) => a.distance - b.distance).slice(0, k);
    }
    size() {
        if (this.wasmIndex) {
            try {
                return this.wasmIndex.len();
            }
            catch {
                // Fall through
            }
        }
        return this.jsVectors.length;
    }
    isWasmAccelerated() {
        return this.wasmIndex !== null;
    }
    euclideanDistance(a, b) {
        let sum = 0;
        for (let i = 0; i < a.length; i++) {
            const diff = a[i] - b[i];
            sum += diff * diff;
        }
        return Math.sqrt(sum);
    }
}
/**
 * EdgeFull Graph Database with Cypher support
 */
export class EdgeFullGraphDB {
    wasmGraph = null;
    jsNodes = new Map();
    jsEdges = new Map();
    nodeCounter = 0;
    edgeCounter = 0;
    constructor() {
        if (edgeFullModule?.graph?.GraphDB) {
            try {
                this.wasmGraph = new edgeFullModule.graph.GraphDB();
            }
            catch (error) {
                logger.debug('Failed to create WASM GraphDB, using JS fallback', { error });
            }
        }
    }
    async cypher(query) {
        if (this.wasmGraph) {
            try {
                return await this.wasmGraph.cypher(query);
            }
            catch {
                // Fall through
            }
        }
        throw new Error('Cypher queries require WASM GraphDB');
    }
    createNode(labels, properties = {}) {
        if (this.wasmGraph) {
            try {
                return this.wasmGraph.createNode(labels, properties);
            }
            catch {
                // Fall through
            }
        }
        const id = `node_${++this.nodeCounter}`;
        this.jsNodes.set(id, { labels, properties });
        return id;
    }
    createEdge(from, to, type, properties = {}) {
        if (this.wasmGraph) {
            try {
                return this.wasmGraph.createEdge(from, to, type, properties);
            }
            catch {
                // Fall through
            }
        }
        const id = `edge_${++this.edgeCounter}`;
        this.jsEdges.set(id, { from, to, type, properties });
        return id;
    }
    getNode(id) {
        if (this.wasmGraph) {
            try {
                return this.wasmGraph.getNode(id);
            }
            catch {
                // Fall through
            }
        }
        return this.jsNodes.get(id) || null;
    }
    isWasmAccelerated() {
        return this.wasmGraph !== null;
    }
}
/**
 * EdgeFull SONA Engine with JS fallback
 */
export class EdgeFullSonaEngine {
    wasmSona = null;
    jsPatterns = [];
    agentScores = new Map();
    constructor(config) {
        if (edgeFullModule?.sona?.SonaEngine) {
            try {
                this.wasmSona = new edgeFullModule.sona.SonaEngine(config);
            }
            catch (error) {
                logger.debug('Failed to create WASM SONA, using JS fallback', { error });
            }
        }
    }
    route(taskEmbedding) {
        if (this.wasmSona) {
            try {
                return this.wasmSona.route(taskEmbedding);
            }
            catch {
                // Fall through
            }
        }
        // JS fallback: select best performing agent
        let bestAgent = 'general';
        let bestScore = 0;
        for (const [agent, stats] of this.agentScores) {
            const avgScore = stats.total / stats.count;
            if (avgScore > bestScore) {
                bestScore = avgScore;
                bestAgent = agent;
            }
        }
        return { agentId: bestAgent, confidence: Math.min(0.9, bestScore) };
    }
    learn(taskEmbedding, agentId, reward) {
        if (this.wasmSona) {
            try {
                this.wasmSona.learn(taskEmbedding, agentId, reward);
                return;
            }
            catch {
                // Fall through
            }
        }
        // JS fallback
        const stats = this.agentScores.get(agentId) || { total: 0, count: 0 };
        stats.total += reward;
        stats.count += 1;
        this.agentScores.set(agentId, stats);
    }
    addPattern(pattern) {
        if (this.wasmSona) {
            try {
                this.wasmSona.addPattern(pattern);
                return;
            }
            catch {
                // Fall through
            }
        }
        this.jsPatterns.push({ ...pattern, reward: pattern.success ? 1 : -1 });
    }
    getStats() {
        if (this.wasmSona) {
            try {
                return this.wasmSona.getStats();
            }
            catch {
                // Fall through
            }
        }
        let totalReward = 0;
        for (const stats of this.agentScores.values()) {
            totalReward += stats.total;
        }
        const count = Array.from(this.agentScores.values()).reduce((a, b) => a + b.count, 0);
        return {
            patterns: this.jsPatterns.length,
            trajectories: count,
            avgReward: count > 0 ? totalReward / count : 0,
        };
    }
    isWasmAccelerated() {
        return this.wasmSona !== null;
    }
}
/**
 * EdgeFull ONNX Embeddings
 */
export class EdgeFullOnnxEmbeddings {
    initialized = false;
    async init() {
        if (!edgeFullModule?.onnx) {
            await initEdgeFull();
        }
        this.initialized = edgeFullModule?.onnx?.isReady?.() ?? false;
        return this.initialized;
    }
    async embed(text) {
        if (!this.initialized)
            await this.init();
        if (edgeFullModule?.onnx?.embed) {
            try {
                return await edgeFullModule.onnx.embed(text);
            }
            catch {
                // Fall through
            }
        }
        // Fallback to simple embedding
        return this.simpleEmbed(text);
    }
    async embedBatch(texts) {
        if (!this.initialized)
            await this.init();
        if (edgeFullModule?.onnx?.embedBatch) {
            try {
                return await edgeFullModule.onnx.embedBatch(texts);
            }
            catch {
                // Fall through
            }
        }
        return texts.map((t) => this.simpleEmbed(t));
    }
    async similarity(text1, text2) {
        if (edgeFullModule?.onnx?.similarity) {
            try {
                return await edgeFullModule.onnx.similarity(text1, text2);
            }
            catch {
                // Fall through
            }
        }
        const [e1, e2] = await Promise.all([this.embed(text1), this.embed(text2)]);
        return this.cosineSimilarity(e1, e2);
    }
    isAvailable() {
        return edgeFullModule?.onnx?.isReady?.() ?? false;
    }
    simpleEmbed(text, dim = 384) {
        const embedding = new Float32Array(dim);
        for (let i = 0; i < text.length; i++) {
            const code = text.charCodeAt(i);
            embedding[i % dim] += code / 255;
            embedding[(i * 7) % dim] += (code * 0.3) / 255;
        }
        let norm = 0;
        for (let i = 0; i < dim; i++)
            norm += embedding[i] * embedding[i];
        norm = Math.sqrt(norm) || 1;
        for (let i = 0; i < dim; i++)
            embedding[i] /= norm;
        return embedding;
    }
    cosineSimilarity(a, b) {
        let dot = 0, normA = 0, normB = 0;
        for (let i = 0; i < a.length; i++) {
            dot += a[i] * b[i];
            normA += a[i] * a[i];
            normB += b[i] * b[i];
        }
        return dot / (Math.sqrt(normA) * Math.sqrt(normB) || 1);
    }
}
/**
 * EdgeFull DAG Workflow
 */
export class EdgeFullDagWorkflow {
    wasmDag = null;
    nodes = new Map();
    edges = [];
    constructor() {
        if (edgeFullModule?.dag?.DagWorkflow) {
            try {
                this.wasmDag = new edgeFullModule.dag.DagWorkflow();
            }
            catch (error) {
                logger.debug('Failed to create WASM DAG, using JS fallback', { error });
            }
        }
    }
    addNode(id, handler) {
        if (this.wasmDag) {
            try {
                this.wasmDag.addNode(id, handler);
                return;
            }
            catch {
                // Fall through
            }
        }
        this.nodes.set(id, handler);
    }
    addEdge(from, to) {
        if (this.wasmDag) {
            try {
                this.wasmDag.addEdge(from, to);
                return;
            }
            catch {
                // Fall through
            }
        }
        this.edges.push({ from, to });
    }
    async execute(input) {
        if (this.wasmDag) {
            try {
                return await this.wasmDag.execute(input);
            }
            catch {
                // Fall through
            }
        }
        // Simple JS execution (topological order)
        const order = this.getTopologicalOrder();
        let result = input;
        for (const nodeId of order) {
            const handler = this.nodes.get(nodeId);
            if (handler) {
                result = await handler(result);
            }
        }
        return result;
    }
    getTopologicalOrder() {
        if (this.wasmDag) {
            try {
                return this.wasmDag.getTopologicalOrder();
            }
            catch {
                // Fall through
            }
        }
        // Simple topological sort
        const visited = new Set();
        const order = [];
        const visit = (node) => {
            if (visited.has(node))
                return;
            visited.add(node);
            for (const edge of this.edges) {
                if (edge.from === node) {
                    visit(edge.to);
                }
            }
            order.unshift(node);
        };
        for (const node of this.nodes.keys()) {
            visit(node);
        }
        return order;
    }
    isWasmAccelerated() {
        return this.wasmDag !== null;
    }
}
// ============================================================================
// RvLite Vector Operations
// ============================================================================
/**
 * Cosine similarity (SIMD-accelerated when available)
 */
export function cosineSimilarity(a, b) {
    if (edgeFullModule?.rvlite?.cosineSimilarity) {
        try {
            return edgeFullModule.rvlite.cosineSimilarity(a, b);
        }
        catch {
            // Fall through
        }
    }
    let dot = 0, normA = 0, normB = 0;
    for (let i = 0; i < a.length; i++) {
        dot += a[i] * b[i];
        normA += a[i] * a[i];
        normB += b[i] * b[i];
    }
    return dot / (Math.sqrt(normA) * Math.sqrt(normB) || 1);
}
/**
 * Dot product (SIMD-accelerated when available)
 */
export function dotProduct(a, b) {
    if (edgeFullModule?.rvlite?.dotProduct) {
        try {
            return edgeFullModule.rvlite.dotProduct(a, b);
        }
        catch {
            // Fall through
        }
    }
    let dot = 0;
    for (let i = 0; i < a.length; i++) {
        dot += a[i] * b[i];
    }
    return dot;
}
/**
 * Normalize vector (SIMD-accelerated when available)
 */
export function normalize(v) {
    if (edgeFullModule?.rvlite?.normalize) {
        try {
            return edgeFullModule.rvlite.normalize(v);
        }
        catch {
            // Fall through
        }
    }
    const result = new Float32Array(v.length);
    let norm = 0;
    for (let i = 0; i < v.length; i++) {
        norm += v[i] * v[i];
    }
    norm = Math.sqrt(norm) || 1;
    for (let i = 0; i < v.length; i++) {
        result[i] = v[i] / norm;
    }
    return result;
}
/**
 * Check if SIMD is enabled
 */
export function isSIMDEnabled() {
    return edgeFullModule?.rvlite?.isSIMDEnabled?.() ?? false;
}
/**
 * Get EdgeFull statistics
 */
export function getEdgeFullStats() {
    return {
        available: isEdgeFullAvailable(),
        version: edgeFullModule?.getVersion?.() || '0.0.0',
        modules: {
            edge: edgeFullModule?.edge?.isReady?.() ?? false,
            graph: edgeFullModule?.graph?.isReady?.() ?? false,
            rvlite: edgeFullModule?.rvlite?.isReady?.() ?? false,
            sona: edgeFullModule?.sona?.isReady?.() ?? false,
            dag: edgeFullModule?.dag?.isReady?.() ?? false,
            onnx: edgeFullModule?.onnx?.isReady?.() ?? false,
        },
        simdEnabled: isSIMDEnabled(),
    };
}
// ============================================================================
// Exports
// ============================================================================
export default {
    // Initialization
    initEdgeFull,
    isEdgeFullAvailable,
    getEdgeFull,
    getEdgeFullStats,
    // Classes
    EdgeFullHnswIndex,
    EdgeFullGraphDB,
    EdgeFullSonaEngine,
    EdgeFullOnnxEmbeddings,
    EdgeFullDagWorkflow,
    // Vector operations
    cosineSimilarity,
    dotProduct,
    normalize,
    isSIMDEnabled,
};
//# sourceMappingURL=edge-full.js.map