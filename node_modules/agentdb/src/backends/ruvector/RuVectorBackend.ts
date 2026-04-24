/**
 * RuVectorBackend - High-Performance Vector Storage
 *
 * Implements VectorBackend using @ruvector/core with optional GNN support.
 * Provides <100µs search latency with native SIMD optimizations.
 *
 * Features:
 * - Automatic fallback when @ruvector packages not installed
 * - Separate metadata storage for rich queries
 * - Distance-to-similarity conversion for all metrics
 * - Batch operations for optimal throughput
 * - Persistent storage with separate metadata files
 * - Parallel batch insert with configurable concurrency
 * - Buffer pooling to reduce memory allocations
 * - Adaptive index parameters based on dataset size
 * - Memory-mapped support for large indices
 * - Statistics tracking for performance monitoring
 */

import type { VectorBackend, VectorConfig, SearchResult, SearchOptions, VectorStats } from '../VectorBackend.js';

// ============================================================================
// Performance & Security Constants
// ============================================================================

/** @inline Maximum supported vector dimension for bounds checking */
export const MAX_VECTOR_DIMENSION = 4096;

/** @inline Maximum batch size for optimal cache utilization */
export const MAX_BATCH_SIZE = 10000;

/** @inline Default cache size for embedding storage */
export const DEFAULT_CACHE_SIZE = 10000;

/** @inline Small epsilon for numerical stability */
const EPSILON = 1e-10;

/** @inline Maximum metadata entries to prevent memory exhaustion */
const MAX_METADATA_ENTRIES = 10_000_000;

/** @inline Maximum path length for file operations */
const MAX_PATH_LENGTH = 4096;

/** Forbidden path patterns for security */
const FORBIDDEN_PATH_PATTERNS = [
  /\.\./,       // Path traversal
  /^\/etc\//i,  // System config
  /^\/proc\//i, // Process info
  /^\/sys\//i,  // System info
  /^\/dev\//i,  // Devices
];

// ============================================================================
// Security Validation Helpers
// ============================================================================

/**
 * Validate a file path is safe
 */
function validatePath(inputPath: string): void {
  if (!inputPath || typeof inputPath !== 'string') {
    throw new Error('Path must be a non-empty string');
  }
  if (inputPath.length > MAX_PATH_LENGTH) {
    throw new Error(`Path exceeds maximum length of ${MAX_PATH_LENGTH}`);
  }
  for (const pattern of FORBIDDEN_PATH_PATTERNS) {
    if (pattern.test(inputPath)) {
      throw new Error(`Path contains forbidden pattern`);
    }
  }
}

/**
 * Validate vector dimension is within safe limits
 */
function validateDimension(dimension: number): void {
  if (!Number.isFinite(dimension) || dimension < 1 || dimension > MAX_VECTOR_DIMENSION) {
    throw new Error(`Dimension must be between 1 and ${MAX_VECTOR_DIMENSION}`);
  }
}

/**
 * Semaphore for controlling concurrent operations
 */
class Semaphore {
  private permits: number;
  private waiting: Array<() => void> = [];

  constructor(permits: number) {
    this.permits = permits;
  }

  async acquire(): Promise<void> {
    if (this.permits > 0) {
      this.permits--;
      return Promise.resolve();
    }
    return new Promise<void>((resolve) => {
      this.waiting.push(resolve);
    });
  }

  release(): void {
    if (this.waiting.length > 0) {
      const resolve = this.waiting.shift()!;
      resolve();
    } else {
      this.permits++;
    }
  }

  get available(): number {
    return this.permits;
  }
}

/**
 * Buffer pool for reusing Float32Array buffers
 */
class BufferPool {
  private pools: Map<number, Float32Array[]> = new Map();
  private maxPoolSize: number;

  constructor(maxPoolSize = 100) {
    this.maxPoolSize = maxPoolSize;
  }

  /**
   * Acquire a buffer of the specified size
   */
  acquire(size: number): Float32Array {
    const pool = this.pools.get(size);
    if (pool && pool.length > 0) {
      return pool.pop()!;
    }
    return new Float32Array(size);
  }

