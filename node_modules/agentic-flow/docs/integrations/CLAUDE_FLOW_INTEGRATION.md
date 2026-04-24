# Claude Flow Integration Guide

Complete guide for using claude-flow memory and coordination capabilities in the Docker Agent SDK.

## Quick Start

### 1. Memory-Enabled Agent

```typescript
import { memoryResearchAgent } from './src/agents/claudeFlowAgent.js';

// Agent will store findings in persistent memory
const result = await memoryResearchAgent(
  'Research TypeScript features and store the top 3 benefits',
  (chunk) => process.stdout.write(chunk)
);
```

### 2. Coordination Agent

```typescript
import { orchestratorAgent } from './src/agents/claudeFlowAgent.js';

// Agent coordinates multiple specialized agents
const result = await orchestratorAgent(
  'Build a TODO app: coordinate researcher, coder, and reviewer',
  (chunk) => process.stdout.write(chunk)
);
```

### 3. Hybrid Agent (Memory + Coordination)

```typescript
import { hybridAgent } from './src/agents/claudeFlowAgent.js';

// Agent has both memory and coordination
const result = await hybridAgent(
  'Research API patterns, store findings, then coordinate coder to implement',
  'api-builder',
  'You are a full-stack agent with memory and coordination',
  (chunk) => process.stdout.write(chunk)
);
```

## Docker Usage

```bash
# Build with claude-flow
./build.sh

# Run agent with memory
docker run --env-file ../../.env claude-agents:cli \
  --agent memory-researcher \
  --task "Research and remember ML frameworks"

# Run orchestrator
docker run --env-file ../../.env claude-agents:cli \
  --agent orchestrator \
  --task "Coordinate agents to build a feature"
```

## Memory System

### What is Persistent Memory?

Claude Flow provides a SQLite-backed memory system that persists information across agent runs.

**Use Cases**:
- Store research findings for later use
- Remember user preferences
- Build knowledge bases incrementally
- Share information between agents

### Memory Operations

#### Store Information
```typescript
// Agent uses this internally
mcp__claude-flow__memory_usage({
  action: 'store',
  key: 'typescript-benefit-1',
  value: 'Type safety reduces runtime errors',
  namespace: 'claude-agents:researcher',
  ttl: 3600 // 1 hour
})
```

#### Retrieve Information
```typescript
mcp__claude-flow__memory_usage({
  action: 'retrieve',
  key: 'typescript-benefit-1',
  namespace: 'claude-agents:researcher'
})
```

#### Search Memory
```typescript
mcp__claude-flow__memory_search({
  pattern: 'TypeScript',
  namespace: 'claude-agents:researcher',
  limit: 10
})
```

### Memory Namespaces

Each agent gets its own namespace:
- `claude-agents:memory-researcher` - Research agent's memory
- `claude-agents:coder` - Coder agent's memory
- `claude-agents:orchestrator` - Orchestrator's memory

**Custom namespace**:
```typescript
const result = await claudeFlowAgent(
  'my-agent',
  'You are a specialized agent',
  'Your task',
  {
    enableMemory: true,
    memoryNamespace: 'my-custom-namespace'
  }
);
```

## Coordination System

### What is Multi-Agent Coordination?

Claude Flow enables agents to spawn and coordinate other specialized agents, creating swarms that work together.

**Use Cases**:
- Break complex tasks into subtasks
- Parallel processing with specialized agents
- Hierarchical workflows
- Dynamic team composition

### Swarm Topologies

#### 1. Mesh (Default)
```
Agent1 ←→ Agent2
  ↕         ↕
Agent3 ←→ Agent4
```
**Best for**: Collaborative problem solving, peer review

#### 2. Hierarchical
```
    Coordinator
    ↙    ↓    ↘
 Agent1 Agent2 Agent3
```
**Best for**: Complex workflows, clear delegation

#### 3. Ring
```
Agent1 → Agent2 → Agent3 → Agent1
```
**Best for**: Sequential processing, token passing

#### 4. Star
```
      Agent2
        ↑
Agent1 ← Coordinator → Agent3
        ↓
      Agent4
```
**Best for**: Centralized control, hub-and-spoke

### Coordination Operations

#### Initialize Swarm
```typescript
mcp__claude-flow__swarm_init({
  topology: 'mesh',  // or 'hierarchical', 'ring', 'star'
  maxAgents: 8,
  strategy: 'balanced'
})
```

