# AgentDB CLI - Local Testing Guide

## Quick Start

### Option 1: Run the Full Test Suite (Recommended)
```bash
# From the project root: /workspaces/agentic-flow/agentic-flow
./scripts/test-agentdb.sh
```

This will run 13 comprehensive tests covering all CLI features!

### Option 2: Manual Testing

#### 1. Ensure you're in the correct directory
```bash
cd /workspaces/agentic-flow/agentic-flow
pwd  # Should show: /workspaces/agentic-flow/agentic-flow
```

#### 2. Build the Project (if not already built)
```bash
npm run build
```

#### 3. Test the CLI Directly
```bash
# Show help (all 17 commands)
node dist/agentdb/cli/agentdb-cli.js --help

# Or use npx (if globally installed)
npx agentdb --help
```

### 3. Create a Test Database
```bash
# Set database path
export AGENTDB_PATH=./test-agentdb.db

# Or specify inline for each command
AGENTDB_PATH=./test-agentdb.db node dist/agentdb/cli/agentdb-cli.js db stats
```

## Test Each Command Category

### 🧠 Reflexion Memory (Episodic Replay)
```bash
# Store an episode with self-critique
node dist/agentdb/cli/agentdb-cli.js reflexion store \
  "session-1" \
  "implement_authentication" \
  0.95 \
  true \
  "Successfully used OAuth2 with JWT tokens" \
  "User login requirement" \
  "Working auth system" \
  1200 \
  5000

# Store a failed episode
node dist/agentdb/cli/agentdb-cli.js reflexion store \
  "session-1" \
  "implement_authentication" \
  0.3 \
  false \
  "Forgot to validate tokens properly" \
  "User login requirement" \
  "Insecure auth" \
  800 \
  3000

# Retrieve relevant episodes
node dist/agentdb/cli/agentdb-cli.js reflexion retrieve \
  "authentication" \
  5 \
  0.5

# Get critique summary from failures
node dist/agentdb/cli/agentdb-cli.js reflexion critique-summary \
  "authentication" \
  true

# Prune old episodes
node dist/agentdb/cli/agentdb-cli.js reflexion prune 30 0.2
```

### 🛠️ Skill Library (Lifelong Learning)
```bash
# Create a skill manually
node dist/agentdb/cli/agentdb-cli.js skill create \
  "jwt_authentication" \
  "Generate and validate JWT tokens for user authentication" \
  "function generateJWT(payload) { return jwt.sign(payload, secret); }"

# Search for skills
node dist/agentdb/cli/agentdb-cli.js skill search \
  "authentication tokens" \
  5

# Auto-consolidate episodes into skills
node dist/agentdb/cli/agentdb-cli.js skill consolidate \
  3 \
  0.7 \
  7

# Prune underperforming skills
node dist/agentdb/cli/agentdb-cli.js skill prune \
  3 \
  0.4 \
  60
```

### 🔗 Causal Memory Graph (Intervention-Based)
```bash
# Add a causal edge manually
node dist/agentdb/cli/agentdb-cli.js causal add-edge \
  "add_unit_tests" \
  "code_quality_score" \
  0.25 \
  0.95 \
  100

# Create an A/B experiment
node dist/agentdb/cli/agentdb-cli.js causal experiment create \
  "test-coverage-vs-bugs" \
  "test_coverage" \
  "bug_rate"

# Add observations (treatment group)
node dist/agentdb/cli/agentdb-cli.js causal experiment add-observation \
  1 \
  true \
  0.15 \
  '{"coverage": 0.85}'

# Add observations (control group)
node dist/agentdb/cli/agentdb-cli.js causal experiment add-observation \
  1 \
  false \
  0.35 \
  '{"coverage": 0.45}'

# Calculate uplift
node dist/agentdb/cli/agentdb-cli.js causal experiment calculate 1

# Query causal edges
node dist/agentdb/cli/agentdb-cli.js causal query \
  "test" \
  "quality" \
  0.6 \
  0.1 \
  10
```

### 🔍 Causal Recall (Utility-Based Reranking)
```bash
# Retrieve with causal utility and provenance certificate
node dist/agentdb/cli/agentdb-cli.js recall with-certificate \
  "implement secure authentication" \
  10 \
  0.7 \
  0.2 \
  0.1
```

### 🌙 Nightly Learner (Automated Discovery)
```bash
# Discover causal edges from patterns (dry run)
node dist/agentdb/cli/agentdb-cli.js learner run \
  3 \
  0.6 \
  0.7 \
  true

# Discover and save edges
node dist/agentdb/cli/agentdb-cli.js learner run \
  3 \
  0.6 \
  0.7 \
  false

# Prune low-quality edges
node dist/agentdb/cli/agentdb-cli.js learner prune \
  0.5 \
  0.05 \
  90
```

### 📊 Database Stats
```bash
# Get comprehensive database statistics
node dist/agentdb/cli/agentdb-cli.js db stats
```

## Full Workflow Example

### Scenario: Learning from Authentication Implementation