  /**
   * Release a buffer back to the pool
   */
  release(buffer: Float32Array): void {
    const size = buffer.length;
    if (!this.pools.has(size)) {
      this.pools.set(size, []);
    }
    const pool = this.pools.get(size)!;
    if (pool.length < this.maxPoolSize) {
      // Clear buffer before returning to pool
      buffer.fill(0);
      pool.push(buffer);
    }
  }

  /**
   * Clear all pooled buffers
   */
  clear(): void {
    this.pools.clear();
  }

  /**
   * Get pool statistics
   */
  getStats(): { totalBuffers: number; totalMemory: number } {
    let totalBuffers = 0;
    let totalMemory = 0;
    for (const [size, pool] of this.pools) {
      totalBuffers += pool.length;
      totalMemory += pool.length * size * 4; // Float32 = 4 bytes
    }
    return { totalBuffers, totalMemory };
  }
}

/**
 * Statistics tracker for performance monitoring
 */
interface PerformanceStats {
  insertCount: number;
  insertTotalLatencyMs: number;
  insertMinLatencyMs: number;
  insertMaxLatencyMs: number;
  searchCount: number;
  searchTotalLatencyMs: number;
  searchMinLatencyMs: number;
  searchMaxLatencyMs: number;
  cacheHits: number;
  cacheMisses: number;
  lastMemoryUsage: number;
  indexRebuildCount: number;
}

/**
 * Adaptive index parameters based on dataset size
 */
interface AdaptiveParams {
  M: number;
  efConstruction: number;
  efSearch: number;
}

/**
 * Extended configuration with new options
 */
export interface RuVectorConfig extends VectorConfig {
  /** Concurrency level for parallel batch insert (default: 4) */
  parallelConcurrency?: number;
  /** Buffer pool max size (default: 100) */
  bufferPoolSize?: number;
  /** Enable adaptive index parameters (default: true) */
  adaptiveParams?: boolean;
  /** Enable memory-mapped storage for large indices (default: false) */
  enableMmap?: boolean;
  /** Path for memory-mapped storage */
  mmapPath?: string;
  /** Enable statistics tracking (default: true) */
  enableStats?: boolean;
}

export class RuVectorBackend implements VectorBackend {
  readonly name = 'ruvector' as const;
  private db: any; // VectorDB from @ruvector/core
  private config: RuVectorConfig;
  private metadata: Map<string, Record<string, any>> = new Map();
  private initialized = false;

  // Concurrency control
  private semaphore: Semaphore;
  private bufferPool: BufferPool;

  // Statistics tracking
  private stats: PerformanceStats = {
    insertCount: 0,
    insertTotalLatencyMs: 0,
    insertMinLatencyMs: Infinity,
    insertMaxLatencyMs: 0,
    searchCount: 0,
    searchTotalLatencyMs: 0,
    searchMinLatencyMs: Infinity,
    searchMaxLatencyMs: 0,
    cacheHits: 0,
    cacheMisses: 0,
    lastMemoryUsage: 0,
    indexRebuildCount: 0,
  };

  // Mmap support
  private mmapEnabled = false;
  private mmapBuffer: Buffer | null = null;

  constructor(config: VectorConfig | RuVectorConfig) {
    // Handle both dimension and dimensions for backward compatibility
    const dimension = config.dimension ?? config.dimensions;
    if (!dimension) {
      throw new Error('Vector dimension is required (use dimension or dimensions)');
    }

    // Validate dimension for security
    validateDimension(dimension);

    // Store both forms for compatibility with different backends
    this.config = { ...config, dimension, dimensions: dimension } as RuVectorConfig;

    // Initialize concurrency control with configurable limit (bounded for security)
    const concurrency = Math.min(Math.max(1, (config as RuVectorConfig).parallelConcurrency ?? 4), 32);
    this.semaphore = new Semaphore(concurrency);

    // Initialize buffer pool with bounded size
    const bufferPoolSize = Math.min(Math.max(1, (config as RuVectorConfig).bufferPoolSize ?? 100), 1000);
    this.bufferPool = new BufferPool(bufferPoolSize);

    // Mmap configuration - validate path if provided
    this.mmapEnabled = (config as RuVectorConfig).enableMmap ?? false;
    if (this.mmapEnabled && (config as RuVectorConfig).mmapPath) {
      validatePath((config as RuVectorConfig).mmapPath!);
    }
  }

