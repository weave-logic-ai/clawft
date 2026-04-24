# Adding Custom MCP Servers to Agentic Flow

This guide shows you how to integrate custom MCP (Model Context Protocol) servers into agentic-flow.

---

## Table of Contents

1. [Overview](#overview)
2. [Integration Methods](#integration-methods)
3. [Method 1: In-SDK MCP Server (Recommended)](#method-1-in-sdk-mcp-server-recommended)
4. [Method 2: External MCP Server via Stdio](#method-2-external-mcp-server-via-stdio)
5. [Method 3: External MCP Server via NPM Package](#method-3-external-mcp-server-via-npm-package)
6. [Environment Variables](#environment-variables)
7. [Testing Your MCP Server](#testing-your-mcp-server)
8. [Troubleshooting](#troubleshooting)

---

## Overview

Agentic Flow supports three ways to add custom MCP servers:

| Method | Type | Performance | Use Case |
|--------|------|-------------|----------|
| **In-SDK** | In-process | Fastest (<1ms) | Custom tools, simple operations |
| **Stdio** | Subprocess | Fast (10-50ms) | Local MCP servers, CLI tools |
| **NPM Package** | Subprocess | Fast (10-50ms) | Published MCP packages |

**Current MCP Servers in Agentic Flow:**

1. **claude-flow-sdk** (In-SDK) - 6 tools
2. **claude-flow** (NPM) - 101 tools
3. **flow-nexus** (NPM) - 96 tools
4. **agentic-payments** (NPM) - 10 tools

---

## Integration Methods

### Method 1: In-SDK MCP Server (Recommended)

**Best for:** Custom tools, simple operations, fastest performance

In-SDK servers run directly in the agent process without spawning subprocesses.

#### Step 1: Create MCP Server File

Create `src/mcp/myCustomServer.ts`:

```typescript
import { createSdkMcpServer, tool } from '@anthropic-ai/claude-agent-sdk';
import { z } from 'zod';
import { logger } from '../utils/logger.js';

/**
 * Create an in-SDK MCP server with custom tools
 */
export const myCustomServer = createSdkMcpServer({
  name: 'my-custom-server',
  version: '1.0.0',

  tools: [
    // Tool 1: Simple calculator
    tool(
      'calculate',
      'Perform mathematical calculations',
      {
        operation: z.enum(['add', 'subtract', 'multiply', 'divide'])
          .describe('Mathematical operation'),
        a: z.number().describe('First number'),
        b: z.number().describe('Second number')
      },
      async ({ operation, a, b }) => {
        logger.info('Executing calculation', { operation, a, b });

        let result: number;
        switch (operation) {
          case 'add': result = a + b; break;
          case 'subtract': result = a - b; break;
          case 'multiply': result = a * b; break;
          case 'divide':
            if (b === 0) throw new Error('Division by zero');
            result = a / b;
            break;
        }

        return {
          operation,
          inputs: { a, b },
          result,
          message: `${a} ${operation} ${b} = ${result}`
        };
      }
    ),

    // Tool 2: Data storage
    tool(
      'store_data',
      'Store key-value data in memory',
      {
        key: z.string().describe('Storage key'),
        value: z.string().describe('Value to store'),
        namespace: z.string().optional().describe('Optional namespace')
      },
      async ({ key, value, namespace }) => {
        // Your storage logic here
        const fullKey = namespace ? `${namespace}:${key}` : key;

        // Store in memory, database, or filesystem
        logger.info('Storing data', { fullKey, value });

        return {
          success: true,
          key: fullKey,
          stored: value
        };
      }
    ),

    // Tool 3: API call wrapper
    tool(
      'fetch_data',
      'Fetch data from external API',
      {
        url: z.string().url().describe('API endpoint URL'),
        method: z.enum(['GET', 'POST']).default('GET').describe('HTTP method')
      },
      async ({ url, method }) => {
        logger.info('Fetching data', { url, method });

        try {
          const response = await fetch(url, { method });
          const data = await response.json();

          return {
            success: true,
            status: response.status,
            data
          };
        } catch (error: any) {
          return {
            success: false,
            error: error.message
          };
        }
      }
    )
  ]
});
```

#### Step 2: Register in Claude Agent

Edit `src/agents/claudeAgent.ts`:

```typescript
// Add import at top
import { myCustomServer } from '../mcp/myCustomServer.js';

// Inside claudeAgent function, around line 128:
if (process.env.ENABLE_MY_CUSTOM_MCP === 'true') {
  mcpServers['my-custom-server'] = myCustomServer;
}
```

#### Step 3: Enable and Use

```bash
# Enable your custom MCP server
export ENABLE_MY_CUSTOM_MCP=true

# Run agent with your custom tools
npx agentic-flow --agent coder --task "Calculate 42 + 58 using the calculate tool"
```

**Pros:**
- ✅ Fastest performance (in-process, <1ms latency)
- ✅ No subprocess overhead
- ✅ Full TypeScript type safety
- ✅ Easy to debug

**Cons:**
- ⚠️ Requires rebuild (`npm run build`)
- ⚠️ Can't use external CLI tools directly

---

### Method 2: External MCP Server via Stdio

**Best for:** Local MCP servers, custom CLI tools, existing MCP implementations

External stdio servers run as subprocesses and communicate via standard input/output.

#### Step 1: Create Standalone MCP Server

Create `my-mcp-server/index.js`:

```javascript
#!/usr/bin/env node
// Standalone MCP server using FastMCP or @modelcontextprotocol/sdk

import { FastMCP } from 'fastmcp';

const server = new FastMCP({
  name: 'my-external-server',
  version: '1.0.0'
});

// Add tools
server.addTool({
  name: 'custom_search',
  description: 'Search custom database',
  parameters: {
    type: 'object',
    properties: {
      query: { type: 'string', description: 'Search query' },
      limit: { type: 'number', description: 'Max results' }
    },
    required: ['query']
  },
  execute: async ({ query, limit = 10 }) => {
    // Your search logic
    return {
      results: [
        { id: 1, title: 'Result 1', score: 0.95 },
        { id: 2, title: 'Result 2', score: 0.87 }
      ],
      count: 2
    };
  }
});

server.addTool({
  name: 'send_notification',
  description: 'Send notification via custom service',
  parameters: {
    type: 'object',
    properties: {
      message: { type: 'string' },
      channel: { type: 'string', enum: ['email', 'slack', 'sms'] }
    },
    required: ['message', 'channel']
  },
  execute: async ({ message, channel }) => {
    // Send notification
    console.error(`Sending ${channel} notification: ${message}`);
    return { success: true, channel, message };
  }
});

// Start stdio transport
server.listen({ transport: 'stdio' });
```

Make it executable:
```bash
chmod +x my-mcp-server/index.js
```

#### Step 2: Register in Claude Agent

Edit `src/agents/claudeAgent.ts`:

```typescript
// Inside claudeAgent function, around line 169:
if (process.env.ENABLE_MY_EXTERNAL_MCP === 'true') {
  mcpServers['my-external-server'] = {
    type: 'stdio',
    command: 'node',
    args: ['/path/to/my-mcp-server/index.js'],
    env: {
      ...process.env,
      MY_SERVER_CONFIG: 'value'
    }
  };
}
```

#### Step 3: Enable and Use

```bash
# Enable external MCP server
export ENABLE_MY_EXTERNAL_MCP=true

# Run agent
npx agentic-flow --agent researcher --task "Search for 'AI trends' using custom_search"
```

**Pros:**
- ✅ No rebuild required
- ✅ Can use any language (Node, Python, Go, etc.)
- ✅ Full access to CLI tools and system commands
- ✅ Isolated process

**Cons:**
- ⚠️ Subprocess overhead (10-50ms startup)
- ⚠️ Requires absolute path or PATH setup
- ⚠️ More complex debugging

---

### Method 3: External MCP Server via NPM Package

**Best for:** Published MCP packages, shared tools, community servers

If your MCP server is published as an NPM package, integrate it like existing servers.

#### Step 1: Publish MCP Package

Create `package.json` for your MCP server:

```json
{
  "name": "my-mcp-package",
  "version": "1.0.0",
  "bin": {
    "my-mcp": "./dist/index.js"
  },
  "main": "./dist/index.js",
  "scripts": {
    "mcp": "node dist/index.js"
  }
}
```

Publish to NPM:
```bash
npm publish my-mcp-package
```

#### Step 2: Register in Claude Agent

Edit `src/agents/claudeAgent.ts`:

```typescript
// Inside claudeAgent function, around line 169:
if (process.env.ENABLE_MY_MCP_PACKAGE === 'true') {
  mcpServers['my-mcp-package'] = {
    type: 'stdio',
    command: 'npx',
    args: ['my-mcp-package@latest', 'mcp'],  // Or just 'my-mcp' if using bin
    env: {
      ...process.env,
      MY_MCP_API_KEY: process.env.MY_MCP_API_KEY || ''
    }
  };
}
```

#### Step 3: Enable and Use

```bash
# Enable MCP package
export ENABLE_MY_MCP_PACKAGE=true
export MY_MCP_API_KEY=your-api-key

# Run agent (will auto-install via npx)
npx agentic-flow --agent coder --task "Use tools from my-mcp-package"
```

**Pros:**
- ✅ Easy distribution via NPM
- ✅ Version management
- ✅ Auto-installation via npx
- ✅ Community sharing

**Cons:**
- ⚠️ Requires NPM publishing
- ⚠️ Network dependency for first install
- ⚠️ Subprocess overhead

---

## Environment Variables

Create environment variables to enable/disable your MCP servers:

```bash
# In-SDK servers
export ENABLE_CLAUDE_FLOW_SDK=true        # Built-in (6 tools)
export ENABLE_MY_CUSTOM_MCP=true          # Your custom in-SDK server

# External stdio servers
export ENABLE_CLAUDE_FLOW_MCP=true        # claude-flow (101 tools)
export ENABLE_FLOW_NEXUS_MCP=true         # flow-nexus (96 tools)
export ENABLE_AGENTIC_PAYMENTS_MCP=true   # agentic-payments (10 tools)
export ENABLE_MY_EXTERNAL_MCP=true        # Your external server
export ENABLE_MY_MCP_PACKAGE=true         # Your NPM package
```

**Convention:**
- In-SDK: `ENABLE_<NAME>_MCP=true`
- External: `ENABLE_<NAME>_MCP=true`
- Config: `<NAME>_API_KEY=xxx`, `<NAME>_CONFIG=xxx`

---

## Testing Your MCP Server

### Test In-SDK Server

```typescript
// Create test file: src/mcp/__tests__/myCustomServer.test.ts
import { myCustomServer } from '../myCustomServer.js';

describe('MyCustomServer', () => {
  it('should calculate correctly', async () => {
    const tools = myCustomServer.tools;
    const calculateTool = tools.find(t => t.name === 'calculate');

    const result = await calculateTool?.execute({
      operation: 'add',
      a: 10,
      b: 20
    });

    expect(result.result).toBe(30);
  });
});
```

### Test External Server

```bash
# Test standalone server
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | node my-mcp-server/index.js

# Test with agentic-flow
export ENABLE_MY_EXTERNAL_MCP=true
npx agentic-flow --agent coder --task "List all available tools"
```

### Integration Test

```bash
# Enable your server
export ENABLE_MY_CUSTOM_MCP=true

# Run with verbose logging
npx agentic-flow --agent coder --task "Test my custom tools" --verbose

# Check logs
# Should see: "Registered MCP server: my-custom-server"
```

---

## Code Reference: Existing Integrations

### In-SDK Example: claude-flow-sdk

**File:** `src/mcp/claudeFlowSdkServer.ts`

```typescript
export const claudeFlowSdkServer = createSdkMcpServer({
  name: 'claude-flow-sdk',
  version: '1.0.0',
  tools: [
    tool('memory_store', 'Store value in memory', {...}),
    tool('memory_retrieve', 'Retrieve value from memory', {...}),
    tool('swarm_init', 'Initialize swarm', {...}),
    // ... 6 tools total
  ]
});
```

**Registration:** `src/agents/claudeAgent.ts:128-130`
```typescript
if (process.env.ENABLE_CLAUDE_FLOW_SDK === 'true') {
  mcpServers['claude-flow-sdk'] = claudeFlowSdkServer;
}
```

### External Example: claude-flow

**Registration:** `src/agents/claudeAgent.ts:134-145`
```typescript
if (process.env.ENABLE_CLAUDE_FLOW_MCP === 'true') {
  mcpServers['claude-flow'] = {
    type: 'stdio',
    command: 'npx',
    args: ['claude-flow@alpha', 'mcp', 'start'],
    env: {
      ...process.env,
      MCP_AUTO_START: 'true',
      PROVIDER: provider
    }
  };
}
```

### External Example: flow-nexus

**Registration:** `src/agents/claudeAgent.ts:147-157`
```typescript
if (process.env.ENABLE_FLOW_NEXUS_MCP === 'true') {
  mcpServers['flow-nexus'] = {
    type: 'stdio',
    command: 'npx',
    args: ['flow-nexus@latest', 'mcp', 'start'],
    env: {
      ...process.env,
      FLOW_NEXUS_AUTO_START: 'true'
    }
  };
}
```

---

## Troubleshooting

### MCP Server Not Loading

**Problem:** Agent doesn't see your MCP tools

**Solutions:**
```bash
# 1. Check environment variable
echo $ENABLE_MY_CUSTOM_MCP  # Should be "true"

# 2. Rebuild if in-SDK
npm run build

# 3. Check logs
npx agentic-flow --agent coder --task "test" --verbose
# Should see: "Registered MCP server: my-custom-server"

# 4. Test server directly
node my-mcp-server/index.js  # For external servers
```

### Tools Not Executing

**Problem:** Tools registered but don't execute

**Solutions:**
```bash
# 1. Check tool name in task
# Correct: "Use the calculate tool to add 5 and 10"
# Wrong: "Calculate 5 + 10" (doesn't reference tool)

# 2. Check Zod schema validation
# Ensure parameters match schema exactly

# 3. Add logging
logger.info('Tool executed', { toolName, params, result });
```

### Subprocess Failures

**Problem:** External MCP server crashes or times out

**Solutions:**
```bash
# 1. Test server standalone
node my-mcp-server/index.js
# Type: {"jsonrpc":"2.0","method":"tools/list","id":1}

# 2. Check command/args
# Ensure absolute path or npx for NPM packages

# 3. Increase timeout
mcpServers['my-server'] = {
  type: 'stdio',
  command: 'node',
  args: ['/path/to/server.js'],
  timeout: 60000  // 60 seconds
};
```

### Type Errors

**Problem:** TypeScript errors with In-SDK tools

**Solutions:**
```typescript
// Ensure correct imports
import { createSdkMcpServer, tool } from '@anthropic-ai/claude-agent-sdk';
import { z } from 'zod';

// Rebuild after changes
npm run build

// Check types
npx tsc --noEmit
```

---

## Best Practices

1. **Start with In-SDK for Simple Tools**
   - Faster development iteration
   - Better type safety
   - Easier debugging

2. **Use External for Complex Operations**
   - System commands
   - Heavy computations
   - External APIs

3. **Use NPM Packages for Distribution**
   - Share with community
   - Version management
   - Easy updates

4. **Always Add Logging**
   ```typescript
   logger.info('Tool executed', { tool, params, result });
   ```

5. **Test Standalone First**
   - Test MCP server independently
   - Then integrate with agentic-flow

6. **Document Your Tools**
   - Clear descriptions
   - Parameter documentation
   - Example usage

---

## Summary

**Quick Reference:**

| Need | Method | Steps |
|------|--------|-------|
| Simple custom tool | In-SDK | 1. Create `src/mcp/myServer.ts`<br>2. Register in `claudeAgent.ts`<br>3. `npm run build` |
| Local MCP server | Stdio | 1. Create standalone server<br>2. Register with command/args<br>3. Enable env var |
| Published package | NPM | 1. Publish to NPM<br>2. Register with `npx` command<br>3. Enable env var |

**Next Steps:**
- See `src/mcp/claudeFlowSdkServer.ts` for in-SDK example
- See `src/agents/claudeAgent.ts:134-169` for external examples
- Read [MCP Specification](https://modelcontextprotocol.io) for protocol details

---

**Need help?** Open an issue on [GitHub](https://github.com/ruvnet/agentic-flow/issues) or check the [MCP docs](https://modelcontextprotocol.io).
