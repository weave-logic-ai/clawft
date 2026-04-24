import { ModelCapabilities } from '../utils/modelCapabilities.js';
export declare class AnthropicToRequestyProxy {
    private app;
    private requestyApiKey;
    private requestyBaseUrl;
    private defaultModel;
    private capabilities?;
    constructor(config: {
        requestyApiKey: string;
        requestyBaseUrl?: string;
        defaultModel?: string;
        capabilities?: ModelCapabilities;
    });
    private setupMiddleware;
    private setupRoutes;
    private handleRequest;
    private handleNativeRequest;
    private handleEmulatedRequest;
    private callRequesty;
    /**
     * Sanitize JSON Schema to be OpenAI-compatible
     * Fixes array properties without items, removes unsupported keywords
     */
    private sanitizeJsonSchema;
    private convertAnthropicToOpenAI;
    private parseStructuredCommands;
    private convertOpenAIToAnthropic;
    private convertOpenAIStreamToAnthropic;
    private extractProvider;
    private mapFinishReason;
    start(port: number): void;
}
//# sourceMappingURL=anthropic-to-requesty.d.ts.map