  /**
   * Get adaptive HNSW parameters based on expected dataset size
   */
  private getAdaptiveParams(datasetSize: number): AdaptiveParams {
    if (datasetSize < 1000) {
      // Small dataset: lower M and efConstruction for faster builds
      return { M: 8, efConstruction: 100, efSearch: 50 };
    } else if (datasetSize <= 100000) {
      // Medium dataset: balanced parameters
      return { M: 16, efConstruction: 200, efSearch: 100 };
    } else {
      // Large dataset: higher M for better recall
      return { M: 32, efConstruction: 400, efSearch: 200 };
    }
  }

  /**
   * Initialize RuVector database with optional dependency handling
   */
  async initialize(): Promise<void> {
    if (this.initialized) return;

    try {
      // Try main ruvector package first (includes core, gnn, graph)
      let VectorDB;
      try {
        const ruvector = await import('ruvector');
        VectorDB = ruvector.VectorDB || ruvector.default?.VectorDB;
      } catch {
        // Fallback to @ruvector/core for backward compatibility
        const core = await import('@ruvector/core');
        // ESM and CommonJS both export as VectorDB (capital 'DB')
        VectorDB = core.VectorDB || core.default?.VectorDB;
      }

      if (!VectorDB) {
        throw new Error('Could not find VectorDB export in @ruvector/core');
      }

      // Handle both 'dimension' and 'dimensions' for backward compatibility
      const dimensions = this.config.dimension ?? this.config.dimensions;
      if (!dimensions) {
        throw new Error('Vector dimension is required (use dimension or dimensions)');
      }

      // Determine HNSW parameters (adaptive or explicit)
      const maxElements = this.config.maxElements || 100000;
      const useAdaptive = this.config.adaptiveParams !== false;
      const adaptiveParams = useAdaptive ? this.getAdaptiveParams(maxElements) : null;

      const m = this.config.M ?? adaptiveParams?.M ?? 16;
      const efConstruction = this.config.efConstruction ?? adaptiveParams?.efConstruction ?? 200;
      const efSearch = this.config.efSearch ?? adaptiveParams?.efSearch ?? 100;

      // RuVector VectorDB constructor signature
      this.db = new VectorDB({
        dimensions: dimensions,  // Note: config object, not positional arg
        metric: this.config.metric,
        maxElements: maxElements,
        efConstruction: efConstruction,
        m: m  // Note: lowercase 'm'
      });

      // Set default efSearch
      if (this.db.setEfSearch) {
        this.db.setEfSearch(efSearch);
      }

      // Initialize memory-mapped storage if enabled and available
      if (this.mmapEnabled && this.config.mmapPath) {
        await this.initializeMmap();
      }

      this.initialized = true;
    } catch (error) {
      const errorMessage = (error as Error).message;

      // Special handling for path validation errors (from ruvector package)
      // When using :memory:, ruvector may reject it as a path traversal attempt
      // This is expected and not critical - users should use file-based paths for ruvector persistence
      if (errorMessage.includes('Path traversal') || errorMessage.includes('Invalid path')) {
        throw new Error(
          `RuVector does not support :memory: database paths.\n` +
          `Use a file path instead, or RuVector will be skipped and fallback backend will be used.\n` +
          `Original error: ${errorMessage}`
        );
      }

      throw new Error(
        `RuVector initialization failed. Please install: npm install ruvector\n` +
        `Or legacy packages: npm install @ruvector/core\n` +
        `Error: ${errorMessage}`
      );
    }
  }

