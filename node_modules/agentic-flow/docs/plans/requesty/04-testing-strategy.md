# Requesty.ai Integration - Testing Strategy

## Testing Overview

This document outlines a comprehensive testing strategy for the Requesty.ai integration, covering unit tests, integration tests, end-to-end scenarios, and validation criteria.

## Test Categories

| Category | Scope | Duration | Automation |
|----------|-------|----------|------------|
| Unit Tests | Proxy functions | 30 min | Jest/Vitest |
| Integration Tests | CLI → Proxy → API | 60 min | Manual + Script |
| E2E Tests | Full user workflows | 45 min | Manual |
| Performance Tests | Latency, throughput | 30 min | Script |
| Security Tests | API key handling | 15 min | Manual |
| **Total** | | **3 hours** | |

---

## 1. Unit Tests

### Test File: `src/proxy/anthropic-to-requesty.test.ts`

#### 1.1 Proxy Initialization

```typescript
describe('AnthropicToRequestyProxy - Initialization', () => {
  it('should initialize with default configuration', () => {
    const proxy = new AnthropicToRequestyProxy({
      requestyApiKey: 'test-key'
    });

    expect(proxy.requestyBaseUrl).toBe('https://router.requesty.ai/v1');
    expect(proxy.defaultModel).toBe('openai/gpt-4o-mini');
  });

  it('should accept custom base URL', () => {
    const proxy = new AnthropicToRequestyProxy({
      requestyApiKey: 'test-key',
      requestyBaseUrl: 'https://custom.requesty.ai/v1'
    });

    expect(proxy.requestyBaseUrl).toBe('https://custom.requesty.ai/v1');
  });

  it('should accept custom default model', () => {
    const proxy = new AnthropicToRequestyProxy({
      requestyApiKey: 'test-key',
      defaultModel: 'anthropic/claude-3.5-sonnet'
    });

    expect(proxy.defaultModel).toBe('anthropic/claude-3.5-sonnet');
  });
});
```

#### 1.2 Format Conversion (Anthropic → OpenAI)

```typescript
describe('AnthropicToRequestyProxy - Request Conversion', () => {
  let proxy: AnthropicToRequestyProxy;

  beforeEach(() => {
    proxy = new AnthropicToRequestyProxy({
      requestyApiKey: 'test-key'
    });
  });

  it('should convert simple message', () => {
    const anthropicReq = {
      model: 'openai/gpt-4o-mini',
      messages: [
        { role: 'user', content: 'Hello' }
      ]
    };

    const openaiReq = proxy.convertAnthropicToOpenAI(anthropicReq);

    expect(openaiReq.model).toBe('openai/gpt-4o-mini');
    expect(openaiReq.messages).toHaveLength(2); // system + user
    expect(openaiReq.messages[1].content).toBe('Hello');
  });

  it('should convert system prompt (string)', () => {
    const anthropicReq = {
      model: 'openai/gpt-4o-mini',
      system: 'You are a helpful assistant',
      messages: [{ role: 'user', content: 'Hi' }]
    };

    const openaiReq = proxy.convertAnthropicToOpenAI(anthropicReq);

    expect(openaiReq.messages[0].role).toBe('system');
    expect(openaiReq.messages[0].content).toContain('You are a helpful assistant');
  });

  it('should convert system prompt (array)', () => {
    const anthropicReq = {
      model: 'openai/gpt-4o-mini',
      system: [
        { type: 'text', text: 'You are helpful' },
        { type: 'text', text: 'Be concise' }
      ],
      messages: [{ role: 'user', content: 'Hi' }]
    };

    const openaiReq = proxy.convertAnthropicToOpenAI(anthropicReq);

    expect(openaiReq.messages[0].content).toContain('You are helpful');
    expect(openaiReq.messages[0].content).toContain('Be concise');
  });

  it('should convert tools to OpenAI format', () => {
    const anthropicReq = {
      model: 'openai/gpt-4o-mini',
      messages: [{ role: 'user', content: 'Read file' }],
      tools: [{
        name: 'Read',
        description: 'Read a file',
        input_schema: {
          type: 'object',
          properties: {
            file_path: { type: 'string' }
          },
          required: ['file_path']
        }
      }]
    };

    const openaiReq = proxy.convertAnthropicToOpenAI(anthropicReq);

    expect(openaiReq.tools).toHaveLength(1);
    expect(openaiReq.tools[0].type).toBe('function');
    expect(openaiReq.tools[0].function.name).toBe('Read');
    expect(openaiReq.tools[0].function.parameters).toEqual(anthropicReq.tools[0].input_schema);
  });
});
```

