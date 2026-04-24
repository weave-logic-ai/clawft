# Release Notes - agentic-flow v1.2.0

**Release Date:** 2025-10-06
**Version:** 1.2.0
**Major Feature:** MCP CLI for User-Friendly Server Configuration

---

## 🚀 What's New

### MCP CLI Manager - Add Custom Servers Without Code Editing!

**The Problem:** Previously, adding custom MCP servers required editing TypeScript code and rebuilding the project.

**The Solution:** New CLI commands let end users add custom MCP servers without any code editing, similar to Claude Desktop's approach.

### New Commands

```bash
# Add MCP server (Claude Desktop style JSON config)
npx agentic-flow mcp add weather '{"command":"npx","args":["-y","weather-mcp"],"env":{"API_KEY":"xxx"}}'

# Add MCP server (simple flag-based config)
npx agentic-flow mcp add github --npm @modelcontextprotocol/server-github --env "GITHUB_TOKEN=ghp_xxx"

# Add local MCP server
npx agentic-flow mcp add my-tools --local /path/to/server.js

# List configured servers
npx agentic-flow mcp list

# Enable/disable servers
npx agentic-flow mcp enable weather
npx agentic-flow mcp disable weather

# Remove server
npx agentic-flow mcp remove weather

# Update configuration
npx agentic-flow mcp update weather --env "API_KEY=new-key"
```

### Key Benefits

- ✅ **No Code Editing** - Add servers via CLI commands
- ✅ **No TypeScript Required** - Suitable for non-developers
- ✅ **No Rebuilding** - Changes take effect immediately
- ✅ **Claude Desktop Compatible** - Same JSON config format
- ✅ **Automatic Loading** - Servers load automatically in all agents
- ✅ **Persistent Storage** - Configuration stored in `~/.agentic-flow/mcp-config.json`
- ✅ **100% Backward Compatible** - No breaking changes

---

## 📚 Documentation

### User Guides
- **[ADDING-MCP-SERVERS-CLI.md](./guides/ADDING-MCP-SERVERS-CLI.md)** (516 lines)
  - Complete end-user guide for CLI commands
  - Step-by-step examples
  - Popular MCP servers list
  - Troubleshooting section

- **[ADDING-MCP-SERVERS.md](./guides/ADDING-MCP-SERVERS.md)** (570 lines)
  - Developer integration guide
  - Code examples for in-SDK and external servers
  - Best practices

### Validation Reports
- **[MCP-CLI-VALIDATION-REPORT.md](./mcp-validation/MCP-CLI-VALIDATION-REPORT.md)**
  - Complete validation results
  - Live agent test with strange-loops MCP
  - 100% test pass rate (8/8 tests)

- **[IMPLEMENTATION-SUMMARY.md](./mcp-validation/IMPLEMENTATION-SUMMARY.md)**
  - Technical implementation details
  - Before/after comparison
  - Security considerations

- **[strange-loops-test.md](./mcp-validation/strange-loops-test.md)**
  - Live agent test output
  - Proof of integration working

---

## 🔧 Technical Details

### New Files

**Implementation:**
- `src/cli/mcp-manager.ts` (617 lines)
  - Complete CLI tool with 8 commands
  - JSON and flag-based config support
  - Configuration management

**Documentation:**
- `docs/guides/ADDING-MCP-SERVERS-CLI.md`
- `docs/guides/ADDING-MCP-SERVERS.md`
- `docs/mcp-validation/MCP-CLI-VALIDATION-REPORT.md`
- `docs/mcp-validation/IMPLEMENTATION-SUMMARY.md`
- `docs/mcp-validation/strange-loops-test.md`
- `docs/NPM-PUBLISH-GUIDE-v1.2.0.md`

### Modified Files

**Agent Integration:**
- `src/agents/claudeAgent.ts` (lines 171-203)
  - Auto-load user-configured MCP servers
  - Read from `~/.agentic-flow/mcp-config.json`
  - Merge with built-in servers

**Documentation:**
- Root `README.md` - Added "Add Custom MCP Servers" section
- NPM `README.md` - Added "Add Custom MCP Servers" section

### Configuration Format

```json
{
  "servers": {
    "server-name": {
      "enabled": true,
      "type": "npm" | "local",
      "package": "npm-package@version",
      "command": "npx" | "node" | "python3" | "docker",
      "args": ["arg1", "arg2"],
      "env": {
        "API_KEY": "value"
      },
      "description": "Server description"
    }
  }
}
```

---

## ✅ Validation Results

### Live Agent Test

**Test:** Added strange-loops MCP server and ran agent

**Command:**
```bash
npx agentic-flow mcp add strange-loops '{"command":"npx","args":["-y","strange-loops","mcp","start"]}'
npx agentic-flow --agent researcher --task "List all MCP tools"
```

**Result:** ✅ SUCCESS
- Agent loaded strange-loops MCP server
- Detected all 9 tools from the server
- Tools immediately available to agent

**Console Output:**
```
[agentic-flow] Loaded MCP server: strange-loops
```