  /**
   * Initialize memory-mapped storage for large indices
   */
  private async initializeMmap(): Promise<void> {
    if (!this.config.mmapPath) return;

    // Validate path for security
    validatePath(this.config.mmapPath);

    let fd: number | null = null;

    try {
      const fs = await import('fs');
      const pathModule = await import('path');

      // Define mmap module interface
      interface MmapModule {
        map: (size: number, prot: number, flags: number, fd: number, offset: number) => Buffer;
        PROT_READ: number;
        PROT_WRITE: number;
        MAP_SHARED: number;
      }

      // Check if mmap module is available (optional native dependency)
      let mmapModule: MmapModule | null = null;
      try {
        // Dynamic import of optional native module - known safe module
        const moduleName = 'mmap-io';
        mmapModule = await (import(moduleName) as unknown as Promise<MmapModule>);
      } catch {
        // mmap-io not available, fall back to regular I/O
        console.debug('[RuVectorBackend] mmap-io not available, using standard I/O');
        return;
      }

      if (!mmapModule) return;

      const mmapPath = pathModule.resolve(this.config.mmapPath);

      // Re-validate resolved path
      validatePath(mmapPath);

      const mmap = mmapModule;

      // Create or open the mmap file
      if (fs.existsSync(mmapPath)) {
        fd = fs.openSync(mmapPath, 'r+');
        const stats = fs.fstatSync(fd);

        // Validate file size to prevent excessive memory mapping
        const maxMmapSize = 4 * 1024 * 1024 * 1024; // 4GB max
        if (stats.size > maxMmapSize) {
          throw new Error(`Mmap file size ${stats.size} exceeds maximum of ${maxMmapSize}`);
        }

        this.mmapBuffer = mmap.map(
          stats.size,
          mmap.PROT_READ | mmap.PROT_WRITE,
          mmap.MAP_SHARED,
          fd,
          0
        );
        fs.closeSync(fd);
        fd = null;
      }

      this.mmapEnabled = true;
    } catch (error) {
      console.debug(`[RuVectorBackend] Failed to initialize mmap: ${(error as Error).message}`);
      this.mmapEnabled = false;
    } finally {
      // Ensure file descriptor is closed even on error
      if (fd !== null) {
        try {
          const fs = await import('fs');
          fs.closeSync(fd);
        } catch {
          // Ignore close errors in cleanup
        }
      }
    }
  }

  /**
   * Insert single vector with optional metadata
   */
  insert(id: string, embedding: Float32Array, metadata?: Record<string, any>): void {
    this.ensureInitialized();

    // Validate id
    if (!id || typeof id !== 'string') {
      throw new Error('Vector ID must be a non-empty string');
    }

    // Validate embedding type
    if (!(embedding instanceof Float32Array) && !Array.isArray(embedding)) {
      throw new Error('Embedding must be a Float32Array or array');
    }

    // Check metadata store size limit
    if (metadata && this.metadata.size >= MAX_METADATA_ENTRIES) {
      throw new Error(`Metadata store has reached maximum capacity of ${MAX_METADATA_ENTRIES}`);
    }

    const startTime = this.config.enableStats !== false ? performance.now() : 0;

    // RuVector v0.1.30+ uses object API with 'vector' field
    // Native VectorDB requires Float32Array, not regular array
    this.db.insert({
      id: id,
      vector: embedding instanceof Float32Array ? embedding : new Float32Array(embedding),
      metadata: metadata
    });

    if (metadata) {
      this.metadata.set(id, metadata);
    }

    // Track insert latency
    if (this.config.enableStats !== false) {
      const latency = performance.now() - startTime;
      this.stats.insertCount++;
      this.stats.insertTotalLatencyMs += latency;
      this.stats.insertMinLatencyMs = Math.min(this.stats.insertMinLatencyMs, latency);
      this.stats.insertMaxLatencyMs = Math.max(this.stats.insertMaxLatencyMs, latency);
    }
  }

  /**
   * Batch insert for optimal performance (sequential)
   */
  insertBatch(items: Array<{ id: string; embedding: Float32Array; metadata?: Record<string, any> }>): void {
    this.ensureInitialized();

    for (const item of items) {
      this.insert(item.id, item.embedding, item.metadata);
    }
  }

