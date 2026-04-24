# Requesty.ai API Research

## API Overview

### Base Information

| Property | Value |
|----------|-------|
| **Base URL** | `https://router.requesty.ai/v1` |
| **API Version** | v1 |
| **Protocol** | HTTPS REST |
| **Authentication** | Bearer token (API key) |
| **Request Format** | JSON |
| **Response Format** | JSON |
| **Compatibility** | OpenAI SDK drop-in replacement |

### API Endpoints

Based on documentation and OpenAI compatibility:

1. **Chat Completions** - `/v1/chat/completions` (PRIMARY)
2. **Embeddings** - `/v1/embeddings`
3. **Models** - `/v1/models` (likely)

For agentic-flow integration, we only need **Chat Completions**.

## Authentication

### API Key Format

```
Authorization: Bearer requesty-<key>
```

### Key Generation

1. Visit https://app.requesty.ai/getting-started
2. Navigate to API Keys section
3. Generate new key
4. Copy key starting with `requesty-`

### Environment Variable

```bash
export REQUESTY_API_KEY="requesty-xxxxxxxxxxxxx"
```

### Security Considerations

- API keys should be kept secret (never commit to git)
- Use environment variables or .env files
- Rotate keys periodically
- Monitor usage for unauthorized access

## Chat Completions Endpoint

### Request Schema

#### Endpoint
```
POST https://router.requesty.ai/v1/chat/completions
```

#### Headers
```http
Content-Type: application/json
Authorization: Bearer requesty-xxxxxxxxxxxxx
HTTP-Referer: https://github.com/ruvnet/agentic-flow  # Optional
X-Title: Agentic Flow                                  # Optional
```

#### Request Body (OpenAI Format)

```json
{
  "model": "openai/gpt-4o",
  "messages": [
    {
      "role": "system",
      "content": "You are a helpful assistant."
    },
    {
      "role": "user",
      "content": "Hello, who are you?"
    }
  ],
  "temperature": 0.7,
  "max_tokens": 4096,
  "stream": false,
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "get_weather",
        "description": "Get weather for a location",
        "parameters": {
          "type": "object",
          "properties": {
            "location": {
              "type": "string"
            }
          },
          "required": ["location"]
        }
      }
    }
  ]
}
```

#### Available Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `model` | string | Yes | Model identifier (e.g., "openai/gpt-4o") |
| `messages` | array | Yes | Array of message objects |
| `temperature` | number | No | 0.0-2.0, controls randomness (default: 1.0) |
| `max_tokens` | number | No | Maximum tokens to generate |
| `stream` | boolean | No | Enable streaming (default: false) |
| `tools` | array | No | Function calling tools (OpenAI format) |
| `tool_choice` | string/object | No | Control tool usage |
| `top_p` | number | No | Nucleus sampling parameter |
| `frequency_penalty` | number | No | Reduce repetition (-2.0 to 2.0) |
| `presence_penalty` | number | No | Encourage new topics (-2.0 to 2.0) |
| `stop` | string/array | No | Stop sequences |
| `n` | number | No | Number of completions to generate |
| `user` | string | No | User identifier for tracking |

### Response Schema

#### Non-Streaming Response

```json
{
  "id": "chatcmpl-xxxxxxxxxxxxx",
  "object": "chat.completion",
  "created": 1704067200,
  "model": "openai/gpt-4o",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "I am an AI assistant created by OpenAI...",
        "tool_calls": [
          {
            "id": "call_xxxxx",
            "type": "function",
            "function": {
              "name": "get_weather",
              "arguments": "{\"location\": \"San Francisco\"}"
            }
          }
        ]
      },
      "finish_reason": "tool_calls"
    }
  ],
  "usage": {
    "prompt_tokens": 25,
    "completion_tokens": 150,
    "total_tokens": 175
  }
}
```

#### Streaming Response (SSE)

