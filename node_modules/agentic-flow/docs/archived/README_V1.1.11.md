# v1.1.11 Release - MCP Tools + Standalone Proxy 🎉

## Quick Start

### Use Gemini/OpenRouter with Claude Code (85-90% cost savings!)

```bash
# Terminal 1: Start proxy
export GOOGLE_GEMINI_API_KEY=your-key-here
npx agentic-flow@1.1.11 proxy

# Terminal 2: Use Claude Code
export ANTHROPIC_BASE_URL=http://localhost:3000
export ANTHROPIC_API_KEY=sk-ant-proxy-dummy-key
claude
```

**That's it!** Claude Code now uses Gemini instead of Anthropic, saving 85% on costs.

## What's New in v1.1.11

### 1. MCP Tools Work Through Proxy ✅
**Before:** MCP tools only worked with direct Anthropic API
**Now:** MCP tools work with Gemini and OpenRouter proxies!

```bash
# Use MCP memory tools with Gemini (85% cheaper)
export ENABLE_CLAUDE_FLOW_SDK=true
export GOOGLE_GEMINI_API_KEY=your-key
npx agentic-flow proxy --provider gemini

# In another terminal
export ANTHROPIC_BASE_URL=http://localhost:3000
npx agentic-flow --agent coder --task "Use memory_store to save config"
```

### 2. Standalone Proxy Command ✅
**Before:** Complex setup to use proxy with Claude Code
**Now:** Simple `npx agentic-flow proxy` command!

```bash
# Gemini proxy (default)
npx agentic-flow proxy

# OpenRouter proxy
npx agentic-flow proxy --provider openrouter --model "openai/gpt-4o-mini"

# Custom port
npx agentic-flow proxy --port 8080

# Help
npx agentic-flow proxy --help
```

## Installation

```bash
npm install -g agentic-flow@1.1.11
```

Or use directly:
```bash
npx agentic-flow@1.1.11
```

## Commands

### Run Agents (Normal Usage)
```bash
npx agentic-flow --agent coder --task "Create REST API"
npx agentic-flow --agent coder --task "Build app" --provider gemini
npx agentic-flow --agent coder --task "Code review" --provider openrouter
```

### Start Standalone Proxy (NEW!)
```bash
npx agentic-flow proxy [OPTIONS]

Options:
  -p, --provider <provider>   gemini, openrouter [default: gemini]
  -P, --port <port>           Port number [default: 3000]
  -m, --model <model>         Model to use
  -h, --help                  Show help
```

## Cost Savings

| Provider | Cost per 1M tokens | Savings vs Anthropic |
|----------|-------------------|---------------------|
| **Anthropic** (Claude Sonnet 4.5) | $3.00 | Baseline |
| **Gemini** (via proxy) | $0.00 (free tier) | **100%** 🎉 |
| **OpenRouter** (gpt-4o-mini) | $0.15 | **95%** |
| **OpenRouter** (deepseek-v3) | $0.014 | **99.5%** |

**Real Example:**
- Generate 10,000 lines of code (500K tokens)
- Anthropic: $1.50
- Gemini: $0.00
- OpenRouter (DeepSeek): $0.007

## MCP Tools Available

When `ENABLE_CLAUDE_FLOW_SDK=true`:

1. **memory_store** - Store persistent memory with TTL
2. **memory_retrieve** - Retrieve from memory
3. **memory_search** - Search memory patterns
4. **swarm_init** - Initialize agent swarm
5. **agent_spawn** - Spawn swarm agents
6. **task_orchestrate** - Orchestrate tasks
7. **swarm_status** - Get swarm status

**All work through proxy!** (New in v1.1.11)

## Use with Claude Code

### Setup
```bash
# Terminal 1: Start proxy
export GOOGLE_GEMINI_API_KEY=your-key-here
npx agentic-flow proxy

# Terminal 2: Configure Claude Code
export ANTHROPIC_BASE_URL=http://localhost:3000
export ANTHROPIC_API_KEY=sk-ant-proxy-dummy-key

# Use Claude Code normally
claude
claude --agent coder --task "Build API"
```

### With MCP Tools
```bash
# Enable MCP
export ENABLE_CLAUDE_FLOW_SDK=true
export ANTHROPIC_BASE_URL=http://localhost:3000
export ANTHROPIC_API_KEY=sk-ant-proxy-dummy-key

# MCP tools now work!
claude --agent coder --task "Store config in memory using MCP"
```

