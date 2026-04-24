import { LLMProvider, ChatParams, ChatResponse, StreamChunk, ProviderConfig } from '../types.js';
export declare class GeminiProvider implements LLMProvider {
    name: string;
    type: "gemini";
    supportsStreaming: boolean;
    supportsTools: boolean;
    supportsMCP: boolean;
    private client;
    private config;
    constructor(config: ProviderConfig);
    validateCapabilities(features: string[]): boolean;
    chat(params: ChatParams): Promise<ChatResponse>;
    stream(params: ChatParams): AsyncGenerator<StreamChunk>;
    private calculateCost;
    private handleError;
}
//# sourceMappingURL=gemini.d.ts.map