```bash
#!/bin/bash

# Set up test database
export AGENTDB_PATH=./auth-learning.db

# 1. Store successful attempts
node dist/agentdb/cli/agentdb-cli.js reflexion store \
  "session-auth-1" "oauth2_implementation" 0.95 true \
  "Used industry-standard OAuth2 flow" \
  "Implement secure login" "Working OAuth2 system" 1500 6000

node dist/agentdb/cli/agentdb-cli.js reflexion store \
  "session-auth-2" "jwt_tokens" 0.90 true \
  "JWT with proper expiration and refresh tokens" \
  "Token management" "Secure JWT system" 1200 5500

# 2. Store failed attempts
node dist/agentdb/cli/agentdb-cli.js reflexion store \
  "session-auth-3" "session_storage" 0.35 false \
  "Insecure session storage in localStorage" \
  "Session management" "Security vulnerability" 800 3000

# 3. Create a skill from successful pattern
node dist/agentdb/cli/agentdb-cli.js skill create \
  "secure_oauth2_jwt" \
  "OAuth2 flow with JWT token management" \
  "const auth = { oauth2: true, jwt: true, refresh: true }"

# 4. Add causal edge
node dist/agentdb/cli/agentdb-cli.js causal add-edge \
  "add_token_refresh" "session_security" 0.40 0.92 50

# 5. Query for authentication guidance
node dist/agentdb/cli/agentdb-cli.js reflexion retrieve \
  "secure authentication" 5 0.8

# 6. Get critique summary of what NOT to do
node dist/agentdb/cli/agentdb-cli.js reflexion critique-summary \
  "authentication" true

# 7. Search for applicable skills
node dist/agentdb/cli/agentdb-cli.js skill search \
  "oauth jwt tokens" 3

# 8. Check database stats
node dist/agentdb/cli/agentdb-cli.js db stats
```

## Verify Installation

### Check Binary Links
```bash
# Should show the CLI binary path
which agentdb

# Or check npm bin
npm bin agentdb
```

### Run from Package
```bash
# If you've run npm install -g or npm link
agentdb --help
```

## Environment Variables

```bash
# Database path (default: ./agentdb.db)
export AGENTDB_PATH=/path/to/your/database.db

# Example with custom path
AGENTDB_PATH=~/my-agent-memory.db node dist/agentdb/cli/agentdb-cli.js db stats
```

## Expected Output Examples

### Successful Episode Storage
```
✓ Stored episode #1 (session: session-1, task: implement_authentication)
  Reward: 0.95 | Success: true | Latency: 1200ms | Tokens: 5000
```

### Skill Search Results
```
🔍 Found 3 skills for "authentication"

#1: jwt_authentication (success rate: 0.90, uses: 5)
    Generate and validate JWT tokens for user authentication

#2: oauth2_flow (success rate: 0.85, uses: 3)
    Complete OAuth2 authorization code flow
```

### Database Stats
```
AgentDB Statistics

Episodes:          15
Skills:            8
Causal Edges:      12
Experiments:       3
Certificates:      5

Total Size:        2.4 MB
```

## Troubleshooting

### Issue: "Cannot find module"
```bash
# Rebuild the project
npm run build

# Check dist folder exists
ls dist/agentdb/cli/agentdb-cli.js
```

### Issue: "Database is locked"
```bash
# Close any open database connections
# Or use a different database path
export AGENTDB_PATH=./test2-agentdb.db
```

### Issue: "Permission denied"
```bash
# Make CLI executable
chmod +x dist/agentdb/cli/agentdb-cli.js
```

## Advanced Testing

### Test with Programmatic API
```typescript
import { AgentDBCLI } from './dist/agentdb/cli/agentdb-cli.js';

const cli = new AgentDBCLI('./test.db');

// Store episode
await cli.reflexionStore({
  sessionId: 'test-1',
  task: 'example',
  reward: 0.9,
  success: true,
  critique: 'Good approach'
});

// Retrieve episodes
await cli.reflexionRetrieve({
  task: 'example',
  k: 5
});
```

### Integration with Your Project
```typescript
// In your agent code
import { AgentDBCLI } from 'agentic-flow/agentdb';

const memory = new AgentDBCLI();

// Learn from task outcomes
async function learnFromTask(task, outcome) {
  await memory.reflexionStore({
    sessionId: getCurrentSession(),
    task: task.name,
    reward: outcome.score,
    success: outcome.passed,
    critique: outcome.feedback,
    input: task.input,
    output: outcome.result,
    latencyMs: outcome.duration,
    tokensUsed: outcome.tokens
  });
}

// Retrieve similar past experiences
async function recallSimilar(task) {
  return await memory.reflexionRetrieve({
    task: task.name,
    k: 5,
    minReward: 0.7
  });
}
```

## Performance Benchmarks

```bash
# Time a full workflow
time bash full-workflow-example.sh

# Check database performance
node dist/agentdb/cli/agentdb-cli.js db stats
```

## Next Steps

1. ✅ Test basic commands (reflexion, skill)
2. ✅ Test causal features (edges, experiments)
3. ✅ Run nightly learner for discovery
4. ✅ Verify causal recall with certificates
5. ✅ Check database stats
6. 🚀 Integrate into your agent workflows

## Resources

- **AgentDB Controllers**: `/src/agentdb/controllers/`
- **CLI Source**: `/src/agentdb/cli/agentdb-cli.ts`
- **Tests**: `/src/agentdb/tests/frontier-features.test.ts`
- **Binary**: `/dist/agentdb/cli/agentdb-cli.js`
