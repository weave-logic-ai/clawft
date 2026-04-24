/**
 * ONNX Runtime Local Inference Provider for Phi-4
 *
 * Uses onnxruntime-node for true local CPU/GPU inference
 */
import type { LLMProvider, ChatParams, ChatResponse, StreamChunk } from '../types.js';
export interface ONNXLocalConfig {
    modelPath?: string;
    executionProviders?: string[];
    maxTokens?: number;
    temperature?: number;
}
export declare class ONNXLocalProvider implements LLMProvider {
    name: string;
    type: "custom";
    supportsStreaming: boolean;
    supportsTools: boolean;
    supportsMCP: boolean;
    private session;
    private config;
    private tokenizer;
    private tiktoken;
    constructor(config?: ONNXLocalConfig);
    /**
     * Load optimized tiktoken tokenizer (cl100k_base for Phi-4)
     */
    private loadTokenizer;
    /**
     * Encode text using tiktoken (fast BPE)
     */
    private encode;
    /**
     * Decode tokens using tiktoken
     */
    private decode;
    /**
     * Initialize ONNX session (with automatic model download)
     */
    private initializeSession;
    /**
     * Format messages for Phi-4 chat template
     */
    private formatMessages;
    /**
     * Initialize KV cache tensors for all 32 layers
     * Phi-4 architecture: 32 layers, 8 KV heads, 128 head_dim
     */
    private initializeKVCache;
    /**
     * Chat completion using ONNX with KV cache
     */
    chat(params: ChatParams): Promise<ChatResponse>;
    /**
     * Streaming not implemented (requires complex generation loop)
     */
    stream(params: ChatParams): AsyncGenerator<StreamChunk>;
    /**
     * Validate capabilities
     */
    validateCapabilities(features: string[]): boolean;
    /**
     * Get model info
     */
    getModelInfo(): {
        modelPath: string;
        executionProviders: string[];
        initialized: boolean;
        tokenizerLoaded: boolean;
    };
    /**
     * Cleanup resources
     */
    dispose(): Promise<void>;
}
//# sourceMappingURL=onnx-local.d.ts.map