/**
 * LLM Router Service - Multi-Provider LLM Integration
 *
 * Supports multiple LLM providers:
 * - RuvLLM (local, self-contained, SIMD-optimized, no external deps)
 * - OpenRouter (99% cost savings, 200+ models)
 * - Google Gemini (free tier available)
 * - Anthropic Claude (highest quality)
 * - ONNX (local models via transformers.js)
 *
 * Automatically selects optimal provider based on:
 * - Cost constraints
 * - Quality requirements
 * - Speed requirements
 * - Privacy requirements (local models via RuvLLM or ONNX)
 */
export interface LLMConfig {
    provider?: 'ruvllm' | 'openrouter' | 'gemini' | 'anthropic' | 'onnx';
    model?: string;
    temperature?: number;
    maxTokens?: number;
    apiKey?: string;
    priority?: 'quality' | 'balanced' | 'cost' | 'speed' | 'privacy';
    /** RuvLLM-specific: embedding dimension (384, 768, 1024) */
    embeddingDim?: number;
    /** RuvLLM-specific: enable adaptive learning */
    learningEnabled?: boolean;
}
export interface LLMResponse {
    content: string;
    tokensUsed: number;
    cost: number;
    provider: string;
    model: string;
    latencyMs: number;
}
export declare class LLMRouter {
    private config;
    private envLoaded;
    constructor(config?: LLMConfig);
    /**
     * Load environment variables from root .env file
     */
    private loadEnv;
    /**
     * Select default provider based on available API keys and installed packages
     */
    private selectDefaultProvider;
    private ruvllmAvailable;
    /**
     * Select default model for provider
     */
    private selectDefaultModel;
    /**
     * Get API key for provider from environment
     */
    private getApiKey;
    /**
     * Initialize async components (call after construction for RuvLLM support)
     */
    initialize(): Promise<void>;
    /**
     * Check if RuvLLM is available
     */
    isRuvLLMAvailable(): boolean;
    /**
     * Generate completion using configured provider
     */
    generate(prompt: string, options?: Partial<LLMConfig>): Promise<LLMResponse>;
    /**
     * Call RuvLLM for local inference (no API keys, no external services)
     *
     * Features:
     * - SIMD-optimized CPU inference
     * - SONA adaptive learning
     * - HNSW memory for context
     * - FastGRNN routing
     * - Zero cost, full privacy
     */
    private callRuvLLM;
    /**
     * Get embeddings using RuvLLM (768-dimensional by default)
     */
    getEmbedding(text: string): Promise<Float32Array | null>;
    /**
     * Compute similarity between two texts using RuvLLM
     */
    computeSimilarity(text1: string, text2: string): Promise<number | null>;
    /**
     * Add to RuvLLM's HNSW memory
     */
    addToMemory(content: string, metadata?: Record<string, unknown>): Promise<number | null>;
    /**
     * Search RuvLLM's HNSW memory
     */
    searchMemory(query: string, k?: number): Promise<any[] | null>;
    /**
     * Get RuvLLM statistics
     */
    getRuvLLMStats(): any | null;
    /**
     * Call OpenRouter API
     */
    private callOpenRouter;
    /**
     * Call Google Gemini API
     */
    private callGemini;
    /**
     * Call Anthropic API
     */
    private callAnthropic;
    /**
     * Generate local fallback response (simple template-based)
     */
    private generateLocalFallback;
    /**
     * Optimize model selection based on task priority
     */
    optimizeModelSelection(taskDescription: string, priority: 'quality' | 'balanced' | 'cost' | 'speed' | 'privacy'): LLMConfig;
    /**
     * Get current configuration
     */
    getConfig(): Required<LLMConfig>;
    /**
     * Check if provider is available (has API key or is local)
     */
    isProviderAvailable(provider: 'ruvllm' | 'openrouter' | 'gemini' | 'anthropic' | 'onnx'): boolean;
    /**
     * Get list of available providers
     */
    getAvailableProviders(): string[];
}
/**
 * Check if RuvLLM is available (static helper)
 */
export declare function isRuvLLMInstalled(): Promise<boolean>;
//# sourceMappingURL=LLMRouter.d.ts.map