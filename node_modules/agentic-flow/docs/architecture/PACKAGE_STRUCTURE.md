# agentic-flow Package Structure

## Overview

The `agentic-flow` npm package includes all necessary files for agent execution, including 76 pre-built agent definitions in the `.claude/agents/` directory.

## Package Contents

When you install `agentic-flow` via npm, you get:

```
agentic-flow/
├── dist/                    # Compiled JavaScript
│   ├── cli-proxy.js         # Main CLI entry point
│   ├── agents/              # Agent implementations
│   ├── router/              # Multi-provider router
│   └── utils/               # Utilities
├── .claude/                 # Agent definitions (76 files)
│   └── agents/
│       ├── core/            # Core agents (coder, planner, reviewer, etc.)
│       ├── consensus/       # Distributed consensus agents
│       ├── github/          # GitHub integration agents
│       ├── flow-nexus/      # Flow Nexus cloud agents
│       ├── sparc/           # SPARC methodology agents
│       └── ...              # More specialized categories
├── docs/                    # Documentation
├── README.md
└── LICENSE
```

## Agent Loading System

### Package Agents (Bundled)

All 76 agent definitions are included in the npm package at:
```
node_modules/agentic-flow/.claude/agents/
```

These are automatically loaded when you run `npx agentic-flow`.

### Local Agents (User Custom)

You can create custom agents in your project:
```
your-project/
└── .claude/
    └── agents/
        └── custom/
            └── my-agent.md
```

**Local agents override package agents** with the same relative path.

## Agent Discovery Order

1. **Package agents**: Load from `node_modules/agentic-flow/.claude/agents/`
2. **Local agents**: Load from `./claude/agents/` (overrides package agents if same path)
3. **Custom directory**: Use `--agents-dir` flag to specify alternative location

## Verification

To verify your installation includes all agents:

```bash
# List all available agents
npx agentic-flow --list

# Should show 73 agents (76 files, some without proper frontmatter)
```

## Custom Agent Creation

Create custom agents that augment or replace package agents:

```bash
# Interactive creation
npx agentic-flow agent create

# Manual creation
mkdir -p .claude/agents/custom
cat > .claude/agents/custom/my-agent.md << 'EOF'
---
name: my-agent
description: My custom agent
---

You are a specialized agent for [purpose].
Follow these guidelines:
- [guideline 1]
- [guideline 2]
EOF
```

## Package Maintenance

### Building

```bash
npm run build
```

### Verifying Package Structure

```bash
./scripts/verify-package.sh
```

### Creating Package

```bash
npm pack
```

### Testing Installation

```bash
# Install in test directory
mkdir -p /tmp/test-install
cd /tmp/test-install
npm install /path/to/agentic-flow-1.1.2.tgz

# Verify agents loaded
./node_modules/.bin/agentic-flow --list
```

## Environment Configuration

The package automatically loads `.env` files from:
1. Current directory
2. Parent directories (recursively up to root)

This ensures API keys work from any directory:

```bash
# Works from project root
cd /workspaces/myproject
npx agentic-flow --agent coder --task "test" --provider gemini

# Also works from subdirectory (finds parent .env)
cd /workspaces/myproject/src
npx agentic-flow --agent coder --task "test" --provider gemini
```

## Files Included in Package

See `package.json`:
```json
{
  "files": [
    "dist",
    "docs",
    ".claude",
    "README.md",
    "LICENSE"
  ]
}
```

## Files Excluded (.npmignore)

- Source files (`src/`, `*.ts`)
- Tests (`tests/`, `validation/`)
- Development files (`.env`, `tsconfig.json`)
- Runtime state directories (`.claude-flow/`, `.swarm/`, `memory/`)
- ONNX models (`*.onnx`, `models/`)

## Agent Categories

The 76 included agents span:

- **Core** (5): coder, planner, researcher, reviewer, tester
- **Consensus** (7): Byzantine, CRDT, Gossip, Raft, Quorum, etc.
- **GitHub** (14): PR management, issue tracking, release automation
- **Flow Nexus** (9): Cloud sandboxes, neural networks, workflows
- **SPARC** (4): Specification, Pseudocode, Architecture, Refinement
- **Optimization** (5): Resource allocation, load balancing, benchmarks
- **Goal Planning** (2): GOAP, sublinear algorithms
- **Swarm** (3): Hierarchical, mesh, adaptive coordination
- **Payments** (1): Agentic payment authorization
- **Templates** (10): Automation, orchestration, migration
- **Testing** (2): TDD, production validation
- **Specialized** (varies): Analysis, architecture, data, development, DevOps, documentation

## Total Agent Count

- **76 agent files** in `.claude/agents/`
- **73 valid agents** (3 files missing required frontmatter)
- **All core agents** (coder, planner, researcher, reviewer, tester) working

## Summary

✅ All 76 agent definitions are packaged and distributed via npm
✅ Agent loading works automatically from `node_modules/`
✅ Local `.claude/agents/` can override package agents
✅ Environment variable loading works recursively
✅ Package structure verified and tested

The `.claude/` directory is a first-class part of the npm package, ensuring all users have immediate access to the complete agent library upon installation.