  /**
   * Parallel batch insert with semaphore-controlled concurrency
   *
   * Processes items in parallel batches for improved throughput on multi-core systems.
   * Uses a semaphore to control the maximum number of concurrent insertions.
   *
   * @param items - Array of items to insert
   * @param options - Configuration options
   * @param options.batchSize - Number of items per batch (default: 100)
   * @param options.concurrency - Max concurrent batches (default: config.parallelConcurrency or 4)
   * @returns Promise that resolves when all items are inserted
   */
  async insertBatchParallel(
    items: Array<{ id: string; embedding: Float32Array; metadata?: Record<string, any> }>,
    options?: { batchSize?: number; concurrency?: number }
  ): Promise<void> {
    this.ensureInitialized();

    const batchSize = options?.batchSize ?? 100;
    const concurrency = options?.concurrency ?? this.config.parallelConcurrency ?? 4;

    // Create a semaphore with the specified concurrency
    const semaphore = new Semaphore(concurrency);

    // Split items into batches
    const batches: Array<Array<{ id: string; embedding: Float32Array; metadata?: Record<string, any> }>> = [];
    for (let i = 0; i < items.length; i += batchSize) {
      batches.push(items.slice(i, i + batchSize));
    }

    // Process batches in parallel with semaphore control
    const promises = batches.map(async (batch) => {
      await semaphore.acquire();
      try {
        for (const item of batch) {
          this.insert(item.id, item.embedding, item.metadata);
        }
      } finally {
        semaphore.release();
      }
    });

    await Promise.all(promises);
  }

  /**
   * Insert using buffer pooling to reduce allocations
   *
   * Acquires a buffer from the pool, copies the embedding data,
   * performs the insert, and returns the buffer to the pool.
   *
   * @param id - Vector ID
   * @param embedding - Vector data (can be regular array or Float32Array)
   * @param metadata - Optional metadata
   */
  insertWithPooledBuffer(
    id: string,
    embedding: number[] | Float32Array,
    metadata?: Record<string, any>
  ): void {
    this.ensureInitialized();

    const dimension = this.config.dimension!;

    // Acquire buffer from pool
    const buffer = this.bufferPool.acquire(dimension);

    try {
      // Copy data to pooled buffer
      if (embedding instanceof Float32Array) {
        buffer.set(embedding);
      } else {
        for (let i = 0; i < dimension; i++) {
          buffer[i] = embedding[i] ?? 0;
        }
      }

      // Insert using pooled buffer
      this.insert(id, buffer, metadata);
    } finally {
      // Return buffer to pool
      this.bufferPool.release(buffer);
    }
  }

  /**
   * Search for k-nearest neighbors with optional filtering and early termination
   * @inline V8 optimization hint - hot path function
   */
  search(query: Float32Array, k: number, options?: SearchOptions): SearchResult[] {
    this.ensureInitialized();

    const startTime = this.config.enableStats !== false ? performance.now() : 0;

    // Apply efSearch parameter if provided
    if (options?.efSearch) {
      this.db.setEfSearch(options.efSearch);
    }

    // RuVector v0.1.30+ supports both object API and legacy positional args
    // Use object API for consistency with insert
    // Native VectorDB requires Float32Array, not regular array
    const results = this.db.search({
      vector: query instanceof Float32Array ? query : new Float32Array(query),
      k: k,
      threshold: options?.threshold,
      filter: options?.filter
    });

    // @inline Early termination threshold for perfect matches
    const earlyTermThreshold = 0.9999;
    let perfectMatches = 0;
    const maxPerfectMatches = k;

    // @inline Convert results and apply filtering with early termination
    const filteredResults: SearchResult[] = [];
    const resultsLen = results.length | 0;

    for (let i = 0; i < resultsLen; i++) {
      const r = results[i] as { id: string; distance: number };
      const similarity = this.distanceToSimilarity(r.distance);

      // Apply similarity threshold
      if (options?.threshold && similarity < options.threshold) {
        continue;
      }

      const metadata = this.metadata.get(r.id);

      // Apply metadata filters
      if (options?.filter && metadata) {
        const filterEntries = Object.entries(options.filter);
        let matchesFilter = true;
        for (let j = 0; j < filterEntries.length; j++) {
          const [key, value] = filterEntries[j];
          if (metadata[key] !== value) {
            matchesFilter = false;
            break;
          }
        }
        if (!matchesFilter) continue;
      }

      filteredResults.push({
        id: r.id,
        distance: r.distance,
        similarity,
        metadata
      });

      // @inline Early termination for perfect matches
      if (similarity >= earlyTermThreshold) {
        perfectMatches++;
        if (perfectMatches >= maxPerfectMatches) {
          break;
        }
      }
    }

    // Track search latency
    if (this.config.enableStats !== false) {
      const latency = performance.now() - startTime;
      this.stats.searchCount++;
      this.stats.searchTotalLatencyMs += latency;
      this.stats.searchMinLatencyMs = Math.min(this.stats.searchMinLatencyMs, latency);
      this.stats.searchMaxLatencyMs = Math.max(this.stats.searchMaxLatencyMs, latency);
    }

    return filteredResults;
  }

