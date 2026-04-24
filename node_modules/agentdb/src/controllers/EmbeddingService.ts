/**
 * EmbeddingService - Text Embedding Generation
 *
 * Handles text-to-vector embedding generation using various models.
 * Supports both local (transformers.js) and remote (OpenAI, etc.) embeddings.
 */

export interface EmbeddingConfig {
  model: string;
  dimension: number;
  provider: 'transformers' | 'openai' | 'local';
  apiKey?: string;
}

export class EmbeddingService {
  private config: EmbeddingConfig;
  private pipeline: any; // transformers.js pipeline
  private cache: Map<string, Float32Array>;

  constructor(config: EmbeddingConfig) {
    this.config = config;
    this.cache = new Map();
  }

  /**
   * Initialize the embedding service
   */
  async initialize(): Promise<void> {
    if (this.config.provider === 'transformers') {
      // Use transformers.js for local embeddings
      try {
        const transformers = await import('@xenova/transformers');

        // Set Hugging Face token if available from environment
        const hfToken = process.env.HUGGINGFACE_API_KEY || process.env.HF_TOKEN;
        if (hfToken) {
          // Set the token for Transformers.js to use
          if (transformers.env && typeof transformers.env === 'object') {
            (transformers.env as any).HF_TOKEN = hfToken;
            console.log('🔑 Using Hugging Face API key from environment');
          }
        }

        this.pipeline = await transformers.pipeline('feature-extraction', this.config.model);
        console.log(`✅ Transformers.js loaded: ${this.config.model}`);
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        console.warn(`⚠️  Transformers.js initialization failed: ${errorMessage}`);
        console.warn('   Falling back to mock embeddings for testing');
        console.warn('   This is normal if:');
        console.warn('     • Running offline/without internet access');
        console.warn('     • Model not yet downloaded (~90MB on first use)');
        console.warn('     • Network connectivity issues');
        console.warn('   To use real embeddings:');
        console.warn('     • Ensure internet connectivity for first-time model download');
        console.warn('     • Or pre-download: npx agentdb install-embeddings');
        this.pipeline = null;
      }
    }
  }

  /**
   * Generate embedding for text
   */
  async embed(text: string): Promise<Float32Array> {
    // Check cache
    const cacheKey = `${this.config.model}:${text}`;
    if (this.cache.has(cacheKey)) {
      return this.cache.get(cacheKey)!;
    }

    let embedding: Float32Array;

    if (this.config.provider === 'transformers' && this.pipeline) {
      // Use transformers.js
      const output = await this.pipeline(text, { pooling: 'mean', normalize: true });
      embedding = new Float32Array(output.data);
    } else if (this.config.provider === 'openai' && this.config.apiKey) {
      // Use OpenAI API
      embedding = await this.embedOpenAI(text);
    } else {
      // Mock embedding for testing
      embedding = this.mockEmbedding(text);
    }

    // Cache result
    if (this.cache.size > 10000) {
      // Simple LRU: clear half the cache
      const keysToDelete = Array.from(this.cache.keys()).slice(0, 5000);
      keysToDelete.forEach(k => this.cache.delete(k));
    }
    this.cache.set(cacheKey, embedding);

    return embedding;
  }

  /**
   * Batch embed multiple texts
   */
  async embedBatch(texts: string[]): Promise<Float32Array[]> {
    return Promise.all(texts.map(text => this.embed(text)));
  }

  /**
   * Clear embedding cache
   */
  clearCache(): void {
    this.cache.clear();
  }

  // ========================================================================
  // Private Methods
  // ========================================================================

  private async embedOpenAI(text: string): Promise<Float32Array> {
    const response = await fetch('https://api.openai.com/v1/embeddings', {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.config.apiKey}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        model: this.config.model,
        input: text
      })
    });

    const data: any = await response.json();
    return new Float32Array(data.data[0].embedding);
  }

  private mockEmbedding(text: string): Float32Array {
    // Simple deterministic mock embedding for testing
    const embedding = new Float32Array(this.config.dimension);

    // Handle null/undefined/empty text
    if (!text || text.length === 0) {
      return new Array(this.config.dimension).fill(0) as any as Float32Array;
    }

    // Use simple hash-based generation
    let hash = 0;
    for (let i = 0; i < text.length; i++) {
      hash = ((hash << 5) - hash) + text.charCodeAt(i);
      hash = hash & hash; // Convert to 32bit integer
    }

    // Fill embedding with pseudo-random values based on hash
    for (let i = 0; i < this.config.dimension; i++) {
      const seed = hash + i * 31;
      embedding[i] = Math.sin(seed) * Math.cos(seed * 0.5);
    }

    // Normalize
    let norm = 0;
    for (let i = 0; i < embedding.length; i++) {
      norm += embedding[i] * embedding[i];
    }
    norm = Math.sqrt(norm);

    for (let i = 0; i < embedding.length; i++) {
      embedding[i] /= norm;
    }

    return embedding;
  }
}
