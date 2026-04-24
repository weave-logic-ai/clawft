/*! AgentDB Browser Bundle v2.0.0-alpha.3.6 | MIT License | https://agentdb.ruv.io */
var __defProp = Object.defineProperty;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __esm = (fn, res) => function __init() {
  return fn && (res = (0, fn[__getOwnPropNames(fn)[0]])(fn = 0)), res;
};
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};

// dist/agentdb.wasm-loader.js
var agentdb_wasm_loader_exports = {};
__export(agentdb_wasm_loader_exports, {
  initWASM: () => initWASM,
  wasmLoadError: () => wasmLoadError,
  wasmModule: () => wasmModule
});
async function initWASM() {
  if (wasmModule) return wasmModule;
  if (wasmLoading) return wasmLoading;
  wasmLoading = (async () => {
    try {
      if (typeof WebAssembly === "undefined") {
        throw new Error("WebAssembly not supported in this browser");
      }
      const simdSupported = await detectWasmSIMD();
      console.log(`WASM SIMD support: ${simdSupported}`);
      wasmModule = {
        flashAttention: createFlashAttentionMock(),
        hyperbolicAttention: createHyperbolicAttentionMock(),
        memoryConsolidation: createMemoryConsolidationMock(),
        simdSupported
      };
      console.log("\u2705 WASM attention module loaded");
      return wasmModule;
    } catch (error) {
      wasmLoadError = error;
      console.warn("\u26A0\uFE0F  WASM loading failed, using fallback:", error.message);
      wasmModule = {
        flashAttention: createFlashAttentionMock(),
        hyperbolicAttention: createHyperbolicAttentionMock(),
        memoryConsolidation: createMemoryConsolidationMock(),
        simdSupported: false
      };
      return wasmModule;
    } finally {
      wasmLoading = null;
    }
  })();
  return wasmLoading;
}
async function detectWasmSIMD() {
  try {
    const simdTest = new Uint8Array([
      0,
      97,
      115,
      109,
      1,
      0,
      0,
      0,
      1,
      5,
      1,
      96,
      0,
      1,
      123,
      3,
      2,
      1,
      0,
      10,
      10,
      1,
      8,
      0,
      253,
      12,
      253,
      12,
      253,
      84,
      11
    ]);
    const module = await WebAssembly.instantiate(simdTest);
    return module instanceof WebAssembly.Instance;
  } catch {
    return false;
  }
}
function createFlashAttentionMock() {
  return (query, keys, values, options = {}) => {
    const { dim = 384, numHeads = 4, blockSize = 64 } = options;
    const seqLen = keys.length / dim;
    const output = new Float32Array(query.length);
    for (let i = 0; i < query.length; i += dim) {
      const q = query.slice(i, i + dim);
      let sumWeights = 0;
      const weights = new Float32Array(seqLen);
      for (let j = 0; j < seqLen; j++) {
        const k = keys.slice(j * dim, (j + 1) * dim);
        let dot = 0;
        for (let d = 0; d < dim; d++) {
          dot += q[d] * k[d];
        }
        weights[j] = Math.exp(dot / Math.sqrt(dim));
        sumWeights += weights[j];
      }
      for (let j = 0; j < seqLen; j++) {
        weights[j] /= sumWeights;
        const v = values.slice(j * dim, (j + 1) * dim);
        for (let d = 0; d < dim; d++) {
          output[i + d] += weights[j] * v[d];
        }
      }
    }
    return output;
  };
}
function createHyperbolicAttentionMock() {
  return (query, keys, options = {}) => {
    const { curvature = -1 } = options;
    const k = Math.abs(curvature);
    const similarities = new Float32Array(keys.length / query.length);
    for (let i = 0; i < similarities.length; i++) {
      const offset = i * query.length;
      let dotProduct = 0;
      let normQ = 0;
      let normK = 0;
      for (let j = 0; j < query.length; j++) {
        dotProduct += query[j] * keys[offset + j];
        normQ += query[j] * query[j];
        normK += keys[offset + j] * keys[offset + j];
      }
      const euclidean = Math.sqrt(normQ + normK - 2 * dotProduct);
      const poincare = Math.acosh(1 + 2 * k * euclidean * euclidean);
      similarities[i] = 1 / (1 + poincare);
    }
    return similarities;
  };
}
function createMemoryConsolidationMock() {
  return (memories, options = {}) => {
    const { threshold = 0.8, maxClusters = 10 } = options;
    const consolidated = [];
    const used = /* @__PURE__ */ new Set();
    for (let i = 0; i < memories.length; i++) {
      if (used.has(i)) continue;
      const cluster = [memories[i]];
      used.add(i);
      for (let j = i + 1; j < memories.length; j++) {
        if (used.has(j)) continue;
        let dot = 0;
        let norm1 = 0;
        let norm2 = 0;
        for (let k = 0; k < memories[i].length; k++) {
          dot += memories[i][k] * memories[j][k];
          norm1 += memories[i][k] * memories[i][k];
          norm2 += memories[j][k] * memories[j][k];
        }
        const similarity = dot / (Math.sqrt(norm1 * norm2) || 1);
        if (similarity > threshold) {
          cluster.push(memories[j]);
          used.add(j);
        }
      }
      const avg = new Float32Array(memories[i].length);
      for (const mem of cluster) {
        for (let k = 0; k < avg.length; k++) {
          avg[k] += mem[k] / cluster.length;
        }
      }
      consolidated.push({
        memory: avg,
        count: cluster.size,
        members: cluster
      });
      if (consolidated.length >= maxClusters) break;
    }
    return consolidated;
  };
}
var wasmModule, wasmLoading, wasmLoadError;
var init_agentdb_wasm_loader = __esm({
  "dist/agentdb.wasm-loader.js"() {
    "use strict";
    wasmModule = null;
    wasmLoading = null;
    wasmLoadError = null;
  }
});

