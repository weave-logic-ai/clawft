# Multi-Model Router Implementation Plan

## 🎯 Overview

Enable **agentic-flow** to work with alternative LLM providers beyond Anthropic, including OpenAI, OpenRouter, and local models (Ollama, LM Studio). This provides flexibility, cost optimization, and privacy options while maintaining the powerful Agent SDK capabilities.

## 📋 Research Summary

### Current State
- **Claude Agent SDK** v0.1.5 - Built for Anthropic models
- Supports Amazon Bedrock and Google Vertex AI routing
- MCP (Model Context Protocol) integration ready
- 111 MCP tools available

### Integration Patterns Discovered

1. **LiteLLM Approach**
   - Universal LLM gateway supporting 100+ providers
   - OpenAI-compatible API
   - Supports cloud, self-hosted, and local models

2. **OpenRouter Integration**
   - Access to 200+ models via single API
   - Intelligent routing and fallback
   - Cost optimization through model selection

3. **Direct Provider SDKs**
   - OpenAI SDK with compatibility layer
   - Direct Anthropic API (current)
   - Custom endpoint support for local models

## 🏗️ Architecture Design

### Model Router Layer

```
┌─────────────────────────────────────────┐
│         Agentic Flow CLI                │
│   (--provider, --model, --router-mode)  │
└────────────────┬────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────┐
│         Model Router Middleware          │
│  ┌─────────────────────────────────┐   │
│  │  Provider Abstraction Layer     │   │
│  │  - Normalize requests/responses │   │
│  │  - Handle streaming             │   │
│  │  - Tool calling translation     │   │
│  └─────────────────────────────────┘   │
└────────────────┬────────────────────────┘
                 │
      ┌──────────┴──────────┐
      ▼                     ▼
┌─────────────┐      ┌─────────────┐
│  LiteLLM    │      │  Direct     │
│  Gateway    │      │  Providers  │
└──────┬──────┘      └──────┬──────┘
       │                    │
   ┌───┴────┬───────────┬───┴────┐
   ▼        ▼           ▼        ▼
┌──────┐ ┌──────┐   ┌──────┐ ┌──────┐
│Claude│ │OpenAI│   │Ollama│ │Custom│
└──────┘ └──────┘   └──────┘ └──────┘
```

## 🔧 Implementation Plan

### Phase 1: Core Router Infrastructure (Week 1)

#### 1.1 Provider Abstraction Interface
```typescript
// src/router/types.ts
export interface LLMProvider {
  name: string;
  supportsStreaming: boolean;
  supportsTools: boolean;
  supportsMCP: boolean;

  chat(params: ChatParams): Promise<ChatResponse>;
  stream(params: ChatParams): AsyncIterator<StreamChunk>;
  validateCapabilities(features: string[]): boolean;
}

export interface ChatParams {
  model: string;
  messages: Message[];
  tools?: Tool[];
  temperature?: number;
  maxTokens?: number;
  stream?: boolean;
}
```

#### 1.2 Provider Implementations
- **AnthropicProvider** (current, refactor)
- **OpenAIProvider** (new)
- **OpenRouterProvider** (new)
- **LiteLLMProvider** (new, universal fallback)
- **OllamaProvider** (new, local models)

#### 1.3 Router Core
```typescript
// src/router/router.ts
export class ModelRouter {
  private providers: Map<string, LLMProvider>;
  private config: RouterConfig;

  async route(request: ChatParams): Promise<ChatResponse> {
    const provider = this.selectProvider(request);
    return provider.chat(request);
  }

  private selectProvider(request: ChatParams): LLMProvider {
    // Intelligent routing based on:
    // - Explicit provider selection
    // - Model availability
    // - Feature requirements
    // - Cost optimization
    // - Fallback chain
  }
}
```

### Phase 2: CLI Integration (Week 1-2)

#### 2.1 New CLI Options
```bash
# Provider selection
npx agentic-flow --provider openai --model gpt-4 --task "..."
npx agentic-flow --provider claude --model claude-sonnet-4-5 --task "..."
npx agentic-flow --provider ollama --model llama3 --task "..."
npx agentic-flow --provider openrouter --model "anthropic/claude-3-opus" --task "..."

# Router modes
npx agentic-flow --router-mode fallback --task "..."  # Try providers in order
npx agentic-flow --router-mode cost-optimized --task "..."  # Cheapest capable model
npx agentic-flow --router-mode local-first --task "..."  # Prefer local models

# Configuration file
npx agentic-flow --router-config ./router.config.json --task "..."
```

