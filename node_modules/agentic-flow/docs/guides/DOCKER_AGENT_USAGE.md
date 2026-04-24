# Docker Agent Usage Guide

This guide explains how to use the Claude Agent SDK Docker container with different agents and modes.

## Quick Start

### 1. Build the Docker Image

```bash
docker build -t claude-agents:latest .
```

### 2. List Available Agents

```bash
docker run claude-agents:latest --list
```

This shows all 74+ agents loaded from `.claude/agents/` directory.

### 3. Run a Specific Agent

```bash
docker run --env-file ../../.env claude-agents:latest \
  --agent goal-planner \
  --task "Create a plan to improve API performance"
```

## Usage Modes

### Parallel Mode (Default)

Runs 3 agents in parallel (research, code review, data):

```bash
# Using docker run
docker run --env-file ../../.env claude-agents:latest

# Using docker-compose
docker-compose up
```

**Environment Variables:**
- `TOPIC` - Research topic (default: "payments mandates")
- `DIFF` - Code diff for review (default: "refactor: route validation")
- `DATASET` - Data analysis hint (default: "transactions last 30 days")

### Agent Mode

Run a specific agent with a custom task:

```bash
docker run --env-file ../../.env claude-agents:latest \
  --agent <agent-name> \
  --task "Your task description" \
  [--stream]
```

**Options:**
- `--agent, -a` - Agent name (e.g., goal-planner, coder, reviewer)
- `--task, -t` - Task description for the agent
- `--stream, -s` - Enable real-time streaming output

**Examples:**

```bash
# Goal planner with streaming
docker run --env-file ../../.env claude-agents:latest \
  --agent goal-planner \
  --task "Plan a microservices migration" \
  --stream

# Code implementation
docker run --env-file ../../.env claude-agents:latest \
  --agent coder \
  --task "Implement JWT authentication middleware"

# Code review
docker run --env-file ../../.env claude-agents:latest \
  --agent reviewer \
  --task "Review src/utils/retry.ts for best practices"

# Testing
docker run --env-file ../../.env claude-agents:latest \
  --agent tester \
  --task "Create integration tests for API endpoints"

# Research
docker run --env-file ../../.env claude-agents:latest \
  --agent researcher \
  --task "Research database sharding strategies"
```

### List Mode

List all available agents:

```bash
docker run claude-agents:latest --list
```

## Docker Compose Examples

### Using Profiles

The `docker-compose.agent.yml` file includes examples for different agents using profiles:

```bash
# List agents
docker-compose -f docker-compose.agent.yml --profile list up list-agents

# Run goal planner
docker-compose -f docker-compose.agent.yml --profile goal-planner up goal-planner

# Run coder
docker-compose -f docker-compose.agent.yml --profile coder up coder

# Run reviewer
docker-compose -f docker-compose.agent.yml --profile reviewer up reviewer

# Run parallel mode
docker-compose -f docker-compose.agent.yml --profile parallel up parallel
```

### Custom Agent Tasks

Edit `docker-compose.agent.yml` to customize tasks:

```yaml
services:
  my-custom-task:
    build: .
    command:
      - "--agent"
      - "coder"
      - "--task"
      - "Your custom task here"
    environment:
      ANTHROPIC_API_KEY: ${ANTHROPIC_API_KEY}
```

## Environment Variables

### Required
- `ANTHROPIC_API_KEY` - Your Anthropic API key

### Optional
- `ENABLE_STREAMING` - Enable streaming output (true/false)
- `HEALTH_PORT` - Health check port (default: 8080)
- `NODE_ENV` - Environment (production for JSON logs)
- `KEEP_ALIVE` - Keep health server running (true/false)

### Parallel Mode Only
- `TOPIC` - Research topic
- `DIFF` - Code diff for review
- `DATASET` - Data analysis hint

### Agent Mode Only
- `AGENT` - Agent name (alternative to --agent)
- `TASK` - Task description (alternative to --task)

## Available Agents

### Planning & Strategy
- `goal-planner` - GOAP-based planning and optimization
- `planner` - Strategic planning and task orchestration

### Development
- `coder` - Clean code implementation
- `reviewer` - Code review specialist
- `tester` - Testing and QA specialist