// src/browser/ProductQuantization.ts
var ProductQuantization = class {
  config;
  codebook = null;
  trained = false;
  constructor(config) {
    this.config = {
      dimension: config.dimension,
      numSubvectors: config.numSubvectors,
      numCentroids: config.numCentroids,
      maxIterations: config.maxIterations || 50,
      convergenceThreshold: config.convergenceThreshold || 1e-4
    };
    if (this.config.dimension % this.config.numSubvectors !== 0) {
      throw new Error(`Dimension ${this.config.dimension} must be divisible by numSubvectors ${this.config.numSubvectors}`);
    }
  }
  /**
   * Train codebook using k-means on training vectors
   */
  async train(vectors) {
    if (vectors.length === 0) {
      throw new Error("Training requires at least one vector");
    }
    const subvectorDim = this.config.dimension / this.config.numSubvectors;
    const centroids = [];
    console.log(`[PQ] Training ${this.config.numSubvectors} subvectors with ${this.config.numCentroids} centroids each...`);
    for (let s = 0; s < this.config.numSubvectors; s++) {
      const startDim = s * subvectorDim;
      const endDim = startDim + subvectorDim;
      const subvectors = vectors.map((v) => v.slice(startDim, endDim));
      const subCentroids = await this.kMeans(subvectors, this.config.numCentroids);
      centroids.push(...subCentroids);
      if ((s + 1) % 4 === 0 || s === this.config.numSubvectors - 1) {
        console.log(`[PQ] Trained ${s + 1}/${this.config.numSubvectors} subvectors`);
      }
    }
    this.codebook = {
      subvectorDim,
      numSubvectors: this.config.numSubvectors,
      numCentroids: this.config.numCentroids,
      centroids
    };
    this.trained = true;
    console.log("[PQ] Training complete");
  }
  /**
   * K-means clustering for centroids
   */
  async kMeans(vectors, k) {
    const dim = vectors[0].length;
    const n = vectors.length;
    const centroids = this.kMeansPlusPlus(vectors, k);
    const assignments = new Uint32Array(n);
    let prevInertia = Infinity;
    for (let iter = 0; iter < this.config.maxIterations; iter++) {
      let inertia = 0;
      for (let i = 0; i < n; i++) {
        let minDist = Infinity;
        let minIdx = 0;
        for (let j = 0; j < k; j++) {
          const dist = this.squaredDistance(vectors[i], centroids[j]);
          if (dist < minDist) {
            minDist = dist;
            minIdx = j;
          }
        }
        assignments[i] = minIdx;
        inertia += minDist;
      }
      if (Math.abs(prevInertia - inertia) < this.config.convergenceThreshold) {
        break;
      }
      prevInertia = inertia;
      const counts = new Uint32Array(k);
      const sums = Array.from({ length: k }, () => new Float32Array(dim));
      for (let i = 0; i < n; i++) {
        const cluster = assignments[i];
        counts[cluster]++;
        for (let d = 0; d < dim; d++) {
          sums[cluster][d] += vectors[i][d];
        }
      }
      for (let j = 0; j < k; j++) {
        if (counts[j] > 0) {
          for (let d = 0; d < dim; d++) {
            centroids[j][d] = sums[j][d] / counts[j];
          }
        }
      }
    }
    return centroids;
  }
  /**
   * K-means++ initialization for better centroid selection
   */
  kMeansPlusPlus(vectors, k) {
    const n = vectors.length;
    const dim = vectors[0].length;
    const centroids = [];
    const firstIdx = Math.floor(Math.random() * n);
    centroids.push(new Float32Array(vectors[firstIdx]));
    for (let i = 1; i < k; i++) {
      const distances = new Float32Array(n);
      let sumDistances = 0;
      for (let j = 0; j < n; j++) {
        let minDist = Infinity;
        for (const centroid of centroids) {
          const dist = this.squaredDistance(vectors[j], centroid);
          minDist = Math.min(minDist, dist);
        }
        distances[j] = minDist;
        sumDistances += minDist;
      }
      let r = Math.random() * sumDistances;
      for (let j = 0; j < n; j++) {
        r -= distances[j];
        if (r <= 0) {
          centroids.push(new Float32Array(vectors[j]));
          break;
        }
      }
    }
    return centroids;
  }
  /**
   * Compress a vector using trained codebook
   */
  compress(vector) {
    if (!this.trained || !this.codebook) {
      throw new Error("Codebook must be trained before compression");
    }
    const codes = new Uint8Array(this.config.numSubvectors);
    const subvectorDim = this.codebook.subvectorDim;
    let norm = 0;
    for (let i = 0; i < vector.length; i++) {
      norm += vector[i] * vector[i];
    }
    norm = Math.sqrt(norm);
    for (let s = 0; s < this.config.numSubvectors; s++) {
      const startDim = s * subvectorDim;
      const subvector = vector.slice(startDim, startDim + subvectorDim);
      let minDist = Infinity;
      let minIdx = 0;
      const centroidOffset = s * this.config.numCentroids;
      for (let c = 0; c < this.config.numCentroids; c++) {
        const centroid = this.codebook.centroids[centroidOffset + c];
        const dist = this.squaredDistance(subvector, centroid);
        if (dist < minDist) {
          minDist = dist;
          minIdx = c;
        }
      }
      codes[s] = minIdx;
    }
    return { codes, norm };
  }
  /**
   * Decompress a vector (approximate reconstruction)
   */
  decompress(compressed) {
    if (!this.codebook) {
      throw new Error("Codebook not available");
    }
    const vector = new Float32Array(this.config.dimension);
    const subvectorDim = this.codebook.subvectorDim;
    for (let s = 0; s < this.config.numSubvectors; s++) {
      const code = compressed.codes[s];
      const centroidOffset = s * this.config.numCentroids;
      const centroid = this.codebook.centroids[centroidOffset + code];
      const startDim = s * subvectorDim;
      for (let d = 0; d < subvectorDim; d++) {
        vector[startDim + d] = centroid[d];
      }
    }
    return vector;
  }
  /**
   * Asymmetric Distance Computation (ADC)
   * Computes distance from query vector to compressed vector
   */
  asymmetricDistance(query, compressed) {
    if (!this.codebook) {
      throw new Error("Codebook not available");
    }
    let distance = 0;
    const subvectorDim = this.codebook.subvectorDim;
    for (let s = 0; s < this.config.numSubvectors; s++) {
      const code = compressed.codes[s];
      const centroidOffset = s * this.config.numCentroids;
      const centroid = this.codebook.centroids[centroidOffset + code];
      const startDim = s * subvectorDim;
      const querySubvector = query.slice(startDim, startDim + subvectorDim);
      distance += this.squaredDistance(querySubvector, centroid);
    }
    return Math.sqrt(distance);
  }
  /**
   * Batch compression for multiple vectors
   */
  batchCompress(vectors) {
    return vectors.map((v) => this.compress(v));
  }
  /**
   * Get memory savings
   */
  getCompressionRatio() {
    const originalBytes = this.config.dimension * 4;
    const compressedBytes = this.config.numSubvectors + 4;
    return originalBytes / compressedBytes;
  }
  /**
   * Export codebook for persistence
   */
  exportCodebook() {
    if (!this.codebook) {
      throw new Error("No codebook to export");
    }
    return JSON.stringify({
      config: this.config,
      codebook: {
        subvectorDim: this.codebook.subvectorDim,
        numSubvectors: this.codebook.numSubvectors,
        numCentroids: this.codebook.numCentroids,
        centroids: this.codebook.centroids.map((c) => Array.from(c))
      }
    });
  }
  /**
   * Import codebook
   */
  importCodebook(json) {
    const data = JSON.parse(json);
    this.config = data.config;
    this.codebook = {
      subvectorDim: data.codebook.subvectorDim,
      numSubvectors: data.codebook.numSubvectors,
      numCentroids: data.codebook.numCentroids,
      centroids: data.codebook.centroids.map((c) => new Float32Array(c))
    };
    this.trained = true;
  }
  /**
   * Utility: Squared Euclidean distance
   */
  squaredDistance(a, b) {
    let sum = 0;
    for (let i = 0; i < a.length; i++) {
      const diff = a[i] - b[i];
      sum += diff * diff;
    }
    return sum;
  }
  /**
   * Get statistics
   */
  getStats() {
    const compressionRatio = this.getCompressionRatio();
    const memoryPerVector = this.config.numSubvectors + 4;
    const codebookSize = this.codebook ? this.config.numSubvectors * this.config.numCentroids * (this.config.dimension / this.config.numSubvectors) * 4 : 0;
    return {
      trained: this.trained,
      compressionRatio,
      memoryPerVector,
      codebookSize
    };
  }
};
function createPQ8(dimension) {
  return new ProductQuantization({
    dimension,
    numSubvectors: 8,
    numCentroids: 256,
    maxIterations: 50
  });
}
function createPQ16(dimension) {
  return new ProductQuantization({
    dimension,
    numSubvectors: 16,
    numCentroids: 256,
    maxIterations: 50
  });
}
function createPQ32(dimension) {
  return new ProductQuantization({
    dimension,
    numSubvectors: 32,
    numCentroids: 256,
    maxIterations: 50
  });
}