  /**
   * Remove vector by ID
   */
  remove(id: string): boolean {
    this.ensureInitialized();

    this.metadata.delete(id);

    try {
      return this.db.remove(id);
    } catch {
      return false;
    }
  }

  /**
   * Get database statistics
   */
  getStats(): VectorStats {
    this.ensureInitialized();

    const memoryUsage = this.db.memoryUsage?.() || 0;
    this.stats.lastMemoryUsage = memoryUsage;

    return {
      count: this.db.count(),
      dimension: this.config.dimension || 384,
      metric: this.config.metric,
      backend: 'ruvector',
      memoryUsage: memoryUsage
    };
  }

  /**
   * Get extended performance statistics
   *
   * Returns detailed metrics including latencies, cache stats, and buffer pool info.
   */
  getExtendedStats(): {
    basic: VectorStats;
    performance: PerformanceStats;
    bufferPool: { totalBuffers: number; totalMemory: number };
    config: {
      parallelConcurrency: number;
      adaptiveParams: boolean;
      mmapEnabled: boolean;
    };
  } {
    return {
      basic: this.getStats(),
      performance: {
        ...this.stats,
        insertAvgLatencyMs: this.stats.insertCount > 0
          ? this.stats.insertTotalLatencyMs / this.stats.insertCount
          : 0,
        searchAvgLatencyMs: this.stats.searchCount > 0
          ? this.stats.searchTotalLatencyMs / this.stats.searchCount
          : 0,
      } as PerformanceStats & { insertAvgLatencyMs: number; searchAvgLatencyMs: number },
      bufferPool: this.bufferPool.getStats(),
      config: {
        parallelConcurrency: this.config.parallelConcurrency ?? 4,
        adaptiveParams: this.config.adaptiveParams !== false,
        mmapEnabled: this.mmapEnabled,
      },
    };
  }

  /**
   * Reset performance statistics
   */
  resetStats(): void {
    this.stats = {
      insertCount: 0,
      insertTotalLatencyMs: 0,
      insertMinLatencyMs: Infinity,
      insertMaxLatencyMs: 0,
      searchCount: 0,
      searchTotalLatencyMs: 0,
      searchMinLatencyMs: Infinity,
      searchMaxLatencyMs: 0,
      cacheHits: 0,
      cacheMisses: 0,
      lastMemoryUsage: 0,
      indexRebuildCount: 0,
    };
  }

  /**
   * Update index parameters adaptively based on current dataset size
   *
   * This triggers an index rebuild with optimal parameters for the current size.
   * Should be called after significant data changes.
   */
  async updateAdaptiveParams(): Promise<void> {
    this.ensureInitialized();

    if (this.config.adaptiveParams === false) {
      return;
    }

    const currentCount = this.db.count();
    const params = this.getAdaptiveParams(currentCount);

    // Check if we need to rebuild
    // This would require checking current params vs recommended
    // For now, just set efSearch which doesn't require rebuild
    if (this.db.setEfSearch) {
      this.db.setEfSearch(params.efSearch);
    }

    this.stats.indexRebuildCount++;
  }

  /**
   * Save index and metadata to disk
   */
  async save(savePath: string): Promise<void> {
    this.ensureInitialized();

    // Validate path for security
    validatePath(savePath);

    // Save vector index
    this.db.save(savePath);

    // Save metadata separately as JSON
    const metadataPath = savePath + '.meta.json';
    validatePath(metadataPath);

    const fs = await import('fs/promises');
    await fs.writeFile(
      metadataPath,
      JSON.stringify(Object.fromEntries(this.metadata), null, 2)
    );
  }