#### 2.2 CLI Implementation
```typescript
// src/cli/router.ts
export interface RouterCLIOptions {
  provider?: 'anthropic' | 'openai' | 'openrouter' | 'ollama' | 'litellm' | 'auto';
  model?: string;
  routerMode?: 'fallback' | 'cost-optimized' | 'local-first' | 'performance';
  routerConfig?: string;
  apiKey?: string; // Provider-specific API key
  baseUrl?: string; // For local models
}
```

### Phase 3: Configuration System (Week 2)

#### 3.1 Router Configuration File
```json
// router.config.json
{
  "version": "1.0",
  "defaultProvider": "anthropic",
  "fallbackChain": ["anthropic", "openai", "ollama"],

  "providers": {
    "anthropic": {
      "apiKey": "${ANTHROPIC_API_KEY}",
      "models": ["claude-sonnet-4-5", "claude-3-opus"],
      "defaultModel": "claude-sonnet-4-5",
      "maxRetries": 3
    },
    "openai": {
      "apiKey": "${OPENAI_API_KEY}",
      "models": ["gpt-4", "gpt-4-turbo", "gpt-3.5-turbo"],
      "defaultModel": "gpt-4-turbo",
      "organization": "${OPENAI_ORG_ID}"
    },
    "openrouter": {
      "apiKey": "${OPENROUTER_API_KEY}",
      "models": ["*"],
      "siteUrl": "https://agentic-flow.app",
      "siteName": "Agentic Flow"
    },
    "ollama": {
      "baseUrl": "http://localhost:11434",
      "models": ["llama3", "codellama", "mistral"],
      "defaultModel": "llama3"
    },
    "litellm": {
      "proxyUrl": "http://localhost:4000",
      "fallbackModels": ["gpt-3.5-turbo", "claude-3-haiku"]
    }
  },

  "routing": {
    "mode": "cost-optimized",
    "rules": [
      {
        "condition": "task.includes('code')",
        "provider": "anthropic",
        "model": "claude-sonnet-4-5"
      },
      {
        "condition": "task.length < 100",
        "provider": "openai",
        "model": "gpt-3.5-turbo"
      },
      {
        "condition": "privacy === true",
        "provider": "ollama",
        "model": "llama3"
      }
    ]
  },

  "features": {
    "streaming": true,
    "toolCalling": true,
    "mcpIntegration": true,
    "fallbackOnError": true,
    "costTracking": true
  }
}
```

#### 3.2 Environment Variables
```bash
# .env.router
# Anthropic
ANTHROPIC_API_KEY=sk-ant-...
CLAUDE_CODE_USE_BEDROCK=0
CLAUDE_CODE_USE_VERTEX=0

# OpenAI
OPENAI_API_KEY=sk-...
OPENAI_ORG_ID=org-...

# OpenRouter
OPENROUTER_API_KEY=sk-or-...

# Ollama
OLLAMA_BASE_URL=http://localhost:11434

# LiteLLM
LITELLM_PROXY_URL=http://localhost:4000

# Router
ROUTER_DEFAULT_PROVIDER=anthropic
ROUTER_MODE=fallback
ROUTER_CONFIG_PATH=./router.config.json
```

### Phase 4: Provider Implementations (Week 2-3)

#### 4.1 OpenAI Provider
```typescript
// src/router/providers/openai.ts
import OpenAI from 'openai';

export class OpenAIProvider implements LLMProvider {
  name = 'openai';
  supportsStreaming = true;
  supportsTools = true;
  supportsMCP = true; // Via adapter

  private client: OpenAI;

  async chat(params: ChatParams): Promise<ChatResponse> {
    const response = await this.client.chat.completions.create({
      model: params.model,
      messages: this.convertMessages(params.messages),
      tools: this.convertTools(params.tools),
      stream: false
    });

    return this.normalizeResponse(response);
  }

  private convertTools(tools?: Tool[]): any[] {
    // Convert MCP tools to OpenAI function calling format
  }

  private normalizeResponse(response: any): ChatResponse {
    // Normalize to standard format
  }
}
```

