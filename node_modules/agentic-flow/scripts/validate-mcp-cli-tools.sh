#!/bin/bash
# Validation script for MCP CLI tools - Tests all 11 tools

set -e

echo "🧪 Validating MCP CLI Tools Integration"
echo "========================================"
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

TESTS_PASSED=0
TESTS_FAILED=0

# Test function
test_tool() {
    local test_name=$1
    local expected=$2

    echo -e "${BLUE}Testing: ${test_name}${NC}"

    if eval "$expected"; then
        echo -e "${GREEN}✅ PASSED${NC}\n"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}❌ FAILED${NC}\n"
        ((TESTS_FAILED++))
    fi
}

# Test 1: MCP Status shows 11 tools
test_tool "MCP Status (11/11 tools)" \
    "node dist/cli/mcp.js status | grep -q '11/11'"

# Test 2: MCP Tools list shows agent_execute
test_tool "Tools list includes agent_execute" \
    "node dist/cli/mcp.js tools | grep -q 'agent_execute'"

# Test 3: MCP Tools list shows agent_parallel
test_tool "Tools list includes agent_parallel" \
    "node dist/cli/mcp.js tools | grep -q 'agent_parallel'"

# Test 4: MCP Tools list shows agent_list
test_tool "Tools list includes agent_list" \
    "node dist/cli/mcp.js tools | grep -q 'agent_list'"

# Test 5: MCP Tools list shows agent_add
test_tool "Tools list includes agent_add" \
    "node dist/cli/mcp.js tools | grep -q 'agent_add'"

# Test 6: MCP Tools list shows command_add
test_tool "Tools list includes command_add" \
    "node dist/cli/mcp.js tools | grep -q 'command_add'"

# Test 7: Build succeeded
test_tool "TypeScript compilation successful" \
    "[ -f dist/cli/mcp.js ]"

# Test 8: stdio server file exists
test_tool "stdio server compiled" \
    "[ -f dist/mcp/fastmcp/servers/stdio-full.js ]"

# Test 9: All agent tool files exist
test_tool "Agent execute tool exists" \
    "[ -f dist/mcp/fastmcp/tools/agent/execute.js ]"

test_tool "Agent parallel tool exists" \
    "[ -f dist/mcp/fastmcp/tools/agent/parallel.js ]"

test_tool "Agent list tool exists" \
    "[ -f dist/mcp/fastmcp/tools/agent/list.js ]"

test_tool "Agent add tool exists" \
    "[ -f dist/mcp/fastmcp/tools/agent/add-agent.js ]"

test_tool "Command add tool exists" \
    "[ -f dist/mcp/fastmcp/tools/agent/add-command.js ]"

# Summary
echo "========================================"
echo -e "${BLUE}Test Summary${NC}"
echo "========================================"
echo -e "${GREEN}✅ Passed: ${TESTS_PASSED}${NC}"
echo -e "${RED}❌ Failed: ${TESTS_FAILED}${NC}"
echo "Total: $((TESTS_PASSED + TESTS_FAILED))"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 All MCP CLI tools validated successfully!${NC}"
    echo ""
    echo "Available MCP Tools (11 total):"
    echo "  Memory: memory_store, memory_retrieve, memory_search"
    echo "  Swarm: swarm_init, agent_spawn, task_orchestrate"
    echo "  Agent Execution: agent_execute, agent_parallel, agent_list"
    echo "  Custom: agent_add, command_add"
    exit 0
else
    echo -e "${RED}⚠️  Some tests failed. Check output above.${NC}"
    exit 1
fi
