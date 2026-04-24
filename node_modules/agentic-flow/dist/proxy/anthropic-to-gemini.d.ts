export declare class AnthropicToGeminiProxy {
    private app;
    private geminiApiKey;
    private geminiBaseUrl;
    private defaultModel;
    constructor(config: {
        geminiApiKey: string;
        geminiBaseUrl?: string;
        defaultModel?: string;
    });
    private setupMiddleware;
    private setupRoutes;
    private convertAnthropicToGemini;
    private parseStructuredCommands;
    private convertGeminiToAnthropic;
    private convertGeminiStreamToAnthropic;
    private mapFinishReason;
    start(port: number): void;
}
//# sourceMappingURL=anthropic-to-gemini.d.ts.map