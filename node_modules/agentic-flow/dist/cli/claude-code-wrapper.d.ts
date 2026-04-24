#!/usr/bin/env node
/**
 * Claude Code Wrapper for Agentic Flow
 *
 * Automatically spawns Claude Code with the correct ANTHROPIC_BASE_URL
 * and environment variables based on the provider/args used.
 *
 * Usage:
 *   npx agentic-flow claude-code --provider openrouter "Write a function"
 *   npx agentic-flow claude-code --provider gemini "Create a REST API"
 *   npx agentic-flow claude-code --provider anthropic "Help me debug"
 *
 * Features:
 * - Auto-starts proxy server in background if not running
 * - Sets ANTHROPIC_BASE_URL to proxy endpoint
 * - Configures provider-specific API keys
 * - Supports all Claude Code native arguments
 * - Cleans up proxy on exit (optional)
 */
export {};
//# sourceMappingURL=claude-code-wrapper.d.ts.map