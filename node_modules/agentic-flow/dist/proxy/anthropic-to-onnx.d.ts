export declare class AnthropicToONNXProxy {
    private app;
    private onnxProvider;
    private port;
    private server;
    constructor(config?: {
        port?: number;
        modelPath?: string;
        executionProviders?: string[];
    });
    private setupMiddleware;
    private setupRoutes;
    start(): Promise<void>;
    stop(): Promise<void>;
    dispose(): Promise<void>;
}
//# sourceMappingURL=anthropic-to-onnx.d.ts.map