// Anthropic provider implementation
import Anthropic from '@anthropic-ai/sdk';
export class AnthropicProvider {
    name = 'anthropic';
    type = 'anthropic';
    supportsStreaming = true;
    supportsTools = true;
    supportsMCP = true; // Native support
    client;
    config;
    constructor(config) {
        this.config = config;
        if (!config.apiKey) {
            throw new Error('Anthropic API key is required');
        }
        this.client = new Anthropic({
            apiKey: config.apiKey,
            baseURL: config.baseUrl,
            timeout: config.timeout || 120000,
            maxRetries: config.maxRetries || 3
        });
    }
    validateCapabilities(features) {
        const supported = ['chat', 'streaming', 'tools', 'mcp'];
        return features.every(f => supported.includes(f));
    }
    async chat(params) {
        try {
            // Extract system message if present (Anthropic requires it as top-level parameter)
            const systemMessage = params.messages.find(m => m.role === 'system');
            const nonSystemMessages = params.messages.filter(m => m.role !== 'system');
            const response = await this.client.messages.create({
                model: params.model,
                messages: nonSystemMessages,
                system: systemMessage ? (typeof systemMessage.content === 'string' ? systemMessage.content : JSON.stringify(systemMessage.content)) : undefined,
                temperature: params.temperature,
                max_tokens: params.maxTokens || 4096,
                tools: params.tools,
                tool_choice: params.toolChoice
            });
            return {
                id: response.id,
                model: response.model,
                content: response.content,
                stopReason: response.stop_reason,
                usage: {
                    inputTokens: response.usage.input_tokens,
                    outputTokens: response.usage.output_tokens
                },
                metadata: {
                    provider: 'anthropic',
                    cost: this.calculateCost(response.usage),
                    latency: 0 // Will be set by router
                }
            };
        }
        catch (error) {
            throw this.handleError(error);
        }
    }
    async *stream(params) {
        try {
            // Extract system message if present (Anthropic requires it as top-level parameter)
            const systemMessage = params.messages.find(m => m.role === 'system');
            const nonSystemMessages = params.messages.filter(m => m.role !== 'system');
            const stream = await this.client.messages.create({
                model: params.model,
                messages: nonSystemMessages,
                system: systemMessage ? (typeof systemMessage.content === 'string' ? systemMessage.content : JSON.stringify(systemMessage.content)) : undefined,
                temperature: params.temperature,
                max_tokens: params.maxTokens || 4096,
                tools: params.tools,
                tool_choice: params.toolChoice,
                stream: true
            });
            for await (const event of stream) {
                yield event;
            }
        }
        catch (error) {
            throw this.handleError(error);
        }
    }
    calculateCost(usage) {
        // Claude 3.5 Sonnet pricing: $3/MTok input, $15/MTok output
        const inputCost = (usage.input_tokens / 1_000_000) * 3;
        const outputCost = (usage.output_tokens / 1_000_000) * 15;
        return inputCost + outputCost;
    }
    handleError(error) {
        const providerError = new Error(error.message || 'Anthropic request failed');
        providerError.provider = 'anthropic';
        providerError.statusCode = error.status;
        providerError.retryable = error.status >= 500 || error.status === 429;
        return providerError;
    }
}
//# sourceMappingURL=anthropic.js.map