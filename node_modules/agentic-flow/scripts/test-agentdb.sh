#!/bin/bash
# AgentDB CLI Quick Test Script

set -e

echo "🧪 AgentDB CLI Testing Script"
echo "=============================="
echo ""

# Set test database path
export AGENTDB_PATH="./test-agentdb.db"

# Clean up old test database
if [ -f "$AGENTDB_PATH" ]; then
  echo "🗑️  Removing old test database..."
  rm "$AGENTDB_PATH"
fi

echo "📍 Current directory: $(pwd)"
echo "💾 Database path: $AGENTDB_PATH"
echo ""

# Test 1: Help command
echo "1️⃣  Testing help command..."
echo ""
node dist/agentdb/cli/agentdb-cli.js --help | head -20
echo ""
echo "✅ Help command works!"
echo ""

# Test 2: Store a successful episode
echo "2️⃣  Storing a successful episode..."
node dist/agentdb/cli/agentdb-cli.js reflexion store \
  "test-session-1" \
  "implement_authentication" \
  0.95 \
  true \
  "Successfully implemented OAuth2 with JWT tokens" \
  "User login requirement" \
  "Working auth system with refresh tokens" \
  1200 \
  5000
echo ""

# Test 3: Store a failed episode
echo "3️⃣  Storing a failed episode..."
node dist/agentdb/cli/agentdb-cli.js reflexion store \
  "test-session-1" \
  "implement_authentication" \
  0.35 \
  false \
  "Forgot to validate token expiration" \
  "User login requirement" \
  "Insecure auth system" \
  800 \
  3000
echo ""

# Test 4: Retrieve episodes
echo "4️⃣  Retrieving episodes..."
node dist/agentdb/cli/agentdb-cli.js reflexion retrieve \
  "authentication" \
  5 \
  0.3
echo ""

# Test 5: Get critique summary
echo "5️⃣  Getting critique summary (failures only)..."
node dist/agentdb/cli/agentdb-cli.js reflexion critique-summary \
  "authentication" \
  true
echo ""

# Test 6: Create a skill
echo "6️⃣  Creating a skill..."
node dist/agentdb/cli/agentdb-cli.js skill create \
  "jwt_authentication" \
  "Generate and validate JWT tokens for user authentication" \
  "function generateJWT(payload, secret) { return jwt.sign(payload, secret, { expiresIn: '1h' }); }"
echo ""

# Test 7: Search skills
echo "7️⃣  Searching for skills..."
node dist/agentdb/cli/agentdb-cli.js skill search \
  "authentication" \
  5
echo ""

# Test 8: Add a causal edge
echo "8️⃣  Adding a causal edge..."
node dist/agentdb/cli/agentdb-cli.js causal add-edge \
  "add_unit_tests" \
  "code_quality_score" \
  0.25 \
  0.95 \
  100
echo ""

# Test 9: Create an experiment
echo "9️⃣  Creating an A/B experiment..."
node dist/agentdb/cli/agentdb-cli.js causal experiment create \
  "test-coverage-vs-bugs" \
  "test_coverage" \
  "bug_rate"
echo ""

# Test 10: Add experiment observations
echo "🔟 Adding experiment observations..."
echo "   - Treatment group (high test coverage)..."
node dist/agentdb/cli/agentdb-cli.js causal experiment add-observation \
  1 \
  true \
  0.15 \
  '{"coverage": 0.85, "lines": 5000}'
echo "   - Control group (low test coverage)..."
node dist/agentdb/cli/agentdb-cli.js causal experiment add-observation \
  1 \
  false \
  0.35 \
  '{"coverage": 0.45, "lines": 5000}'
echo ""

# Test 11: Calculate experiment uplift
echo "1️⃣1️⃣  Calculating experiment uplift..."
node dist/agentdb/cli/agentdb-cli.js causal experiment calculate 1
echo ""

# Test 12: Query causal edges
echo "1️⃣2️⃣  Querying causal edges..."
node dist/agentdb/cli/agentdb-cli.js causal query \
  "" \
  "" \
  0.5 \
  0.1 \
  10
echo ""

# Test 13: Database stats
echo "1️⃣3️⃣  Getting database statistics..."
node dist/agentdb/cli/agentdb-cli.js db stats
echo ""

# Summary
echo "=============================="
echo "✅ All 13 tests completed successfully!"
echo ""
echo "📊 Test database created at: $AGENTDB_PATH"
echo "📁 Size: $(du -h "$AGENTDB_PATH" | cut -f1)"
echo ""
echo "🔍 You can now explore the database or run individual commands:"
echo "   node dist/agentdb/cli/agentdb-cli.js --help"
echo "   AGENTDB_PATH=$AGENTDB_PATH node dist/agentdb/cli/agentdb-cli.js reflexion retrieve authentication 5"
echo ""
