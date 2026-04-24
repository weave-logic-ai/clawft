#!/usr/bin/env node
/**
 * Agentic Flow - Standalone Anthropic Proxy Server
 *
 * Run Gemini, OpenRouter, or other providers through an Anthropic-compatible proxy
 * for use with Claude Code, Cursor, and other tools that support ANTHROPIC_BASE_URL
 *
 * Usage:
 *   npx agentic-flow proxy --provider gemini --port 3000
 *   npx agentic-flow proxy --provider openrouter --model "openai/gpt-4o-mini" --port 3000
 *
 * Then in Claude Code or Cursor:
 *   export ANTHROPIC_BASE_URL=http://localhost:3000
 *   export ANTHROPIC_API_KEY=sk-ant-proxy-dummy-key  # Any value works
 */
export {};
//# sourceMappingURL=cli-standalone-proxy.d.ts.map