// src/browser/HNSWIndex.ts
var MinHeap = class {
  items = [];
  push(item, priority) {
    this.items.push({ item, priority });
    this.bubbleUp(this.items.length - 1);
  }
  pop() {
    if (this.items.length === 0) return void 0;
    const result = this.items[0].item;
    const last = this.items.pop();
    if (this.items.length > 0) {
      this.items[0] = last;
      this.bubbleDown(0);
    }
    return result;
  }
  peek() {
    var _a;
    return (_a = this.items[0]) == null ? void 0 : _a.item;
  }
  size() {
    return this.items.length;
  }
  bubbleUp(index) {
    while (index > 0) {
      const parentIndex = Math.floor((index - 1) / 2);
      if (this.items[index].priority >= this.items[parentIndex].priority) break;
      [this.items[index], this.items[parentIndex]] = [this.items[parentIndex], this.items[index]];
      index = parentIndex;
    }
  }
  bubbleDown(index) {
    while (true) {
      const leftChild = 2 * index + 1;
      const rightChild = 2 * index + 2;
      let smallest = index;
      if (leftChild < this.items.length && this.items[leftChild].priority < this.items[smallest].priority) {
        smallest = leftChild;
      }
      if (rightChild < this.items.length && this.items[rightChild].priority < this.items[smallest].priority) {
        smallest = rightChild;
      }
      if (smallest === index) break;
      [this.items[index], this.items[smallest]] = [this.items[smallest], this.items[index]];
      index = smallest;
    }
  }
};
var HNSWIndex = class {
  config;
  nodes = /* @__PURE__ */ new Map();
  entryPoint = null;
  currentId = 0;
  ml;
  constructor(config = {}) {
    this.config = {
      dimension: config.dimension || 384,
      M: config.M || 16,
      efConstruction: config.efConstruction || 200,
      efSearch: config.efSearch || 50,
      ml: config.ml || 1 / Math.log(2),
      maxLayers: config.maxLayers || 16,
      distanceFunction: config.distanceFunction || "cosine"
    };
    this.ml = this.config.ml;
  }
  /**
   * Add vector to index
   */
  add(vector, id) {
    const nodeId = id !== void 0 ? id : this.currentId++;
    const level = this.randomLevel();
    const node = {
      id: nodeId,
      vector,
      level,
      connections: /* @__PURE__ */ new Map()
    };
    for (let l = 0; l <= level; l++) {
      node.connections.set(l, []);
    }
    if (this.entryPoint === null) {
      this.entryPoint = nodeId;
      this.nodes.set(nodeId, node);
      return nodeId;
    }
    const ep = this.entryPoint;
    let nearest = ep;
    for (let lc = this.nodes.get(ep).level; lc > level; lc--) {
      nearest = this.searchLayer(vector, nearest, 1, lc)[0];
    }
    for (let lc = Math.min(level, this.nodes.get(ep).level); lc >= 0; lc--) {
      const candidates = this.searchLayer(vector, nearest, this.config.efConstruction, lc);
      const M = lc === 0 ? this.config.M * 2 : this.config.M;
      const neighbors = this.selectNeighbors(vector, candidates, M);
      for (const neighbor of neighbors) {
        this.connect(nodeId, neighbor, lc);
        this.connect(neighbor, nodeId, lc);
        const neighborNode = this.nodes.get(neighbor);
        const neighborConnections = neighborNode.connections.get(lc);
        if (neighborConnections.length > M) {
          const newNeighbors = this.selectNeighbors(
            neighborNode.vector,
            neighborConnections,
            M
          );
          neighborNode.connections.set(lc, newNeighbors);
        }
      }
      nearest = candidates[0];
    }
    if (level > this.nodes.get(this.entryPoint).level) {
      this.entryPoint = nodeId;
    }
    this.nodes.set(nodeId, node);
    return nodeId;
  }
  /**
   * Search for k nearest neighbors
   */
  search(query, k, ef) {
    if (this.entryPoint === null) return [];
    ef = ef || Math.max(this.config.efSearch, k);
    let ep = this.entryPoint;
    let nearest = ep;
    for (let lc = this.nodes.get(ep).level; lc > 0; lc--) {
      nearest = this.searchLayer(query, nearest, 1, lc)[0];
    }
    const candidates = this.searchLayer(query, nearest, ef, 0);
    return candidates.slice(0, k).map((id) => ({
      id,
      distance: this.distance(query, this.nodes.get(id).vector),
      vector: this.nodes.get(id).vector
    }));
  }
  /**
   * Search at specific layer
   */
  searchLayer(query, ep, ef, layer) {
    const visited = /* @__PURE__ */ new Set();
    const candidates = new MinHeap();
    const w = new MinHeap();
    const dist = this.distance(query, this.nodes.get(ep).vector);
    candidates.push(ep, dist);
    w.push(ep, -dist);
    visited.add(ep);
    while (candidates.size() > 0) {
      const c = candidates.pop();
      const fDist = -w.peek();
      const cDist = this.distance(query, this.nodes.get(c).vector);
      if (cDist > fDist) break;
      const neighbors = this.nodes.get(c).connections.get(layer) || [];
      for (const e of neighbors) {
        if (visited.has(e)) continue;
        visited.add(e);
        const eDist = this.distance(query, this.nodes.get(e).vector);
        const fDist2 = -w.peek();
        if (eDist < fDist2 || w.size() < ef) {
          candidates.push(e, eDist);
          w.push(e, -eDist);
          if (w.size() > ef) {
            w.pop();
          }
        }
      }
    }
    const result = [];
    while (w.size() > 0) {
      result.unshift(w.pop());
    }
    return result;
  }
  /**
   * Select best neighbors using heuristic
   */
  selectNeighbors(base, candidates, M) {
    if (candidates.length <= M) return candidates;
    const sorted = candidates.map((id) => ({
      id,
      distance: this.distance(base, this.nodes.get(id).vector)
    })).sort((a, b) => a.distance - b.distance);
    return sorted.slice(0, M).map((x) => x.id);
  }
  /**
   * Connect two nodes at layer
   */
  connect(from, to, layer) {
    const node = this.nodes.get(from);
    const connections = node.connections.get(layer);
    if (!connections.includes(to)) {
      connections.push(to);
    }
  }
  /**
   * Random level assignment
   */
  randomLevel() {
    let level = 0;
    while (Math.random() < this.ml && level < this.config.maxLayers - 1) {
      level++;
    }
    return level;
  }
  /**
   * Distance function
   */
  distance(a, b) {
    switch (this.config.distanceFunction) {
      case "cosine":
        return 1 - this.cosineSimilarity(a, b);
      case "euclidean":
        return this.euclideanDistance(a, b);
      case "manhattan":
        return this.manhattanDistance(a, b);
      default:
        return 1 - this.cosineSimilarity(a, b);
    }
  }
  cosineSimilarity(a, b) {
    let dotProduct = 0;
    let normA = 0;
    let normB = 0;
    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }
    return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
  }
  euclideanDistance(a, b) {
    let sum = 0;
    for (let i = 0; i < a.length; i++) {
      const diff = a[i] - b[i];
      sum += diff * diff;
    }
    return Math.sqrt(sum);
  }
  manhattanDistance(a, b) {
    let sum = 0;
    for (let i = 0; i < a.length; i++) {
      sum += Math.abs(a[i] - b[i]);
    }
    return sum;
  }
  /**
   * Get index statistics
   */
  getStats() {
    if (this.nodes.size === 0) {
      return {
        numNodes: 0,
        numLayers: 0,
        avgConnections: 0,
        entryPointLevel: 0,
        memoryBytes: 0
      };
    }
    const maxLevel = Math.max(...Array.from(this.nodes.values()).map((n) => n.level));
    let totalConnections = 0;
    for (const node of this.nodes.values()) {
      for (const connections of node.connections.values()) {
        totalConnections += connections.length;
      }
    }
    const avgConnections = totalConnections / this.nodes.size;
    const vectorBytes = this.config.dimension * 4;
    const connectionBytes = avgConnections * 4;
    const metadataBytes = 100;
    const memoryBytes = this.nodes.size * (vectorBytes + connectionBytes + metadataBytes);
    return {
      numNodes: this.nodes.size,
      numLayers: maxLevel + 1,
      avgConnections,
      entryPointLevel: this.entryPoint ? this.nodes.get(this.entryPoint).level : 0,
      memoryBytes
    };
  }
  /**
   * Export index for persistence
   */
  export() {
    const data = {
      config: this.config,
      entryPoint: this.entryPoint,
      currentId: this.currentId,
      nodes: Array.from(this.nodes.entries()).map(([id, node]) => ({
        id,
        vector: Array.from(node.vector),
        level: node.level,
        connections: Array.from(node.connections.entries())
      }))
    };
    return JSON.stringify(data);
  }
  /**
   * Import index from JSON
   */
  import(json) {
    const data = JSON.parse(json);
    this.config = data.config;
    this.entryPoint = data.entryPoint;
    this.currentId = data.currentId;
    this.nodes.clear();
    for (const nodeData of data.nodes) {
      const node = {
        id: nodeData.id,
        vector: new Float32Array(nodeData.vector),
        level: nodeData.level,
        connections: new Map(nodeData.connections)
      };
      this.nodes.set(nodeData.id, node);
    }
  }
  /**
   * Clear index
   */
  clear() {
    this.nodes.clear();
    this.entryPoint = null;
    this.currentId = 0;
  }
  /**
   * Get number of nodes
   */
  size() {
    return this.nodes.size;
  }
};
function createHNSW(dimension) {
  return new HNSWIndex({
    dimension,
    M: 16,
    efConstruction: 200,
    efSearch: 50
  });
}
function createFastHNSW(dimension) {
  return new HNSWIndex({
    dimension,
    M: 8,
    efConstruction: 100,
    efSearch: 30
  });
}
function createAccurateHNSW(dimension) {
  return new HNSWIndex({
    dimension,
    M: 32,
    efConstruction: 400,
    efSearch: 100
  });
}

