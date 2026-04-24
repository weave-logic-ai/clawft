export interface LLMProvider {
    name: string;
    type: ProviderType;
    supportsStreaming: boolean;
    supportsTools: boolean;
    supportsMCP: boolean;
    chat(params: ChatParams): Promise<ChatResponse>;
    stream?(params: ChatParams): AsyncGenerator<StreamChunk>;
    validateCapabilities(features: string[]): boolean;
}
export type ProviderType = 'anthropic' | 'openai' | 'openrouter' | 'ollama' | 'litellm' | 'onnx' | 'gemini' | 'custom';
export interface ChatParams {
    model: string;
    messages: Message[];
    temperature?: number;
    maxTokens?: number;
    tools?: Tool[];
    toolChoice?: 'auto' | 'any' | 'none' | {
        type: 'tool';
        name: string;
    };
    stream?: boolean;
    metadata?: Record<string, any>;
    provider?: string;
}
export interface Message {
    role: 'user' | 'assistant' | 'system';
    content: string | ContentBlock[];
}
export interface ContentBlock {
    type: 'text' | 'tool_use' | 'tool_result';
    text?: string;
    id?: string;
    name?: string;
    input?: any;
    content?: any;
    is_error?: boolean;
}
export interface Tool {
    name: string;
    description: string;
    input_schema: {
        type: 'object';
        properties: Record<string, any>;
        required?: string[];
    };
}
export interface ChatResponse {
    id: string;
    model: string;
    content: ContentBlock[];
    stopReason?: 'end_turn' | 'max_tokens' | 'tool_use' | 'stop_sequence';
    usage?: {
        inputTokens: number;
        outputTokens: number;
    };
    metadata?: {
        provider: string;
        model?: string;
        cost?: number;
        latency?: number;
        executionProviders?: string[];
        [key: string]: any;
    };
}
export interface StreamChunk {
    type: 'content_block_start' | 'content_block_delta' | 'content_block_stop' | 'message_start' | 'message_delta' | 'message_stop';
    delta?: {
        type: 'text_delta' | 'input_json_delta';
        text?: string;
        partial_json?: string;
    };
    content_block?: ContentBlock;
    message?: Partial<ChatResponse>;
    usage?: {
        inputTokens: number;
        outputTokens: number;
    };
}
export interface ProviderConfig {
    apiKey?: string;
    baseUrl?: string;
    organization?: string;
    models?: {
        default: string;
        fast?: string;
        advanced?: string;
        [key: string]: string | undefined;
    };
    timeout?: number;
    maxRetries?: number;
    retryDelay?: number;
    rateLimit?: {
        requestsPerMinute?: number;
        tokensPerMinute?: number;
    };
    preferences?: Record<string, any>;
    modelPath?: string;
    executionProviders?: string[];
    maxTokens?: number;
    temperature?: number;
    localInference?: boolean;
    gpuAcceleration?: boolean;
}
export interface RouterConfig {
    version: string;
    defaultProvider: ProviderType;
    fallbackChain?: ProviderType[];
    providers: Record<ProviderType, ProviderConfig>;
    routing?: RoutingConfig;
    toolCalling?: ToolCallingConfig;
    monitoring?: MonitoringConfig;
    cache?: CacheConfig;
}
export interface RoutingConfig {
    mode: 'manual' | 'cost-optimized' | 'performance-optimized' | 'quality-optimized' | 'rule-based';
    rules?: RoutingRule[];
    costOptimization?: {
        enabled: boolean;
        maxCostPerRequest?: number;
        budgetAlerts?: {
            daily?: number;
            monthly?: number;
        };
        preferCheaper?: boolean;
        costThreshold?: number;
    };
    performance?: {
        timeout?: number;
        concurrentRequests?: number;
        circuitBreaker?: {
            enabled: boolean;
            threshold: number;
            timeout: number;
            resetTimeout?: number;
        };
    };
}
export interface RoutingRule {
    condition: {
        agentType?: string[];
        requiresTools?: boolean;
        complexity?: 'low' | 'medium' | 'high';
        privacy?: 'low' | 'medium' | 'high';
        localOnly?: boolean;
        requiresReasoning?: boolean;
    };
    action: {
        provider: ProviderType;
        model: string;
        temperature?: number;
        maxTokens?: number;
    };
    reason?: string;
}
export interface ToolCallingConfig {
    translationEnabled: boolean;
    defaultFormat: 'anthropic' | 'openai' | 'auto-detect';
    formatMapping?: Record<string, string>;
    fallbackStrategy?: 'disable-tools' | 'use-text' | 'fail';
}
export interface MonitoringConfig {
    enabled: boolean;
    logLevel: 'debug' | 'info' | 'warn' | 'error';
    metrics?: {
        trackCost?: boolean;
        trackLatency?: boolean;
        trackTokens?: boolean;
        trackErrors?: boolean;
    };
    alerts?: {
        costThreshold?: number;
        errorRate?: number;
        latencyThreshold?: number;
    };
}
export interface CacheConfig {
    enabled: boolean;
    ttl?: number;
    maxSize?: number;
    strategy?: 'lru' | 'fifo' | 'lfu';
    providers?: Record<string, {
        ttl?: number;
    }>;
}
export interface RouterMetrics {
    totalRequests: number;
    totalCost: number;
    totalTokens: {
        input: number;
        output: number;
    };
    providerBreakdown: Record<string, {
        requests: number;
        cost: number;
        avgLatency: number;
        errors: number;
    }>;
    agentBreakdown?: Record<string, {
        requests: number;
        cost: number;
    }>;
}
export interface ProviderError extends Error {
    provider: string;
    statusCode?: number;
    retryable: boolean;
}
//# sourceMappingURL=types.d.ts.map