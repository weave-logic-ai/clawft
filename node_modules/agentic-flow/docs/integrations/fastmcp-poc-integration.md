# FastMCP POC Integration Guide

## Overview

The FastMCP POC provides a minimal stdio-based MCP server with 2 basic tools (`memory_store`, `memory_retrieve`) to validate the fastmcp framework integration with agentic-flow.

## Architecture

```
agentic-flow/
├── src/mcp/fastmcp/
│   ├── servers/
│   │   └── poc-stdio.ts      # POC server (stdio transport)
│   ├── tools/
│   │   └── memory/
│   │       ├── store.ts      # Memory store tool
│   │       └── retrieve.ts   # Memory retrieve tool
│   ├── types/
│   │   └── index.ts          # TypeScript definitions
│   └── config/
│       └── mcp-config.json   # MCP client configuration
```

## Installation

Already installed as part of agentic-flow:
```bash
npm install  # fastmcp & zod included
npm run build
```

## Usage

### Option 1: Direct CLI Test
```bash
# Run test script
npm run test:fastmcp

# Or manually:
npm run mcp:fastmcp-poc
```

### Option 2: Claude Code Integration
Add to Claude Code's MCP settings (`~/.config/claude/mcp.json`):
```json
{
  "mcpServers": {
    "fastmcp-poc": {
      "command": "node",
      "args": ["/workspaces/flow-cloud/docker/claude-agent-sdk/dist/mcp/fastmcp/servers/poc-stdio.js"],
      "env": {}
    }
  }
}
```

### Option 3: NPX Integration
```bash
# From package root
npx agentic-flow mcp:fastmcp-poc
```

## Available Tools

### 1. memory_store
Store a value in persistent memory via claude-flow.

**Parameters:**
- `key` (string, required): Memory key
- `value` (string, required): Value to store
- `namespace` (string, optional): Memory namespace (default: "default")
- `ttl` (number, optional): Time-to-live in seconds

**Example:**
```json
{
  "name": "memory_store",
  "arguments": {
    "key": "test-key",
    "value": "test-value",
    "namespace": "poc-test",
    "ttl": 3600
  }
}
```

**Returns:**
```json
{
  "success": true,
  "key": "test-key",
  "namespace": "poc-test",
  "size": 10,
  "ttl": 3600,
  "timestamp": "2025-10-03T20:41:00.000Z",
  "message": "Memory stored successfully"
}
```

### 2. memory_retrieve
Retrieve a value from persistent memory.

**Parameters:**
- `key` (string, required): Memory key
- `namespace` (string, optional): Memory namespace (default: "default")

**Example:**
```json
{
  "name": "memory_retrieve",
  "arguments": {
    "key": "test-key",
    "namespace": "poc-test"
  }
}
```

**Returns:**
```json
{
  "success": true,
  "key": "test-key",
  "namespace": "poc-test",
  "value": "test-value",
  "timestamp": "2025-10-03T20:42:00.000Z"
}
```

## Testing

### Automated Test
```bash
npm run test:fastmcp
```

### Manual Test with MCP Protocol
```bash
# Test memory_store
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"memory_store","arguments":{"key":"test","value":"hello","namespace":"poc"}}}' | npm run mcp:fastmcp-poc

# Test memory_retrieve
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"memory_retrieve","arguments":{"key":"test","namespace":"poc"}}}' | npm run mcp:fastmcp-poc
```

## Integration Status

✅ **Phase 0 Complete** - POC with stdio transport
- [x] Created POC server with 2 tools
- [x] Implemented stdio transport
- [x] Tools use claude-flow backend via execSync
- [x] TypeScript compilation successful
- [x] Test scripts created
- [x] MCP configuration provided

🔄 **Phase 1 Pending** - In-process migration (claude-flow-sdk)
- [ ] Migrate 6 claude-flow-sdk tools to fastmcp
- [ ] Remove execSync, use direct imports
- [ ] Validate all tools work in stdio

## Known Limitations

1. **Backend Dependency**: Currently calls `npx claude-flow@alpha` via execSync
   - Phase 1 will replace with direct imports for in-process execution

2. **Progress Reporting**: Removed temporarily (fastmcp API investigation needed)
   - Context.onProgress not available in current fastmcp API

3. **Error Handling**: Basic throw-based errors
   - Will be enhanced with structured error types in Phase 1

## Next Steps (Phase 1)

1. Migrate claude-flow-sdk tools (6 tools):
   - memory_store, memory_retrieve, memory_search
   - swarm_init, agent_spawn, task_orchestrate

2. Replace execSync with direct imports:
   ```typescript
   // Before (Phase 0):
   const result = execSync(`npx claude-flow@alpha memory store ...`);

   // After (Phase 1):
   import { MemoryManager } from '../../memory/manager.js';
   const memory = new MemoryManager();
   const result = await memory.store(key, value, namespace, ttl);
   ```

3. Add progress reporting when fastmcp API clarified

4. Implement comprehensive error types

## Resources

- FastMCP Implementation Plan: `docs/mcp/fastmcp-implementation-plan.md`
- POC Server: `src/mcp/fastmcp/servers/poc-stdio.ts`
- Tool Definitions: `src/mcp/fastmcp/tools/memory/`
- Test Script: `scripts/test-fastmcp-poc.sh`
- MCP Config: `src/mcp/fastmcp/config/mcp-config.json`
