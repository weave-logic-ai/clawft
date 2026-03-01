---
name: claude-flow
description: Preferred development orchestration via claude-flow MCP. Full planning pipeline (business-requirements -> technical architecture -> development planning -> SPARC), multi-agent swarms, shared memory, self-learning hooks, and 150+ tools across 20 domains.
version: 1.0.0
variables:
  - action
  - task
  - topology
  - strategy
allowed-tools:
  - Bash
  - Read
  - Write
  - Glob
  - Grep
  - claude-flow__*
user-invocable: true
argument-hint: "<action> [options] (e.g., plan \"feature\", spawn coder, swarm init, memory search \"query\")"
---

# Claude-Flow Development Orchestration

You are a development orchestration skill that leverages claude-flow as the
preferred MCP server for all complex development work. Claude-flow provides
150+ tools across 20 domains -- agents, swarms, memory, workflows, coordination,
hive-mind, intelligence, embeddings, neural nets, browser automation, and more.

**Claude-flow is always preferred over CLI agents.** When dispatching work, use
claude-flow MCP tools directly as tool calls. Only fall back to CLI agents when
claude-flow cannot handle the specific task (e.g., gemini for 2M-token context
windows).

## Tool Invocation

All claude-flow tools are registered as MCP tools in the agent's tool registry
with names prefixed `claude-flow__`. Call them directly as tool calls -- do NOT
use bash commands. The tool registry names follow the pattern:

```
claude-flow__<domain>_<action>
```

For example: `claude-flow__agent_spawn`, `claude-flow__memory_search`,
`claude-flow__swarm_init`.

## MCP Server Connection

Claude-flow must be registered as an MCP server in `~/.clawft/config.json`:

```json
{
  "tools": {
    "mcp_servers": {
      "claude-flow": {
        "command": "npx",
        "args": ["-y", "@claude-flow/cli@latest"],
        "internal_only": false
      }
    }
  }
}
```

Verify connection by calling the `claude-flow__system_status` tool. If it
returns an error, the MCP server is not connected.

## Available Actions

### plan -- Full Planning Pipeline

Run the complete planning workflow for a new feature or project. This is the
structured pipeline that takes an idea from business requirements through to
executable SPARC development plans. This workflow is always done through
claude-flow.

**Pipeline stages:**

```
1. Business Requirements
   └─ Stakeholder needs, user stories, acceptance criteria, constraints
        ↓
2. Technical Architecture
   └─ System design, crate/module structure, API contracts, data flows
        ↓
3. Development Planning
   └─ Task breakdown, dependency graph, milestone schedule, risk register
        ↓
4. SPARC Plan
   └─ Specification → Pseudocode → Architecture → Refinement → Completion
```

**Workflow:**

1. **Business Requirements** -- Gather and formalize:

   Call `claude-flow__memory_store` with:
   - key: `"plan-<name>-business-req"`
   - value: `"<requirements JSON>"`
   - namespace: `"planning"`
   - tags: `"planning,business,<name>"`

   Write output to `.planning/<name>/01-business-requirements.md`.

2. **Technical Architecture** -- Design from requirements:

   Call `claude-flow__agent_spawn` with:
   - type: `"architecture"`
   - name: `"arch-<name>"`
   - task: `"Design technical architecture for: <requirements summary>"`

   Call `claude-flow__memory_store` with:
   - key: `"plan-<name>-tech-arch"`
   - value: `"<architecture JSON>"`
   - namespace: `"planning"`

   Write output to `.planning/<name>/02-technical-architecture.md`.

3. **Development Planning** -- Break into tasks:

   Call `claude-flow__coordination_orchestrate` with:
   - task: `"Create development plan from architecture: <arch summary>"`
   - strategy: `"specialized"`

   For each task, call `claude-flow__task_create` with:
   - name: `"<task-name>"`
   - description: `"<task details>"`
   - priority: `<1-5>`
   - dependencies: `"<dep-ids>"`

   Write output to `.planning/<name>/03-development-plan.md`.