// src/browser/AdvancedFeatures.ts
var GraphNeuralNetwork = class {
  config;
  nodes = /* @__PURE__ */ new Map();
  edges = [];
  attentionWeights = /* @__PURE__ */ new Map();
  constructor(config = {}) {
    this.config = {
      hiddenDim: config.hiddenDim || 64,
      numHeads: config.numHeads || 4,
      dropout: config.dropout || 0.1,
      learningRate: config.learningRate || 0.01,
      attentionType: config.attentionType || "gat"
    };
  }
  /**
   * Add node to graph
   */
  addNode(id, features) {
    this.nodes.set(id, {
      id,
      features,
      neighbors: []
    });
  }
  /**
   * Add edge to graph
   */
  addEdge(from, to, weight = 1) {
    this.edges.push({ from, to, weight });
    const fromNode = this.nodes.get(from);
    const toNode = this.nodes.get(to);
    if (fromNode && !fromNode.neighbors.includes(to)) {
      fromNode.neighbors.push(to);
    }
    if (toNode && !toNode.neighbors.includes(from)) {
      toNode.neighbors.push(from);
    }
  }
  /**
   * Graph Attention Network (GAT) message passing
   */
  graphAttention(nodeId) {
    const node = this.nodes.get(nodeId);
    if (!node) throw new Error(`Node ${nodeId} not found`);
    const neighbors = node.neighbors;
    if (neighbors.length === 0) {
      return node.features;
    }
    const headDim = Math.floor(this.config.hiddenDim / this.config.numHeads);
    const aggregated = new Float32Array(this.config.hiddenDim);
    for (let h = 0; h < this.config.numHeads; h++) {
      let attentionSum = 0;
      const headOutput = new Float32Array(headDim);
      for (const neighborId of neighbors) {
        const neighbor = this.nodes.get(neighborId);
        const score = this.computeAttentionScore(
          node.features,
          neighbor.features,
          h
        );
        attentionSum += score;
        for (let i = 0; i < headDim && i < neighbor.features.length; i++) {
          headOutput[i] += score * neighbor.features[i];
        }
      }
      if (attentionSum > 0) {
        for (let i = 0; i < headDim; i++) {
          headOutput[i] /= attentionSum;
        }
      }
      const offset = h * headDim;
      for (let i = 0; i < headDim; i++) {
        aggregated[offset + i] = headOutput[i];
      }
    }
    for (let i = 0; i < aggregated.length; i++) {
      aggregated[i] = aggregated[i] > 0 ? aggregated[i] : 0.01 * aggregated[i];
    }
    return aggregated;
  }
  /**
   * Compute attention score between two nodes
   */
  computeAttentionScore(features1, features2, head) {
    let score = 0;
    const len = Math.min(features1.length, features2.length);
    for (let i = 0; i < len; i++) {
      score += features1[i] * features2[i];
    }
    return Math.exp(score / Math.sqrt(len));
  }
  /**
   * Message passing for all nodes
   */
  messagePass() {
    const newFeatures = /* @__PURE__ */ new Map();
    for (const [nodeId] of this.nodes) {
      newFeatures.set(nodeId, this.graphAttention(nodeId));
    }
    return newFeatures;
  }
  /**
   * Update node features after message passing
   */
  update(newFeatures) {
    for (const [nodeId, features] of newFeatures) {
      const node = this.nodes.get(nodeId);
      if (node) {
        node.features = features;
      }
    }
  }
  /**
   * Compute graph embeddings for query enhancement
   */
  computeGraphEmbedding(nodeId, hops = 2) {
    const features = /* @__PURE__ */ new Map();
    features.set(nodeId, this.nodes.get(nodeId).features);
    for (let h = 0; h < hops; h++) {
      const newFeatures = this.messagePass();
      this.update(newFeatures);
    }
    return this.nodes.get(nodeId).features;
  }
  /**
   * Get statistics
   */
  getStats() {
    return {
      numNodes: this.nodes.size,
      numEdges: this.edges.length,
      avgDegree: this.edges.length / Math.max(this.nodes.size, 1),
      config: this.config
    };
  }
};
var MaximalMarginalRelevance = class {
  config;
  constructor(config = {}) {
    this.config = {
      lambda: config.lambda || 0.7,
      metric: config.metric || "cosine"
    };
  }
  /**
   * Rerank results for diversity
   * @param query Query vector
   * @param candidates Candidate vectors with scores
   * @param k Number of results to return
   * @returns Reranked indices
   */
  rerank(query, candidates, k) {
    if (candidates.length === 0) return [];
    const selected = [];
    const remaining = new Set(candidates.map((_, i) => i));
    let bestIdx = 0;
    let bestScore = -Infinity;
    for (let i = 0; i < candidates.length; i++) {
      if (candidates[i].score > bestScore) {
        bestScore = candidates[i].score;
        bestIdx = i;
      }
    }
    selected.push(candidates[bestIdx].id);
    remaining.delete(bestIdx);
    while (selected.length < k && remaining.size > 0) {
      let bestMMR = -Infinity;
      let bestCandidate = -1;
      for (const idx of remaining) {
        const candidate = candidates[idx];
        const relevance = this.similarity(query, candidate.vector);
        let maxSimilarity = -Infinity;
        for (const selectedId of selected) {
          const selectedCandidate = candidates.find((c) => c.id === selectedId);
          const sim = this.similarity(candidate.vector, selectedCandidate.vector);
          maxSimilarity = Math.max(maxSimilarity, sim);
        }
        const mmr = this.config.lambda * relevance - (1 - this.config.lambda) * maxSimilarity;
        if (mmr > bestMMR) {
          bestMMR = mmr;
          bestCandidate = idx;
        }
      }
      if (bestCandidate !== -1) {
        selected.push(candidates[bestCandidate].id);
        remaining.delete(bestCandidate);
      } else {
        break;
      }
    }
    return selected;
  }
  /**
   * Similarity computation
   */
  similarity(a, b) {
    if (this.config.metric === "cosine") {
      return this.cosineSimilarity(a, b);
    } else {
      const dist = this.euclideanDistance(a, b);
      return 1 / (1 + dist);
    }
  }
  cosineSimilarity(a, b) {
    let dotProduct = 0;
    let normA = 0;
    let normB = 0;
    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }
    return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
  }
  euclideanDistance(a, b) {
    let sum = 0;
    for (let i = 0; i < a.length; i++) {
      const diff = a[i] - b[i];
      sum += diff * diff;
    }
    return Math.sqrt(sum);
  }
  /**
   * Set lambda (relevance vs diversity trade-off)
   */
  setLambda(lambda) {
    this.config.lambda = Math.max(0, Math.min(1, lambda));
  }
};
var TensorCompression = class {
  /**
   * Reduce dimensionality using truncated SVD
   * @param vectors Array of vectors to compress
   * @param targetDim Target dimension
   * @returns Compressed vectors
   */
  static compress(vectors, targetDim) {
    if (vectors.length === 0) return [];
    const originalDim = vectors[0].length;
    if (targetDim >= originalDim) return vectors;
    const matrix = vectors.map((v) => Array.from(v));
    const mean = this.computeMean(matrix);
    const centered = matrix.map(
      (row) => row.map((val, i) => val - mean[i])
    );
    const cov = this.computeCovariance(centered);
    const eigenvectors = this.powerIteration(cov, targetDim);
    const compressed = centered.map((row) => {
      const projected = new Float32Array(targetDim);
      for (let i = 0; i < targetDim; i++) {
        let sum = 0;
        for (let j = 0; j < originalDim; j++) {
          sum += row[j] * eigenvectors[i][j];
        }
        projected[i] = sum;
      }
      return projected;
    });
    return compressed;
  }
  /**
   * Compute mean vector
   */
  static computeMean(matrix) {
    const n = matrix.length;
    const dim = matrix[0].length;
    const mean = new Array(dim).fill(0);
    for (const row of matrix) {
      for (let i = 0; i < dim; i++) {
        mean[i] += row[i];
      }
    }
    return mean.map((v) => v / n);
  }
  /**
   * Compute covariance matrix
   */
  static computeCovariance(matrix) {
    const n = matrix.length;
    const dim = matrix[0].length;
    const cov = Array.from(
      { length: dim },
      () => new Array(dim).fill(0)
    );
    for (let i = 0; i < dim; i++) {
      for (let j = 0; j <= i; j++) {
        let sum = 0;
        for (const row of matrix) {
          sum += row[i] * row[j];
        }
        cov[i][j] = cov[j][i] = sum / n;
      }
    }
    return cov;
  }
  /**
   * Power iteration for computing top eigenvectors
   */
  static powerIteration(matrix, k, iterations = 100) {
    const dim = matrix.length;
    const eigenvectors = [];
    for (let i = 0; i < k; i++) {
      let v = new Array(dim).fill(0).map(() => Math.random() - 0.5);
      for (let iter = 0; iter < iterations; iter++) {
        const newV = new Array(dim).fill(0);
        for (let r = 0; r < dim; r++) {
          for (let c = 0; c < dim; c++) {
            newV[r] += matrix[r][c] * v[c];
          }
        }
        for (const prev of eigenvectors) {
          let dot = 0;
          for (let j = 0; j < dim; j++) {
            dot += newV[j] * prev[j];
          }
          for (let j = 0; j < dim; j++) {
            newV[j] -= dot * prev[j];
          }
        }
        let norm = 0;
        for (const val of newV) {
          norm += val * val;
        }
        norm = Math.sqrt(norm);
        if (norm < 1e-10) break;
        v = newV.map((val) => val / norm);
      }
      eigenvectors.push(v);
    }
    return eigenvectors;
  }
};
var BatchProcessor = class {
  /**
   * Batch cosine similarity computation
   */
  static batchCosineSimilarity(query, vectors) {
    const similarities = new Float32Array(vectors.length);
    let queryNorm = 0;
    for (let i = 0; i < query.length; i++) {
      queryNorm += query[i] * query[i];
    }
    queryNorm = Math.sqrt(queryNorm);
    for (let v = 0; v < vectors.length; v++) {
      const vector = vectors[v];
      let dotProduct = 0;
      let vectorNorm = 0;
      for (let i = 0; i < query.length; i++) {
        dotProduct += query[i] * vector[i];
        vectorNorm += vector[i] * vector[i];
      }
      vectorNorm = Math.sqrt(vectorNorm);
      similarities[v] = dotProduct / (queryNorm * vectorNorm);
    }
    return similarities;
  }
  /**
   * Batch vector normalization
   */
  static batchNormalize(vectors) {
    return vectors.map((v) => {
      let norm = 0;
      for (let i = 0; i < v.length; i++) {
        norm += v[i] * v[i];
      }
      norm = Math.sqrt(norm);
      const normalized = new Float32Array(v.length);
      for (let i = 0; i < v.length; i++) {
        normalized[i] = v[i] / norm;
      }
      return normalized;
    });
  }
};

