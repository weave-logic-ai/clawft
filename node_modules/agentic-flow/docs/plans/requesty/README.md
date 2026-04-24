# Requesty.ai Integration - Planning Documentation

## Overview

This directory contains comprehensive planning documentation for integrating Requesty.ai as a new provider in the agentic-flow project.

**Status:** Planning Complete ✅
**Implementation Status:** Not Started
**Estimated Effort:** 13 hours
**Risk Level:** LOW

## What is Requesty.ai?

Requesty.ai is a unified AI gateway providing:
- Access to 300+ AI models from OpenAI, Anthropic, Google, Meta, DeepSeek, and more
- OpenAI-compatible API (drop-in replacement)
- 80% cost savings vs direct Anthropic API
- Built-in analytics, caching, and auto-routing
- Enterprise features (zero downtime, failover, load balancing)

## Documentation Structure

Read the documents in this order:

### 1. [00-overview.md](./00-overview.md) - Start Here
**Read this first!**

- Executive summary
- Integration goals
- Key differentiators vs OpenRouter
- Strategic benefits
- Success criteria
- Risk assessment

**Time to read:** 5 minutes

### 2. [01-api-research.md](./01-api-research.md) - Technical Details
**For developers implementing the integration**

- Complete API specification
- Authentication methods
- Request/response schemas
- Tool calling format
- Model naming conventions
- Rate limits and pricing
- Error handling
- Comparison with OpenRouter and Anthropic

**Time to read:** 15 minutes

### 3. [02-architecture.md](./02-architecture.md) - System Design
**For architects and lead developers**

- High-level architecture diagrams
- Component breakdown
- Data flow diagrams
- File structure
- Configuration management
- Error handling strategy
- Performance considerations
- Security architecture

**Time to read:** 20 minutes

### 4. [03-implementation-phases.md](./03-implementation-phases.md) - Action Plan
**For developers ready to implement**

- Step-by-step implementation guide
- 5 phases with clear deliverables
- Code examples
- Acceptance criteria
- Timeline estimates
- Post-implementation checklist

**Time to read:** 25 minutes
**Implementation time:** 13 hours

### 5. [04-testing-strategy.md](./04-testing-strategy.md) - Quality Assurance
**For QA engineers and testers**

- Unit test specifications
- Integration test scenarios
- E2E user workflows
- Model-specific tests
- Performance benchmarks
- Security tests
- Acceptance criteria

**Time to read:** 15 minutes
**Testing time:** 3 hours

### 6. [05-migration-guide.md](./05-migration-guide.md) - User Documentation
**For end users**

- Quick start guide (3 steps)
- Usage examples
- Model recommendations
- Configuration options
- Migration from other providers
- Troubleshooting
- FAQ

**Time to read:** 10 minutes

## Key Findings

### High Compatibility with OpenRouter

The research revealed that Requesty.ai uses **almost identical API format** to OpenRouter:

| Aspect | OpenRouter | Requesty | Compatibility |
|--------|-----------|----------|---------------|
| API Format | OpenAI `/chat/completions` | OpenAI `/chat/completions` | 100% |
| Tool Calling | OpenAI functions | OpenAI functions | 100% |
| Streaming | SSE (OpenAI) | SSE (OpenAI) | 100% |
| Auth Method | Bearer token | Bearer token | 100% |
| Request Schema | OpenAI | OpenAI | 100% |
| Response Schema | OpenAI | OpenAI | 100% |

**Implication:** We can clone the OpenRouter proxy with minimal changes (~95% code reuse).

### Implementation Approach

**Strategy:** Clone and adapt the existing OpenRouter proxy

**Effort Breakdown:**
- **Phase 1:** Core Proxy (4 hours) - Clone OpenRouter proxy
- **Phase 2:** CLI Integration (2 hours) - Add provider detection
- **Phase 3:** Model Support (2 hours) - Add model definitions
- **Phase 4:** Testing (3 hours) - Comprehensive validation
- **Phase 5:** Documentation (2 hours) - User guides

**Total:** 13 hours