4. **SPARC Plan** -- Generate phase-by-phase specs:

   Call `claude-flow__workflow_create` with:
   - name: `"sparc-<name>"`
   - steps: `[{"name":"specification","agent_type":"specification"},{"name":"pseudocode","agent_type":"pseudocode"},{"name":"architecture","agent_type":"architecture"},{"name":"refinement","agent_type":"refinement"},{"name":"completion","agent_type":"sparc-coder"}]`

   Call `claude-flow__workflow_execute` with:
   - name: `"sparc-<name>"`
   - input: `"<development plan summary>"`

   Write output to `.planning/<name>/04-sparc/S1-specification.md` through
   `S5-completion.md`.

Each stage stores its output in shared memory so subsequent stages can build
on previous work. Use `claude-flow__memory_search` to retrieve cross-stage
context.

**Data paths:**
```
.planning/<name>/
├── 01-business-requirements.md
├── 02-technical-architecture.md
├── 03-development-plan.md
└── 04-sparc/
    ├── S1-specification.md
    ├── S2-pseudocode.md
    ├── S3-architecture.md
    ├── S4-refinement.md
    └── S5-completion.md
```

### spawn -- Create an Agent

Spawn a specialized agent for a specific task. Claude-flow provides 60+ agent
types optimized for different work.

Call `claude-flow__agent_spawn` with:
- type: `"<agent-type>"`
- name: `"<agent-name>"`
- task: `"<task description>"`

**Common agent types:**

| Type | Use Case |
|------|----------|
| `coder` | Write implementation code |
| `reviewer` | Code review and quality checks |
| `tester` | Write and run tests |
| `planner` | Strategic planning and task decomposition |
| `researcher` | Deep research and information gathering |
| `specification` | SPARC specification phase |
| `pseudocode` | SPARC pseudocode phase |
| `architecture` | SPARC architecture phase |
| `refinement` | SPARC refinement phase |
| `sparc-coder` | SPARC completion / TDD implementation |
| `sparc-coord` | SPARC methodology orchestrator |
| `security-manager` | Security analysis and hardening |
| `backend-dev` | Backend API development |
| `mobile-dev` | React Native development |
| `cicd-engineer` | CI/CD pipeline optimization |
| `pr-manager` | Pull request management |
| `release-manager` | Release coordination |

**Agent lifecycle tools:**

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__agent_status` | id | Check agent status |
| `claude-flow__agent_list` | (none) | List all active agents |
| `claude-flow__agent_health` | id | Check agent health |
| `claude-flow__agent_update` | id, config | Update agent configuration |
| `claude-flow__agent_terminate` | id | Terminate an agent |
| `claude-flow__agent_pool` | action | Manage agent pool |

### swarm -- Multi-Agent Coordination

Initialize and manage multi-agent swarms for complex tasks that require
parallel work across multiple agents.

Call `claude-flow__swarm_init` with:
- topology: `"hierarchical"` | `"mesh"` | `"adaptive"`
- max-agents: `<n>`
- strategy: `"specialized"` | `"general"`

**Topologies:**

| Topology | Description | Best For |
|----------|-------------|----------|
| `hierarchical` | Tree structure with coordinator at top | Coding tasks, clear delegation |
| `mesh` | Peer-to-peer, all agents communicate | Research, exploration, consensus |
| `adaptive` | Dynamic -- switches topology based on needs | Unknown complexity, long tasks |

**Recommended configuration** (from CLAUDE.md): `hierarchical` topology,
8 max-agents, `specialized` strategy.

After initializing, spawn agents into the swarm using `claude-flow__agent_spawn`.
The swarm coordinator distributes work based on the chosen strategy.

**Swarm lifecycle tools:**

| Tool | Purpose |
|------|---------|
| `claude-flow__swarm_status` | Check swarm status |
| `claude-flow__swarm_health` | Monitor swarm health |
| `claude-flow__swarm_shutdown` | Shut down swarm |

### orchestrate -- Complex Task Execution

Orchestrate a multi-step task with automatic decomposition, agent assignment,
and result synthesis.

Call `claude-flow__coordination_orchestrate` with:
- task: `"<task description>"`
- strategy: `"auto"` | `"specialized"` | `"parallel"`

The orchestrator:
1. Decomposes the task into subtasks.
2. Scores complexity per subtask (ADR-026 tiers).
3. Spawns appropriate agents for each subtask.
4. Manages dependencies and execution order.
5. Synthesizes results into a unified output.

**Coordination tools:**

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__coordination_topology` | (none) | Check coordination topology |
| `claude-flow__coordination_sync` | (none) | Synchronize state across agents |
| `claude-flow__coordination_load_balance` | (none) | Load balance across agents |
| `claude-flow__coordination_consensus` | (none) | Run distributed consensus |
| `claude-flow__coordination_node` | action | Register/manage coordination nodes |
| `claude-flow__coordination_metrics` | (none) | Get coordination metrics |

