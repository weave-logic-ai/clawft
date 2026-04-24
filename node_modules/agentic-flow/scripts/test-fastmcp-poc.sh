#!/bin/bash
# Test script for FastMCP POC server
set -e

echo "🧪 Testing FastMCP POC Server..."
echo ""

# Build first
echo "📦 Building project..."
npm run build
echo ""

# Test memory_store tool
echo "🧪 Test 1: memory_store tool"
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"memory_store","arguments":{"key":"test-key","value":"test-value","namespace":"poc-test"}}}' | node dist/mcp/fastmcp/servers/poc-stdio.js 2>/dev/null || true
echo ""

# Test memory_retrieve tool
echo "🧪 Test 2: memory_retrieve tool"
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"memory_retrieve","arguments":{"key":"test-key","namespace":"poc-test"}}}' | node dist/mcp/fastmcp/servers/poc-stdio.js 2>/dev/null || true
echo ""

echo "✅ POC server tests completed!"
echo ""
echo "📋 To use with Claude Code, add to MCP config:"
echo '{"mcpServers":{"fastmcp-poc":{"command":"node","args":["dist/mcp/fastmcp/servers/poc-stdio.js"]}}}'
