// Multi-model router core implementation
import { readFileSync, existsSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';
import { OpenRouterProvider } from './providers/openrouter.js';
import { AnthropicProvider } from './providers/anthropic.js';
import { ONNXLocalProvider } from './providers/onnx-local.js';
import { GeminiProvider } from './providers/gemini.js';
export class ModelRouter {
    config;
    providers = new Map();
    metrics;
    constructor(configPath) {
        this.config = this.loadConfig(configPath);
        this.initializeProviders();
        this.metrics = this.initializeMetrics();
    }
    loadConfig(configPath) {
        const paths = [
            configPath,
            process.env.AGENTIC_FLOW_ROUTER_CONFIG,
            join(homedir(), '.agentic-flow', 'router.config.json'),
            join(process.cwd(), 'router.config.json'),
            join(process.cwd(), 'config', 'router.config.json'),
            join(process.cwd(), 'router.config.example.json')
        ].filter(Boolean);
        for (const path of paths) {
            if (existsSync(path)) {
                const content = readFileSync(path, 'utf-8');
                const config = JSON.parse(content);
                // Substitute environment variables
                return this.substituteEnvVars(config);
            }
        }
        // If no config file found, create config from environment variables
        return this.createConfigFromEnv();
    }
    createConfigFromEnv() {
        // Create minimal config from environment variables
        const config = {
            version: '1.0',
            defaultProvider: process.env.PROVIDER || 'anthropic',
            routing: { mode: 'manual' },
            providers: {}
        };
        // Add Anthropic if API key exists
        if (process.env.ANTHROPIC_API_KEY) {
            config.providers.anthropic = {
                apiKey: process.env.ANTHROPIC_API_KEY,
                baseUrl: process.env.ANTHROPIC_BASE_URL
            };
        }
        // Add OpenRouter if API key exists
        if (process.env.OPENROUTER_API_KEY) {
            config.providers.openrouter = {
                apiKey: process.env.OPENROUTER_API_KEY,
                baseUrl: process.env.OPENROUTER_BASE_URL
            };
        }
        // Add Gemini if API key exists
        if (process.env.GOOGLE_GEMINI_API_KEY) {
            config.providers.gemini = {
                apiKey: process.env.GOOGLE_GEMINI_API_KEY
            };
        }
        // ONNX is always available (no API key needed)
        config.providers.onnx = {
            modelPath: process.env.ONNX_MODEL_PATH,
            executionProviders: ['cpu']
        };
        return config;
    }
    substituteEnvVars(obj) {
        if (typeof obj === 'string') {
            // Replace ${VAR_NAME} with environment variable value
            return obj.replace(/\$\{([^}]+)\}/g, (_, key) => {
                const [varName, defaultValue] = key.split(':-');
                return process.env[varName] || defaultValue || '';
            });
        }
        if (Array.isArray(obj)) {
            return obj.map(item => this.substituteEnvVars(item));
        }
        if (obj && typeof obj === 'object') {
            const result = {};
            for (const [key, value] of Object.entries(obj)) {
                result[key] = this.substituteEnvVars(value);
            }
            return result;
        }
        return obj;
    }
    initializeProviders() {
        const verbose = process.env.ROUTER_VERBOSE === 'true';
        // Initialize Anthropic
        if (this.config.providers.anthropic) {
            try {
                const provider = new AnthropicProvider(this.config.providers.anthropic);
                this.providers.set('anthropic', provider);
                if (verbose)
                    console.log('✅ Anthropic provider initialized');
            }
            catch (error) {
                if (verbose)
                    console.error('❌ Failed to initialize Anthropic:', error);
            }
        }
        // Initialize OpenRouter
        if (this.config.providers.openrouter) {
            try {
                const provider = new OpenRouterProvider(this.config.providers.openrouter);
                this.providers.set('openrouter', provider);
                if (verbose)
                    console.log('✅ OpenRouter provider initialized');
            }
            catch (error) {
                if (verbose)
                    console.error('❌ Failed to initialize OpenRouter:', error);
            }
        }
        // Initialize ONNX Local
        if (this.config.providers.onnx) {
            try {
                const provider = new ONNXLocalProvider({
                    modelPath: this.config.providers.onnx.modelPath || './models/phi-4/cpu_and_mobile/cpu-int4-rtn-block-32-acc-level-4/model.onnx',
                    executionProviders: this.config.providers.onnx.executionProviders || ['cpu'],
                    maxTokens: this.config.providers.onnx.maxTokens || 100,
                    temperature: this.config.providers.onnx.temperature || 0.7
                });
                this.providers.set('onnx', provider);
                if (verbose)
                    console.log('✅ ONNX Local provider initialized');
            }
            catch (error) {
                if (verbose)
                    console.error('❌ Failed to initialize ONNX:', error);
            }
        }
        // Initialize Gemini
        if (this.config.providers.gemini) {
            try {
                const provider = new GeminiProvider(this.config.providers.gemini);
                this.providers.set('gemini', provider);
                if (verbose)
                    console.log('✅ Gemini provider initialized');
            }
            catch (error) {
                if (verbose)
                    console.error('❌ Failed to initialize Gemini:', error);
            }
        }
        // TODO: Initialize other providers (OpenAI, Ollama, LiteLLM)
        // Will be implemented in Phase 1
    }
    initializeMetrics() {
        return {
            totalRequests: 0,
            totalCost: 0,
            totalTokens: { input: 0, output: 0 },
            providerBreakdown: {},
            agentBreakdown: {}
        };
    }
    async chat(params, agentType) {
        const startTime = Date.now();
        const provider = await this.selectProvider(params, agentType);
        try {
            const response = await provider.chat(params);
            // Update metrics
            this.updateMetrics(provider.name, response, Date.now() - startTime, agentType);
            // Add metadata
            response.metadata = {
                ...response.metadata,
                provider: provider.name,
                latency: Date.now() - startTime
            };
            return response;
        }
        catch (error) {
            return this.handleProviderError(error, params, agentType);
        }
    }
    async *stream(params, agentType) {
        const provider = await this.selectProvider(params, agentType);
        if (!provider.stream) {
            throw new Error(`Provider ${provider.name} does not support streaming`);
        }
        try {
            const iterator = provider.stream(params);
            for await (const chunk of iterator) {
                yield chunk;
            }
        }
        catch (error) {
            throw error;
        }
    }
    async selectProvider(params, agentType) {
        // If provider is explicitly specified in params, use it
        if (params.provider) {
            const forcedProvider = this.providers.get(params.provider);
            if (forcedProvider) {
                return forcedProvider;
            }
            console.warn(`⚠️  Requested provider '${params.provider}' not available, falling back to routing logic`);
        }
        const routingMode = this.config.routing?.mode || 'manual';
        switch (routingMode) {
            case 'manual':
                return this.getDefaultProvider();
            case 'rule-based':
                return this.selectByRules(params, agentType);
            case 'cost-optimized':
                return this.selectByCost(params);
            case 'performance-optimized':
                return this.selectByPerformance(params);
            default:
                return this.getDefaultProvider();
        }
    }
    getDefaultProvider() {
        const provider = this.providers.get(this.config.defaultProvider);
        if (!provider) {
            throw new Error(`Default provider ${this.config.defaultProvider} not initialized`);
        }
        return provider;
    }
    selectByRules(params, agentType) {
        const rules = this.config.routing?.rules || [];
        for (const rule of rules) {
            if (this.matchesRule(rule.condition, params, agentType)) {
                const provider = this.providers.get(rule.action.provider);
                if (provider) {
                    console.log(`🎯 Routing via rule: ${rule.reason || 'matched condition'}`);
                    return provider;
                }
            }
        }
        return this.getDefaultProvider();
    }
    matchesRule(condition, params, agentType) {
        if (condition.agentType && agentType) {
            if (!condition.agentType.includes(agentType)) {
                return false;
            }
        }
        if (condition.requiresTools !== undefined) {
            if (condition.requiresTools && (!params.tools || params.tools.length === 0)) {
                return false;
            }
        }
        // TODO: Add more condition matching logic
        return true;
    }
    selectByCost(params) {
        // For now, prefer cheaper providers
        // TODO: Implement actual cost calculation
        const providerOrder = ['openrouter', 'anthropic', 'openai'];
        for (const providerType of providerOrder) {
            const provider = this.providers.get(providerType);
            if (provider) {
                console.log(`💰 Cost-optimized routing: selected ${provider.name}`);
                return provider;
            }
        }
        return this.getDefaultProvider();
    }
    selectByPerformance(params) {
        // For now, use metrics to select fastest provider
        let fastestProvider = null;
        let lowestLatency = Infinity;
        for (const [providerType, provider] of this.providers) {
            const breakdown = this.metrics.providerBreakdown[providerType];
            if (breakdown && breakdown.avgLatency < lowestLatency) {
                lowestLatency = breakdown.avgLatency;
                fastestProvider = provider;
            }
        }
        if (fastestProvider) {
            console.log(`⚡ Performance-optimized routing: selected ${fastestProvider.name}`);
            return fastestProvider;
        }
        return this.getDefaultProvider();
    }
    async handleProviderError(error, params, agentType) {
        console.error(`❌ Provider error from ${error.provider}:`, error.message);
        // Try fallback chain
        const fallbackChain = this.config.fallbackChain || [];
        for (const providerType of fallbackChain) {
            if (providerType === error.provider)
                continue; // Skip failed provider
            const provider = this.providers.get(providerType);
            if (provider) {
                console.log(`🔄 Falling back to ${provider.name}`);
                try {
                    return await provider.chat(params);
                }
                catch (fallbackError) {
                    console.error(`❌ Fallback provider ${provider.name} also failed`);
                    continue;
                }
            }
        }
        throw error; // No fallback succeeded
    }
    updateMetrics(providerName, response, latency, agentType) {
        this.metrics.totalRequests++;
        if (response.usage) {
            this.metrics.totalTokens.input += response.usage.inputTokens;
            this.metrics.totalTokens.output += response.usage.outputTokens;
        }
        if (response.metadata?.cost) {
            this.metrics.totalCost += response.metadata.cost;
        }
        // Provider breakdown
        if (!this.metrics.providerBreakdown[providerName]) {
            this.metrics.providerBreakdown[providerName] = {
                requests: 0,
                cost: 0,
                avgLatency: 0,
                errors: 0
            };
        }
        const breakdown = this.metrics.providerBreakdown[providerName];
        breakdown.requests++;
        breakdown.cost += response.metadata?.cost || 0;
        breakdown.avgLatency = (breakdown.avgLatency * (breakdown.requests - 1) + latency) / breakdown.requests;
        // Agent breakdown
        if (agentType) {
            if (!this.metrics.agentBreakdown) {
                this.metrics.agentBreakdown = {};
            }
            if (!this.metrics.agentBreakdown[agentType]) {
                this.metrics.agentBreakdown[agentType] = { requests: 0, cost: 0 };
            }
            this.metrics.agentBreakdown[agentType].requests++;
            this.metrics.agentBreakdown[agentType].cost += response.metadata?.cost || 0;
        }
    }
    getMetrics() {
        return { ...this.metrics };
    }
    getConfig() {
        return { ...this.config };
    }
    getProviders() {
        return new Map(this.providers);
    }
}
//# sourceMappingURL=router.js.map