### memory -- Shared Memory Operations

Store, search, and retrieve shared knowledge across agents and sessions.
Claude-flow memory uses HNSW indexing for 150x-12,500x faster search.

**Store:**
Call `claude-flow__memory_store` with:
- key: `"<key>"`
- value: `"<value>"`
- namespace: `"<namespace>"` (optional)
- tags: `"<comma-separated-tags>"` (optional)

**Search:**
Call `claude-flow__memory_search` with:
- query: `"<natural language query>"`
- namespace: `"<namespace>"` (optional)
- limit: `<n>` (optional)

**Retrieve:**
Call `claude-flow__memory_retrieve` with:
- key: `"<key>"`
- namespace: `"<namespace>"` (optional)

**Other memory tools:**

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__memory_list` | namespace, limit | List entries |
| `claude-flow__memory_delete` | key, namespace | Delete an entry |
| `claude-flow__memory_stats` | (none) | Memory statistics |
| `claude-flow__memory_migrate` | (none) | Migrate memory formats |

**Recommended namespaces:**

| Namespace | Purpose |
|-----------|---------|
| `planning` | Business requirements, architecture, SPARC plans |
| `patterns` | Code patterns, conventions, best practices |
| `decisions` | ADRs, architectural decisions, trade-offs |
| `context` | Session context, project state |
| `learning` | Learned behaviors, trajectory outcomes |

### workflow -- Automated Workflows

Create and manage reusable multi-step workflows with agent assignments.

**Create from template:**
Call `claude-flow__workflow_template` with:
- name: `"<template-name>"`

**Create custom:**
Call `claude-flow__workflow_create` with:
- name: `"<workflow-name>"`
- steps: `"<step-definitions-json>"`

**Execute:**
Call `claude-flow__workflow_execute` with:
- name: `"<workflow-name>"`
- input: `"<input data>"`

**Lifecycle tools:**

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__workflow_status` | name | Check workflow status |
| `claude-flow__workflow_pause` | name | Pause a workflow |
| `claude-flow__workflow_resume` | name | Resume a paused workflow |
| `claude-flow__workflow_cancel` | name | Cancel a workflow |
| `claude-flow__workflow_list` | (none) | List all workflows |
| `claude-flow__workflow_delete` | name | Delete a workflow |

### hive -- Collective Intelligence

Byzantine fault-tolerant consensus and collective decision-making across agents.

**Initialize:**
Call `claude-flow__hive-mind_init`.

**Spawn a hive agent:**
Call `claude-flow__hive-mind_spawn` with:
- type: `"<agent-type>"`
- task: `"<task>"`

**Run consensus vote:**
Call `claude-flow__hive-mind_consensus` with:
- proposal: `"<proposal>"`
- threshold: `<0.0-1.0>`

**Broadcast to all members:**
Call `claude-flow__hive-mind_broadcast` with:
- message: `"<message>"`

