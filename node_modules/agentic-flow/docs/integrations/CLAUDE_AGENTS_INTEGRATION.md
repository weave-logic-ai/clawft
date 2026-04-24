# .claude/agents Integration

This document explains how to use agents from `.claude/agents/` directory with the Claude Agent SDK.

## Overview

The Claude Agent SDK now supports loading and using agent definitions from your `.claude/agents/` directory. This allows you to:

- **Reuse existing agents** from your Claude Code configuration
- **Centralize agent definitions** in one location
- **Mix and match** specialized agents for different tasks
- **Leverage agent metadata** like descriptions and tools

## Quick Start

### 1. Using a Single Agent

```typescript
import { getAgent } from './utils/agentLoader.js';
import { claudeAgent } from './agents/claudeAgent.js';

// Load the goal-planner agent
const goalPlanner = getAgent('goal-planner');

// Use it
const result = await claudeAgent(
  goalPlanner,
  'Create a plan to improve our API performance'
);

console.log(result.output);
```

### 2. Using Multiple Agents

```typescript
import { loadAgents } from './utils/agentLoader.js';
import { claudeAgent } from './agents/claudeAgent.js';

// Load all agents
const agents = loadAgents();

// Use different agents for different tasks
const planner = agents.get('goal-planner');
const coder = agents.get('coder');
const reviewer = agents.get('reviewer');

// Execute in sequence
const plan = await claudeAgent(planner, 'Plan feature X');
const code = await claudeAgent(coder, 'Implement ' + plan.output);
const review = await claudeAgent(reviewer, 'Review: ' + code.output);
```

### 3. Parallel Execution

```typescript
const agents = loadAgents();

// Run multiple agents in parallel
const [research, analysis, design] = await Promise.all([
  claudeAgent(agents.get('researcher'), 'Research topic X'),
  claudeAgent(agents.get('code-analyzer'), 'Analyze codebase'),
  claudeAgent(agents.get('system-architect'), 'Design solution')
]);
```

## Agent Definition Format

Agents in `.claude/agents/` use markdown files with YAML frontmatter:

```markdown
---
name: my-agent
description: "What this agent does"
color: purple
tools: Read, Write, Bash
---

You are a specialized agent that...

Your capabilities:
- Capability 1
- Capability 2

[Rest of system prompt]
```

**Required fields**:
- `name`: Unique identifier
- `description`: Short description of agent's purpose

**Optional fields**:
- `color`: Display color in UI
- `tools`: Comma-separated list of tools

## Available Agents

Run this to see all loaded agents:

```bash
npm run example:multi-agent
```

### Some Notable Agents

From `.claude/agents/`:

**Planning & Strategy**:
- `goal-planner` - GOAP-based planning and optimization
- `planner` - Strategic planning and task orchestration

**Development**:
- `coder` - Clean code implementation
- `reviewer` - Code review specialist
- `tester` - Testing and QA

**Flow Nexus**:
- `flow-nexus-swarm` - AI swarm orchestration
- `flow-nexus-neural` - Neural network training
- `flow-nexus-workflow` - Workflow automation

**GitHub Integration**:
- `github-modes` - GitHub workflow orchestration
- `pr-manager` - PR management
- `code-review-swarm` - Automated code reviews

**Consensus & Coordination**:
- `raft-manager` - Raft consensus
- `byzantine-coordinator` - Byzantine fault tolerance
- `gossip-coordinator` - Gossip protocols

## Examples

### Example 1: Goal-Planner Agent

```bash
npm run example:goal-planner
```

This demonstrates using the goal-planner agent to create improvement plans.

### Example 2: Multi-Agent Orchestration

```bash
npm run example:multi-agent
```

This shows:
- Loading all available agents
- Using multiple agents in sequence
- Agent discovery and listing

## API Reference

### `loadAgents(agentsDir?: string): Map<string, AgentDefinition>`

Load all agents from directory.

**Parameters**:
- `agentsDir` - Path to agents directory (default: `/workspaces/flow-cloud/.claude/agents`)

**Returns**: Map of agent name to AgentDefinition

**Example**:
```typescript
const agents = loadAgents();
console.log(`Loaded ${agents.size} agents`);
```

