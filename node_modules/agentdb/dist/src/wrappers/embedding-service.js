/**
 * Production Embedding Service
 *
 * Replaces mock embeddings with real implementations:
 * 1. OpenAI Embeddings API (text-embedding-3-small/large)
 * 2. Local Transformers.js (runs in Node.js/browser)
 * 3. Custom ONNX models
 * 4. Fallback hash-based embeddings (for development)
 */
import { EventEmitter } from 'events';
/**
 * Base embedding service interface
 */
export class EmbeddingService extends EventEmitter {
    config;
    cache = new Map();
    constructor(config) {
        super();
        this.config = {
            cacheSize: 1000,
            ...config
        };
    }
    /**
     * Get cached embedding if available
     */
    getCached(text) {
        return this.cache.get(text) || null;
    }
    /**
     * Cache embedding with LRU eviction
     */
    setCached(text, embedding) {
        const cacheSize = this.config.cacheSize ?? 1000;
        if (this.cache.size >= cacheSize) {
            // Remove oldest entry (first in map)
            const firstKey = this.cache.keys().next().value;
            if (firstKey) {
                this.cache.delete(firstKey);
            }
        }
        this.cache.set(text, embedding);
    }
    /**
     * Clear cache
     */
    clearCache() {
        this.cache.clear();
    }
}
/**
 * OpenAI Embeddings Service
 *
 * Uses OpenAI's text-embedding-3-small (1536D) or text-embedding-3-large (3072D)
 * https://platform.openai.com/docs/guides/embeddings
 */
export class OpenAIEmbeddingService extends EmbeddingService {
    apiKey;
    model;
    baseURL = 'https://api.openai.com/v1/embeddings';
    constructor(config) {
        super({ ...config, provider: 'openai' });
        this.apiKey = config.apiKey;
        this.model = config.model || 'text-embedding-3-small';
    }
    async embed(text) {
        // Check cache
        const cached = this.getCached(text);
        if (cached) {
            return {
                embedding: cached,
                latency: 0
            };
        }
        const start = Date.now();
        try {
            const response = await fetch(this.baseURL, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    Authorization: `Bearer ${this.apiKey}`
                },
                body: JSON.stringify({
                    model: this.model,
                    input: text,
                    dimensions: this.config.dimensions || undefined
                })
            });
            if (!response.ok) {
                throw new Error(`OpenAI API error: ${response.statusText}`);
            }
            const data = await response.json();
            const embedding = data.data[0].embedding;
            // Cache it
            this.setCached(text, embedding);
            const latency = Date.now() - start;
            this.emit('embed', { text, latency });
            return {
                embedding,
                usage: data.usage,
                latency
            };
        }
        catch (error) {
            throw new Error(`OpenAI embedding failed: ${error.message}`);
        }
    }
    async embedBatch(texts) {
        const start = Date.now();
        try {
            const response = await fetch(this.baseURL, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    Authorization: `Bearer ${this.apiKey}`
                },
                body: JSON.stringify({
                    model: this.model,
                    input: texts,
                    dimensions: this.config.dimensions || undefined
                })
            });
            if (!response.ok) {
                throw new Error(`OpenAI API error: ${response.statusText}`);
            }
            const data = await response.json();
            const latency = Date.now() - start;
            return data.data.map((item, index) => {
                const embedding = item.embedding;
                this.setCached(texts[index], embedding);
                return {
                    embedding,
                    usage: {
                        promptTokens: Math.floor(data.usage.prompt_tokens / texts.length),
                        totalTokens: Math.floor(data.usage.total_tokens / texts.length)
                    },
                    latency: Math.floor(latency / texts.length)
                };
            });
        }
        catch (error) {
            throw new Error(`OpenAI batch embedding failed: ${error.message}`);
        }
    }
}
/**
 * Transformers.js Local Embedding Service
 *
 * Runs locally without API calls using ONNX runtime
 * https://huggingface.co/docs/transformers.js
 */
