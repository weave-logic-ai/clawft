/**
 * ONNX Runtime Provider for Local Model Inference
 *
 * Supports CPU and GPU execution providers for optimized local inference
 * Compatible with Phi-3, Llama, and other ONNX models
 */
import type { LLMProvider, ChatParams, ChatResponse, StreamChunk } from '../types.js';
export interface ONNXConfig {
    modelPath?: string;
    modelId?: string;
    executionProviders?: string[];
    sessionOptions?: any;
    maxTokens?: number;
    temperature?: number;
}
export declare class ONNXProvider implements LLMProvider {
    name: string;
    type: "custom";
    supportsStreaming: boolean;
    supportsTools: boolean;
    supportsMCP: boolean;
    private session;
    private generator;
    private config;
    private executionProviders;
    constructor(config?: ONNXConfig);
    /**
     * Detect available execution providers
     */
    private detectExecutionProviders;
    /**
     * Initialize ONNX session with model
     */
    private initializeSession;
    /**
     * Format messages for model input
     */
    private formatMessages;
    /**
     * Chat completion
     */
    chat(params: ChatParams): Promise<ChatResponse>;
    /**
     * Streaming generation
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
        modelId: string;
        executionProviders: string[];
        supportsGPU: boolean;
        initialized: boolean;
    };
    /**
     * Cleanup resources
     */
    dispose(): Promise<void>;
}
//# sourceMappingURL=onnx.d.ts.map