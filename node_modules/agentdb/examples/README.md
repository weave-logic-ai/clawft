# AgentDB Examples

Practical examples demonstrating how to use AgentDB programmatically.

## Prerequisites

```bash
npm install agentdb@alpha
```

## Examples

### 1. quickstart.js - Basic Usage

Simple example showing database initialization and version checking.

```bash
node examples/quickstart.js
```

**What it demonstrates**:
- Creating a database instance
- Accessing package version
- Basic error handling

### 2. reflexion-memory.js _(Coming in alpha.3)_

Store and retrieve episodic memories using the Reflexion pattern.

**Features**:
- Episode storage
- Similarity-based retrieval
- Pattern recognition

### 3. skill-library.js _(Coming in alpha.3)_

Manage reusable skills with version control.

**Features**:
- Skill storage
- Version management
- Skill retrieval

### 4. causal-reasoning.js _(Coming in alpha.3)_

Build causal graphs for explainable AI decisions.

**Features**:
- Causal edge creation
- Counterfactual reasoning
- Causal path queries

## Current Limitations (Alpha.2)

**Note**: The programmatic API is under active development. For alpha.2:

1. **Use CLI for initialization**:
   ```bash
   npx agentdb@alpha init --db ./my-database.db
   ```

2. **Schemas not auto-created**: You must run `agentdb init` before using the database programmatically

3. **Limited examples**: More comprehensive examples coming in alpha.3

## Recommended Workflow (Alpha.2)

### Step 1: Initialize via CLI
```bash
npx agentdb@alpha init --db ./agent-memory.db --dimensions 384
```

### Step 2: Use programmatically
```javascript
import { createDatabase } from 'agentdb';

const db = await createDatabase('./agent-memory.db');
// Database is ready to use
```

## Coming in Alpha.3

- ✅ Auto-initialization: `AgentDB.create(config)` factory method
- ✅ Complete examples for all features
- ✅ TypeScript examples
- ✅ Integration examples

## Get Help

- **Documentation**: https://github.com/ruvnet/agentic-flow/tree/main/packages/agentdb
- **Issues**: https://github.com/ruvnet/agentic-flow/issues
- **CLI Help**: `npx agentdb@alpha --help`

## Contributing Examples

Have a cool example? Submit a PR!

1. Create your example in `/examples/your-example.js`
2. Add documentation to this README
3. Ensure it works with current alpha version
4. Submit PR with description

---

**Note**: These examples are for AgentDB v2.0 alpha. The API may change before stable release.