## Examples

### Example 1: Gemini Proxy
```bash
export GOOGLE_GEMINI_API_KEY=AIza...
npx agentic-flow proxy
```

### Example 2: OpenRouter with GPT-4o-mini
```bash
export OPENROUTER_API_KEY=sk-or-v1-...
npx agentic-flow proxy --provider openrouter --model "openai/gpt-4o-mini"
```

### Example 3: Direct Agent Execution
```bash
# No proxy - uses provider directly
export GOOGLE_GEMINI_API_KEY=AIza...
npx agentic-flow --agent coder --task "Build scraper" --provider gemini
```

### Example 4: MCP with OpenRouter
```bash
# Start proxy
export OPENROUTER_API_KEY=sk-or-v1-...
npx agentic-flow proxy --provider openrouter

# Use with MCP
export ENABLE_CLAUDE_FLOW_SDK=true
export ANTHROPIC_BASE_URL=http://localhost:3000
npx agentic-flow --agent coder --task "Use swarm_init to create agents"
```

## Environment Variables

| Variable | Required For | Description |
|----------|--------------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic provider | Your Anthropic API key |
| `GOOGLE_GEMINI_API_KEY` | Gemini provider | Your Google Gemini API key |
| `OPENROUTER_API_KEY` | OpenRouter provider | Your OpenRouter API key |
| `ENABLE_CLAUDE_FLOW_SDK` | MCP tools | Enable MCP integration |
| `ANTHROPIC_BASE_URL` | Proxy usage | Proxy URL (e.g., http://localhost:3000) |

## Automatic .env Loading

Create `.env` file in your project:

```bash
# .env
GOOGLE_GEMINI_API_KEY=AIza...
OPENROUTER_API_KEY=sk-or-v1-...
ANTHROPIC_API_KEY=sk-ant-...
ENABLE_CLAUDE_FLOW_SDK=true
```

agentic-flow automatically loads `.env` files! 🎉

## Documentation

- **[Standalone Proxy Guide](docs/STANDALONE_PROXY_GUIDE.md)** - Complete proxy setup guide
- **[MCP Proxy Fix](docs/V1.1.11_MCP_PROXY_FIX.md)** - Technical implementation details
- **[Complete Validation](docs/V1.1.11_COMPLETE_VALIDATION.md)** - Full validation results

## Troubleshooting

### Proxy won't start
```bash
# Check if port is in use
lsof -ti:3000

# Use different port
npx agentic-flow proxy --port 8080
```

### Claude Code hangs
```bash
# Verify proxy is running
curl http://localhost:3000/health

# Should return: {"status":"ok"}
```

### MCP tools not found
```bash
# Make sure ENABLE_CLAUDE_FLOW_SDK is set
export ENABLE_CLAUDE_FLOW_SDK=true

# Verify it's set
echo $ENABLE_CLAUDE_FLOW_SDK  # Should print: true
```

## Known Limitations

1. **Gemini Free Tier:** 10 requests/minute, 50/day
2. **Cursor Support:** Not yet supported (waiting for ANTHROPIC_BASE_URL feature)
3. **Proxy Security:** Currently binds to 0.0.0.0 (all interfaces)

## Upgrade from v1.1.10

**No changes required!** Just upgrade:

```bash
npm install -g agentic-flow@1.1.11
```

All existing functionality works exactly the same. New features are additive.

## What's Next (v1.2.0)

- [ ] `--host` option for localhost-only binding
- [ ] Built-in TLS/HTTPS support
- [ ] Request/response logging
- [ ] Prometheus metrics
- [ ] AWS Bedrock proxy
- [ ] Azure OpenAI proxy

## Support

- **GitHub:** https://github.com/ruvnet/agentic-flow
- **Issues:** https://github.com/ruvnet/agentic-flow/issues
- **NPM:** https://www.npmjs.com/package/agentic-flow

## Contributors

- @ruvnet - Main developer
- Claude Sonnet 4.5 - AI pair programmer

## License

MIT

---

**Published:** 2025-10-05
**Version:** 1.1.11
**Status:** Production Ready ✅
**Breaking Changes:** None
**Cost Savings:** Up to 100% with Gemini free tier 🎉
