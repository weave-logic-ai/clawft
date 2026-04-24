# Hotfix v1.2.1 - Critical Bug Fixes

**Release Date:** 2025-10-06
**Type:** Patch Release
**Fixes:** 2 critical issues found in v1.2.0

---

## Issues Fixed

### Issue #1: CLI Router Not Wired 🔴 CRITICAL - FIXED

**Problem:** The main CLI didn't route `mcp` subcommands to `mcp-manager.js`

**Symptoms:**
```bash
npx agentic-flow mcp add    # ❌ Started MCP server instead
npx agentic-flow mcp list   # ❌ Started MCP server instead
```

**Root Cause:** The CLI parser detected `mcp` as a mode but didn't distinguish between MCP manager commands (add, list, remove) and MCP server commands (start, stop, status).

**Fix Applied:**
- Updated `src/utils/cli.ts` to detect MCP manager subcommands
- Added new mode `'mcp-manager'` to CliOptions
- Added routing in `src/cli-proxy.ts` to spawn mcp-manager.js for manager commands

**Result:** ✅ FIXED
```bash
npx agentic-flow mcp list   # ✅ Now shows configured servers
npx agentic-flow mcp add    # ✅ Now adds servers to config
```

**Test Output:**
```
$ node dist/cli-proxy.js mcp list
Configured MCP Servers:

✅ strange-loops (enabled)
   Type: local
   Command: npx -y strange-loops mcp start
   Description: Strange Loops MCP server for testing
```

---

### Issue #2: Model Optimization Filter 🟡 ENHANCEMENT - FIXED

**Problem:** `--optimize` flag selected models without tool support (DeepSeek R1)

**Symptoms:**
```bash
npx agentic-flow --agent coder --task "..." --optimize
# Selected: DeepSeek R1 (doesn't support tool use)
# Error: Tools not available
```

**Root Cause:** Model optimizer didn't filter by tool-use capability. All agents have MCP tools available, so models must support function calling.

**Fix Applied:**
1. Added `supports_tools` field to all models in MODEL_DATABASE
2. Set `deepseek-r1: supports_tools: false` (confirmed no tool support)
3. Added `requiresTools` parameter to OptimizationCriteria
4. Added filtering logic in `ModelOptimizer.optimize()`
5. Set `requiresTools: true` in cli-proxy.ts when optimizing

**Result:** ✅ FIXED

**Test Output:**
```javascript
// Test 1: WITH tool requirement (as used in CLI)
ModelOptimizer.optimize({
  agent: 'coder',
  task: 'Simple hello world',
  priority: 'cost',
  requiresTools: true  // <-- Filters out DeepSeek R1
});
// Selected: DeepSeek Chat V3.1 (supports tools)

// Test 2: WITHOUT tool requirement
ModelOptimizer.optimize({
  agent: 'coder',
  task: 'Simple hello world',
  priority: 'cost',
  requiresTools: false
});
// Selected: DeepSeek R1 (cheapest, no tools needed)
```

---

## Files Changed

### src/utils/cli.ts
**Changes:**
- Added detection for MCP manager commands (add, list, remove, etc.)
- Added new mode: `'mcp-manager'`
- Router now distinguishes between MCP manager and MCP server commands

```typescript
// Check for MCP command
if (args[0] === 'mcp') {
  const mcpSubcommand = args[1];

  // MCP Manager commands (CLI configuration)
  const managerCommands = ['add', 'list', 'remove', 'enable', 'disable', 'update', 'test', 'info', 'export', 'import'];

  if (managerCommands.includes(mcpSubcommand)) {
    options.mode = 'mcp-manager';
    return options;
  }

  // MCP Server commands (start/stop server)
  options.mode = 'mcp';
  options.mcpCommand = mcpSubcommand || 'start';
  options.mcpServer = args[2] || 'all';
  return options;
}
```

### src/cli-proxy.ts
**Changes:**
- Added routing for `'mcp-manager'` mode
- Spawns `mcp-manager.js` with proper args
- Added `requiresTools: true` to model optimization

```typescript
if (options.mode === 'mcp-manager') {
  // Handle MCP manager commands (add, list, remove, etc.)
  const { spawn } = await import('child_process');
  const { resolve, dirname } = await import('path');
  const { fileURLToPath } = await import('url');

  const __filename = fileURLToPath(import.meta.url);
  const __dirname = dirname(__filename);
  const mcpManagerPath = resolve(__dirname, './cli/mcp-manager.js');

  // Pass all args after 'mcp' to mcp-manager
  const mcpArgs = process.argv.slice(3);

  const proc = spawn('node', [mcpManagerPath, ...mcpArgs], {
    stdio: 'inherit'
  });

  proc.on('exit', (code) => {
    process.exit(code || 0);
  });

  process.on('SIGINT', () => proc.kill('SIGINT'));
  process.on('SIGTERM', () => proc.kill('SIGTERM'));
  return;
}
```

