# Requesty.ai Integration - Overview

## Executive Summary

This document outlines the plan to integrate Requesty.ai as a new provider in the agentic-flow project, following the same architectural pattern as the existing OpenRouter integration.

### What is Requesty.ai?

Requesty.ai is a unified AI gateway that provides:
- **300+ AI Models** - Access to models from OpenAI, Anthropic, Google, and other providers
- **OpenAI-Compatible API** - Drop-in replacement for OpenAI SDK with `base_url` override
- **Cost Optimization** - 80% cost savings through intelligent routing and caching
- **Built-in Analytics** - Real-time usage tracking and performance monitoring
- **Enterprise Features** - Zero downtime guarantee, automatic failover, load balancing

### Integration Goals

1. **Provider Parity** - Requesty should work alongside Anthropic, OpenRouter, Gemini, and ONNX
2. **Minimal Code Changes** - Leverage existing proxy pattern from OpenRouter
3. **Model Flexibility** - Support 300+ models through Requesty's router
4. **Tool Support** - Maintain full MCP tool calling compatibility
5. **Cost Optimization** - Enable users to access cheaper models with same quality

### Key Differentiators from OpenRouter

| Feature | OpenRouter | Requesty.ai |
|---------|-----------|-------------|
| Model Count | 100+ | 300+ |
| API Format | OpenAI `/chat/completions` | OpenAI `/chat/completions` |
| Base URL | `https://openrouter.ai/api/v1` | `https://router.requesty.ai/v1` |
| Authentication | `Authorization: Bearer sk-or-...` | `Authorization: Bearer requesty-...` |
| Cost Savings | ~90% vs Claude | ~80% vs Claude |
| Tool Calling | Native OpenAI format | Native OpenAI format |
| Unique Features | Leaderboard tracking | Auto-routing, caching, analytics |

### Strategic Benefits

1. **Provider Diversity** - Reduces dependency on single gateway
2. **Model Access** - 300+ models vs OpenRouter's 100+
3. **Cost Flexibility** - Users can choose based on price/performance
4. **Redundancy** - Fallback option if OpenRouter has issues
5. **Enterprise Features** - Built-in analytics and monitoring

## Integration Approach

### Architecture Pattern

Requesty will follow the **same proxy architecture** as OpenRouter:

```
User Request (Anthropic Format)
    ↓
Agentic Flow CLI
    ↓
Provider Detection (--provider requesty)
    ↓
Anthropic-to-Requesty Proxy Server
    ↓
Format Conversion (Anthropic → OpenAI)
    ↓
Requesty Router (https://router.requesty.ai/v1)
    ↓
Model Execution (300+ models)
    ↓
Response Conversion (OpenAI → Anthropic)
    ↓
Return to User
```

### File Structure

New files to create:
```
agentic-flow/
├── src/
│   └── proxy/
│       └── anthropic-to-requesty.ts    # NEW - Requesty proxy (clone from OpenRouter)
├── docs/
│   └── plans/
│       └── requesty/
│           ├── 00-overview.md          # This file
│           ├── 01-api-research.md
│           ├── 02-architecture.md
│           ├── 03-implementation-phases.md
│           ├── 04-testing-strategy.md
│           └── 05-migration-guide.md
```

Existing files to modify:
```
agentic-flow/
├── src/
│   ├── cli-proxy.ts                    # Add Requesty provider detection
│   ├── agents/claudeAgent.ts           # Add Requesty to provider list
│   └── utils/
│       ├── modelCapabilities.ts        # Add Requesty model mappings
│       └── modelOptimizer.ts           # Include Requesty models in optimizer
```

## Success Criteria

### Must Have (MVP)
- [ ] Users can use `--provider requesty` flag
- [ ] Requesty API key via `REQUESTY_API_KEY` environment variable
- [ ] Chat completions work with at least 10 tested models
- [ ] Native tool calling support (MCP tools work)
- [ ] Streaming responses supported
- [ ] Error handling and logging
- [ ] Model override via `--model` flag

### Should Have (V1)
- [ ] Tool emulation for models without native support
- [ ] Model capability detection for Requesty models
- [ ] Integration with model optimizer (`--optimize`)
- [ ] Analytics and usage tracking
- [ ] Proxy mode for Claude Code/Cursor
- [ ] Cost estimation and reporting

### Nice to Have (Future)
- [ ] Auto-routing based on cost/performance
- [ ] Caching integration
- [ ] Fallback to other providers on error
- [ ] Model benchmarking and comparison

## Timeline Estimate

| Phase | Tasks | Estimated Time |
|-------|-------|----------------|
| Phase 1 | Research & Planning | 2 hours (DONE) |
| Phase 2 | Core Proxy Implementation | 4 hours |
| Phase 3 | CLI Integration | 2 hours |
| Phase 4 | Testing & Validation | 3 hours |
| Phase 5 | Documentation | 2 hours |
| **Total** | | **13 hours** |

## Risk Assessment

### Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| API format differences from OpenRouter | Medium | Medium | Thorough testing, fallback handling |
| Model compatibility issues | Low | Medium | Model capability detection system |
| Tool calling format incompatibility | Low | High | Test with multiple models early |
| Rate limiting differences | Medium | Low | Document limits, add retry logic |

### Business Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Requesty API changes | Medium | Medium | Version pinning, changelog monitoring |
| Service availability issues | Low | High | Multi-provider support already in place |
| Cost model changes | Low | Medium | Document pricing, update optimizer |

## Next Steps

1. Read `01-api-research.md` for detailed API analysis
2. Review `02-architecture.md` for technical design
3. Follow `03-implementation-phases.md` for step-by-step implementation
4. Use `04-testing-strategy.md` for comprehensive testing
5. Reference `05-migration-guide.md` for user documentation

## Open Questions

1. Does Requesty support streaming for all models or only specific ones?
2. Are there any model-specific quirks in tool calling format?
3. What are the exact rate limits per tier?
4. Does Requesty offer a free tier for testing?
5. How does auto-routing work - can we control it programmatically?

## References

- Requesty.ai Documentation: https://docs.requesty.ai
- Requesty.ai Base URL: https://router.requesty.ai/v1
- OpenRouter Integration (reference): `src/proxy/anthropic-to-openrouter.ts`
- Gemini Integration (reference): `src/proxy/anthropic-to-gemini.ts`
