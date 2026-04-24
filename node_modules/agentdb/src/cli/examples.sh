#!/bin/bash
# AgentDB CLI Examples - Frontier Features

set -e

echo "🚀 AgentDB CLI Examples - Frontier Features"
echo "==========================================="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Set database path
export AGENTDB_PATH="./agentdb-example.db"

echo -e "${BLUE}📊 Example 1: Manual Causal Edge${NC}"
echo "Adding causal relationship: add_tests → code_quality"
npx tsx src/agentdb/cli/agentdb-cli.ts causal add-edge \
  "add_tests" "code_quality" 0.25 0.95 100
echo ""

echo -e "${BLUE}🧪 Example 2: A/B Experiment${NC}"
echo "Testing hypothesis: Higher test coverage reduces bug rate"
echo ""

# Create experiment
echo "Creating experiment..."
npx tsx src/agentdb/cli/agentdb-cli.ts causal experiment create \
  "test-coverage-experiment" "test_coverage_high" "bug_rate"
echo ""

# Add treatment group observations (high coverage → lower bugs)
echo "Adding treatment observations (high coverage)..."
for i in {1..10}; do
  OUTCOME=$(awk -v min=0.05 -v max=0.20 'BEGIN{srand(); print min+rand()*(max-min)}')
  npx tsx src/agentdb/cli/agentdb-cli.ts causal experiment add-observation \
    1 true $OUTCOME '{"coverage": 0.85}'
done
echo ""

# Add control group observations (low coverage → higher bugs)
echo "Adding control observations (low coverage)..."
for i in {1..10}; do
  OUTCOME=$(awk -v min=0.25 -v max=0.45 'BEGIN{srand(); print min+rand()*(max-min)}')
  npx tsx src/agentdb/cli/agentdb-cli.ts causal experiment add-observation \
    1 false $OUTCOME '{"coverage": 0.45}'
done
echo ""

# Calculate uplift
echo "Calculating uplift and significance..."
npx tsx src/agentdb/cli/agentdb-cli.ts causal experiment calculate 1
echo ""

echo -e "${BLUE}🔍 Example 3: Causal Query${NC}"
echo "Finding all high-confidence causal edges..."
npx tsx src/agentdb/cli/agentdb-cli.ts causal query \
  "" "" 0.7 0.1 10
echo ""

echo -e "${BLUE}📈 Example 4: Database Statistics${NC}"
npx tsx src/agentdb/cli/agentdb-cli.ts db stats
echo ""

echo -e "${GREEN}✅ Examples Complete!${NC}"
echo ""
echo "Try these commands yourself:"
echo ""
echo -e "${YELLOW}# Query causal edges for specific effect${NC}"
echo "npx agentdb causal query '' 'code_quality' 0.8"
echo ""
echo -e "${YELLOW}# Retrieve with causal utility${NC}"
echo "npx agentdb recall with-certificate 'implement authentication' 10"
echo ""
echo -e "${YELLOW}# Run nightly learner${NC}"
echo "npx agentdb learner run 3 0.6 0.7"
echo ""
echo -e "${YELLOW}# Prune low-quality edges${NC}"
echo "npx agentdb learner prune 0.5 0.05 90"
echo ""