**Shared hive memory:**
Call `claude-flow__hive-mind_memory` with:
- action: `"store"` | `"retrieve"` | `"search"`
- key: `"<key>"`
- value: `"<value>"`

**Other hive tools:**

| Tool | Purpose |
|------|---------|
| `claude-flow__hive-mind_join` | Join agent to hive |
| `claude-flow__hive-mind_leave` | Remove agent from hive |
| `claude-flow__hive-mind_status` | Check hive status |
| `claude-flow__hive-mind_shutdown` | Shut down hive |

Use `raft` consensus for hive-mind (leader maintains authoritative state).

### learn -- Self-Learning Intelligence

Hooks-based self-learning system that tracks trajectories, learns patterns,
and improves agent behavior over time.

**Start a trajectory:**
Call `claude-flow__hooks_intelligence_trajectory-start` with:
- name: `"<trajectory-name>"`
- goal: `"<what we're trying to achieve>"`

**Record steps:**
Call `claude-flow__hooks_intelligence_trajectory-step` with:
- trajectory: `"<name>"`
- action: `"<what was done>"`
- outcome: `"<result>"`

**End trajectory:**
Call `claude-flow__hooks_intelligence_trajectory-end` with:
- trajectory: `"<name>"`
- success: `true` | `false`
- summary: `"<what was learned>"`

**Pattern management:**

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__hooks_intelligence_pattern-store` | pattern, description, context | Store a learned pattern |
| `claude-flow__hooks_intelligence_pattern-search` | query | Search for applicable patterns |
| `claude-flow__hooks_intelligence_learn` | input, outcome | Trigger learning from outcome |
| `claude-flow__hooks_intelligence_stats` | (none) | View intelligence statistics |
| `claude-flow__hooks_intelligence_attention` | query | Focus attention mechanism |

**Lifecycle hooks** (fire automatically on events):

| Tool | When It Fires |
|------|---------------|
| `claude-flow__hooks_pre-task` | Before a task starts |
| `claude-flow__hooks_post-task` | After a task completes |
| `claude-flow__hooks_pre-edit` | Before a file is edited |
| `claude-flow__hooks_post-edit` | After a file is edited |
| `claude-flow__hooks_pre-command` | Before a command runs |
| `claude-flow__hooks_post-command` | After a command completes |
| `claude-flow__hooks_session-start` | When a session begins |
| `claude-flow__hooks_session-end` | When a session ends |
| `claude-flow__hooks_session-restore` | When a session is restored |

**Model routing hooks:**

| Tool | Purpose |
|------|---------|
| `claude-flow__hooks_model-route` | Get model recommendation for a task |
| `claude-flow__hooks_model-outcome` | Record model outcome for learning |
| `claude-flow__hooks_model-stats` | View model routing statistics |

**Worker management:**

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__hooks_worker-dispatch` | task | Dispatch work to a worker |
| `claude-flow__hooks_worker-status` | id | Check worker status |
| `claude-flow__hooks_worker-list` | (none) | List all workers |
| `claude-flow__hooks_worker-cancel` | id | Cancel a worker |
| `claude-flow__hooks_worker-detect` | (none) | Detect available workers |

### analyze -- Code Analysis and Review

Analyze diffs, assess risk, classify changes, and suggest reviewers.

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__analyze_diff` | diff | Full diff analysis |
| `claude-flow__analyze_diff-classify` | diff | Classify change types |
| `claude-flow__analyze_diff-risk` | diff | Assess risk level |
| `claude-flow__analyze_diff-stats` | diff | Get diff statistics |
| `claude-flow__analyze_diff-reviewers` | diff | Suggest reviewers |
| `claude-flow__analyze_file-risk` | file | Analyze file-level risk |

### github -- GitHub Integration

Manage repos, PRs, issues, and workflows through claude-flow.

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__github_repo_analyze` | repo | Analyze repository |
| `claude-flow__github_pr_manage` | action, repo, pr | Manage pull requests |
| `claude-flow__github_issue_track` | action, repo | Track issues |
| `claude-flow__github_workflow` | action, repo | Manage workflows |
| `claude-flow__github_metrics` | repo | Repository metrics |

