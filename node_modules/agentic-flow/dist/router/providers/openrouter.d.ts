import { LLMProvider, ChatParams, ChatResponse, StreamChunk, ProviderConfig } from '../types.js';
export declare class OpenRouterProvider implements LLMProvider {
    name: string;
    type: "openrouter";
    supportsStreaming: boolean;
    supportsTools: boolean;
    supportsMCP: boolean;
    private client;
    private config;
    constructor(config: ProviderConfig);
    validateCapabilities(features: string[]): boolean;
    chat(params: ChatParams): Promise<ChatResponse>;
    stream(params: ChatParams): AsyncGenerator<StreamChunk>;
    private formatRequest;
    private formatResponse;
    private formatStreamChunk;
    private mapFinishReason;
    private calculateCost;
    private handleError;
}
//# sourceMappingURL=openrouter.d.ts.map