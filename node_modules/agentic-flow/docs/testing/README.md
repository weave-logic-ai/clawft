# Testing & Validation Documentation

Comprehensive testing reports, validation summaries, and quality assurance documentation.

## Current Test Reports

- [Agent System Validation](AGENT-SYSTEM-VALIDATION.md) - Multi-agent system testing
- [Final Testing Summary](FINAL-TESTING-SUMMARY.md) - Comprehensive test coverage report
- [Regression Test Results](REGRESSION-TEST-RESULTS.md) - Regression testing outcomes
- [Streaming and MCP Validation](STREAMING-AND-MCP-VALIDATION.md) - MCP integration tests

## Test Categories

### Agent System Tests
- Multi-agent coordination
- Swarm topology validation
- Task orchestration
- Memory and state management

### Integration Tests
- MCP server integration
- Provider compatibility (Anthropic, OpenRouter, ONNX, Gemini)
- Streaming functionality
- Real-time coordination

### Regression Tests
- Backward compatibility
- Performance benchmarks
- API contract validation

## Running Tests

```bash
# Run all tests
npm test

# Run specific test suite
npm test -- --grep "agent"

# Run with coverage
npm run test:coverage
```

## Historical Test Reports

Older validation reports can be found in the [validation/](../validation/) and [archived/](../archived/) directories.