### Major Benefits

1. **300+ Models** (vs OpenRouter's 100+)
2. **Built-in Analytics** (OpenRouter lacks this)
3. **Auto-Routing** (intelligent model selection)
4. **Caching** (reduce API costs further)
5. **80% Cost Savings** (vs direct Anthropic API)

### Risks

**Technical Risks:** LOW
- API format is well-documented (OpenAI-compatible)
- Pattern is proven (OpenRouter already works)
- 95% code reuse minimizes bugs

**Business Risks:** LOW
- Multi-provider architecture already supports fallbacks
- Users can easily switch providers
- No vendor lock-in

## Quick Reference

### Files to Create

```
agentic-flow/
└── src/
    └── proxy/
        └── anthropic-to-requesty.ts   (~750 lines, 95% from OpenRouter)
```

### Files to Modify

```
agentic-flow/
├── src/
│   ├── cli-proxy.ts                   (+ ~80 lines)
│   ├── agents/claudeAgent.ts          (+ ~15 lines)
│   └── utils/
│       ├── modelCapabilities.ts       (+ ~50 lines)
│       └── modelOptimizer.ts          (+ ~100 lines)
└── README.md                          (+ Requesty section)
```

### Total Code Impact

| Metric | Count |
|--------|-------|
| New files | 1 |
| Modified files | 4 |
| New lines of code | ~1,000 |
| Reused lines | ~750 (95% from OpenRouter) |
| Original code | ~250 |

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

## Implementation Checklist

Use this checklist when implementing:

### Phase 1: Core Proxy ✅ Planned
- [ ] Clone `anthropic-to-openrouter.ts` to `anthropic-to-requesty.ts`
- [ ] Update class name, base URL, API key variable
- [ ] Update logging messages
- [ ] Test compilation

### Phase 2: CLI Integration ✅ Planned
- [ ] Add `shouldUseRequesty()` method
- [ ] Add `startRequestyProxy()` method
- [ ] Integrate into start flow
- [ ] Update runAgent method
- [ ] Test CLI detection

### Phase 3: Model Support ✅ Planned
- [ ] Add 15+ models to `modelCapabilities.ts`
- [ ] Update `claudeAgent.ts` provider detection
- [ ] Add 10+ models to model optimizer
- [ ] Test model detection

### Phase 4: Testing ✅ Planned
- [ ] Write unit tests (>90% coverage)
- [ ] Run integration tests (5+ models)
- [ ] Test tool calling
- [ ] Test streaming
- [ ] Validate error handling

### Phase 5: Documentation ✅ Planned
- [ ] Update README.md
- [ ] Create migration guide
- [ ] Update help text
- [ ] Update .env.example

## Next Steps

1. **Review Planning Docs** - Read 00-overview.md through 05-migration-guide.md
2. **Get Stakeholder Approval** - Present plan to team/maintainers
3. **Set Up Test Account** - Get Requesty.ai API key for testing
4. **Begin Implementation** - Follow 03-implementation-phases.md
5. **Test Thoroughly** - Use 04-testing-strategy.md
6. **Ship to Users** - Deploy with 05-migration-guide.md

## Questions?

If you have questions about the implementation plan:

1. Check the FAQ in `05-migration-guide.md`
2. Review the specific planning document
3. Open a GitHub issue with questions
4. Tag the planning document author

## Contributing

If you find gaps in the planning documentation:

1. Open an issue describing the gap
2. Submit a PR with improvements
3. Update this README with new findings

## Changelog

- **2025-01-07** - Initial planning documentation created
- Research completed on Requesty.ai API
- All 6 planning documents written
- Ready for implementation

## Credits

**Planning Author:** Claude Code
**Project:** agentic-flow
**Based On:** OpenRouter integration pattern
**Documentation Standard:** SPARC methodology

---

**Ready to implement?** Start with [03-implementation-phases.md](./03-implementation-phases.md)

**Need user docs?** Jump to [05-migration-guide.md](./05-migration-guide.md)

**Want technical details?** Read [02-architecture.md](./02-architecture.md)
