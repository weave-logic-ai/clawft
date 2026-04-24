/**
 * ONNX Runtime Provider for Phi-4 Model
 *
 * Hybrid implementation with fallback to HuggingFace Inference API
 * when local ONNX model is not available
 */
import type { LLMProvider, ChatParams, ChatResponse, StreamChunk } from '../types.js';
export interface ONNXPhi4Config {
    modelId?: string;
    useLocalONNX?: boolean;
    huggingfaceApiKey?: string;
    maxTokens?: number;
    temperature?: number;
}
export declare class ONNXPhi4Provider implements LLMProvider {
    name: string;
    type: "custom";
    supportsStreaming: boolean;
    supportsTools: boolean;
    supportsMCP: boolean;
    private config;
    private hf;
    private modelPath;
    constructor(config?: ONNXPhi4Config);
    /**
     * Format messages for Phi-4 chat template
     */
    private formatMessages;
    /**
     * Chat completion via HuggingFace Inference API
     */
    private chatViaAPI;
    /**
     * Chat completion via local ONNX (not yet implemented)
     */
    private chatViaONNX;
    /**
     * Chat completion (uses API or local ONNX based on config)
     */
    chat(params: ChatParams): Promise<ChatResponse>;
    /**
     * Streaming generation via HuggingFace API
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
        mode: string;
        supportsLocalInference: boolean;
        modelPath: string;
        apiKey: string;
    };
    /**
     * Switch between API and local ONNX
     */
    setMode(useLocalONNX: boolean): void;
}
//# sourceMappingURL=onnx-phi4.d.ts.map