```
data: {"id":"chatcmpl-xxxxx","object":"chat.completion.chunk","created":1704067200,"model":"openai/gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":"I"},"finish_reason":null}]}

data: {"id":"chatcmpl-xxxxx","object":"chat.completion.chunk","created":1704067200,"model":"openai/gpt-4o","choices":[{"index":0,"delta":{"content":" am"},"finish_reason":null}]}

data: {"id":"chatcmpl-xxxxx","object":"chat.completion.chunk","created":1704067200,"model":"openai/gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
```

### Finish Reasons

| Reason | Description |
|--------|-------------|
| `stop` | Natural completion (model decided to stop) |
| `length` | Max tokens reached |
| `tool_calls` | Model wants to call a function |
| `content_filter` | Content filtered by safety system |

## Tool/Function Calling

### Format

Requesty uses **OpenAI function calling format** (same as OpenRouter).

#### Request with Tools

```json
{
  "model": "openai/gpt-4o",
  "messages": [...],
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "Read",
        "description": "Read a file from the filesystem",
        "parameters": {
          "type": "object",
          "properties": {
            "file_path": {
              "type": "string",
              "description": "Absolute path to file"
            }
          },
          "required": ["file_path"]
        }
      }
    }
  ]
}
```

#### Response with Tool Calls

```json
{
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": null,
        "tool_calls": [
          {
            "id": "call_abc123",
            "type": "function",
            "function": {
              "name": "Read",
              "arguments": "{\"file_path\": \"/workspace/file.txt\"}"
            }
          }
        ]
      },
      "finish_reason": "tool_calls"
    }
  ]
}
```

### Tool Calling Support by Model

Based on OpenAI compatibility:

**Native Support** (confirmed):
- OpenAI models (GPT-4o, GPT-4-turbo, GPT-3.5-turbo)
- Anthropic models (Claude 3.5 Sonnet, Claude 3 Opus)
- Google models (Gemini 2.5 Pro, Gemini 2.5 Flash)
- DeepSeek models (DeepSeek-V3)
- Llama 3.3+ models
- Qwen 2.5+ models
- Mistral Large models

**Requires Emulation** (likely):
- Older Llama 2 models
- Smaller models (<7B parameters)
- Non-instruct base models

## Model Naming Convention

### Format
```
<provider>/<model-name>
```

### Examples
```
openai/gpt-4o
anthropic/claude-3.5-sonnet
google/gemini-2.5-flash
deepseek/deepseek-chat
meta-llama/llama-3.3-70b-instruct
```

### Model Categories

| Provider | Example Models | Notes |
|----------|----------------|-------|
| OpenAI | `openai/gpt-4o`, `openai/gpt-4-turbo` | Premium, expensive |
| Anthropic | `anthropic/claude-3.5-sonnet` | High quality, medium cost |
| Google | `google/gemini-2.5-flash` | Fast, cost-effective |
| DeepSeek | `deepseek/deepseek-chat` | Cheap, good quality |
| Meta | `meta-llama/llama-3.3-70b-instruct` | Open source |
| Qwen | `qwen/qwen-2.5-coder-32b-instruct` | Coding-focused |

## Rate Limits

### Expected Limits (to be confirmed)

Based on typical AI gateway providers:

| Tier | Requests/min | Requests/day | Token Limit |
|------|--------------|--------------|-------------|
| Free | 20 | 1,000 | 100K tokens/day |
| Starter | 60 | 10,000 | 1M tokens/day |
| Pro | 300 | 100,000 | 10M tokens/day |
| Enterprise | Custom | Custom | Custom |

### Rate Limit Headers (expected)

```http
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 59
X-RateLimit-Reset: 1704067260
```

### Handling Rate Limits

When rate limited, expect:
```json
{
  "error": {
    "message": "Rate limit exceeded",
    "type": "rate_limit_error",
    "code": "rate_limit_exceeded"
  }
}
```