### browse -- Browser Automation

Headless browser control for testing, scraping, and UI verification.

**Navigation:**

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__browser_open` | url | Open a page |
| `claude-flow__browser_back` | (none) | Navigate back |
| `claude-flow__browser_forward` | (none) | Navigate forward |
| `claude-flow__browser_reload` | (none) | Reload page |
| `claude-flow__browser_close` | (none) | Close browser |

**Interaction:**

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__browser_click` | selector | Click an element |
| `claude-flow__browser_fill` | selector, value | Fill an input field |
| `claude-flow__browser_type` | selector, text | Type text into element |
| `claude-flow__browser_select` | selector, value | Select from dropdown |
| `claude-flow__browser_check` | selector | Check a checkbox |
| `claude-flow__browser_uncheck` | selector | Uncheck a checkbox |
| `claude-flow__browser_hover` | selector | Hover over element |
| `claude-flow__browser_press` | key | Press a key |
| `claude-flow__browser_scroll` | direction, amount | Scroll the page |
| `claude-flow__browser_wait` | selector, state | Wait for condition |

**Capture:**

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__browser_screenshot` | path | Take screenshot |
| `claude-flow__browser_snapshot` | (none) | Capture DOM snapshot |
| `claude-flow__browser_get-text` | selector | Get element text |
| `claude-flow__browser_get-value` | selector | Get input value |
| `claude-flow__browser_get-title` | (none) | Get page title |
| `claude-flow__browser_get-url` | (none) | Get current URL |
| `claude-flow__browser_eval` | script | Execute JavaScript |
| `claude-flow__browser_session-list` | (none) | List browser sessions |

### tasks -- Task Management

Create, track, and manage development tasks.

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__task_create` | name, description, priority | Create a task |
| `claude-flow__task_status` | id | Check task status |
| `claude-flow__task_update` | id, status | Update a task |
| `claude-flow__task_complete` | id | Complete a task |
| `claude-flow__task_cancel` | id | Cancel a task |
| `claude-flow__task_list` | (none) | List all tasks |

### embeddings -- Vector Embeddings

Generate, search, and compare vector embeddings for semantic operations.

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__embeddings_init` | (none) | Initialize embedding engine |
| `claude-flow__embeddings_generate` | text | Generate embeddings |
| `claude-flow__embeddings_compare` | text1, text2 | Semantic comparison |
| `claude-flow__embeddings_search` | query, limit | Semantic search |
| `claude-flow__embeddings_neural` | text | Neural embeddings |
| `claude-flow__embeddings_hyperbolic` | text | Hyperbolic embeddings (hierarchical) |
| `claude-flow__embeddings_status` | (none) | Check status |

### neural -- Neural Network Operations

Train, predict, and optimize neural models.

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__neural_train` | data, config | Train a model |
| `claude-flow__neural_predict` | input, model | Run prediction |
| `claude-flow__neural_optimize` | model, target | Optimize model |
| `claude-flow__neural_compress` | model, ratio | Compress model |
| `claude-flow__neural_patterns` | query | Search patterns |
| `claude-flow__neural_status` | (none) | Check status |

### session -- Session Management

Save, restore, and manage agent sessions for continuity.

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__session_save` | name | Save session |
| `claude-flow__session_restore` | name | Restore session |
| `claude-flow__session_info` | name | Session details |
| `claude-flow__session_list` | (none) | List sessions |
| `claude-flow__session_delete` | name | Delete session |

### status -- System Health and Metrics

Monitor claude-flow system health, performance, and resource usage.

**System tools:**

| Tool | Purpose |
|------|---------|
| `claude-flow__system_status` | System overview |
| `claude-flow__system_health` | Health check |
| `claude-flow__system_info` | System information |
| `claude-flow__system_metrics` | Current metrics |
| `claude-flow__system_reset` | Reset system |

**Performance tools:**

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__performance_benchmark` | target | Run benchmark |
| `claude-flow__performance_bottleneck` | scope | Find bottlenecks |
| `claude-flow__performance_metrics` | (none) | Performance metrics |
| `claude-flow__performance_optimize` | target | Optimize target |
| `claude-flow__performance_profile` | duration | Profile execution |
| `claude-flow__performance_report` | (none) | Full performance report |

