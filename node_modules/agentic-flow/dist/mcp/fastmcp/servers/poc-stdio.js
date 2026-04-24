#!/usr/bin/env node
// POC: FastMCP server with stdio transport and 2 basic tools
import { FastMCP } from 'fastmcp';
import { z } from 'zod';
import { execSync } from 'child_process';
console.error('🚀 Starting FastMCP POC Server (stdio transport)...');
// Create server
const server = new FastMCP({
    name: 'fastmcp-poc',
    version: '0.1.0'
});
// Tool 1: Memory Store
server.addTool({
    name: 'memory_store',
    description: 'Store a value in persistent memory',
    parameters: z.object({
        key: z.string().min(1).describe('Memory key'),
        value: z.string().describe('Value to store'),
        namespace: z.string().optional().default('default').describe('Memory namespace'),
        ttl: z.number().positive().optional().describe('Time-to-live in seconds')
    }),
    execute: async ({ key, value, namespace, ttl }) => {
        try {
            const cmd = [
                'npx claude-flow@alpha memory store',
                `"${key}"`,
                `"${value}"`,
                `--namespace "${namespace}"`,
                ttl ? `--ttl ${ttl}` : ''
            ].filter(Boolean).join(' ');
            const result = execSync(cmd, { encoding: 'utf-8', maxBuffer: 10 * 1024 * 1024 });
            // Return as text content (fastmcp requirement)
            return JSON.stringify({
                success: true,
                key,
                namespace,
                size: value.length,
                ttl,
                timestamp: new Date().toISOString(),
                message: 'Memory stored successfully'
            }, null, 2);
        }
        catch (error) {
            throw new Error(`Failed to store memory: ${error.message}`);
        }
    }
});
// Tool 2: Memory Retrieve
server.addTool({
    name: 'memory_retrieve',
    description: 'Retrieve a value from persistent memory',
    parameters: z.object({
        key: z.string().min(1).describe('Memory key'),
        namespace: z.string().optional().default('default').describe('Memory namespace')
    }),
    execute: async ({ key, namespace }) => {
        try {
            const cmd = `npx claude-flow@alpha memory retrieve "${key}" --namespace "${namespace}"`;
            const result = execSync(cmd, { encoding: 'utf-8', maxBuffer: 10 * 1024 * 1024 });
            // Return as text content (fastmcp requirement)
            return JSON.stringify({
                success: true,
                key,
                namespace,
                value: result.trim(),
                timestamp: new Date().toISOString()
            }, null, 2);
        }
        catch (error) {
            throw new Error(`Failed to retrieve memory: ${error.message}`);
        }
    }
});
console.error('📦 Registered 2 tools: memory_store, memory_retrieve');
console.error('🔌 Starting stdio transport...');
// Start with stdio transport
server.start({ transportType: 'stdio' }).then(() => {
    console.error('✅ FastMCP POC server running on stdio');
}).catch((error) => {
    console.error('❌ Failed to start server:', error);
    process.exit(1);
});
//# sourceMappingURL=poc-stdio.js.map