#### 4.2 OpenRouter Provider
```typescript
// src/router/providers/openrouter.ts
export class OpenRouterProvider implements LLMProvider {
  name = 'openrouter';
  private baseUrl = 'https://openrouter.ai/api/v1';

  async chat(params: ChatParams): Promise<ChatResponse> {
    const response = await fetch(`${this.baseUrl}/chat/completions`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.apiKey}`,
        'HTTP-Referer': this.config.siteUrl,
        'X-Title': this.config.siteName
      },
      body: JSON.stringify({
        model: params.model,
        messages: params.messages,
        // OpenRouter-specific params
        transforms: ['middle-out'],
        route: 'fallback'
      })
    });

    return this.normalizeResponse(await response.json());
  }
}
```

#### 4.3 Ollama Provider
```typescript
// src/router/providers/ollama.ts
export class OllamaProvider implements LLMProvider {
  name = 'ollama';
  private baseUrl: string;

  async chat(params: ChatParams): Promise<ChatResponse> {
    const response = await fetch(`${this.baseUrl}/api/chat`, {
      method: 'POST',
      body: JSON.stringify({
        model: params.model,
        messages: params.messages,
        stream: false,
        options: {
          temperature: params.temperature,
          num_predict: params.maxTokens
        }
      })
    });

    return this.normalizeResponse(await response.json());
  }

  async listModels(): Promise<string[]> {
    const response = await fetch(`${this.baseUrl}/api/tags`);
    const data = await response.json();
    return data.models.map((m: any) => m.name);
  }
}
```

#### 4.4 LiteLLM Universal Provider
```typescript
// src/router/providers/litellm.ts
export class LiteLLMProvider implements LLMProvider {
  name = 'litellm';

  async chat(params: ChatParams): Promise<ChatResponse> {
    // LiteLLM handles routing to 100+ providers
    const response = await fetch(`${this.proxyUrl}/chat/completions`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.apiKey}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        model: params.model, // e.g., "anthropic/claude-3-opus"
        messages: params.messages,
        stream: params.stream
      })
    });

    return this.normalizeResponse(await response.json());
  }
}
```

### Phase 5: Tool Calling Translation (Week 3)

#### 5.1 Tool Format Conversion
Different providers use different tool calling formats. Create adapters:

```typescript
// src/router/adapters/tool-adapter.ts
export class ToolAdapter {
  // MCP -> OpenAI function calling
  mcpToOpenAI(mcpTools: MCPTool[]): OpenAIFunction[] {
    return mcpTools.map(tool => ({
      type: 'function',
      function: {
        name: tool.name,
        description: tool.description,
        parameters: tool.inputSchema
      }
    }));
  }

  // MCP -> Anthropic tool use
  mcpToAnthropic(mcpTools: MCPTool[]): AnthropicTool[] {
    return mcpTools.map(tool => ({
      name: tool.name,
      description: tool.description,
      input_schema: tool.inputSchema
    }));
  }

  // Response normalization
  normalizeToolCall(response: any, provider: string): ToolCall[] {
    switch (provider) {
      case 'openai':
        return this.openAIToStandard(response);
      case 'anthropic':
        return this.anthropicToStandard(response);
      // ... other providers
    }
  }
}
```

### Phase 6: MCP Integration (Week 3-4)

#### 6.1 MCP Compatibility Layer
Ensure all 111 MCP tools work across providers:

```typescript
// src/router/mcp-bridge.ts
export class MCPBridge {
  async executeToolViaProvider(
    tool: MCPTool,
    params: any,
    provider: LLMProvider
  ): Promise<any> {
    // Convert tool to provider format
    const providerTool = this.adapter.convert(tool, provider.name);

    // Execute via provider's tool calling
    const response = await provider.chat({
      model: provider.defaultModel,
      messages: this.buildToolCallMessages(tool, params),
      tools: [providerTool]
    });

    // Extract and return tool result
    return this.extractToolResult(response);
  }
}
```

### Phase 7: Cost Optimization (Week 4)

#### 7.1 Cost Tracking
```typescript
// src/router/cost-tracker.ts
export class CostTracker {
  private costs: Map<string, number> = new Map();

  async trackRequest(
    provider: string,
    model: string,
    inputTokens: number,
    outputTokens: number
  ): Promise<void> {
    const cost = this.calculateCost(provider, model, inputTokens, outputTokens);
    this.costs.set(`${provider}:${model}`,
      (this.costs.get(`${provider}:${model}`) || 0) + cost
    );
  }

  private calculateCost(provider: string, model: string, input: number, output: number): number {
    const pricing = PRICING_TABLE[`${provider}:${model}`];
    return (input * pricing.inputPer1k / 1000) + (output * pricing.outputPer1k / 1000);
  }

