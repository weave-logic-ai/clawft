import { ModelCapabilities } from '../utils/modelCapabilities.js';
export declare class AnthropicToOpenRouterProxy {
    private app;
    private openrouterApiKey;
    private openrouterBaseUrl;
    private defaultModel;
    private capabilities?;
    constructor(config: {
        openrouterApiKey: string;
        openrouterBaseUrl?: string;
        defaultModel?: string;
        capabilities?: ModelCapabilities;
    });
    private setupMiddleware;
    private setupRoutes;
    private handleRequest;
    private handleNativeRequest;
    private handleEmulatedRequest;
    private callOpenRouter;
    private convertAnthropicToOpenAI;
    private parseStructuredCommands;
    private convertOpenAIToAnthropic;
    private convertOpenAIStreamToAnthropic;
    private extractProvider;
    private mapFinishReason;
    start(port: number): void;
}
//# sourceMappingURL=anthropic-to-openrouter.d.ts.map