### `getAgent(name: string, agentsDir?: string): AgentDefinition | undefined`

Get a specific agent by name.

**Parameters**:
- `name` - Agent name
- `agentsDir` - Optional agents directory path

**Returns**: AgentDefinition or undefined

**Example**:
```typescript
const goalPlanner = getAgent('goal-planner');
if (goalPlanner) {
  console.log(goalPlanner.description);
}
```

### `listAgents(agentsDir?: string): AgentDefinition[]`

Get array of all agents.

**Parameters**:
- `agentsDir` - Optional agents directory path

**Returns**: Array of AgentDefinition

**Example**:
```typescript
const allAgents = listAgents();
allAgents.forEach(agent => {
  console.log(`${agent.name}: ${agent.description}`);
});
```

### `claudeAgent(agent: AgentDefinition, input: string, onStream?: (chunk: string) => void)`

Execute an agent with given input.

**Parameters**:
- `agent` - AgentDefinition from loadAgents()
- `input` - Prompt/task for the agent
- `onStream` - Optional streaming callback

**Returns**: `{ output: string, agent: string }`

**Example**:
```typescript
const result = await claudeAgent(
  goalPlanner,
  'Create a 3-step plan',
  (chunk) => process.stdout.write(chunk)
);
```

## AgentDefinition Type

```typescript
interface AgentDefinition {
  name: string;           // Agent identifier
  description: string;    // What the agent does
  systemPrompt: string;   // Full system prompt
  color?: string;         // Optional UI color
  tools?: string[];       // Optional tool list
  filePath: string;       // Source file path
}
```

## Creating Custom Agents

1. Create a markdown file in `.claude/agents/`:

```markdown
---
name: my-custom-agent
description: "Specialized agent for my use case"
color: blue
---

You are a specialized agent for...

Your focus areas:
- Area 1
- Area 2
```

2. Use it in your code:

```typescript
const myAgent = getAgent('my-custom-agent');
const result = await claudeAgent(myAgent, 'Do something');
```

## Docker Integration

The agent loader works in Docker containers if you mount the `.claude/agents` directory:

```bash
docker run -v /path/to/.claude/agents:/app/.claude/agents \
  claude-agents:latest
```

Or in docker-compose.yml:

```yaml
services:
  agents:
    volumes:
      - ../../.claude/agents:/app/.claude/agents:ro
```

## Troubleshooting

### Agent Not Found

```typescript
const agent = getAgent('my-agent');
if (!agent) {
  console.error('Agent not found. Available agents:');
  listAgents().forEach(a => console.log(`  - ${a.name}`));
}
```

### Missing Frontmatter

Agents must have valid frontmatter:
```
---
name: agent-name
description: "Agent description"
---
```

Check logs for warnings about missing metadata.

### Path Issues in Docker

Ensure the path in agentLoader.ts matches your Docker setup:
```typescript
const agents = loadAgents('/app/.claude/agents');
```

## Best Practices

1. **Use Descriptive Names**: Agent names should be clear and specific
2. **Keep System Prompts Focused**: Each agent should have a clear, single purpose
3. **Document Tools**: List required tools in frontmatter
4. **Version Control**: Keep agent definitions in git
5. **Test Agents**: Validate new agents work before deploying

## Advanced: Custom Agent Directory

You can organize agents in subdirectories:

```
.claude/agents/
  ├── planning/
  │   ├── goal-planner.md
  │   └── strategic-planner.md
  ├── development/
  │   ├── coder.md
  │   └── reviewer.md
  └── operations/
      └── deployment.md
```

The loader recursively scans all subdirectories.

## Performance

- **Load Time**: ~50ms for 74 agents
- **Memory**: ~10KB per agent definition
- **Caching**: Agents loaded once at startup

## Next Steps

- Explore existing agents in `.claude/agents/`
- Create custom agents for your use cases
- Combine agents in workflows
- Add error handling and retry logic
- Implement agent chaining patterns

## See Also

- [Quick Wins Implementation](../docs/QUICK_WINS.md)
- [Improvement Plan](../docs/IMPROVEMENT_PLAN.md)
- [Examples](../src/examples/)