**Tools Detected:**
1. `mcp__strange-loops__system_info`
2. `mcp__strange-loops__benchmark_run`
3. `mcp__strange-loops__nano_swarm_create`
4. `mcp__strange-loops__nano_swarm_run`
5. `mcp__strange-loops__quantum_container_create`
6. `mcp__strange-loops__quantum_superposition`
7. `mcp__strange-loops__quantum_measure`
8. `mcp__strange-loops__temporal_predictor_create`
9. `mcp__strange-loops__temporal_predict`

### Test Results Summary

| Test | Status | Evidence |
|------|--------|----------|
| TypeScript Build | ✅ PASS | Compilation successful |
| CLI Commands | ✅ PASS | All commands working |
| JSON Config | ✅ PASS | Claude Desktop format supported |
| Flag Config | ✅ PASS | Flag-based format supported |
| Config Persistence | ✅ PASS | Stored in ~/.agentic-flow/ |
| Agent Integration | ✅ PASS | Auto-load working |
| Tool Discovery | ✅ PASS | All tools detected |
| Backward Compatibility | ✅ PASS | No breaking changes |

**Overall:** ✅ **100% PASS RATE (8/8 tests)**

---

## 🎯 Use Cases

### Add GitHub MCP Server
```bash
# Add official GitHub MCP
npx agentic-flow mcp add github \
  --npm @modelcontextprotocol/server-github \
  --env "GITHUB_TOKEN=ghp_your_token"

# Use it with agent
npx agentic-flow --agent coder --task "Create issue on repo owner/repo"
```

### Add Weather MCP Server
```bash
# Add weather MCP
npx agentic-flow mcp add weather \
  --npm weather-mcp \
  --env "WEATHER_API_KEY=your_key"

# Use it
npx agentic-flow --agent researcher --task "Get weather forecast for San Francisco"
```

### Add Local Development Server
```bash
# Add local MCP server
npx agentic-flow mcp add dev-tools \
  --local /home/user/projects/my-mcp/server.js

# Use it
npx agentic-flow --agent coder --task "Use my custom tools"
```

---

## 🚫 Breaking Changes

**None.** This is a purely additive feature.

- Existing environment variable approach still works
- Existing code-based MCP registration still works
- User-configured servers load alongside built-in servers
- No changes to existing APIs

---

## 🔮 Future Enhancements (v1.2.1+)

### Planned Commands (Not in v1.2.0)

**1. Test Command**
```bash
npx agentic-flow mcp test weather
# Will test if server starts and responds correctly
```

**2. Info Command**
```bash
npx agentic-flow mcp info weather
# Will show detailed server information
```

**3. Tools Command**
```bash
npx agentic-flow mcp tools weather
# Will list all tools from specific server
```

**4. Export/Import**
```bash
npx agentic-flow mcp export > config.json
npx agentic-flow mcp import < config.json
# Share configurations with team
```

### Other Future Features
- Shell completion (bash/zsh)
- API key encryption in config
- Server signature verification
- Automatic server updates
- Usage statistics

---

## 📦 Installation

### Global Install
```bash
npm install -g agentic-flow@1.2.0
```

### Use with npx (No Install)
```bash
npx agentic-flow@1.2.0 mcp add my-server --npm my-mcp-package
```

### Upgrade from Previous Version
```bash
npm update -g agentic-flow
```

---

## 🔗 Links

**GitHub:**
- Pull Request: https://github.com/ruvnet/agentic-flow/pull/4
- Repository: https://github.com/ruvnet/agentic-flow
- Issues: https://github.com/ruvnet/agentic-flow/issues

**NPM:**
- Package: https://www.npmjs.com/package/agentic-flow
- Version: https://www.npmjs.com/package/agentic-flow/v/1.2.0

**Documentation:**
- User Guide: https://github.com/ruvnet/agentic-flow/blob/main/agentic-flow/docs/guides/ADDING-MCP-SERVERS-CLI.md
- Developer Guide: https://github.com/ruvnet/agentic-flow/blob/main/agentic-flow/docs/guides/ADDING-MCP-SERVERS.md

---

## 👏 Credits

**Built by:** [@ruvnet](https://github.com/ruvnet)
**Implemented with:** [Claude Code](https://claude.com/claude-code)
**Based on:** [Claude Agent SDK](https://docs.claude.com/en/api/agent-sdk) by Anthropic

---

## 📋 Quick Reference

| Task | Command |
|------|---------|
| Add NPM server | `npx agentic-flow mcp add NAME --npm PACKAGE` |
| Add local server | `npx agentic-flow mcp add NAME --local PATH` |
| Add with JSON | `npx agentic-flow mcp add NAME '{"command":...}'` |
| List servers | `npx agentic-flow mcp list` |
| Enable server | `npx agentic-flow mcp enable NAME` |
| Disable server | `npx agentic-flow mcp disable NAME` |
| Remove server | `npx agentic-flow mcp remove NAME` |

**Config File:** `~/.agentic-flow/mcp-config.json`

---

**Release Status:** ✅ READY FOR PRODUCTION

**Validation:** ✅ 100% TEST PASS RATE

**Documentation:** ✅ COMPLETE

**Ready to Publish:** ✅ YES
