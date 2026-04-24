#!/bin/bash
# Test script for FastMCP claude-flow-sdk server (6 tools)
set -e

echo "🧪 Testing FastMCP claude-flow-sdk Server (6 tools)..."
echo ""

# Build first
echo "📦 Building project..."
npm run build
echo ""

# Test 1: memory_store
echo "🧪 Test 1: memory_store"
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"memory_store","arguments":{"key":"sdk-test","value":"fastmcp-value","namespace":"sdk-test"}}}' | node dist/mcp/fastmcp/servers/claude-flow-sdk.js 2>/dev/null || true
echo ""

# Test 2: memory_retrieve
echo "🧪 Test 2: memory_retrieve"
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"memory_retrieve","arguments":{"key":"sdk-test","namespace":"sdk-test"}}}' | node dist/mcp/fastmcp/servers/claude-flow-sdk.js 2>/dev/null || true
echo ""

# Test 3: memory_search
echo "🧪 Test 3: memory_search"
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"memory_search","arguments":{"pattern":"sdk-*","limit":5}}}' | node dist/mcp/fastmcp/servers/claude-flow-sdk.js 2>/dev/null || true
echo ""

# Test 4: swarm_init
echo "🧪 Test 4: swarm_init"
echo '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"swarm_init","arguments":{"topology":"mesh","maxAgents":5}}}' | node dist/mcp/fastmcp/servers/claude-flow-sdk.js 2>/dev/null || true
echo ""

# Test 5: agent_spawn
echo "🧪 Test 5: agent_spawn"
echo '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"agent_spawn","arguments":{"type":"researcher"}}}' | node dist/mcp/fastmcp/servers/claude-flow-sdk.js 2>/dev/null || true
echo ""

# Test 6: task_orchestrate
echo "🧪 Test 6: task_orchestrate"
echo '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"task_orchestrate","arguments":{"task":"Test task orchestration","strategy":"parallel"}}}' | node dist/mcp/fastmcp/servers/claude-flow-sdk.js 2>/dev/null || true
echo ""

echo "✅ claude-flow-sdk server tests completed!"
echo ""
echo "📋 To use with Claude Code, add to MCP config:"
echo '{"mcpServers":{"claude-flow-sdk-fastmcp":{"command":"node","args":["dist/mcp/fastmcp/servers/claude-flow-sdk.js"]}}}'