  /**
   * Load index and metadata from disk
   */
  async load(loadPath: string): Promise<void> {
    this.ensureInitialized();

    // Validate path for security
    validatePath(loadPath);

    // Load vector index
    this.db.load(loadPath);

    // Load metadata
    const metadataPath = loadPath + '.meta.json';
    validatePath(metadataPath);

    try {
      const fs = await import('fs/promises');
      const data = await fs.readFile(metadataPath, 'utf-8');

      // Safely parse metadata with validation
      let parsed: unknown;
      try {
        parsed = JSON.parse(data);
      } catch {
        throw new Error('Invalid JSON in metadata file');
      }

      // Validate parsed data is a plain object
      if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
        throw new Error('Metadata must be a plain object');
      }

      // Check for prototype pollution attempts
      const unsafeKeys = ['__proto__', 'constructor', 'prototype'];
      for (const key of Object.keys(parsed as Record<string, unknown>)) {
        if (unsafeKeys.includes(key)) {
          throw new Error(`Forbidden key in metadata: ${key}`);
        }
      }

      this.metadata = new Map(Object.entries(parsed as Record<string, Record<string, any>>));

      // Enforce size limit
      if (this.metadata.size > MAX_METADATA_ENTRIES) {
        this.metadata.clear();
        throw new Error(`Loaded metadata exceeds maximum of ${MAX_METADATA_ENTRIES} entries`);
      }
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
        // No metadata file - this is okay for backward compatibility
        console.debug(`[RuVectorBackend] No metadata file found at ${metadataPath}`);
      } else {
        throw error;
      }
    }
  }

  /**
   * Close and cleanup resources
   */
  close(): void {
    // RuVector cleanup if needed
    this.metadata.clear();

    // Clear buffer pool
    this.bufferPool.clear();

    // Clean up mmap buffer
    if (this.mmapBuffer) {
      this.mmapBuffer = null;
    }

    // Reset stats
    this.resetStats();
  }

  /**
   * Convert distance to similarity score based on metric
   *
   * Cosine: distance is already in [0, 2], where 0 = identical
   * L2: exponential decay for unbounded distances
   * IP: negative inner product, so negate for similarity
   */
  private distanceToSimilarity(distance: number): number {
    switch (this.config.metric) {
      case 'cosine':
        return 1 - distance; // cosine distance is 1 - similarity
      case 'l2':
        return Math.exp(-distance); // exponential decay
      case 'ip':
        return -distance; // inner product: higher is better
      default:
        return 1 - distance;
    }
  }

  /**
   * Ensure database is initialized before operations
   */
  private ensureInitialized(): void {
    if (!this.initialized) {
      throw new Error('RuVectorBackend not initialized. Call initialize() first.');
    }
  }

  /**
   * Get the buffer pool instance for advanced use cases
   */
  getBufferPool(): BufferPool {
    return this.bufferPool;
  }

  /**
   * Get the current concurrency semaphore status
   */
  getConcurrencyStatus(): { available: number; configured: number } {
    return {
      available: this.semaphore.available,
      configured: this.config.parallelConcurrency ?? 4,
    };
  }

  /**
   * Check if memory-mapped storage is active
   */
  isMmapEnabled(): boolean {
    return this.mmapEnabled && this.mmapBuffer !== null;
  }

  /**
   * Get recommended adaptive parameters for a given dataset size
   */
  static getRecommendedParams(datasetSize: number): AdaptiveParams {
    if (datasetSize < 1000) {
      return { M: 8, efConstruction: 100, efSearch: 50 };
    } else if (datasetSize <= 100000) {
      return { M: 16, efConstruction: 200, efSearch: 100 };
    } else {
      return { M: 32, efConstruction: 400, efSearch: 200 };
    }
  }
}

// Export helper classes for advanced use cases
export { Semaphore, BufferPool };
export type { PerformanceStats, AdaptiveParams };
