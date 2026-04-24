import { LLMProvider, ChatParams, ChatResponse, StreamChunk, ProviderConfig } from '../types.js';
export declare class AnthropicProvider implements LLMProvider {
    name: string;
    type: "anthropic";
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
//# sourceMappingURL=anthropic.d.ts.map