#### 1.3 Format Conversion (OpenAI → Anthropic)

```typescript
describe('AnthropicToRequestyProxy - Response Conversion', () => {
  it('should convert text response', () => {
    const openaiRes = {
      id: 'chatcmpl-123',
      model: 'openai/gpt-4o-mini',
      choices: [{
        index: 0,
        message: {
          role: 'assistant',
          content: 'Hello, how can I help?'
        },
        finish_reason: 'stop'
      }],
      usage: {
        prompt_tokens: 10,
        completion_tokens: 20,
        total_tokens: 30
      }
    };

    const anthropicRes = proxy.convertOpenAIToAnthropic(openaiRes);

    expect(anthropicRes.id).toBe('chatcmpl-123');
    expect(anthropicRes.role).toBe('assistant');
    expect(anthropicRes.content).toHaveLength(1);
    expect(anthropicRes.content[0].type).toBe('text');
    expect(anthropicRes.content[0].text).toBe('Hello, how can I help?');
    expect(anthropicRes.stop_reason).toBe('end_turn');
    expect(anthropicRes.usage.input_tokens).toBe(10);
    expect(anthropicRes.usage.output_tokens).toBe(20);
  });

  it('should convert tool_calls response', () => {
    const openaiRes = {
      id: 'chatcmpl-123',
      model: 'openai/gpt-4o-mini',
      choices: [{
        message: {
          role: 'assistant',
          content: null,
          tool_calls: [{
            id: 'call_abc123',
            type: 'function',
            function: {
              name: 'Read',
              arguments: '{"file_path": "/test.txt"}'
            }
          }]
        },
        finish_reason: 'tool_calls'
      }],
      usage: { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 }
    };

    const anthropicRes = proxy.convertOpenAIToAnthropic(openaiRes);

    expect(anthropicRes.content).toHaveLength(1);
    expect(anthropicRes.content[0].type).toBe('tool_use');
    expect(anthropicRes.content[0].id).toBe('call_abc123');
    expect(anthropicRes.content[0].name).toBe('Read');
    expect(anthropicRes.content[0].input).toEqual({ file_path: '/test.txt' });
    expect(anthropicRes.stop_reason).toBe('tool_use');
  });

  it('should convert mixed content response', () => {
    const openaiRes = {
      id: 'chatcmpl-123',
      model: 'openai/gpt-4o-mini',
      choices: [{
        message: {
          role: 'assistant',
          content: 'Let me read that file',
          tool_calls: [{
            id: 'call_abc123',
            type: 'function',
            function: {
              name: 'Read',
              arguments: '{"file_path": "/test.txt"}'
            }
          }]
        },
        finish_reason: 'tool_calls'
      }],
      usage: { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 }
    };

    const anthropicRes = proxy.convertOpenAIToAnthropic(openaiRes);

    expect(anthropicRes.content).toHaveLength(2); // tool_use + text
    expect(anthropicRes.content[0].type).toBe('tool_use');
    expect(anthropicRes.content[1].type).toBe('text');
  });
});
```

#### 1.4 Error Handling