export class TransformersEmbeddingService extends EmbeddingService {
    pipeline = null;
    modelName;
    constructor(config) {
        super({ ...config, provider: 'transformers' });
        this.modelName = config.model || 'Xenova/all-MiniLM-L6-v2';
    }
    async initialize() {
        if (this.pipeline)
            return;
        try {
            // Dynamically import transformers.js
            const { pipeline } = await import('@xenova/transformers');
            this.pipeline = await pipeline('feature-extraction', this.modelName);
            this.emit('initialized', { model: this.modelName });
        }
        catch (error) {
            throw new Error(`Failed to initialize transformers.js: ${error.message}`);
        }
    }
    async embed(text) {
        await this.initialize();
        // Check cache
        const cached = this.getCached(text);
        if (cached) {
            return {
                embedding: cached,
                latency: 0
            };
        }
        const start = Date.now();
        try {
            const output = await this.pipeline(text, { pooling: 'mean', normalize: true });
            // Convert to regular array
            const embedding = Array.from(output.data);
            // Cache it
            this.setCached(text, embedding);
            const latency = Date.now() - start;
            this.emit('embed', { text, latency });
            return {
                embedding,
                latency
            };
        }
        catch (error) {
            throw new Error(`Transformers.js embedding failed: ${error.message}`);
        }
    }
    async embedBatch(texts) {
        await this.initialize();
        const start = Date.now();
        try {
            const results = [];
            for (const text of texts) {
                const cached = this.getCached(text);
                if (cached) {
                    results.push({
                        embedding: cached,
                        latency: 0
                    });
                }
                else {
                    const output = await this.pipeline(text, {
                        pooling: 'mean',
                        normalize: true
                    });
                    const embedding = Array.from(output.data);
                    this.setCached(text, embedding);
                    results.push({
                        embedding,
                        latency: Math.floor((Date.now() - start) / texts.length)
                    });
                }
            }
            return results;
        }
        catch (error) {
            throw new Error(`Transformers.js batch embedding failed: ${error.message}`);
        }
    }
}
/**
 * Mock Embedding Service (for development/testing)
 *
 * Generates deterministic hash-based embeddings
 * Fast but not semantically meaningful
 */
export class MockEmbeddingService extends EmbeddingService {
    constructor(config) {
        super({
            provider: 'mock',
            dimensions: 384,
            ...config
        });
    }
    async embed(text) {
        // Check cache
        const cached = this.getCached(text);
        if (cached) {
            return {
                embedding: cached,
                latency: 0
            };
        }
        const start = Date.now();
        // Generate hash-based embedding
        const embedding = this.hashEmbedding(text);
        // Cache it
        this.setCached(text, embedding);
        const latency = Date.now() - start;
        return {
            embedding,
            latency
        };
    }
    async embedBatch(texts) {
        return Promise.all(texts.map(text => this.embed(text)));
    }
    hashEmbedding(text) {
        const dimensions = this.config.dimensions || 384;
        const embedding = new Array(dimensions);
        // Seed with text hash
        let hash = 0;
        for (let i = 0; i < text.length; i++) {
            hash = (hash << 5) - hash + text.charCodeAt(i);
            hash = hash & hash;
        }
        // Generate pseudo-random embedding
        for (let i = 0; i < dimensions; i++) {
            const seed = hash + i * 2654435761;
            const x = Math.sin(seed) * 10000;
            embedding[i] = x - Math.floor(x);
        }
        // Normalize to unit vector
        const norm = Math.sqrt(embedding.reduce((sum, v) => sum + v * v, 0));
        return embedding.map(v => v / norm);
    }
}
/**
 * Factory function to create appropriate embedding service
 */
export function createEmbeddingService(config) {
    switch (config.provider) {
        case 'openai':
            if (!config.apiKey) {
                throw new Error('OpenAI API key required');
            }
            return new OpenAIEmbeddingService(config);
        case 'transformers':
            return new TransformersEmbeddingService(config);
        case 'mock':
            return new MockEmbeddingService(config);
        default:
            console.warn(`Unknown provider: ${config.provider}, using mock`);
            return new MockEmbeddingService(config);
    }
}
/**
 * Convenience function for quick embeddings
 */
export async function getEmbedding(text, config) {
    const service = createEmbeddingService({
        provider: 'mock',
        ...config
    });
    const result = await service.embed(text);
    return result.embedding;
}
/**
 * Benchmark different embedding providers
 */
export async function benchmarkEmbeddings(testText = 'Hello world') {
    const results = {};
    // Test mock
    const mockService = new MockEmbeddingService({ dimensions: 384 });
    const mockResult = await mockService.embed(testText);
    results.mock = {
        latency: mockResult.latency,
        dimensions: mockResult.embedding.length
    };
    // Test transformers (if available)
    try {
        const transformersService = new TransformersEmbeddingService({
            model: 'Xenova/all-MiniLM-L6-v2'
        });
        const transformersResult = await transformersService.embed(testText);
        results.transformers = {
            latency: transformersResult.latency,
            dimensions: transformersResult.embedding.length
        };
    }
    catch (error) {
        results.transformers = {
            error: error.message
        };
    }
    // Test OpenAI (if API key available)
    const apiKey = process.env.OPENAI_API_KEY;
    if (apiKey) {
        try {
            const openaiService = new OpenAIEmbeddingService({
                apiKey,
                model: 'text-embedding-3-small'
            });
            const openaiResult = await openaiService.embed(testText);
            results.openai = {
                latency: openaiResult.latency,
                dimensions: openaiResult.embedding.length
            };
        }
        catch (error) {
            results.openai = {
                error: error.message
            };
        }
    }
    return results;
}
//# sourceMappingURL=embedding-service.js.map