### Research & Analysis
- `researcher` - Deep research specialist
- `code-analyzer` - Advanced code analysis
- `system-architect` - System design expert

### Flow Nexus
- `flow-nexus-swarm` - AI swarm orchestration
- `flow-nexus-neural` - Neural network training
- `flow-nexus-workflow` - Workflow automation
- `flow-nexus-sandbox` - E2B sandbox management

### GitHub Integration
- `github-modes` - GitHub workflow orchestration
- `pr-manager` - Pull request management
- `code-review-swarm` - Automated code reviews

### Consensus & Coordination
- `raft-manager` - Raft consensus
- `byzantine-coordinator` - Byzantine fault tolerance
- `gossip-coordinator` - Gossip protocols

**See full list with:** `docker run claude-agents:latest --list`

## Kubernetes Deployment

The container includes a health check endpoint for orchestration:

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: claude-agent
spec:
  containers:
  - name: agent
    image: claude-agents:latest
    args: ["--agent", "goal-planner", "--task", "Plan deployment"]
    env:
    - name: ANTHROPIC_API_KEY
      valueFrom:
        secretKeyRef:
          name: anthropic-secret
          key: api-key
    livenessProbe:
      httpGet:
        path: /health
        port: 8080
      initialDelaySeconds: 10
      periodSeconds: 30
```

## Advanced Usage

### Mounting .claude/agents Directory

If you want to use custom agents from a different location:

```bash
docker run -v /path/to/.claude/agents:/app/.claude/agents \
  --env-file .env \
  claude-agents:latest \
  --agent my-custom-agent \
  --task "Custom task"
```

### Saving Output to File

```bash
docker run --env-file ../../.env claude-agents:latest \
  --agent coder \
  --task "Generate API client" > output.txt
```

### Running in Background

```bash
docker run -d --name my-agent \
  --env-file ../../.env \
  claude-agents:latest \
  --agent researcher \
  --task "Research AI safety"

# Check logs later
docker logs my-agent
```

### Health Checks

```bash
# Start container with health checks enabled
docker run -d --env-file ../../.env \
  -e KEEP_ALIVE=true \
  -p 8080:8080 \
  claude-agents:latest

# Check health
curl http://localhost:8080/health
```

## Troubleshooting

### Agent Not Found

If you get "Agent not found" error:

1. List all agents: `docker run claude-agents:latest --list`
2. Check spelling - agent names are case-sensitive
3. Verify .claude/agents directory is mounted correctly

### API Key Issues

```bash
# Verify API key is set
docker run --env-file ../../.env claude-agents:latest \
  sh -c 'echo $ANTHROPIC_API_KEY'

# Check health endpoint
curl http://localhost:8080/health
```

### No Output

1. Check if streaming is enabled: add `--stream` flag
2. Check logs: `docker logs <container-id>`
3. Verify task is provided: `--task "Your task"`

### Build Issues

```bash
# Clean build
docker build --no-cache -t claude-agents:latest .

# Check TypeScript compilation
npm run build
```

## Examples

### Complete Workflow

```bash
# 1. Build image
docker build -t claude-agents:latest .

# 2. List available agents
docker run claude-agents:latest --list

# 3. Plan with goal-planner
docker run --env-file ../../.env claude-agents:latest \
  --agent goal-planner \
  --task "Plan feature X implementation" > plan.txt

# 4. Implement with coder
docker run --env-file ../../.env claude-agents:latest \
  --agent coder \
  --task "$(cat plan.txt)" > implementation.txt

# 5. Review with reviewer
docker run --env-file ../../.env claude-agents:latest \
  --agent reviewer \
  --task "Review: $(cat implementation.txt)" > review.txt

# 6. Create tests with tester
docker run --env-file ../../.env claude-agents:latest \
  --agent tester \
  --task "Create tests for: $(cat implementation.txt)"
```

## Performance

- **Startup Time**: ~2-5 seconds
- **Agent Load Time**: ~50ms for 74 agents
- **Streaming**: Real-time token streaming when enabled
- **Health Checks**: <10ms response time

## See Also

- [Claude Agents Integration](./CLAUDE_AGENTS_INTEGRATION.md)
- [Quick Wins Implementation](./QUICK_WINS.md)
- [Validation Report](../validation/reports/quick-wins-validation.md)