```typescript
describe('AnthropicToRequestyProxy - Error Handling', () => {
  it('should handle invalid API key', async () => {
    const proxy = new AnthropicToRequestyProxy({
      requestyApiKey: 'invalid-key'
    });

    await expect(proxy.handleRequest(validRequest, mockRes))
      .rejects.toThrow('Invalid API key');
  });

  it('should handle rate limit errors', async () => {
    // Mock 429 response
    global.fetch = jest.fn().mockResolvedValue({
      ok: false,
      status: 429,
      headers: new Map([['Retry-After', '60']]),
      text: async () => 'Rate limit exceeded'
    });

    await expect(proxy.handleRequest(validRequest, mockRes))
      .rejects.toThrow('Rate limit exceeded');
  });

  it('should handle model not found', async () => {
    const req = {
      model: 'invalid/model',
      messages: [{ role: 'user', content: 'Test' }]
    };

    await expect(proxy.handleRequest(req, mockRes))
      .rejects.toThrow('Model not found');
  });
});
```

### Unit Test Coverage Goals

- [ ] Proxy initialization: 100%
- [ ] Request conversion: 95%
- [ ] Response conversion: 95%
- [ ] Tool calling: 100%
- [ ] Error handling: 90%
- [ ] **Overall: >90%**

---

## 2. Integration Tests

### 2.1 CLI to Proxy Integration

```bash
#!/bin/bash
# tests/integration/requesty-cli.sh

echo "=== Requesty CLI Integration Tests ==="

# Test 1: Basic provider detection
echo "Test 1: Provider detection via flag"
npx agentic-flow --agent coder \
  --task "Say hello" \
  --provider requesty \
  --model "openai/gpt-4o-mini" | grep "Requesty"

# Test 2: Provider detection via env var
echo "Test 2: Provider detection via USE_REQUESTY"
USE_REQUESTY=true npx agentic-flow --agent coder \
  --task "Say hello" | grep "Requesty"

# Test 3: API key validation (missing key)
echo "Test 3: Missing API key error"
REQUESTY_API_KEY= npx agentic-flow --agent coder \
  --task "Test" \
  --provider requesty 2>&1 | grep "REQUESTY_API_KEY required"

# Test 4: Proxy startup
echo "Test 4: Proxy starts on port 3000"
npx agentic-flow --agent coder \
  --task "Test" \
  --provider requesty 2>&1 | grep "http://localhost:3000"

# Test 5: Model override
echo "Test 5: Model override works"
npx agentic-flow --agent coder \
  --task "Test" \
  --provider requesty \
  --model "anthropic/claude-3.5-sonnet" 2>&1 | grep "claude-3.5-sonnet"
```

### 2.2 Proxy to Requesty API Integration

```bash
#!/bin/bash
# tests/integration/requesty-api.sh

echo "=== Requesty API Integration Tests ==="

# Test 1: Chat completions endpoint
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: test-key" \
  -d '{
    "model": "openai/gpt-4o-mini",
    "messages": [{"role": "user", "content": "Hello"}]
  }'

# Test 2: Tool calling
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: test-key" \
  -d '{
    "model": "openai/gpt-4o-mini",
    "messages": [{"role": "user", "content": "Read file.txt"}],
    "tools": [{
      "name": "Read",
      "input_schema": {
        "type": "object",
        "properties": {"file_path": {"type": "string"}},
        "required": ["file_path"]
      }
    }]
  }'

# Test 3: Health check
curl http://localhost:3000/health
```

### 2.3 End-to-End User Workflows

#### Workflow 1: Simple Code Generation

```bash
npx agentic-flow --agent coder \
  --task "Create a Python function that adds two numbers" \
  --provider requesty \
  --model "openai/gpt-4o-mini"
```

**Expected:**
- Proxy starts
- Request sent to Requesty
- Python function generated
- Exit code 0

#### Workflow 2: File Operations with Tools

```bash
npx agentic-flow --agent coder \
  --task "Create a file hello.py with a hello world function" \
  --provider requesty
```

**Expected:**
- Tool calling works
- File created: `hello.py`
- Function is valid Python
- Exit code 0

#### Workflow 3: Research with Streaming

```bash
npx agentic-flow --agent researcher \
  --task "Explain machine learning in simple terms" \
  --provider requesty \
  --model "anthropic/claude-3.5-sonnet" \
  --stream
```

**Expected:**
- Streaming output (real-time)
- Coherent explanation
- Exit code 0