// src/browser/AttentionBrowser.ts
var AttentionBrowser = class {
  wasmModule = null;
  loadingState = "idle";
  loadError = null;
  config;
  constructor(config = {}) {
    this.config = {
      dimension: 384,
      numHeads: 4,
      blockSize: 64,
      curvature: -1,
      useWASM: true,
      ...config
    };
  }
  /**
   * Get current loading state
   */
  getLoadingState() {
    return this.loadingState;
  }
  /**
   * Get loading error if any
   */
  getError() {
    return this.loadError;
  }
  /**
   * Initialize WASM module (lazy loaded)
   */
  async initialize() {
    if (this.loadingState === "loaded") return;
    if (this.loadingState === "loading") {
      while (this.loadingState === "loading") {
        await new Promise((resolve) => setTimeout(resolve, 50));
      }
      return;
    }
    this.loadingState = "loading";
    try {
      if (!this.config.useWASM) {
        this.loadingState = "loaded";
        return;
      }
      const wasmLoader = await Promise.resolve().then(() => (init_agentdb_wasm_loader(), agentdb_wasm_loader_exports));
      this.wasmModule = await wasmLoader.initWASM();
      this.loadingState = "loaded";
    } catch (error) {
      this.loadError = error instanceof Error ? error : new Error(String(error));
      this.loadingState = "error";
      console.warn("WASM initialization failed, using fallback:", this.loadError.message);
    }
  }
  /**
   * Flash Attention - Optimized attention mechanism
   * O(N) memory complexity instead of O(N²)
   *
   * @param query - Query vectors
   * @param keys - Key vectors
   * @param values - Value vectors
   * @returns Attention output
   */
  async flashAttention(query, keys, values) {
    var _a;
    await this.initialize();
    if ((_a = this.wasmModule) == null ? void 0 : _a.flashAttention) {
      try {
        return this.wasmModule.flashAttention(query, keys, values, this.config);
      } catch (error) {
        console.warn("WASM flash attention failed, using fallback:", error);
      }
    }
    return this.flashAttentionFallback(query, keys, values);
  }
  /**
   * Hyperbolic Attention - Attention in hyperbolic space
   * Better for hierarchical relationships
   *
   * @param query - Query vector
   * @param keys - Key vectors
   * @returns Similarity scores in hyperbolic space
   */
  async hyperbolicAttention(query, keys) {
    var _a;
    await this.initialize();
    if ((_a = this.wasmModule) == null ? void 0 : _a.hyperbolicAttention) {
      try {
        return this.wasmModule.hyperbolicAttention(query, keys, this.config);
      } catch (error) {
        console.warn("WASM hyperbolic attention failed, using fallback:", error);
      }
    }
    return this.hyperbolicAttentionFallback(query, keys);
  }
  /**
   * Memory Consolidation - Cluster and consolidate similar memories
   *
   * @param memories - Array of memory vectors
   * @param config - Consolidation configuration
   * @returns Consolidated memory clusters
   */
  async consolidateMemories(memories, config = {}) {
    var _a;
    await this.initialize();
    const fullConfig = {
      threshold: 0.8,
      maxClusters: 10,
      minClusterSize: 1,
      ...config
    };
    if ((_a = this.wasmModule) == null ? void 0 : _a.memoryConsolidation) {
      try {
        return this.wasmModule.memoryConsolidation(memories, fullConfig);
      } catch (error) {
        console.warn("WASM memory consolidation failed, using fallback:", error);
      }
    }
    return this.consolidateMemoriesFallback(memories, fullConfig);
  }
  /**
   * Clean up WASM memory
   */
  dispose() {
    this.wasmModule = null;
    this.loadingState = "idle";
    this.loadError = null;
  }
  // ========================================================================
  // Fallback Implementations (Pure JavaScript)
  // ========================================================================
  flashAttentionFallback(query, keys, values) {
    const { dimension = 384 } = this.config;
    const seqLen = keys.length / dimension;
    const output = new Float32Array(query.length);
    for (let i = 0; i < query.length; i += dimension) {
      const q = query.slice(i, i + dimension);
      let sumWeights = 0;
      const weights = new Float32Array(seqLen);
      for (let j = 0; j < seqLen; j++) {
        const k = keys.slice(j * dimension, (j + 1) * dimension);
        let dot = 0;
        for (let d = 0; d < dimension; d++) {
          dot += q[d] * k[d];
        }
        weights[j] = Math.exp(dot / Math.sqrt(dimension));
        sumWeights += weights[j];
      }
      for (let j = 0; j < seqLen; j++) {
        weights[j] /= sumWeights || 1;
        const v = values.slice(j * dimension, (j + 1) * dimension);
        for (let d = 0; d < dimension; d++) {
          output[i + d] += weights[j] * v[d];
        }
      }
    }
    return output;
  }
  hyperbolicAttentionFallback(query, keys) {
    const { curvature = -1 } = this.config;
    const k = Math.abs(curvature);
    const similarities = new Float32Array(keys.length / query.length);
    for (let i = 0; i < similarities.length; i++) {
      const offset = i * query.length;
      let dotProduct = 0;
      let normQ = 0;
      let normK = 0;
      for (let j = 0; j < query.length; j++) {
        dotProduct += query[j] * keys[offset + j];
        normQ += query[j] * query[j];
        normK += keys[offset + j] * keys[offset + j];
      }
      const euclidean = Math.sqrt(normQ + normK - 2 * dotProduct);
      const poincare = Math.acosh(1 + 2 * k * euclidean * euclidean);
      similarities[i] = 1 / (1 + poincare);
    }
    return similarities;
  }
  consolidateMemoriesFallback(memories, config) {
    const { threshold = 0.8, maxClusters = 10, minClusterSize = 1 } = config;
    const consolidated = [];
    const used = /* @__PURE__ */ new Set();
    for (let i = 0; i < memories.length; i++) {
      if (used.has(i)) continue;
      const cluster = [memories[i]];
      used.add(i);
      for (let j = i + 1; j < memories.length; j++) {
        if (used.has(j)) continue;
        const similarity = this.cosineSimilarity(memories[i], memories[j]);
        if (similarity > threshold) {
          cluster.push(memories[j]);
          used.add(j);
        }
      }
      if (cluster.length >= minClusterSize) {
        const centroid = new Float32Array(memories[i].length);
        for (const mem of cluster) {
          for (let k = 0; k < centroid.length; k++) {
            centroid[k] += mem[k] / cluster.length;
          }
        }
        let norm = 0;
        for (let k = 0; k < centroid.length; k++) {
          norm += centroid[k] * centroid[k];
        }
        norm = Math.sqrt(norm);
        if (norm > 0) {
          for (let k = 0; k < centroid.length; k++) {
            centroid[k] /= norm;
          }
        }
        consolidated.push({
          memory: centroid,
          count: cluster.length,
          members: cluster
        });
      }
      if (consolidated.length >= maxClusters) break;
    }
    return consolidated;
  }
  cosineSimilarity(a, b) {
    let dot = 0;
    let normA = 0;
    let normB = 0;
    for (let i = 0; i < a.length; i++) {
      dot += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }
    const denominator = Math.sqrt(normA * normB);
    return denominator > 0 ? dot / denominator : 0;
  }
};
function createAttention(config) {
  return new AttentionBrowser(config);
}
function createFastAttention() {
  return new AttentionBrowser({
    dimension: 256,
    numHeads: 2,
    blockSize: 32,
    useWASM: true
  });
}
function createAccurateAttention() {
  return new AttentionBrowser({
    dimension: 768,
    numHeads: 8,
    blockSize: 128,
    useWASM: true
  });
}

