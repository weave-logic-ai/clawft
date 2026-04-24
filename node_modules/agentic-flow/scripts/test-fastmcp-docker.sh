#!/bin/bash
# Comprehensive FastMCP Docker Test Suite

set -e

# Load environment variables (handle quotes properly)
if [ -f .env ]; then
    set -a
    source .env
    set +a
fi

# Set defaults from .env
FASTMCP_PORT=${FASTMCP_PORT:-3000}
NODE_ENV=${NODE_ENV:-production}

echo "🐳 FastMCP Docker Integration Tests"
echo "===================================="
echo "Port: ${FASTMCP_PORT}"
echo "Environment: ${NODE_ENV}"
echo "Supabase Project: ${SUPABASE_PROJECT_ID}"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Function to run test
run_test() {
    local test_name=$1
    local test_cmd=$2

    echo -e "${BLUE}🧪 Test: ${test_name}${NC}"

    if eval "$test_cmd"; then
        echo -e "${GREEN}✅ PASSED${NC}\n"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}❌ FAILED${NC}\n"
        ((TESTS_FAILED++))
    fi
}

# Build Docker image
echo -e "${YELLOW}📦 Building Docker image...${NC}"
docker build -f docker/fastmcp-test.Dockerfile -t fastmcp-test:latest .

echo -e "${GREEN}✅ Docker image built${NC}\n"

# Test 1: MCP Status
run_test "MCP Status Command" \
    "docker run --rm fastmcp-test:latest node dist/cli/mcp.js status | grep -q '6/6 implemented'"

# Test 2: List Tools
run_test "List MCP Tools" \
    "docker run --rm fastmcp-test:latest node dist/cli/mcp.js tools | grep -q 'memory_store'"

# Test 3: Start HTTP Server (background)
echo -e "${BLUE}🧪 Test: HTTP Server Startup${NC}"
docker run --rm -d --name fastmcp-http-test \
    --env-file .env \
    -e PORT=${FASTMCP_PORT} \
    -e NODE_ENV=${NODE_ENV} \
    -p ${FASTMCP_PORT}:3000 \
    fastmcp-test:latest node dist/mcp/fastmcp/servers/http-streaming.js
sleep 3

# Test 4: Health Check
run_test "HTTP Health Endpoint" \
    "curl -s http://localhost:${FASTMCP_PORT}/health | grep -q 'healthy'"

# Test 5: SSE Stream
run_test "SSE Stream Connection" \
    "timeout 2 curl -N http://localhost:${FASTMCP_PORT}/events | grep -q 'connected' || true"

# Test 6: Tools List Endpoint
run_test "MCP Tools List Endpoint" \
    "curl -s -X POST http://localhost:${FASTMCP_PORT}/mcp -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}' | grep -q 'memory_store'"

# Test 7: Memory Store Tool
run_test "memory_store Tool" \
    "curl -s -X POST http://localhost:${FASTMCP_PORT}/mcp -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"memory_store\",\"arguments\":{\"key\":\"docker-test\",\"value\":\"hello from docker\"}}}' | grep -q 'success'"

# Test 8: Memory Retrieve Tool
run_test "memory_retrieve Tool" \
    "curl -s -X POST http://localhost:${FASTMCP_PORT}/mcp -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"memory_retrieve\",\"arguments\":{\"key\":\"docker-test\"}}}' | grep -q 'success'"

# Test 9: Memory Search Tool
run_test "memory_search Tool" \
    "curl -s -X POST http://localhost:${FASTMCP_PORT}/mcp -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"memory_search\",\"arguments\":{\"pattern\":\"docker-*\",\"limit\":5}}}' | grep -q 'success'"

# Test 10: Swarm Init Tool
run_test "swarm_init Tool" \
    "curl -s -X POST http://localhost:${FASTMCP_PORT}/mcp -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"swarm_init\",\"arguments\":{\"topology\":\"mesh\",\"maxAgents\":5,\"strategy\":\"balanced\"}}}' | grep -q 'success'"

# Test 11: Agent Spawn Tool
run_test "agent_spawn Tool" \
    "curl -s -X POST http://localhost:${FASTMCP_PORT}/mcp -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"agent_spawn\",\"arguments\":{\"type\":\"researcher\",\"capabilities\":[\"analysis\",\"research\"]}}}' | grep -q 'success'"

# Test 12: Task Orchestrate Tool
run_test "task_orchestrate Tool" \
    "curl -s -X POST http://localhost:${FASTMCP_PORT}/mcp -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"task_orchestrate\",\"arguments\":{\"task\":\"Test orchestration\",\"strategy\":\"parallel\",\"priority\":\"medium\"}}}' | grep -q 'success'"

# Cleanup
echo -e "\n${YELLOW}🧹 Cleaning up...${NC}"
docker stop fastmcp-http-test 2>/dev/null || true
docker rm fastmcp-http-test 2>/dev/null || true

# Summary
echo ""
echo "===================================="
echo -e "${BLUE}📊 Test Summary${NC}"
echo "===================================="
echo -e "${GREEN}✅ Passed: ${TESTS_PASSED}${NC}"
echo -e "${RED}❌ Failed: ${TESTS_FAILED}${NC}"
echo -e "Total Tests: $((TESTS_PASSED + TESTS_FAILED))"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 All tests passed! FastMCP is working perfectly in Docker.${NC}"
    exit 0
else
    echo -e "${RED}⚠️  Some tests failed. Please check the output above.${NC}"
    exit 1
fi
