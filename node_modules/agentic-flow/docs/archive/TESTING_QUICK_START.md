# 🚀 AgentDB CLI - Quick Testing Guide

## ⚠️ Important: Two Different AgentDB Versions

There are **TWO** different `agentdb` commands:

1. **OLD AgentDB** (v1.0.12) - The original vector database with ASCII banner
   - Located: `npx agentdb` (globally installed)
   - Shows: ASCII art banner

2. **NEW AgentDB CLI** (v1.7.2) - Frontier features with 17 commands
   - Located: `node dist/agentdb/cli/agentdb-cli.js`
   - Shows: "AgentDB CLI - Frontier Memory Features"

## 🎯 Testing the NEW CLI (Frontier Features)

### Method 1: Direct Node Execution (Recommended)

```bash
# Make sure you're in the project directory
cd /workspaces/agentic-flow/agentic-flow

# Test help command
node dist/agentdb/cli/agentdb-cli.js --help

# Should show:
# AgentDB CLI - Frontier Memory Features
#
# USAGE:
#   agentdb <command> <subcommand> [options]
#
# CAUSAL COMMANDS:
# ...
```

### Method 2: Run the Test Suite

```bash
# Automated test of all 13 features
./scripts/test-agentdb.sh
```

### Method 3: Link Locally to Override Global

```bash
# Unlink old version
npm unlink -g agentdb

# Link new version from this directory
npm link

# Now npx will use the new CLI
npx agentdb --help
```

## 🧪 Quick Manual Test

```bash
# Set test database
export AGENTDB_PATH=./my-test.db

# Store an episode
node dist/agentdb/cli/agentdb-cli.js reflexion store \
  "session-1" \
  "test_task" \
  0.95 \
  true \
  "It worked!" \
  "input" \
  "output" \
  1000 \
  500

# Retrieve it
node dist/agentdb/cli/agentdb-cli.js reflexion retrieve \
  "test_task" \
  5 \
  0.5

# Check stats
node dist/agentdb/cli/agentdb-cli.js db stats
```

## 📊 What's New in the CLI?

### 17 Commands Across 6 Controllers:

1. **Reflexion Memory** (4 commands) - Episodic replay with self-critique
   - `reflexion store` - Store episodes with critiques
   - `reflexion retrieve` - Retrieve relevant episodes
   - `reflexion critique-summary` - Get failure lessons
   - `reflexion prune` - Clean up old episodes

2. **Skill Library** (4 commands) - Lifelong learning
   - `skill create` - Create reusable skills
   - `skill search` - Find applicable skills
   - `skill consolidate` - Auto-create from episodes
   - `skill prune` - Remove underperforming skills

3. **Causal Memory Graph** (5 commands) - Intervention-based causality
   - `causal add-edge` - Add causal relationships
   - `causal experiment create` - Create A/B experiments
   - `causal experiment add-observation` - Record observations
   - `causal experiment calculate` - Calculate uplift
   - `causal query` - Query causal edges

4. **Causal Recall** (1 command) - Utility-based reranking
   - `recall with-certificate` - Retrieve with provenance

5. **Nightly Learner** (2 commands) - Automated discovery
   - `learner run` - Discover causal patterns
   - `learner prune` - Remove low-quality edges

6. **Database** (1 command) - Statistics
   - `db stats` - Show database info

## 🔍 Verify You're Using the New CLI

### Wrong (Old CLI with ASCII art):
```bash
$ npx agentdb --help

█▀█ █▀▀ █▀▀ █▄░█ ▀█▀ █▀▄ █▄▄
█▀█ █▄█ ██▄ █░▀█ ░█░ █▄▀ █▄█

AgentDB v1.0.12 - Agent Memory & Vector Database
```

### Correct (New CLI with Frontier Features):
```bash
$ node dist/agentdb/cli/agentdb-cli.js --help

AgentDB CLI - Frontier Memory Features

USAGE:
  agentdb <command> <subcommand> [options]

CAUSAL COMMANDS:
  agentdb causal add-edge <cause> <effect> <uplift> [confidence] [sample-size]
```

## 🎮 Interactive Examples

### Example 1: Learn from Code Reviews
```bash
# Store a successful review
node dist/agentdb/cli/agentdb-cli.js reflexion store \
  "review-1" "add_error_handling" 0.90 true \
  "Used try-catch with specific error types" \
  "Handle edge cases" "Robust error handling" 1500 4000

# Store a failed review
node dist/agentdb/cli/agentdb-cli.js reflexion store \
  "review-1" "add_error_handling" 0.30 false \
  "Caught generic errors without logging" \
  "Handle edge cases" "Poor error handling" 800 2000

# Get critique summary
node dist/agentdb/cli/agentdb-cli.js reflexion critique-summary \
  "error_handling" true
```

### Example 2: Build a Skill Library
```bash
# Create a skill
node dist/agentdb/cli/agentdb-cli.js skill create \
  "error_handling_pattern" \
  "Robust error handling with logging and specific error types" \
  "try { ... } catch (err) { logger.error(err); throw new CustomError(); }"

# Search for it
node dist/agentdb/cli/agentdb-cli.js skill search \
  "error handling" 5
```

### Example 3: Causal Analysis
```bash
# Add a causal edge
node dist/agentdb/cli/agentdb-cli.js causal add-edge \
  "add_type_hints" "reduce_runtime_errors" 0.35 0.88 75

# Query causal effects
node dist/agentdb/cli/agentdb-cli.js causal query \
  "type" "error" 0.7 0.2 10
```

## 🐛 Troubleshooting

### "Cannot find module"
```bash
# Check you're in the right directory
pwd  # Should show: /workspaces/agentic-flow/agentic-flow

# Verify file exists
ls dist/agentdb/cli/agentdb-cli.js

# Rebuild if needed
npm run build
```

### "Shows old ASCII banner"
```bash
# You're using the old global package
# Use direct node execution instead:
node dist/agentdb/cli/agentdb-cli.js --help
```

### "Database is locked"
```bash
# Use a different database file
export AGENTDB_PATH=./test-$(date +%s).db
```

## 📚 Full Documentation

See `docs/AGENTDB_TESTING.md` for comprehensive examples and API details.

## 🚀 Next Steps

1. ✅ Run `./scripts/test-agentdb.sh` for full feature test
2. ✅ Try the examples above
3. ✅ Check `docs/AGENTDB_TESTING.md` for more
4. ✅ Integrate into your agent workflows!