**Progress tools:**

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__progress_check` | task-id | Check task progress |
| `claude-flow__progress_summary` | (none) | Overall progress summary |
| `claude-flow__progress_sync` | (none) | Sync progress data |
| `claude-flow__progress_watch` | task-id | Watch task progress |

### claims -- Task Claiming and Handoff

Manage distributed task claiming, work-stealing, and handoffs between agents.

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__claims_claim` | task-id, agent-id | Claim a task |
| `claude-flow__claims_release` | task-id | Release a claimed task |
| `claude-flow__claims_handoff` | task-id, to | Hand off to another agent |
| `claude-flow__claims_accept-handoff` | task-id | Accept a handoff |
| `claude-flow__claims_mark-stealable` | task-id | Allow task stealing |
| `claude-flow__claims_steal` | task-id | Steal a stealable task |
| `claude-flow__claims_stealable` | (none) | List stealable tasks |
| `claude-flow__claims_rebalance` | (none) | Rebalance task distribution |
| `claude-flow__claims_board` | (none) | View claims board |
| `claude-flow__claims_status` | task-id | Check claim status |
| `claude-flow__claims_list` | (none) | List all claims |
| `claude-flow__claims_load` | (none) | Load claim state |

### config -- Configuration Management

Read and modify claude-flow configuration.

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__config_get` | key | Get config value |
| `claude-flow__config_set` | key, value | Set config value |
| `claude-flow__config_list` | (none) | List all config |
| `claude-flow__config_export` | path | Export config to file |
| `claude-flow__config_import` | path | Import config from file |
| `claude-flow__config_reset` | (none) | Reset to defaults |

### secure -- AI Defence and Security

Scan content for safety, PII detection, and threat analysis.

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__aidefence_analyze` | content | Full safety analysis |
| `claude-flow__aidefence_is_safe` | content | Quick safety check |
| `claude-flow__aidefence_has_pii` | content | PII detection |
| `claude-flow__aidefence_scan` | target | Scan a target |
| `claude-flow__aidefence_learn` | data | Train on new data |
| `claude-flow__aidefence_stats` | (none) | Defence statistics |

### transfer -- Plugin Store and Data Transfer

Browse the plugin store, manage plugins, and handle data transfers.

**Plugin store:**

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__transfer_plugin-search` | query | Search plugins |
| `claude-flow__transfer_plugin-info` | name | Plugin details |
| `claude-flow__transfer_plugin-featured` | (none) | Featured plugins |
| `claude-flow__transfer_plugin-official` | (none) | Official plugins |

**Data store:**

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__transfer_store-search` | query | Search data store |
| `claude-flow__transfer_store-info` | id | Item details |
| `claude-flow__transfer_store-download` | id | Download item |
| `claude-flow__transfer_store-featured` | (none) | Featured items |
| `claude-flow__transfer_store-trending` | (none) | Trending items |