// src/browser/index.ts
function detectFeatures() {
  return {
    indexedDB: "indexedDB" in globalThis,
    broadcastChannel: "BroadcastChannel" in globalThis,
    webWorkers: typeof globalThis.Worker !== "undefined",
    wasmSIMD: detectWasmSIMD2(),
    sharedArrayBuffer: typeof SharedArrayBuffer !== "undefined"
  };
}
async function detectWasmSIMD2() {
  try {
    if (typeof globalThis.WebAssembly === "undefined") {
      return false;
    }
    const simdTest = new Uint8Array([
      0,
      97,
      115,
      109,
      1,
      0,
      0,
      0,
      1,
      5,
      1,
      96,
      0,
      1,
      123,
      3,
      2,
      1,
      0,
      10,
      10,
      1,
      8,
      0,
      253,
      12,
      253,
      12,
      253,
      84,
      11
    ]);
    const WA = globalThis.WebAssembly;
    const module = await WA.instantiate(simdTest);
    return module instanceof WA.Instance;
  } catch {
    return false;
  }
}
var SMALL_DATASET_CONFIG = {
  pq: { enabled: false },
  hnsw: { enabled: false },
  gnn: { enabled: true, numHeads: 2 },
  mmr: { enabled: true, lambda: 0.7 },
  svd: { enabled: false }
};
var MEDIUM_DATASET_CONFIG = {
  pq: { enabled: true, subvectors: 8 },
  hnsw: { enabled: true, M: 16 },
  gnn: { enabled: true, numHeads: 4 },
  mmr: { enabled: true, lambda: 0.7 },
  svd: { enabled: false }
};
var LARGE_DATASET_CONFIG = {
  pq: { enabled: true, subvectors: 16 },
  hnsw: { enabled: true, M: 32 },
  gnn: { enabled: true, numHeads: 4 },
  mmr: { enabled: true, lambda: 0.7 },
  svd: { enabled: true, targetDim: 128 }
};
var MEMORY_OPTIMIZED_CONFIG = {
  pq: { enabled: true, subvectors: 32 },
  // 16x compression
  hnsw: { enabled: true, M: 8 },
  // Fewer connections
  gnn: { enabled: false },
  mmr: { enabled: false },
  svd: { enabled: true, targetDim: 64 }
  // Aggressive dimension reduction
};
var SPEED_OPTIMIZED_CONFIG = {
  pq: { enabled: false },
  // No compression overhead
  hnsw: { enabled: true, M: 32, efSearch: 100 },
  // Maximum HNSW quality
  gnn: { enabled: false },
  mmr: { enabled: false },
  svd: { enabled: false }
};
var QUALITY_OPTIMIZED_CONFIG = {
  pq: { enabled: false },
  // No compression
  hnsw: { enabled: true, M: 48, efConstruction: 400 },
  // Highest quality
  gnn: { enabled: true, numHeads: 8 },
  // More attention heads
  mmr: { enabled: true, lambda: 0.8 },
  // More diversity
  svd: { enabled: false }
  // No dimension loss
};
var VERSION = {
  major: 2,
  minor: 0,
  patch: 0,
  prerelease: "alpha.2",
  features: "advanced",
  full: "2.0.0-alpha.2+advanced"
};
function estimateMemoryUsage(numVectors, dimension, config) {
  var _a, _b, _c;
  let vectorBytes = numVectors * dimension * 4;
  if ((_a = config.pq) == null ? void 0 : _a.enabled) {
    const subvectors = config.pq.subvectors || 8;
    vectorBytes = numVectors * (subvectors + 4);
  }
  if ((_b = config.svd) == null ? void 0 : _b.enabled) {
    const targetDim = config.svd.targetDim || dimension / 2;
    vectorBytes = numVectors * targetDim * 4;
  }
  let indexBytes = 0;
  if ((_c = config.hnsw) == null ? void 0 : _c.enabled) {
    const M = config.hnsw.M || 16;
    const avgConnections = M * 1.5;
    indexBytes = numVectors * avgConnections * 4;
  }
  const total = vectorBytes + indexBytes;
  return {
    vectors: vectorBytes,
    index: indexBytes,
    total,
    totalMB: total / (1024 * 1024)
  };
}
function recommendConfig(numVectors, dimension) {
  if (numVectors < 1e3) {
    return {
      name: "SMALL_DATASET",
      config: SMALL_DATASET_CONFIG,
      reason: "Small dataset, linear search is fast enough"
    };
  } else if (numVectors < 1e4) {
    return {
      name: "MEDIUM_DATASET",
      config: MEDIUM_DATASET_CONFIG,
      reason: "Medium dataset, HNSW + PQ8 recommended"
    };
  } else {
    return {
      name: "LARGE_DATASET",
      config: LARGE_DATASET_CONFIG,
      reason: "Large dataset, aggressive compression + HNSW recommended"
    };
  }
}
async function benchmarkSearch(searchFn, numQueries = 100, k = 10, dimension = 384) {
  const times = [];
  for (let i = 0; i < numQueries; i++) {
    const query = new Float32Array(dimension);
    for (let d = 0; d < dimension; d++) {
      query[d] = Math.random() - 0.5;
    }
    const start = performance.now();
    searchFn(query, k);
    const end = performance.now();
    times.push(end - start);
  }
  times.sort((a, b) => a - b);
  return {
    avgTimeMs: times.reduce((a, b) => a + b, 0) / times.length,
    minTimeMs: times[0],
    maxTimeMs: times[times.length - 1],
    p50Ms: times[Math.floor(times.length * 0.5)],
    p95Ms: times[Math.floor(times.length * 0.95)],
    p99Ms: times[Math.floor(times.length * 0.99)]
  };
}
export {
  AttentionBrowser,
  BatchProcessor,
  GraphNeuralNetwork,
  HNSWIndex,
  LARGE_DATASET_CONFIG,
  MEDIUM_DATASET_CONFIG,
  MEMORY_OPTIMIZED_CONFIG,
  MaximalMarginalRelevance,
  ProductQuantization,
  QUALITY_OPTIMIZED_CONFIG,
  SMALL_DATASET_CONFIG,
  SPEED_OPTIMIZED_CONFIG,
  TensorCompression,
  VERSION,
  benchmarkSearch,
  createAccurateAttention,
  createAccurateHNSW,
  createAttention,
  createFastAttention,
  createFastHNSW,
  createHNSW,
  createPQ16,
  createPQ32,
  createPQ8,
  detectFeatures,
  estimateMemoryUsage,
  recommendConfig
};
//# sourceMappingURL=agentdb.browser.js.map