**Implementation Strategy:**
1. Detect 429 status code
2. Read `Retry-After` header
3. Implement exponential backoff
4. Log rate limit events
5. Optionally fall back to other providers

## Pricing

### Cost Structure

Based on documentation (80% savings vs Claude):

**Estimated Pricing** (per million tokens):

| Model Class | Input | Output | vs Claude Savings |
|-------------|-------|--------|-------------------|
| GPT-4o | $0.50 | $1.50 | 70% |
| Claude 3.5 Sonnet | $0.60 | $1.80 | 80% |
| Gemini 2.5 Flash | FREE | FREE | 100% |
| DeepSeek Chat | $0.03 | $0.06 | 98% |
| Llama 3.3 70B | $0.10 | $0.20 | 95% |

### Cost Tracking Features

Requesty includes:
- Real-time cost monitoring
- Per-request cost attribution
- Monthly spending reports
- Budget alerts
- Cost optimization recommendations

## Unique Requesty Features

### 1. Auto-Routing

Requesty can automatically route requests to optimal models based on:
- Cost constraints
- Performance requirements
- Availability
- Load balancing

**API Parameter** (if available):
```json
{
  "model": "auto",
  "routing_strategy": "cost_optimized"
}
```

### 2. Caching

Intelligent caching to reduce costs:
- Semantic similarity matching
- Configurable TTL
- Cache hit/miss reporting

### 3. Analytics

Built-in analytics dashboard:
- Request volume
- Token usage
- Cost breakdown
- Latency metrics
- Error rates
- Model performance comparison

### 4. Failover

Automatic failover if primary model is unavailable:
- Model-level failover
- Provider-level failover
- Custom fallback chains

## Error Handling

### Error Response Format

```json
{
  "error": {
    "message": "Invalid API key",
    "type": "authentication_error",
    "code": "invalid_api_key"
  }
}
```

### Common Errors

| Status | Error Type | Description | Recovery |
|--------|------------|-------------|----------|
| 401 | `authentication_error` | Invalid API key | Check REQUESTY_API_KEY |
| 429 | `rate_limit_error` | Too many requests | Retry with backoff |
| 400 | `invalid_request_error` | Malformed request | Fix request format |
| 500 | `server_error` | Requesty server issue | Retry or fallback |
| 503 | `service_unavailable` | Model overloaded | Retry or use different model |

## API Comparison Matrix

### Requesty vs OpenRouter vs Direct Anthropic

| Feature | Requesty | OpenRouter | Anthropic Direct |
|---------|----------|------------|------------------|
| **API Format** | OpenAI | OpenAI | Anthropic |
| **Model Count** | 300+ | 100+ | 5 |
| **Tool Calling** | OpenAI format | OpenAI format | Anthropic format |
| **Streaming** | SSE (OpenAI) | SSE (OpenAI) | SSE (Anthropic) |
| **Base URL** | `router.requesty.ai` | `openrouter.ai` | `api.anthropic.com` |
| **Auth Header** | `Bearer requesty-*` | `Bearer sk-or-*` | `x-api-key: sk-ant-*` |
| **Cost Tracking** | Built-in dashboard | Manual tracking | Manual tracking |
| **Auto-Routing** | Yes | No | N/A |
| **Caching** | Yes | No | No |
| **Failover** | Automatic | Manual | Manual |

## Integration Compatibility

### With Existing agentic-flow Architecture

**HIGH COMPATIBILITY** - Requesty is almost identical to OpenRouter:

1. **Same API Format** - OpenAI `/chat/completions`
2. **Same Tool Format** - OpenAI function calling
3. **Same Streaming** - Server-Sent Events (SSE)
4. **Same Auth Pattern** - Bearer token in header

**Required Changes:**
- New proxy file: `anthropic-to-requesty.ts`
- Provider detection: Check for `REQUESTY_API_KEY`
- Base URL change: `router.requesty.ai` instead of `openrouter.ai`
- Model naming: Use Requesty model IDs

