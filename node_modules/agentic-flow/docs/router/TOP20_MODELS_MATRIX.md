# Top 20 OpenRouter Models - Tool Calling Functionality Matrix

Generated: 2025-10-05T05:09:37.845Z

## Summary Statistics

- **Total Models Tested:** 20
- **Successful Responses:** 14
- **Models Using Tools:** 13
- **Tool Success Rate:** 92.9%
- **Free Models:** 3
- **Avg Response Time:** 1686ms

## Functionality Matrix

| Rank | Model | Provider | Free | Status | Tools | Native | Response Time | Notes |
|------|-------|----------|------|--------|-------|--------|---------------|-------|
| 1 | Grok Code Fast 1 | x-ai | ✗ | ✅ | 🔧 1 | ✗ | 1591ms | OK |
| 2 | Grok 4 Fast (free) | x-ai | ✓ | ❌ | ⚠️ 0 | ✗ | 218ms | Error: HTTP 404 |
| 3 | Claude Sonnet 4 | anthropic | ✗ | ✅ | 🔧 1 | ✓ | 2171ms | OK |
| 4 | Gemini 2.5 Flash | google | ✗ | ✅ | 🔧 1 | ✗ | 483ms | OK |
| 5 | Claude Sonnet 4.5 | anthropic | ✗ | ✅ | 🔧 1 | ✓ | 2249ms | OK |
| 6 | DeepSeek V3.1 (free) | deepseek | ✓ | ❌ | ⚠️ 0 | ✗ | 96ms | Error: HTTP 400 |
| 7 | GPT-4.1 Mini | openai | ✗ | ✅ | 🔧 1 | ✓ | 2279ms | OK |
| 8 | Gemini 2.0 Flash | google | ✓ | ❌ | ⚠️ 0 | ✗ | 318ms | Error: HTTP 400 |
| 9 | Gemini 2.5 Pro | google | ✗ | ✅ | 🔧 1 | ✗ | 2220ms | OK |
| 10 | Gemini 2.5 Flash Lite | google | ✗ | ✅ | 🔧 1 | ✗ | 536ms | OK |
| 11 | DeepSeek V3 0324 | deepseek | ✗ | ❌ | ⚠️ 0 | ✗ | 102ms | Error: HTTP 400 |
| 12 | Gemma 3 12B | google | ✗ | ❌ | ⚠️ 0 | ✗ | 220ms | Error: HTTP 400 |
| 13 | GPT-5 | openai | ✗ | ✅ | 🔧 2 | ✗ | 4343ms | OK |
| 14 | Claude 3.7 Sonnet | anthropic | ✗ | ✅ | 🔧 1 | ✓ | 1920ms | OK |
| 15 | gpt-oss-120b | openai | ✗ | ✅ | ⚠️ 0 | ✗ | 651ms | OK |
| 16 | gpt-oss-20b | openai | ✗ | ✅ | 🔧 1 | ✗ | 999ms | OK |
| 17 | Grok 4 Fast | x-ai | ✗ | ✅ | 🔧 1 | ✗ | 1597ms | OK |
| 18 | GPT-4o-mini | openai | ✗ | ✅ | 🔧 1 | ✓ | 1416ms | OK |
| 19 | Llama 3.1 8B Instruct | meta-llama | ✗ | ✅ | 🔧 1 | ✗ | 1155ms | OK |
| 20 | GLM 4.6 | z-ai | ✗ | ❌ | ⚠️ 0 | ✗ | 76ms | Error: HTTP 400 |

## Models Requiring Custom Instructions

Based on test results, the following models may need model-specific tool instructions:

### gpt-oss-120b (openai/gpt-oss-120b)
- **Provider:** openai
- **Issue:** Responded with text but didn't use structured commands
- **Response:** 
- **Recommendation:** Create provider-specific prompt template


## Provider-Specific Recommendations

### x-ai
- **Tool Success Rate:** 100.0% (2/2)
- **Models Tested:** Grok Code Fast 1, Grok 4 Fast (free), Grok 4 Fast

### anthropic
- **Tool Success Rate:** 100.0% (3/3)
- **Models Tested:** Claude Sonnet 4, Claude Sonnet 4.5, Claude 3.7 Sonnet

### google
- **Tool Success Rate:** 100.0% (3/3)
- **Models Tested:** Gemini 2.5 Flash, Gemini 2.0 Flash, Gemini 2.5 Pro, Gemini 2.5 Flash Lite, Gemma 3 12B

### deepseek
- **Tool Success Rate:** 0.0% (0/0)
- **Models Tested:** DeepSeek V3.1 (free), DeepSeek V3 0324

### openai
- **Tool Success Rate:** 80.0% (4/5)
- **Models Tested:** GPT-4.1 Mini, GPT-5, gpt-oss-120b, gpt-oss-20b, GPT-4o-mini
- **Action:** Consider provider-specific instruction template

### meta-llama
- **Tool Success Rate:** 100.0% (1/1)
- **Models Tested:** Llama 3.1 8B Instruct

### z-ai
- **Tool Success Rate:** 0.0% (0/0)
- **Models Tested:** GLM 4.6