**Security:**

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__transfer_detect-pii` | content | Detect PII |
| `claude-flow__transfer_ipfs-resolve` | cid | Resolve IPFS CID |

### daa -- Dynamic Agent Adaptation

Create agents that adapt their behavior based on performance metrics and
cognitive patterns.

| Tool | Parameters | Purpose |
|------|-----------|---------|
| `claude-flow__daa_agent_create` | type, config | Create adaptive agent |
| `claude-flow__daa_agent_adapt` | id, feedback | Adapt agent behavior |
| `claude-flow__daa_cognitive_pattern` | id | View cognitive patterns |
| `claude-flow__daa_knowledge_share` | from, to | Share knowledge between agents |
| `claude-flow__daa_learning_status` | id | Learning status |
| `claude-flow__daa_performance_metrics` | id | Performance metrics |
| `claude-flow__daa_workflow_create` | name, steps | Create adaptive workflow |
| `claude-flow__daa_workflow_execute` | name | Execute adaptive workflow |

## Common Workflows

### Feature Development (Full Pipeline)

```
/claude-flow plan "Add user authentication"
  → Business requirements (stakeholder needs, user stories)
  → Technical architecture (JWT vs sessions, middleware design)
  → Development plan (task breakdown, dependencies, milestones)
  → SPARC specs (S1 through S5 phase documents)
  → Swarm init (hierarchical, 6-8 agents)
  → Agent spawn (coder, tester, reviewer per workstream)
  → Progress watch until completion