#### Workflow 4: Multi-Step Task

```bash
npx agentic-flow --agent coder \
  --task "Create a REST API with Express.js - include routes for GET /users and POST /users" \
  --provider requesty \
  --model "openai/gpt-4o"
```

**Expected:**
- Multiple files created
- Valid Express.js code
- Includes route handlers
- Exit code 0

### Integration Test Matrix

| Test ID | Component | Input | Expected Output | Status |
|---------|-----------|-------|-----------------|--------|
| INT-01 | CLI Detection | `--provider requesty` | Proxy starts | [ ] |
| INT-02 | CLI Detection | `USE_REQUESTY=true` | Proxy starts | [ ] |
| INT-03 | CLI Validation | Missing API key | Error message | [ ] |
| INT-04 | Proxy Startup | Start proxy | Port 3000 listening | [ ] |
| INT-05 | Proxy Health | GET /health | 200 OK | [ ] |
| INT-06 | API Request | Simple chat | Valid response | [ ] |
| INT-07 | Tool Calling | Request with tools | Tool use response | [ ] |
| INT-08 | Streaming | Stream flag | SSE events | [ ] |
| INT-09 | Model Override | Different model | Model changed | [ ] |
| INT-10 | Error Handling | Invalid key | 401 error | [ ] |

---

## 3. Model-Specific Tests

### Test Different Model Families

#### 3.1 OpenAI Models

```bash
# GPT-4o Mini (fast, cheap)
npx agentic-flow --agent coder \
  --task "Create a hello function" \
  --provider requesty \
  --model "openai/gpt-4o-mini"

# GPT-4o (premium quality)
npx agentic-flow --agent coder \
  --task "Create a complex sorting algorithm" \
  --provider requesty \
  --model "openai/gpt-4o"

# GPT-4 Turbo
npx agentic-flow --agent researcher \
  --task "Research quantum computing" \
  --provider requesty \
  --model "openai/gpt-4-turbo"
```

#### 3.2 Anthropic Models

```bash
# Claude 3.5 Sonnet
npx agentic-flow --agent coder \
  --task "Write a file parser" \
  --provider requesty \
  --model "anthropic/claude-3.5-sonnet"

# Claude 3 Haiku (fast)
npx agentic-flow --agent coder \
  --task "Simple function" \
  --provider requesty \
  --model "anthropic/claude-3-haiku"
```

#### 3.3 Google Models

```bash
# Gemini 2.5 Flash (FREE)
npx agentic-flow --agent coder \
  --task "Create a calculator" \
  --provider requesty \
  --model "google/gemini-2.5-flash"

# Gemini 2.5 Pro
npx agentic-flow --agent researcher \
  --task "Analyze AI trends" \
  --provider requesty \
  --model "google/gemini-2.5-pro"
```

#### 3.4 DeepSeek Models

```bash
# DeepSeek Chat V3 (cheap)
npx agentic-flow --agent coder \
  --task "Create API endpoint" \
  --provider requesty \
  --model "deepseek/deepseek-chat-v3"

# DeepSeek Coder
npx agentic-flow --agent coder \
  --task "Write Python script" \
  --provider requesty \
  --model "deepseek/deepseek-coder"
```

#### 3.5 Meta/Llama Models

```bash
# Llama 3.3 70B
npx agentic-flow --agent coder \
  --task "Create function" \
  --provider requesty \
  --model "meta-llama/llama-3.3-70b-instruct"

# Llama 3.3 8B (fast)
npx agentic-flow --agent coder \
  --task "Simple task" \
  --provider requesty \
  --model "meta-llama/llama-3.3-8b-instruct"
```

### Model Test Matrix