#### Spawn Agent
```typescript
mcp__claude-flow__agent_spawn({
  type: 'researcher',  // or 'coder', 'analyst', 'optimizer'
  capabilities: ['search', 'analyze'],
  name: 'research-agent-1'
})
```

#### Orchestrate Task
```typescript
mcp__claude-flow__task_orchestrate({
  task: 'Analyze codebase and suggest improvements',
  strategy: 'parallel',  // or 'sequential', 'adaptive'
  priority: 'high'       // or 'low', 'medium', 'critical'
})
```

#### Check Status
```typescript
mcp__claude-flow__swarm_status({
  swarmId: 'optional-id'
})
```

## Available MCP Tools

### Memory Tools
- `mcp__claude-flow__memory_usage` - Store/retrieve/list/delete
- `mcp__claude-flow__memory_search` - Pattern-based search
- `mcp__claude-flow__memory_persist` - Cross-session persistence
- `mcp__claude-flow__memory_namespace` - Namespace management

### Coordination Tools
- `mcp__claude-flow__swarm_init` - Initialize swarm
- `mcp__claude-flow__agent_spawn` - Create agent
- `mcp__claude-flow__task_orchestrate` - Distribute tasks
- `mcp__claude-flow__swarm_status` - Check status
- `mcp__claude-flow__coordination_sync` - Sync agents

### Swarm Tools
- `mcp__claude-flow__swarm_scale` - Scale up/down
- `mcp__claude-flow__load_balance` - Distribute load
- `mcp__claude-flow__agent_metrics` - Performance metrics
- `mcp__claude-flow__swarm_monitor` - Real-time monitoring

## Example Workflows

### Workflow 1: Research with Memory

```typescript
// Step 1: Research and store
await memoryResearchAgent(
  'Research top 5 React patterns and store each with keys: pattern_1 through pattern_5'
);

// Step 2: Later, retrieve and use
await memoryResearchAgent(
  'Retrieve all 5 React patterns from memory and create a summary'
);
```

### Workflow 2: Multi-Agent Development

```typescript
// Orchestrator coordinates researcher, coder, and reviewer
await orchestratorAgent(`
  Build a user authentication system:
  1. Initialize a hierarchical swarm
  2. Spawn a researcher to find best auth patterns
  3. Spawn a coder to implement based on research
  4. Spawn a reviewer to check the implementation
  5. Report results
`);
```

### Workflow 3: Incremental Knowledge Building

```typescript
// Session 1: Initial research
await memoryResearchAgent('Research REST API design and store key principles');

// Session 2: Add more knowledge
await memoryResearchAgent('Research GraphQL and compare with REST (use stored REST info)');

// Session 3: Apply knowledge
await orchestratorAgent('Build an API using all stored knowledge about REST and GraphQL');
```

## Configuration

### Enable/Disable Features

```typescript
import { claudeFlowAgent } from './src/agents/claudeFlowAgent.js';

// Memory only
await claudeFlowAgent(
  'my-agent',
  'System prompt',
  'Task',
  {
    enableMemory: true,
    enableCoordination: false
  }
);

// Coordination only
await claudeFlowAgent(
  'my-agent',
  'System prompt',
  'Task',
  {
    enableMemory: false,
    enableCoordination: true
  }
);

// Both
await claudeFlowAgent(
  'my-agent',
  'System prompt',
  'Task',
  {
    enableMemory: true,
    enableCoordination: true,
    swarmTopology: 'hierarchical'
  }
);
```

### Custom Configuration

```typescript
import { ClaudeFlowConfig } from './src/config/claudeFlow.js';

const config: ClaudeFlowConfig = {
  enableMemory: true,
  enableCoordination: true,
  enableSwarm: true,
  memoryNamespace: 'my-app',
  coordinationTopology: 'mesh'
};
```

## Docker Persistence

### Persist Memory Across Restarts

```bash
# Create volume for memory
docker volume create claude-agent-memory

# Run with volume mount
docker run \
  -v claude-agent-memory:/app/.swarm \
  --env-file .env \
  claude-agents:cli \
  --agent memory-researcher \
  --task "Your task"
```

### Docker Compose with Persistence