```typescript
// Apply model optimization if requested
if (options.optimize && options.agent && options.task) {
  const recommendation = ModelOptimizer.optimize({
    agent: options.agent,
    task: options.task,
    priority: options.optimizePriority || 'balanced',
    maxCostPerTask: options.maxCost,
    requiresTools: true // Agents have MCP tools available, so require tool support
  });
```

### src/utils/modelOptimizer.ts
**Changes:**
- Added `requiresTools?: boolean` to OptimizationCriteria interface
- Added `supports_tools` field to all models in MODEL_DATABASE
- Added filtering logic to exclude models without tool support when required

```typescript
export interface OptimizationCriteria {
  agent: string;
  task: string;
  priority?: 'quality' | 'balanced' | 'cost' | 'speed' | 'privacy';
  maxCostPerTask?: number;
  requiresReasoning?: boolean;
  requiresMultimodal?: boolean;
  requiresTools?: boolean; // NEW: Filter models that support tool/function calling
  taskComplexity?: 'simple' | 'moderate' | 'complex' | 'expert';
}
```

```typescript
// Filter models that support tools if required
let availableModels = Object.entries(MODEL_DATABASE);

if (criteria.requiresTools) {
  availableModels = availableModels.filter(([key, model]) => model.supports_tools !== false);
  logger.info(`Filtered to ${availableModels.length} models with tool support`);
}

// Score all models
const scoredModels = availableModels.map(([key, model]) => {
  // ... scoring logic
});
```

**Model Database Updates:**
```typescript
'deepseek-r1': {
  // ...
  supports_tools: false, // DeepSeek R1 does NOT support tool/function calling
  weaknesses: ['newer-model', 'no-tool-use'],
  // ...
},
'deepseek-chat-v3': {
  // ...
  supports_tools: true,
  // ...
},
// All other models: supports_tools: true (except local ONNX)
```

---

## Test Results

### Test 1: MCP CLI Routing ✅ PASS
```bash
# Before (v1.2.0):
$ npx agentic-flow mcp list
Starting MCP server... (WRONG)

# After (v1.2.1):
$ npx agentic-flow mcp list
Configured MCP Servers:
✅ strange-loops (enabled)
```

### Test 2: Model Optimizer Tool Filtering ✅ PASS
```javascript
// WITH tool requirement (default for CLI)
Selected: DeepSeek Chat V3.1 (supports tools) ✅

// WITHOUT tool requirement
Selected: DeepSeek R1 (no tools, cheaper) ✅
```

### Test 3: End-to-End Agent Execution ✅ PASS
```bash
$ npx agentic-flow --agent coder --task "Create calculator" --optimize
Selected: Claude Sonnet 4.5 (supports tools) ✅
Agent successfully used MCP tools ✅
```

---

## Breaking Changes

**None.** This is a patch release with bug fixes only.

---

## Upgrade Instructions

### For Users
```bash
# Update globally
npm install -g agentic-flow@1.2.1

# Or use npx (always uses latest)
npx agentic-flow mcp list
```

### Verify Fix
```bash
# Test MCP CLI routing
npx agentic-flow mcp list

# Test model optimization
npx agentic-flow --agent coder --task "test" --optimize
# Should NOT select DeepSeek R1
```

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.2.1 | 2025-10-06 | **HOTFIX** - MCP CLI routing + model optimizer tool filtering |
| 1.2.0 | 2025-10-06 | MCP CLI for user-friendly configuration |
| 1.1.14 | 2025-10-05 | OpenRouter proxy fix (80% success rate) |

---

## Credits

**Reported By:** Testing and validation
**Fixed By:** Claude Code
**Release Type:** Patch (Point Release)

---

## Summary

✅ **Both Issues Fixed**
- MCP CLI commands now route correctly
- Model optimizer filters by tool support

✅ **All Tests Pass**
- MCP list/add commands work
- Optimization selects tool-supporting models

✅ **Ready for NPM Publish**
- Version: 1.2.1
- Type: Patch release
- Breaking changes: None

---

**Status:** ✅ READY FOR RELEASE
