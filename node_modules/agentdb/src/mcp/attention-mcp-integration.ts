/**
 * Attention MCP Integration Helper
 * This file provides the integration code for adding attention tools to agentdb-mcp-server.ts
 *
 * To integrate, add the following to agentdb-mcp-server.ts:
 *
 * 1. Import (near top with other imports):
 *    import {
 *      attentionTools,
 *      attentionToolHandlers,
 *      attentionHelperFunctions
 *    } from './attention-mcp-integration.js';
 *
 * 2. In tools array (merge with existing tools):
 *    const tools = [
 *      ...existingTools,
 *      ...attentionTools,
 *    ];
 *
 * 3. In CallToolRequestSchema handler, add helper functions at the top:
 *    server.setRequestHandler(CallToolRequestSchema, async (request) => {
 *      const { name, arguments: args } = request.params;
 *
 *      // Add helper functions (paste attentionHelperFunctions here)
 *      ${attentionHelperFunctions}
 *
 *      switch (name) {
 *        // ... existing cases ...
 *        ${attentionToolHandlers}
 *        // ... rest of cases ...
 *      }
 *    });
 */

import {
  attentionTools,
  attentionComputeHandler,
  attentionBenchmarkHandler,
  attentionConfigureHandler,
  attentionMetricsHandler,
  attentionHelperFunctions,
} from './attention-tools-handlers.js';

/**
 * Export tools for registration
 */
export { attentionTools };

/**
 * Combined tool handlers for easy integration
 */
export const attentionToolHandlers = `
        ${attentionComputeHandler}

        ${attentionBenchmarkHandler}

        ${attentionConfigureHandler}

        ${attentionMetricsHandler}
`;

/**
 * Helper functions to be added to MCP server
 */
export { attentionHelperFunctions };

/**
 * Quick reference for tool names
 */
export const attentionToolNames = [
  'agentdb_attention_compute',
  'agentdb_attention_benchmark',
  'agentdb_attention_configure',
  'agentdb_attention_metrics',
];

/**
 * Integration instructions
 */
export const integrationInstructions = `
# MCP Server Integration Instructions

## Step 1: Import Attention Tools

Add to imports section:
\`\`\`typescript
import {
  attentionTools,
  attentionToolHandlers,
  attentionHelperFunctions
} from './attention-mcp-integration.js';
\`\`\`

## Step 2: Register Tools

Add to tools array:
\`\`\`typescript
const tools = [
  // ... existing tools ...
  ...attentionTools,
];
\`\`\`

## Step 3: Add Tool Handlers

In the CallToolRequestSchema handler, add helper functions and cases:
\`\`\`typescript
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  // Add helper functions at the top of the handler
  ${attentionHelperFunctions.slice(0, 200)}...

  switch (name) {
    // ... existing cases ...

    // Attention mechanism tools
    ${attentionToolHandlers.slice(0, 200)}...

    // ... rest of cases ...
  }
});
\`\`\`

## Step 4: Test the Integration

Test with MCP client:
\`\`\`bash
# Start MCP server
npx agentdb mcp start

# Test tools (via Claude Desktop or MCP client)
# Tool: agentdb_attention_compute
# Tool: agentdb_attention_benchmark
# Tool: agentdb_attention_configure
# Tool: agentdb_attention_metrics
\`\`\`

## Available Tools

1. **agentdb_attention_compute**: Compute attention for query-key-value triplets
2. **agentdb_attention_benchmark**: Benchmark attention mechanism performance
3. **agentdb_attention_configure**: Configure attention mechanism parameters
4. **agentdb_attention_metrics**: Get attention usage metrics and statistics
`;