| Model | Provider | Tools | Stream | Expected Time | Status |
|-------|----------|-------|--------|---------------|--------|
| gpt-4o-mini | OpenAI | ✓ | ✓ | <5s | [ ] |
| gpt-4o | OpenAI | ✓ | ✓ | <10s | [ ] |
| claude-3.5-sonnet | Anthropic | ✓ | ✓ | <15s | [ ] |
| claude-3-haiku | Anthropic | ✓ | ✓ | <5s | [ ] |
| gemini-2.5-flash | Google | ✓ | ✓ | <3s | [ ] |
| gemini-2.5-pro | Google | ✓ | ✓ | <10s | [ ] |
| deepseek-chat-v3 | DeepSeek | ✓ | ✓ | <8s | [ ] |
| llama-3.3-70b | Meta | ✓ | ✓ | <12s | [ ] |

---

## 4. Performance Tests

### 4.1 Latency Testing

```bash
#!/bin/bash
# tests/performance/latency.sh

echo "=== Requesty Performance Tests ==="

for i in {1..10}; do
  start=$(date +%s%N)

  npx agentic-flow --agent coder \
    --task "Say hello" \
    --provider requesty \
    --model "openai/gpt-4o-mini" > /dev/null

  end=$(date +%s%N)
  duration=$(( (end - start) / 1000000 ))

  echo "Request $i: ${duration}ms"
done
```

**Success Criteria:**
- Average latency < 3000ms
- P95 latency < 5000ms
- No timeouts

### 4.2 Concurrent Requests

```bash
#!/bin/bash
# tests/performance/concurrent.sh

echo "=== Concurrent Request Test ==="

for i in {1..5}; do
  npx agentic-flow --agent coder \
    --task "Task $i" \
    --provider requesty &
done

wait
echo "All concurrent requests completed"
```

**Success Criteria:**
- All requests complete successfully
- No proxy crashes
- No rate limit errors (or proper retry)

### 4.3 Large Context Test

```bash
# Test with large system prompt
npx agentic-flow --agent coder \
  --task "$(cat large-context.txt)" \
  --provider requesty \
  --model "google/gemini-2.5-pro"  # Large context window
```

**Success Criteria:**
- Request succeeds
- No context truncation errors
- Response is relevant

---

## 5. Security Tests

### 5.1 API Key Handling

**Test:** Verify API key is not logged

```bash
# Run with verbose logging
VERBOSE=true npx agentic-flow --agent coder \
  --task "Test" \
  --provider requesty 2>&1 | grep -c "requesty-"
```

**Expected:** 0 matches (only prefix should be logged)

### 5.2 API Key Validation

```bash
# Test invalid key format
REQUESTY_API_KEY="invalid" npx agentic-flow --agent coder \
  --task "Test" \
  --provider requesty
```

**Expected:** Error message about invalid key

### 5.3 Environment Isolation

```bash
# Verify proxy doesn't leak API key to client
curl http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{"model": "openai/gpt-4o-mini", "messages": [{"role": "user", "content": "Hello"}]}' \
  | grep -c "requesty-"
```

**Expected:** 0 matches (API key should not be in response)

---

## 6. Error Handling Tests

### 6.1 Invalid API Key

```bash
REQUESTY_API_KEY="requesty-invalid" npx agentic-flow --agent coder \
  --task "Test" \
  --provider requesty
```

**Expected:**
- HTTP 401 Unauthorized
- Clear error message
- Exit code 1

### 6.2 Rate Limiting

```bash
# Send many requests quickly
for i in {1..50}; do
  npx agentic-flow --agent coder \
    --task "Test $i" \
    --provider requesty &
done
wait
```

**Expected:**
- Some requests succeed
- Some requests retry (429 errors)
- All eventually complete or fail gracefully

### 6.3 Model Not Found

```bash
npx agentic-flow --agent coder \
  --task "Test" \
  --provider requesty \
  --model "invalid/model-123"
```

**Expected:**
- HTTP 404 or 400
- Clear error message
- Exit code 1

### 6.4 Network Timeout

```bash
# Mock network timeout (requires proxy modification for testing)
REQUESTY_TIMEOUT=1 npx agentic-flow --agent coder \
  --task "Long task" \
  --provider requesty
```

**Expected:**
- Timeout error
- Retry attempt
- Clear error message

---

## 7. Compatibility Tests

### 7.1 Cross-Platform

Test on multiple OS:

- [ ] Linux (Ubuntu 22.04)
- [ ] macOS (Ventura+)
- [ ] Windows (WSL2)

### 7.2 Node.js Versions

Test with different Node versions:

- [ ] Node.js 18.x
- [ ] Node.js 20.x
- [ ] Node.js 22.x

### 7.3 Package Managers

Test with different package managers:

- [ ] npm
- [ ] yarn
- [ ] pnpm

---

## 8. Regression Tests

### 8.1 Existing Functionality

Verify Requesty doesn't break existing providers:

```bash
# Test Anthropic (should still work)
npx agentic-flow --agent coder \
  --task "Test" \
  --provider anthropic

# Test OpenRouter (should still work)
npx agentic-flow --agent coder \
  --task "Test" \
  --provider openrouter

# Test Gemini (should still work)
npx agentic-flow --agent coder \
  --task "Test" \
  --provider gemini

# Test ONNX (should still work)
npx agentic-flow --agent coder \
  --task "Test" \
  --provider onnx
```

**Success Criteria:**
- All providers still work
- No interference between providers
- Proxy ports don't conflict

---

## 9. Acceptance Criteria

### Must Pass (MVP)

- [ ] Unit tests: >90% coverage
- [ ] At least 5 models tested successfully
- [ ] Tool calling works (1+ model)
- [ ] Streaming works (1+ model)
- [ ] Error handling for invalid API key
- [ ] No regressions in existing providers
- [ ] Documentation complete

### Should Pass (V1)

- [ ] 10+ models tested
- [ ] Performance tests pass
- [ ] Security tests pass
- [ ] Cross-platform tested
- [ ] Concurrent requests work
- [ ] Rate limiting handled

### Nice to Have (Future)

- [ ] 20+ models tested
- [ ] Load testing (100+ concurrent)
- [ ] Automated test suite
- [ ] CI/CD integration

---

## 10. Test Execution Plan

### Day 1: Unit Tests
- Write test files
- Run unit tests
- Fix any failures
- Achieve >90% coverage

### Day 2: Integration Tests
- Test CLI integration
- Test proxy integration
- Test API integration
- Document results

### Day 3: Model Testing
- Test 5+ models
- Test tool calling
- Test streaming
- Document results

### Day 4: Final Validation
- Run all tests
- Fix any regressions
- Update documentation
- Sign off

---

## 11. Bug Reporting Template

When filing bugs, use this template:

```markdown
## Bug Report: Requesty Integration

**Test Category:** [Unit/Integration/E2E/Performance/Security]
**Test ID:** [e.g., INT-07]

**Description:**
[Clear description of the issue]

**Steps to Reproduce:**
1. Step 1
2. Step 2
3. Step 3

**Expected Behavior:**
[What should happen]

**Actual Behavior:**
[What actually happened]

**Environment:**
- OS: [e.g., Ubuntu 22.04]
- Node.js: [e.g., 20.10.0]
- agentic-flow: [e.g., 1.3.0]
- Requesty Model: [e.g., openai/gpt-4o-mini]

**Logs:**
[Paste relevant logs]

**Screenshots:**
[If applicable]
```

---

## 12. Test Results Dashboard

### Summary

| Category | Total Tests | Passed | Failed | Skipped | Coverage |
|----------|-------------|--------|--------|---------|----------|
| Unit | 0 | 0 | 0 | 0 | 0% |
| Integration | 0 | 0 | 0 | 0 | N/A |
| E2E | 0 | 0 | 0 | 0 | N/A |
| Performance | 0 | 0 | 0 | 0 | N/A |
| Security | 0 | 0 | 0 | 0 | N/A |
| **Total** | **0** | **0** | **0** | **0** | **0%** |

*(To be filled during testing)*

---

## Conclusion

This testing strategy ensures comprehensive validation of the Requesty integration across all critical dimensions: functionality, performance, security, and compatibility. By following this plan, we can confidently release the Requesty provider to users.

**Total Testing Time:** ~3 hours
**Test Coverage Goal:** >90%
**Model Coverage Goal:** 5+ models (MVP), 10+ models (V1)
