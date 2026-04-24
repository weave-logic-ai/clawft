#!/usr/bin/env node
// FastMCP implementation of claude-flow-sdk server (in-process, 6 tools)
// Phase 1: Migration from claudeFlowSdkServer.ts to fastmcp
import { FastMCP } from 'fastmcp';
import { z } from 'zod';
import { execSync } from 'child_process';
import { logger } from '../../../utils/logger.js';
console.error('🚀 Starting FastMCP claude-flow-sdk Server...');
// Create server
const server = new FastMCP({
    name: 'claude-flow-sdk',
    version: '1.0.0'
});
// Tool 1: Memory Store
server.addTool({
    name: 'memory_store',
    description: 'Store a value in persistent memory with optional namespace and TTL',
    parameters: z.object({
        key: z.string().min(1).describe('Memory key'),
        value: z.string().describe('Value to store'),
        namespace: z.string().optional().default('default').describe('Memory namespace'),
        ttl: z.number().positive().optional().describe('Time-to-live in seconds')
    }),
    execute: async ({ key, value, namespace, ttl }) => {
        try {
            logger.info('Storing memory', { key, namespace });
            const cmd = [
                'npx claude-flow@alpha memory store',
                `"${key}"`,
                `"${value}"`,
                `--namespace "${namespace}"`,
                ttl ? `--ttl ${ttl}` : ''
            ].filter(Boolean).join(' ');
            const result = execSync(cmd, { encoding: 'utf-8', maxBuffer: 10 * 1024 * 1024 });
            logger.info('Memory stored successfully', { key });
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
            logger.error('Failed to store memory', { error: error.message });
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
// Tool 3: Memory Search
server.addTool({
    name: 'memory_search',
    description: 'Search for keys matching a pattern in memory',
    parameters: z.object({
        pattern: z.string().min(1).describe('Search pattern (supports wildcards)'),
        namespace: z.string().optional().describe('Memory namespace to search in'),
        limit: z.number().positive().optional().default(10).describe('Maximum results to return')
    }),
    execute: async ({ pattern, namespace, limit }) => {
        try {
            const cmd = `npx claude-flow@alpha memory search "${pattern}"${namespace ? ` --namespace "${namespace}"` : ''} --limit ${limit}`;
            const result = execSync(cmd, { encoding: 'utf-8', maxBuffer: 10 * 1024 * 1024 });
            return JSON.stringify({
                success: true,
                pattern,
                namespace: namespace || 'all',
                results: result.trim(),
                limit,
                timestamp: new Date().toISOString()
            }, null, 2);
        }
        catch (error) {
            throw new Error(`Failed to search memory: ${error.message}`);
        }
    }
});
// Tool 4: Swarm Init
server.addTool({
    name: 'swarm_init',
    description: 'Initialize a multi-agent swarm with specified topology',
    parameters: z.object({
        topology: z.enum(['mesh', 'hierarchical', 'ring', 'star']).describe('Swarm topology'),
        maxAgents: z.number().positive().optional().default(8).describe('Maximum number of agents'),
        strategy: z.enum(['balanced', 'specialized', 'adaptive']).optional().default('balanced').describe('Agent distribution strategy')
    }),
    execute: async ({ topology, maxAgents, strategy }) => {
        try {
            const cmd = `npx claude-flow@alpha swarm init --topology ${topology} --max-agents ${maxAgents} --strategy ${strategy}`;
            const result = execSync(cmd, { encoding: 'utf-8', maxBuffer: 10 * 1024 * 1024 });
            return JSON.stringify({
                success: true,
                topology,
                maxAgents,
                strategy,
                result: result.trim(),
                timestamp: new Date().toISOString()
            }, null, 2);
        }
        catch (error) {
            throw new Error(`Failed to initialize swarm: ${error.message}`);
        }
    }
});
// Tool 5: Agent Spawn
server.addTool({
    name: 'agent_spawn',
    description: 'Spawn a new agent in the swarm',
    parameters: z.object({
        type: z.enum(['researcher', 'coder', 'analyst', 'optimizer', 'coordinator']).describe('Agent type'),
        capabilities: z.array(z.string()).optional().describe('Agent capabilities'),
        name: z.string().optional().describe('Custom agent name')
    }),
    execute: async ({ type, capabilities, name }) => {
        try {
            const capStr = capabilities ? ` --capabilities "${capabilities.join(',')}"` : '';
            const nameStr = name ? ` --name "${name}"` : '';
            const cmd = `npx claude-flow@alpha agent spawn --type ${type}${capStr}${nameStr}`;
            const result = execSync(cmd, { encoding: 'utf-8', maxBuffer: 10 * 1024 * 1024 });
            return JSON.stringify({
                success: true,
                type,
                capabilities,
                name,
                result: result.trim(),
                timestamp: new Date().toISOString()
            }, null, 2);
        }
        catch (error) {
            throw new Error(`Failed to spawn agent: ${error.message}`);
        }
    }
});
// Tool 6: Task Orchestrate
server.addTool({
    name: 'task_orchestrate',
    description: 'Orchestrate a complex task across the swarm',
    parameters: z.object({
        task: z.string().min(1).describe('Task description or instructions'),
        strategy: z.enum(['parallel', 'sequential', 'adaptive']).optional().default('adaptive').describe('Execution strategy'),
        priority: z.enum(['low', 'medium', 'high', 'critical']).optional().default('medium').describe('Task priority'),
        maxAgents: z.number().positive().optional().describe('Maximum agents to use for this task')
    }),
    execute: async ({ task, strategy, priority, maxAgents }) => {
        try {
            const maxStr = maxAgents ? ` --max-agents ${maxAgents}` : '';
            const cmd = `npx claude-flow@alpha task orchestrate "${task}" --strategy ${strategy} --priority ${priority}${maxStr}`;
            const result = execSync(cmd, { encoding: 'utf-8', maxBuffer: 10 * 1024 * 1024 });
            return JSON.stringify({
                success: true,
                task,
                strategy,
                priority,
                maxAgents,
                result: result.trim(),
                timestamp: new Date().toISOString()
            }, null, 2);
        }
        catch (error) {
            throw new Error(`Failed to orchestrate task: ${error.message}`);
        }
    }
});
console.error('📦 Registered 6 tools: memory_store, memory_retrieve, memory_search, swarm_init, agent_spawn, task_orchestrate');
console.error('🔌 Starting stdio transport...');
// Start with stdio transport
server.start({ transportType: 'stdio' }).then(() => {
    console.error('✅ FastMCP claude-flow-sdk server running on stdio');
}).catch((error) => {
    console.error('❌ Failed to start server:', error);
    process.exit(1);
});
//# sourceMappingURL=claude-flow-sdk.js.map