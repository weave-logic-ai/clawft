# Changelog

All notable changes to this project will be documented in this file.

## [2.0.1-alpha.4] - 2025-12-03

### Added
- **SONA v0.1.4 Federated Learning Integration**: Complete integration with AgentDB
  - Updated AgentDB dependency to 2.0.0-alpha.2.16
  - Full support for `EphemeralLearningAgent`, `FederatedLearningCoordinator`, and `FederatedLearningManager`
  - Quality-based filtering and weighted aggregation
  - Large-scale federation (50+ agents with configurable limits)

### Changed
- **Dependencies Updated**:
  - `agentdb`: 2.0.0-alpha.2.15 → 2.0.0-alpha.2.16 (SONA v0.1.4 federated learning)

### Documentation
- Comprehensive federated learning guide available in AgentDB package
- 5 detailed use cases for distributed learning
- API documentation and performance tuning recommendations

### Tested
- ✅ Complete federated learning workflow with 50+ agents
- ✅ Quality filtering and weighted consolidation
- ✅ Multi-agent coordination and automatic aggregation
- ✅ All test suites passing

---

## [1.10.2] - 2025-01-10

### Fixed
- **Critical Bug**: Fixed ANTHROPIC_API_KEY overriding `--provider` CLI argument ([#60](https://github.com/ruvnet/agentic-flow/issues/60))
  - CLI arguments (`--provider`, `--openrouter-key`, `--anthropic-key`, `--model`) now properly propagate to environment variables
  - Removed silent fallback to `ANTHROPIC_API_KEY` for non-Anthropic providers (OpenRouter, Gemini, Requesty)
  - Added proper validation requiring provider-specific API keys with helpful error messages
  - CLI arguments now correctly override environment variables as expected

### Added
- Comprehensive test suite for provider CLI argument handling
- Clear error messages when provider-specific API keys are missing
- Logging for provider selection from CLI arguments

### Changed
- **Breaking Change**: Providers now require their specific API keys (no fallback to ANTHROPIC_API_KEY)
  - OpenRouter requires `OPENROUTER_API_KEY` or `--openrouter-key`
  - Gemini requires `GOOGLE_GEMINI_API_KEY`
  - Requesty requires `REQUESTY_API_KEY`
  - Anthropic continues to require `ANTHROPIC_API_KEY` or `--anthropic-key`

### Migration Guide
If you were relying on the fallback behavior:
```bash
# Before (relied on ANTHROPIC_API_KEY fallback)
export ANTHROPIC_API_KEY=sk-ant-...
npx agentic-flow --provider openrouter --task "test"

# After (requires provider-specific key)
export OPENROUTER_API_KEY=sk-or-...
npx agentic-flow --provider openrouter --task "test"

# Or use CLI argument
npx agentic-flow --provider openrouter --openrouter-key sk-or-... --task "test"
```

## [1.10.0] - Previous Release
- See git history for previous changes
