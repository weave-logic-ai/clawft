import { LLMProvider, RouterConfig, ChatParams, ChatResponse, StreamChunk, ProviderType, RouterMetrics } from './types.js';
export declare class ModelRouter {
    private config;
    private providers;
    private metrics;
    constructor(configPath?: string);
    private loadConfig;
    private createConfigFromEnv;
    private substituteEnvVars;
    private initializeProviders;
    private initializeMetrics;
    chat(params: ChatParams, agentType?: string): Promise<ChatResponse>;
    stream(params: ChatParams, agentType?: string): AsyncGenerator<StreamChunk>;
    private selectProvider;
    private getDefaultProvider;
    private selectByRules;
    private matchesRule;
    private selectByCost;
    private selectByPerformance;
    private handleProviderError;
    private updateMetrics;
    getMetrics(): RouterMetrics;
    getConfig(): RouterConfig;
    getProviders(): Map<ProviderType, LLMProvider>;
}
//# sourceMappingURL=router.d.ts.map