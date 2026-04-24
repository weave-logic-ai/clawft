/**
 * ONNX LLM Text Generation for RuVector
 *
 * Provides real local LLM inference using ONNX Runtime via transformers.js
 * Supports small models that run efficiently on CPU:
 * - SmolLM 135M - Smallest, fast (~135MB)
 * - SmolLM 360M - Better quality (~360MB)
 * - TinyLlama 1.1B - Best small model quality (~1GB quantized)
 * - Qwen2.5 0.5B - Good balance (~500MB)
 *
 * Features:
 * - Automatic model downloading and caching
 * - Quantized INT4/INT8 models for efficiency
 * - Streaming generation support
 * - Temperature, top-k, top-p sampling
 * - KV cache for efficient multi-turn conversations
 */
export interface OnnxLLMConfig {
    /** Model ID (default: 'Xenova/smollm-135m-instruct') */
    modelId?: string;
    /** Cache directory for models */
    cacheDir?: string;
    /** Use quantized model (default: true) */
    quantized?: boolean;
    /** Device: 'cpu' | 'webgpu' (default: 'cpu') */
    device?: 'cpu' | 'webgpu';
    /** Maximum context length */
    maxLength?: number;
}
export interface GenerationConfig {
    /** Maximum new tokens to generate (default: 128) */
    maxNewTokens?: number;
    /** Temperature for sampling (default: 0.7) */
    temperature?: number;
    /** Top-p nucleus sampling (default: 0.9) */
    topP?: number;
    /** Top-k sampling (default: 50) */
    topK?: number;
    /** Repetition penalty (default: 1.1) */
    repetitionPenalty?: number;
    /** Stop sequences */
    stopSequences?: string[];
    /** System prompt for chat models */
    systemPrompt?: string;
    /** Enable streaming (callback for each token) */
    onToken?: (token: string) => void;
}
export interface GenerationResult {
    /** Generated text */
    text: string;
    /** Number of tokens generated */
    tokensGenerated: number;
    /** Time taken in milliseconds */
    timeMs: number;
    /** Tokens per second */
    tokensPerSecond: number;
    /** Model used */
    model: string;
    /** Whether model was loaded from cache */
    cached: boolean;
}
export declare const AVAILABLE_MODELS: {
    readonly 'trm-tinystories': {
        readonly id: "Xenova/TinyStories-33M";
        readonly name: "TinyStories 33M (TRM)";
        readonly size: "~65MB";
        readonly description: "Ultra-tiny model for stories and basic generation";
        readonly contextLength: 512;
    };
    readonly 'trm-gpt2-tiny': {
        readonly id: "Xenova/gpt2";
        readonly name: "GPT-2 124M (TRM)";
        readonly size: "~250MB";
        readonly description: "Classic GPT-2 tiny for general text";
        readonly contextLength: 1024;
    };
    readonly 'trm-distilgpt2': {
        readonly id: "Xenova/distilgpt2";
        readonly name: "DistilGPT-2 (TRM)";
        readonly size: "~82MB";
        readonly description: "Distilled GPT-2, fastest general model";
        readonly contextLength: 1024;
    };
    readonly 'smollm-135m': {
        readonly id: "HuggingFaceTB/SmolLM-135M-Instruct";
        readonly name: "SmolLM 135M";
        readonly size: "~135MB";
        readonly description: "Smallest instruct model, very fast";
        readonly contextLength: 2048;
    };
    readonly 'smollm-360m': {
        readonly id: "HuggingFaceTB/SmolLM-360M-Instruct";
        readonly name: "SmolLM 360M";
        readonly size: "~360MB";
        readonly description: "Small model, fast, better quality";
        readonly contextLength: 2048;
    };
    readonly 'smollm2-135m': {
        readonly id: "HuggingFaceTB/SmolLM2-135M-Instruct";
        readonly name: "SmolLM2 135M";
        readonly size: "~135MB";
        readonly description: "Latest SmolLM v2, improved capabilities";
        readonly contextLength: 2048;
    };
    readonly 'smollm2-360m': {
        readonly id: "HuggingFaceTB/SmolLM2-360M-Instruct";
        readonly name: "SmolLM2 360M";
        readonly size: "~360MB";
        readonly description: "Latest SmolLM v2, better quality";
        readonly contextLength: 2048;
    };
    readonly 'qwen2.5-0.5b': {
        readonly id: "Qwen/Qwen2.5-0.5B-Instruct";
        readonly name: "Qwen2.5 0.5B";
        readonly size: "~300MB quantized";
        readonly description: "Good balance of speed and quality, multilingual";
        readonly contextLength: 4096;
    };
    readonly tinyllama: {
        readonly id: "TinyLlama/TinyLlama-1.1B-Chat-v1.0";
        readonly name: "TinyLlama 1.1B";
        readonly size: "~600MB quantized";
        readonly description: "Best small model quality, slower";
        readonly contextLength: 2048;
    };
    readonly 'codegemma-2b': {
        readonly id: "google/codegemma-2b";
        readonly name: "CodeGemma 2B";
        readonly size: "~1GB quantized";
        readonly description: "Code generation specialist";
        readonly contextLength: 8192;
    };
    readonly 'deepseek-coder-1.3b': {
        readonly id: "deepseek-ai/deepseek-coder-1.3b-instruct";
        readonly name: "DeepSeek Coder 1.3B";
        readonly size: "~700MB quantized";
        readonly description: "Excellent for code tasks";
        readonly contextLength: 4096;
    };
    readonly 'phi-2': {
        readonly id: "microsoft/phi-2";
        readonly name: "Phi-2 2.7B";
        readonly size: "~1.5GB quantized";
        readonly description: "High quality small model";
        readonly contextLength: 2048;
    };
    readonly 'phi-3-mini': {
        readonly id: "microsoft/Phi-3-mini-4k-instruct";
        readonly name: "Phi-3 Mini";
        readonly size: "~2GB quantized";
        readonly description: "Best quality tiny model";
        readonly contextLength: 4096;
    };
};
export type ModelKey = keyof typeof AVAILABLE_MODELS;
/**
 * Check if transformers.js is available
 */
