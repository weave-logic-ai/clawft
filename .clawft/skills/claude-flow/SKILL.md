---
name: claude-flow
description: Orchestrate multi-agent swarms and tasks via claude-flow v3 MCP
version: 1.0.0
allowed-tools:
  - claude-flow__swarm_*
  - claude-flow__agent_*
  - claude-flow__task_*
  - claude-flow__memory_*
  - claude-flow__session_*
  - claude-flow__hive-mind_*
  - claude-flow__workflow_*
  - claude-flow__coordination_*
  - claude-flow__performance_*
  - claude-flow__system_*
user-invocable: true
argument-hint: Swarm objective or task description
---

# Claude Flow v3 MCP Skill

This skill provides access to claude-flow v3 swarm orchestration via the
`claude-flow` MCP server. It enables multi-agent coordination, task
management, shared memory, and workflow automation.

## Key Capability Categories

### Swarm Management (`swarm_*`)
Initialize, monitor, and shut down multi-agent swarms. Supports hierarchical,
mesh, and adaptive topologies with configurable agent counts and strategies.

### Agent Lifecycle (`agent_*`)
Spawn, list, monitor, update, and terminate individual agents. Supports 60+
agent types including coders, reviewers, testers, planners, and specialized
roles like security-architect and performance-engineer.

### Task Orchestration (`task_*`)
Create, list, update, complete, and cancel tasks. Tasks can be assigned to
agents and tracked through their lifecycle with status updates.

### Shared Memory (`memory_*`)
Store, retrieve, search, list, and delete shared knowledge across agents.
Supports namespaces, TTL, tags, and HNSW-powered semantic search for
coordination between agents working on related problems.

### Session Persistence (`session_*`)
Save, restore, list, and manage session state. Enables checkpointing of
agent progress and resuming work across restarts.

### Hive-Mind Consensus (`hive-mind_*`)
Initialize hive-mind clusters with Byzantine fault-tolerant consensus (Raft).
Broadcast messages, reach consensus on decisions, manage shared memory, and
coordinate agent groups for collective decision-making.

### Workflow Automation (`workflow_*`)
Create, execute, pause, resume, cancel, and delete workflows. Supports
workflow templates for repeatable multi-step processes with agent
orchestration.

### Coordination (`coordination_*`)
Node registration, topology management, load balancing, synchronization,
consensus, and metrics collection for distributed agent coordination.

### Performance Monitoring (`performance_*`)
Run benchmarks, detect bottlenecks, collect metrics, generate profiles and
reports, and apply optimizations across the agent infrastructure.

### System Health (`system_*`)
Check system status, health, metrics, and info. Reset system state when
needed.

## Common Usage Patterns

- **Spawning multi-agent swarms**: Use `swarm_init` to set up topology, then
  `agent_spawn` to create specialized agents for parallel work.
- **Coordinating parallel tasks**: Use `task_create` to define work items,
  assign them to agents, and track completion with `task_status`.
- **Sharing knowledge across agents**: Use `memory_store` and `memory_search`
  to let agents share findings, patterns, and decisions.
- **Running workflows**: Use `workflow_create` with templates, then
  `workflow_execute` for repeatable multi-step processes.
- **Consensus decisions**: Use `hive-mind_init` and `hive-mind_consensus` for
  collective decision-making across agent groups.

## Configuration Note

The `claude-flow` MCP server is configured as `internal_only`, meaning it is
not exposed to external consumers. Tools are namespaced as
`claude-flow__<tool>` (e.g. `claude-flow__swarm_init`,
`claude-flow__memory_store`).

## Reference

- Documentation: https://github.com/ruvnet/claude-flow/tree/main/v3