```yaml
services:
  memory-agent:
    build:
      context: ../..
      dockerfile: docker/claude-agent-sdk/Dockerfile
    command: ["--agent", "memory-researcher", "--task", "Research task"]
    environment:
      ANTHROPIC_API_KEY: ${ANTHROPIC_API_KEY}
    volumes:
      - agent-memory:/app/.swarm

volumes:
  agent-memory:
```

## Testing

### Run Memory Tests

```bash
npm run test:memory
```

### Run Coordination Tests

```bash
npm run test:coordination
```

### Run Hybrid Tests

```bash
npm run test:hybrid
```

### Run All Claude Flow Tests

```bash
npm run validate:claude-flow
```

### Docker Validation

```bash
./validation/claude-flow/docker-test.sh
```

## Performance

### Memory Operations
- Store: ~5-10ms
- Retrieve: ~2-5ms
- Search: ~10-50ms (depends on data size)

### Coordination Operations
- Swarm init: ~50-100ms
- Agent spawn: ~20-30ms per agent
- Task orchestrate: ~100-500ms (depends on complexity)

### Overhead
- MCP server startup: ~50ms (one-time per container)
- Per-tool invocation: ~10-20ms

## Troubleshooting

### Memory Not Persisting

**Issue**: Memory lost after container restart

**Solution**: Mount `.swarm` directory as volume
```bash
docker run -v $(pwd)/.swarm:/app/.swarm ...
```

### MCP Tools Not Available

**Issue**: Agent can't use claude-flow tools

**Solution**:
1. Check ANTHROPIC_API_KEY is set
2. Verify claude-flow is installed: `npx claude-flow --version`
3. Check MCP config in `src/config/tools.ts`

### Swarm Initialization Fails

**Issue**: Swarm doesn't initialize

**Solution**:
1. Check max agents limit (default: 8)
2. Verify topology is valid: mesh/hierarchical/ring/star
3. Check logs for detailed error

## Best Practices

### Memory Usage

1. **Use Namespaces**: Isolate different agent types
   ```typescript
   memoryNamespace: 'app:feature:agent'
   ```

2. **Set Appropriate TTL**: Don't store forever
   ```typescript
   ttl: 3600  // 1 hour for temporary data
   ttl: 86400 // 1 day for important data
   ```

3. **Clean Up**: Delete old data regularly
   ```typescript
   action: 'delete', key: 'old-data'
   ```

### Coordination Usage

1. **Choose Right Topology**:
   - Mesh: Collaborative, no leader
   - Hierarchical: Complex workflows, clear structure
   - Ring: Sequential processing
   - Star: Centralized control

2. **Limit Swarm Size**: 3-5 agents for most tasks

3. **Use Priorities**: High for critical tasks

4. **Monitor Status**: Check `swarm_status` regularly

## Advanced Patterns

### Pattern 1: Persistent Research Assistant

```typescript
async function persistentResearcher(topic: string) {
  // Check if we already researched this
  const existing = await searchMemory(topic);

  if (existing.length > 0) {
    return { cached: true, data: existing };
  }

  // New research
  const result = await memoryResearchAgent(
    `Research ${topic} and store findings`
  );

  return { cached: false, data: result };
}
```

### Pattern 2: Self-Improving Agent

```typescript
async function selfImprovingAgent(task: string) {
  // Retrieve past learnings
  const learnings = await searchMemory('lessons-learned');

  // Execute with context
  const result = await hybridAgent(
    `Task: ${task}\nPast learnings: ${learnings}`,
    'learner',
    'Apply past learnings to improve performance'
  );

  // Store new learnings
  await storeMemory('lessons-learned', result.insights);
}
```

### Pattern 3: Dynamic Team Assembly

```typescript
async function assembleTeam(requirements: string[]) {
  await swarmInit({ topology: 'hierarchical' });

  for (const req of requirements) {
    const agentType = determineAgentType(req);
    await spawnAgent({ type: agentType });
  }

  return await orchestrate('Execute requirements');
}
```

## See Also

- [Docker Agent Usage](./DOCKER_AGENT_USAGE.md)
- [Claude Agents Integration](./CLAUDE_AGENTS_INTEGRATION.md)
- [Validation Report](../validation/reports/claude-flow-integration.md)
- [Claude Flow Docs](https://github.com/ruvnet/claude-flow)