export declare function isTransformersAvailable(): Promise<boolean>;
/**
 * Initialize the ONNX LLM with specified model
 */
export declare function initOnnxLLM(config?: OnnxLLMConfig): Promise<boolean>;
/**
 * Generate text using ONNX LLM
 */
export declare function generate(prompt: string, config?: GenerationConfig): Promise<GenerationResult>;
/**
 * Generate with streaming (token by token)
 */
export declare function generateStream(prompt: string, config?: GenerationConfig): Promise<AsyncGenerator<string, GenerationResult, undefined>>;
/**
 * Chat completion with conversation history
 */
export declare function chat(messages: Array<{
    role: 'system' | 'user' | 'assistant';
    content: string;
}>, config?: GenerationConfig): Promise<GenerationResult>;
/**
 * Get model information
 */
export declare function getModelInfo(): {
    model: string | null;
    ready: boolean;
    availableModels: typeof AVAILABLE_MODELS;
};
/**
 * Unload the current model to free memory
 */
export declare function unload(): Promise<void>;
export declare class OnnxLLM {
    private config;
    private initialized;
    constructor(config?: OnnxLLMConfig);
    init(): Promise<boolean>;
    generate(prompt: string, config?: GenerationConfig): Promise<GenerationResult>;
    chat(messages: Array<{
        role: 'system' | 'user' | 'assistant';
        content: string;
    }>, config?: GenerationConfig): Promise<GenerationResult>;
    unload(): Promise<void>;
    get ready(): boolean;
    get model(): string | null;
}
export default OnnxLLM;
//# sourceMappingURL=onnx-llm.d.ts.map