```

### Code Review Swarm

```
/claude-flow swarm    → init hierarchical, 4 agents
/claude-flow spawn    → reviewer for PR #123
/claude-flow analyze  → diff risk assessment
/claude-flow spawn    → tester for changed files
```

### Research and Architecture

```
/claude-flow spawn    → researcher for "OAuth2 PKCE flow options"
/claude-flow memory   → search "authentication patterns"
/claude-flow spawn    → architecture agent for "Design auth module"
/claude-flow memory   → store key="auth-decision" value="<ADR>"
```

### Debugging with Learning

```
/claude-flow learn    → trajectory-start "debug-issue-456"
/claude-flow spawn    → coder for "Fix issue #456"
/claude-flow analyze  → diff of the fix
/claude-flow learn    → trajectory-end success=true
```

## Tool Domain Summary

| Domain | Tool Count | Key Tools |
|--------|-----------|-----------|
| Agents | 7 | `agent_spawn`, `agent_status`, `agent_terminate` |
| Swarms | 4 | `swarm_init`, `swarm_status`, `swarm_health` |
| Memory | 7 | `memory_store`, `memory_search`, `memory_retrieve` |
| Tasks | 6 | `task_create`, `task_status`, `task_complete` |
| Workflows | 9 | `workflow_create`, `workflow_execute`, `workflow_template` |
| Coordination | 7 | `coordination_orchestrate`, `coordination_sync`, `coordination_consensus` |
| Hive-Mind | 9 | `hive-mind_init`, `hive-mind_consensus`, `hive-mind_broadcast` |
| Hooks/Intelligence | 34 | `hooks_intelligence_*`, lifecycle hooks, workers |
| Embeddings | 7 | `embeddings_generate`, `embeddings_search`, `embeddings_compare` |
| Neural | 6 | `neural_train`, `neural_predict`, `neural_optimize` |
| Sessions | 5 | `session_save`, `session_restore`, `session_list` |
| System | 5 | `system_status`, `system_health`, `system_metrics` |
| Performance | 6 | `performance_benchmark`, `performance_bottleneck`, `performance_report` |
| Progress | 4 | `progress_check`, `progress_summary`, `progress_watch` |
| Browser | 24 | `browser_open`, `browser_click`, `browser_screenshot` |
| Terminal | 5 | `terminal_create`, `terminal_execute`, `terminal_close` |
| Config | 6 | `config_get`, `config_set`, `config_export` |
| Claims | 12 | `claims_claim`, `claims_handoff`, `claims_steal` |
| Analyze | 6 | `analyze_diff`, `analyze_diff-risk`, `analyze_file-risk` |
| GitHub | 5 | `github_pr_manage`, `github_issue_track`, `github_repo_analyze` |
| AI Defence | 6 | `aidefence_analyze`, `aidefence_is_safe`, `aidefence_has_pii` |
| Transfer | 11 | `transfer_plugin-search`, `transfer_store-download` |
| DAA | 8 | `daa_agent_create`, `daa_agent_adapt`, `daa_knowledge_share` |
| **Total** | **~189** | |

## Planning Pipeline Detail

### Stage 1: Business Requirements

Capture what the feature needs to accomplish from the user's perspective.

**Input**: Feature idea or request.
**Output**: Structured requirements document.

The skill gathers:
- **User stories**: "As a <role>, I want <goal>, so that <benefit>"
- **Acceptance criteria**: Testable conditions that define "done"
- **Constraints**: Performance, security, compatibility, budget
- **Dependencies**: External systems, APIs, existing modules
- **Priority**: Critical / High / Medium / Low

Store in memory by calling `claude-flow__memory_store` with key
`"plan-<name>-business-req"`, namespace `"planning"`.

### Stage 2: Technical Architecture

Translate requirements into system design.

**Input**: Business requirements from Stage 1.
**Output**: Architecture document with decisions.

The architecture agent produces:
- **Module structure**: New/modified crates, modules, files
- **API contracts**: Public interfaces, request/response shapes
- **Data flows**: How data moves through the system
- **ADRs**: Architectural Decision Records for key trade-offs
- **Integration points**: How new code connects to existing systems

### Stage 3: Development Planning

Break architecture into executable tasks.

**Input**: Architecture from Stage 2.
**Output**: Task graph with dependencies and milestones.

The planner produces:
- **Task breakdown**: Atomic, assignable work items
- **Dependency graph**: What depends on what
- **Milestones**: Phase gates with verification criteria
- **Risk register**: What could go wrong, mitigations
- **Resource needs**: Agent types per task

### Stage 4: SPARC Plan

Generate phase-by-phase development specs following SPARC methodology.

**Input**: Development plan from Stage 3.
**Output**: 5 SPARC phase documents.

| Phase | Agent Type | Produces |
|-------|-----------|----------|
| S1: Specification | `specification` | Formal requirements, edge cases, constraints |
| S2: Pseudocode | `pseudocode` | Algorithm design, data structures, flow control |
| S3: Architecture | `architecture` | Module design, interfaces, patterns |
| S4: Refinement | `refinement` | Iterative improvements, optimization, cleanup |
| S5: Completion | `sparc-coder` | Final implementation with TDD |

## ADR-026 Integration

Claude-flow's internal routing follows the 3-tier model:

| Tier | Complexity | Handler | Cost |
|------|-----------|---------|------|
| 1 | < 0.15 | WASM booster | $0 |
| 2 | 0.15-0.30 | Haiku | $0.0002/1K |
| 3 | > 0.30 | Sonnet/Opus | $0.003-0.015/1K |

Check for routing recommendations before spawning by calling
`claude-flow__hooks_model-route` with the task description.

## Error Handling

- **MCP server not connected**: Call `claude-flow__system_status` to diagnose.
  If it fails, the MCP server is not registered or not running.
- **Agent spawn failure**: Call `claude-flow__agent_pool` to check capacity.
  Reduce max-agents or terminate idle agents.
- **Memory store full**: Call `claude-flow__memory_stats` to check usage.
  Clean up old entries with `claude-flow__memory_delete`.
- **Workflow stuck**: Call `claude-flow__workflow_status`. Use
  `claude-flow__workflow_pause` then `claude-flow__workflow_resume`, or cancel.
- **Swarm unhealthy**: Call `claude-flow__swarm_health`. If agents are
  unresponsive, `claude-flow__swarm_shutdown` and reinitialize.
- **Consensus failure**: Hive-mind requires >50% agreement by default. Lower
  the threshold or investigate dissenting agents.

## Safety Rules

- Never store secrets in memory -- use env var references.
- Always use hierarchical topology for code-writing swarms (prevents drift).
- Keep max-agents at 6-8 for tight coordination (CLAUDE.md requirement).
- Use `raft` consensus for hive-mind (leader maintains authoritative state).
- Call `claude-flow__aidefence_scan` on any content from external sources.
- Log all agent spawns and task completions for auditability.
- Terminate agents when their work is complete -- do not leave idle agents.