  getReport(): CostReport {
    return {
      total: Array.from(this.costs.values()).reduce((a, b) => a + b, 0),
      byProvider: Object.fromEntries(this.costs),
      cheapestProvider: this.findCheapest()
    };
  }
}
```

#### 7.2 Cost-Optimized Routing
```typescript
// Router selects cheapest capable model
async selectCostOptimizedProvider(params: ChatParams): Promise<LLMProvider> {
  const capabilities = this.analyzeRequirements(params);
  const capableProviders = this.providers.filter(p =>
    p.validateCapabilities(capabilities)
  );

  return capableProviders.reduce((cheapest, current) => {
    const cheapestCost = this.estimateCost(cheapest, params);
    const currentCost = this.estimateCost(current, params);
    return currentCost < cheapestCost ? current : cheapest;
  });
}
```

## 📦 Dependencies

### New Packages Required
```json
{
  "dependencies": {
    "openai": "^4.65.0",              // OpenAI SDK
    "litellm-node": "^1.0.0",         // LiteLLM client (if available)
    "axios": "^1.7.0",                // HTTP client for custom endpoints
    "zod": "^3.25.76"                 // Already included - schema validation
  }
}
```

### Optional Dependencies
```json
{
  "optionalDependencies": {
    "@google-cloud/vertex-ai": "^1.0.0",  // Already supported
    "@aws-sdk/client-bedrock": "^3.0.0"   // Already supported
  }
}
```

## 🧪 Testing Strategy

### Unit Tests
- Provider abstraction compliance
- Tool format conversion accuracy
- Response normalization
- Cost calculation accuracy

### Integration Tests
- End-to-end with each provider
- Fallback chain validation
- Streaming with different providers
- Tool calling across providers

### Performance Tests
- Latency comparison
- Token usage optimization
- Cost tracking accuracy

## 📊 Success Metrics

| Metric | Target |
|--------|--------|
| Provider Coverage | 5+ (Anthropic, OpenAI, OpenRouter, Ollama, Custom) |
| Tool Compatibility | 100% (all 111 MCP tools) |
| Fallback Success Rate | >99% |
| Cost Reduction | 30-50% (with optimization) |
| Local Model Support | Yes (Ollama, LM Studio) |
| Performance Overhead | <10ms routing latency |

## 🚀 Deployment Options

### 1. Default (Anthropic Only)
```bash
npx agentic-flow --task "Build API"
# Uses Claude Sonnet 4.5 by default
```

### 2. Cost-Optimized
```bash
npx agentic-flow --router-mode cost-optimized --task "Simple analysis"
# Automatically selects cheapest capable model
```

### 3. Local Privacy
```bash
npx agentic-flow --provider ollama --model llama3 --task "Analyze sensitive data"
# All processing stays local
```

### 4. Best Performance
```bash
npx agentic-flow --provider openrouter --model "anthropic/claude-3-opus" --task "Complex reasoning"
# Routes to most capable model via OpenRouter
```

### 5. Hybrid (Recommended)
```bash
npx agentic-flow --router-config ./router.config.json --task "Multi-step task"
# Intelligent routing:
# - Simple tasks -> GPT-3.5 (cheap)
# - Code tasks -> Claude Sonnet 4.5 (best coding)
# - Privacy tasks -> Ollama (local)
```

## 📝 Documentation Deliverables

1. ✅ **This Plan** - `MULTI_MODEL_ROUTER_PLAN.md`
2. **User Guide** - `ROUTER_USER_GUIDE.md`
3. **Configuration Reference** - `ROUTER_CONFIG_REFERENCE.md`
4. **Provider Setup Guides** - `providers/` directory
5. **Migration Guide** - `MIGRATION_TO_ROUTER.md`
6. **Cost Optimization Guide** - `COST_OPTIMIZATION.md`

## 🎯 Next Steps

1. **Week 1**: Core router infrastructure + CLI integration
2. **Week 2**: Provider implementations (OpenAI, OpenRouter, Ollama)
3. **Week 3**: Tool calling translation + MCP bridge
4. **Week 4**: Cost optimization + testing + documentation
5. **Week 5**: Beta release + user feedback
6. **Week 6**: Production release v2.0.0

## 💡 Future Enhancements

- **Auto-scaling**: Automatically scale to cloud providers under load
- **A/B Testing**: Compare model performance on same tasks
- **Model Fine-tuning**: Support for custom fine-tuned models
- **Prompt Optimization**: Auto-optimize prompts per provider
- **Analytics Dashboard**: Real-time cost and performance metrics

---

**Status**: Planning Complete ✅
**Next**: Implementation Phase 1 - Core Router Infrastructure
**Timeline**: 6 weeks to production release