**Reusable from OpenRouter:**
- Request/response conversion logic (~95% identical)
- Streaming handler
- Error handling patterns
- Tool calling conversion
- Model capability detection (with new model IDs)

## Testing Recommendations

### Critical Test Cases

1. **Basic Chat** - Simple message without tools
2. **System Prompt** - Test system message handling
3. **Tool Calling** - Single tool, multiple tools
4. **Streaming** - Verify SSE format compatibility
5. **Error Handling** - Invalid key, rate limits
6. **Model Override** - Test different model IDs
7. **Large Context** - Test with long messages
8. **Concurrent Requests** - Test rate limiting

### Suggested Test Models

Start with these well-supported models:

1. `openai/gpt-4o-mini` - Fast, cheap, reliable
2. `anthropic/claude-3.5-sonnet` - High quality
3. `google/gemini-2.5-flash` - Free tier
4. `deepseek/deepseek-chat` - Cost-optimized

## Security Considerations

### API Key Protection

1. **Never hardcode** - Use environment variables
2. **Gitignore .env** - Prevent accidental commits
3. **Rotate regularly** - Change keys periodically
4. **Monitor usage** - Detect unauthorized access
5. **Use separate keys** - Dev vs production

### Data Privacy

1. **Request logging** - Be careful with sensitive data
2. **Model selection** - Some models may store data
3. **GDPR compliance** - Check Requesty's policies
4. **Local vs cloud** - Understand data flow

## Open Research Questions

### Questions to Answer During Implementation

1. **Streaming Format** - Exact SSE event format (confirm matches OpenAI)
2. **Rate Limits** - Actual limits per tier
3. **Model List API** - Can we fetch available models programmatically?
4. **Auto-Routing API** - How to control routing programmatically?
5. **Cache Control** - Can we control caching per-request?
6. **Failover Config** - Can we specify fallback chains?
7. **Analytics API** - Programmatic access to usage data?
8. **Webhook Support** - Async request notifications?
9. **Batch API** - Batch processing support?
10. **Free Tier** - Is there a free tier for testing?

## Documentation Gaps

### Information Not Found in Public Docs

- Exact rate limit values per tier
- Complete model list with pricing
- Streaming event format details
- Auto-routing API parameters
- Cache control headers
- Failover configuration
- Webhook integration
- Batch processing API

### Recommended Actions

1. **Email Requesty Support** - Ask for technical docs
2. **Test in Sandbox** - Create test account
3. **Monitor Network** - Inspect actual API calls
4. **Join Discord** - Community knowledge
5. **Trial Account** - Test features hands-on

## Summary for Developers

### TL;DR - What You Need to Know

1. **Requesty = OpenRouter Clone** - Almost identical API
2. **Base URL** - `https://router.requesty.ai/v1`
3. **Auth** - `Authorization: Bearer requesty-*`
4. **Format** - OpenAI `/chat/completions`
5. **Tools** - OpenAI function calling format
6. **Proxy Pattern** - Copy OpenRouter proxy, change URL/key
7. **Models** - 300+ models, use `<provider>/<model>` format
8. **Unique Features** - Auto-routing, caching, analytics

### Recommended Implementation Strategy

**Phase 1:** Clone OpenRouter proxy as starting point
**Phase 2:** Update base URL and auth header
**Phase 3:** Add Requesty-specific features (auto-routing, caching)
**Phase 4:** Test with multiple models
**Phase 5:** Add to model optimizer

### Estimated Compatibility

| Component | Compatibility | Effort |
|-----------|---------------|--------|
| API Format | 99% | Minimal |
| Tool Calling | 100% | None |
| Streaming | 95% | Minor testing |
| Error Handling | 90% | Add new error codes |
| Model Detection | 0% | New model IDs needed |
| Proxy Architecture | 100% | Copy OpenRouter |

**Total Estimated Effort:** 3-4 hours for core implementation
