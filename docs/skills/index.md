# Skill Index

Machine-readable skill registry for all built-in clawft skills. Each entry
documents the skill's API surface: invocation, parameters, actions, tools,
data paths, and error codes.

For an introduction to the skill system (format, discovery, security), see
[Skills and Agents](../guides/skills-and-agents.md).

---

## Registry Summary

| Name | Version | User-Invocable | Allowed Tools | Category |
|------|---------|----------------|---------------|----------|
| [`agent-dispatch`](#agent-dispatch) | 1.1.0 | `/agent-dispatch` | Bash, Read, Write, Glob | routing |
| [`claude-flow`](#claude-flow) | 1.0.0 | `/claude-flow` | Bash, Read, Write, Glob, Grep | orchestration |
| [`discord`](#discord) | 1.0.0 | `/discord` | Bash | channel |
| [`linkedin`](#linkedin) | 1.0.0 | `/linkedin` | Bash, Read, Write, Glob | social |
| [`prompt-log`](#prompt-log) | 1.0.0 | `/prompt-log` | Read, Write, Bash, Glob | utility |
| [`skill-vetting`](#skill-vetting) | 1.0.0 | `/skill-vetting` | Read, Bash, Glob, Grep | security |
| [`social-auth`](#social-auth) | 1.0.0 | `/social-auth` | Bash, Read, Write | auth |
| [`twitter`](#twitter) | 1.0.0 | `/twitter` | Bash, Read, Write, Glob | social |

### Categories

- **auth** -- Authentication and credential management.
- **channel** -- Messaging platform adapters.
- **orchestration** -- Development orchestration, planning pipelines, multi-agent coordination.
- **routing** -- LightLLM task routing and multi-agent dispatch.
- **security** -- Auditing, vetting, and security workflows.
- **social** -- Social media platform integrations.
- **utility** -- Developer tools and session management.

---

## agent-dispatch

Route tasks to external agents via MCP (preferred), delegate tool, or CLI
fallback. LightLLM translation layer with intelligent routing based on task
complexity, cost, and agent capabilities. Extends clawft's internal 3-tier
routing (ADR-026) outward to external agents.

**Source**: [`skills/agent-dispatch/SKILL.md`](../../skills/agent-dispatch/SKILL.md)

### Metadata

| Field | Value |
|-------|-------|
| Version | `1.1.0` |
| Category | routing |
| User-invocable | yes |
| Argument hint | `<action> [--agent <name>] [prompt]` |
| Allowed tools | `Bash`, `Read`, `Write`, `Glob` |

### Variables

| Name | Description |
|------|-------------|
| `action` | The operation: `send`, `route`, `agents`, `history` |
| `agent` | Target agent name (optional -- auto-routed if omitted) |
| `prompt` | The task to dispatch |

### Transport Priority

Transports are resolved per-agent in strict priority order:

| Priority | Transport | Mechanism | When Used |
|----------|-----------|-----------|-----------|
| 1 (preferred) | MCP | `weft tool <server>__<tool>` | Agent registered in `tools.mcp_servers` |
| 2 | Delegate | `weft tool delegate_task` | Claude with API key, no MCP server |
| 3 (fallback) | CLI | `echo '<prompt>' \| <agent>` | No MCP, no delegate available |

### Actions

| Action | Description | Details |
|--------|-------------|---------|
| `send` | Dispatch a task via best available transport | MCP > delegate > CLI |
| `route` | Explain routing decision without dispatching | Shows agent, transport, and reason |
| `agents` | List all available agents and their transports | Scans MCP servers, delegate, and PATH |
| `history` | View recent dispatch log | Reads from workspace dispatch directory |

### Routing Matrix

| Signal | Best Agent | Transport Preference |
|--------|-----------|---------------------|
| Simple question | gemini | CLI |
| Code generation with file context | claude-flow | MCP `task_orchestrate` > delegate > CLI |
| Large file analysis (>100K tokens) | gemini | CLI (2M context) |
| Multi-file refactoring | claude-flow | MCP `agent_spawn` (coder type) |
| Swarm coordination | claude-flow | MCP `swarm_init` + `agent_spawn` |
| Memory-aware tasks | claude-flow | MCP `memory_search` + `memory_store` |
| Research with web access | gemini | CLI (Google Search grounding) |
| Batch code fixes | codex | CLI `codex --quiet` |

### Complexity-to-Agent Mapping

| Complexity | Internal Tier | External Preference |
|------------|---------------|---------------------|
| < 0.15 | Tier 1 (WASM) | Handle locally (skip dispatch) |
| 0.15-0.30 | Tier 2 (Haiku) | gemini or cheapest available |
| 0.30-0.70 | Tier 3 (Sonnet) | claude-flow (MCP) or claude (delegate/CLI) |
| > 0.70 | Tier 3 (Opus) | claude-flow (MCP with orchestration) |

### Data Paths

| Path | Contents |
|------|----------|
| `~/.clawft/config.json` | MCP servers + dispatch agent configs |
| `~/.clawft/workspace/dispatch/responses/<ts>-<agent>.md` | Saved responses |
| `~/.clawft/workspace/dispatch/routing-log.jsonl` | Routing decision + transport log |

### Dependencies

- At least one agent available via MCP, delegate, or CLI.
- MCP agents: configured in `tools.mcp_servers` in `~/.clawft/config.json`.
- Delegate: `ANTHROPIC_API_KEY` env var or key in providers config.
- CLI: agent binary on PATH.
- Leverages `TaskComplexityAnalyzer` / `IntelligentRouter` (ADR-026) for
  complexity scoring.

### Error Codes

| Condition | Skill Behavior |
|-----------|----------------|
| MCP server disconnected | Fall back to delegate or CLI |
| MCP tool not found | Re-list tools, retry with correct name |
| Delegate unavailable (no API key) | Fall back to CLI |
| Agent not found (CLI) | List available agents, suggest installation |
| Agent timeout | Kill process, log, suggest alternative transport |
| Context too large | Route to agent with larger context window |
| Rate limited | Fall back to next-best agent |

### Safety

- Never passes secrets, API keys, or credentials through agent prompts.
- Always reports which agent is being used before dispatching.
- Sanitizes prompts against embedded system-prompt overrides.
- Warns when agent has side effects (e.g., aider auto-commits).
- Logs all dispatches for auditability.

---

## claude-flow

Preferred development orchestration via claude-flow MCP. Provides the full
planning pipeline (business requirements -> technical architecture ->
development planning -> SPARC plan), multi-agent swarms, shared memory,
self-learning hooks, and ~189 tools across 23 domains.

**Source**: [`skills/claude-flow/SKILL.md`](../../skills/claude-flow/SKILL.md)

### Metadata

| Field | Value |
|-------|-------|
| Version | `1.0.0` |
| Category | orchestration |
| User-invocable | yes |
| Argument hint | `<action> [options]` |
| Allowed tools | `Bash`, `Read`, `Write`, `Glob`, `Grep` |

### Variables

| Name | Description |
|------|-------------|
| `action` | The operation: `plan`, `spawn`, `swarm`, `orchestrate`, `memory`, `workflow`, `hive`, `learn`, `analyze`, `github`, `browse`, `tasks`, `embeddings`, `neural`, `session`, `status`, `claims`, `config`, `secure`, `transfer`, `daa` |
| `task` | Task description for spawning, orchestrating, or planning |
| `topology` | Swarm topology: `hierarchical`, `mesh`, `adaptive` |
| `strategy` | Swarm strategy: `specialized`, `general` |

### Planning Pipeline

The `plan` action runs the structured development pipeline that is always
done through claude-flow:

| Stage | Output | Memory Key |
|-------|--------|------------|
| 1. Business Requirements | User stories, acceptance criteria, constraints | `plan-<name>-business-req` |
| 2. Technical Architecture | Module design, API contracts, ADRs | `plan-<name>-tech-arch` |
| 3. Development Planning | Task breakdown, dependencies, milestones | `plan-<name>-dev-plan` |
| 4. SPARC Plan | S1-S5 phase specs (Specification through Completion) | `plan-<name>-sparc` |

Data paths:
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

### Actions

| Action | Description | Key MCP Tools |
|--------|-------------|---------------|
| `plan` | Full planning pipeline (biz-req -> arch -> dev-plan -> SPARC) | `memory_store`, `agent_spawn`, `workflow_create`, `workflow_execute` |
| `spawn` | Create a specialized agent for a task | `agent_spawn`, `agent_status`, `agent_terminate` |
| `swarm` | Initialize and manage multi-agent swarms | `swarm_init`, `swarm_status`, `swarm_health`, `swarm_shutdown` |
| `orchestrate` | Complex task with auto-decomposition | `coordination_orchestrate`, `coordination_sync`, `coordination_consensus` |
| `memory` | Store/search/retrieve shared knowledge | `memory_store`, `memory_search`, `memory_retrieve`, `memory_list` |
| `workflow` | Create and execute automated workflows | `workflow_create`, `workflow_execute`, `workflow_template` |
| `hive` | Collective intelligence and consensus | `hive-mind_init`, `hive-mind_consensus`, `hive-mind_broadcast` |
| `learn` | Self-learning trajectories and patterns | `hooks_intelligence_*`, `hooks_pre-*`, `hooks_post-*` |
| `analyze` | Code diff analysis and risk assessment | `analyze_diff`, `analyze_diff-risk`, `analyze_file-risk` |
| `github` | PR management, issues, repo analysis | `github_pr_manage`, `github_issue_track`, `github_repo_analyze` |
| `browse` | Browser automation and testing | `browser_open`, `browser_click`, `browser_screenshot` |
| `tasks` | Task creation and lifecycle | `task_create`, `task_status`, `task_complete` |
| `embeddings` | Vector embeddings and semantic search | `embeddings_generate`, `embeddings_search`, `embeddings_compare` |
| `neural` | Neural network training and inference | `neural_train`, `neural_predict`, `neural_optimize` |
| `session` | Save and restore agent sessions | `session_save`, `session_restore`, `session_list` |
| `status` | System health, metrics, performance | `system_status`, `performance_report`, `progress_summary` |
| `claims` | Distributed task claiming and handoffs | `claims_claim`, `claims_handoff`, `claims_steal` |
| `config` | Read/modify claude-flow configuration | `config_get`, `config_set`, `config_export` |
| `secure` | AI defence, PII detection, safety scans | `aidefence_analyze`, `aidefence_is_safe`, `aidefence_has_pii` |
| `transfer` | Plugin store and data transfer | `transfer_plugin-search`, `transfer_store-download` |
| `daa` | Dynamic agent adaptation and learning | `daa_agent_create`, `daa_agent_adapt`, `daa_knowledge_share` |

### Tool Domain Summary

| Domain | Tools | Key Tools |
|--------|-------|-----------|
| Agents | 7 | `agent_spawn`, `agent_status`, `agent_terminate` |
| Swarms | 4 | `swarm_init`, `swarm_status`, `swarm_health` |
| Memory | 7 | `memory_store`, `memory_search`, `memory_retrieve` |
| Tasks | 6 | `task_create`, `task_status`, `task_complete` |
| Workflows | 9 | `workflow_create`, `workflow_execute`, `workflow_template` |
| Coordination | 7 | `coordination_orchestrate`, `coordination_sync` |
| Hive-Mind | 9 | `hive-mind_init`, `hive-mind_consensus` |
| Hooks/Intelligence | 34 | `hooks_intelligence_*`, lifecycle hooks, workers |
| Embeddings | 7 | `embeddings_generate`, `embeddings_search` |
| Neural | 6 | `neural_train`, `neural_predict` |
| Sessions | 5 | `session_save`, `session_restore` |
| System | 5 | `system_status`, `system_health` |
| Performance | 6 | `performance_benchmark`, `performance_report` |
| Progress | 4 | `progress_check`, `progress_summary` |
| Browser | 24 | `browser_open`, `browser_click`, `browser_screenshot` |
| Terminal | 5 | `terminal_create`, `terminal_execute` |
| Config | 6 | `config_get`, `config_set` |
| Claims | 12 | `claims_claim`, `claims_handoff` |
| Analyze | 6 | `analyze_diff`, `analyze_diff-risk` |
| GitHub | 5 | `github_pr_manage`, `github_issue_track` |
| AI Defence | 6 | `aidefence_analyze`, `aidefence_is_safe` |
| Transfer | 11 | `transfer_plugin-search`, `transfer_store-download` |
| DAA | 8 | `daa_agent_create`, `daa_agent_adapt` |
| **Total** | **~189** | |

### Agent Types

| Type | Use Case |
|------|----------|
| `coder` | Implementation |
| `reviewer` | Code review |
| `tester` | Testing |
| `planner` | Task decomposition |
| `researcher` | Research |
| `specification` | SPARC S1 |
| `pseudocode` | SPARC S2 |
| `architecture` | SPARC S3 |
| `refinement` | SPARC S4 |
| `sparc-coder` | SPARC S5 |
| `sparc-coord` | SPARC orchestrator |
| `security-manager` | Security |
| `backend-dev` | Backend API |
| `pr-manager` | PR management |
| `release-manager` | Release coordination |

### Recommended Configuration

From CLAUDE.md:
- **Topology**: `hierarchical` (prevents drift in coding swarms)
- **Max agents**: 6-8 (tight coordination)
- **Strategy**: `specialized` (clear role boundaries)
- **Consensus**: `raft` (leader maintains authoritative state)

### Dependencies

- Claude-flow MCP server registered in `tools.mcp_servers`.
- `npx -y @claude-flow/cli@latest` (auto-installs).

### Error Codes

| Condition | Skill Behavior |
|-----------|----------------|
| MCP server disconnected | Run `system_status` to diagnose, verify config |
| Agent spawn failure | Check `agent_pool` capacity, terminate idle agents |
| Memory full | Run `memory_stats`, clean up with `memory_delete` |
| Workflow stuck | `workflow_pause` then `workflow_resume`, or cancel and retry |
| Swarm unhealthy | `swarm_health` check, shutdown and reinitialize if needed |
| Consensus failure | Lower threshold or investigate dissenting agents |

### Safety

- Never stores secrets in memory -- uses env var references.
- Uses hierarchical topology for code-writing swarms (prevents drift).
- Keeps max-agents at 6-8 (CLAUDE.md requirement).
- Terminates agents when work is complete -- no idle agents.
- Runs `aidefence_scan` on external content.
- Logs all spawns and completions for auditability.

---

## discord

Control Discord via clawft's Discord channel adapter. Send messages, react,
manage threads, polls, and moderation actions.

**Source**: [`skills/discord/SKILL.md`](../../skills/discord/SKILL.md)

### Metadata

| Field | Value |
|-------|-------|
| Version | `1.0.0` |
| Category | channel |
| User-invocable | yes |
| Argument hint | `<action> [options]` |
| Allowed tools | `Bash` |

### Variables

| Name | Description |
|------|-------------|
| `action` | The operation to perform |
| `channel_id` | Target Discord channel ID |

### Actions

| Action | Description | CLI Pattern |
|--------|-------------|-------------|
| `send` | Send a text message to a channel | `weft channel discord send --channel <id> --content "<text>"` |
| `react` | Add an emoji reaction to a message | `weft channel discord react --channel <id> --message <mid> --emoji "<emoji>"` |
| `thread` | Create or archive threads | `weft channel discord thread create --channel <id> --message <mid> --name "<name>"` |
| `poll` | Create a poll in a channel | `weft channel discord poll --channel <id> --question "<q>" --options "<opts>" --duration <h>` |
| `pin` | Pin or unpin messages | `weft channel discord pin --channel <id> --message <mid>` |
| `search` | Search messages in a channel | `weft channel discord search --channel <id> --query "<terms>" --limit <n>` |
| `moderation` | Delete messages, timeout/kick/ban users | `weft channel discord mod <action> ...` |
| `status` | Check adapter connection health | `weft channel discord status` |

### Error Codes

| HTTP Status | Meaning | Skill Behavior |
|-------------|---------|----------------|
| 403 | Missing bot permissions | Report required permission |
| 404 | Channel or message not found | Ask user to verify ID |
| 429 | Rate limited | Wait and retry (max 3 attempts) |
| 5xx | Discord server error | Suggest retrying later |

### Safety

- Moderation actions (kick, ban, timeout, delete) require explicit user
  confirmation.
- Never sends to channels the user has not specified.
- Sanitizes content against Discord markdown injection.
- Does not include `@everyone` or `@here` unless explicitly requested.

---

## linkedin

Interact with LinkedIn -- fetch saved posts, draft articles and updates,
publish content, and view profile info.

**Source**: [`skills/linkedin/SKILL.md`](../../skills/linkedin/SKILL.md)

### Metadata

| Field | Value |
|-------|-------|
| Version | `1.0.0` |
| Category | social |
| User-invocable | yes |
| Argument hint | `<action> [options]` |
| Allowed tools | `Bash`, `Read`, `Write`, `Glob` |

### Variables

| Name | Description |
|------|-------------|
| `action` | The operation to perform |
| `topic` | Subject for content drafting |

### Actions

| Action | Description | API Endpoint |
|--------|-------------|--------------|
| `saved` | Fetch saved/bookmarked posts | `GET /v2/savedPosts?q=member&count=50` |
| `draft` | Compose an update or article | Local -- writes to drafts directory |
| `post` | Publish a draft or new content | `POST /v2/ugcPosts` |
| `profile` | View authenticated user's profile | `GET /v2/me` |

### Data Paths

| Path | Contents |
|------|----------|
| `~/.clawft/tokens/linkedin.json` | OAuth2 tokens (managed by `social-auth`) |
| `~/.clawft/workspace/social/linkedin/saved/<date>.json` | Fetched saved posts |
| `~/.clawft/workspace/social/linkedin/drafts/<slug>.json` | Draft content |

### Draft Format

```json
{
  "topic": "original topic",
  "type": "update|article",
  "title": "Article title (articles only)",
  "body": "Full post content",
  "hashtags": ["#hashtag1", "#hashtag2"],
  "visibility": "PUBLIC",
  "created_at": "ISO timestamp",
  "status": "draft|posted"
}
```

### Dependencies

- Requires `social-auth` for OAuth2 token management.
- Provider config: `oauth2.providers.linkedin` in `~/.clawft/config.json`.
- Required scopes: `openid`, `profile`, `w_member_social`, `r_liteprofile`.

### Error Codes

| HTTP Status | Meaning | Skill Behavior |
|-------------|---------|----------------|
| 401 | Token expired | Auto-refresh via `oauth2_refresh`, retry once |
| 403 | Missing API scope | Report required scope |
| 422 | Content validation failed | Report specific error |
| 429 | Rate limited | Report retry-after duration |
| 5xx | LinkedIn server error | Suggest retrying later |

### Safety

- Never posts without explicit user confirmation.
- Never stores raw tokens in workspace files.
- Warns if content seems inappropriate for professional context.

---

## prompt-log

Extract conversation transcripts from clawft session logs. Parses `.jsonl`
session files into clean, readable Markdown transcripts.

**Source**: [`skills/prompt-log/SKILL.md`](../../skills/prompt-log/SKILL.md)

### Metadata

| Field | Value |
|-------|-------|
| Version | `1.0.0` |
| Category | utility |
| User-invocable | yes |
| Argument hint | `<session-file> [--output <path>] [--after <ISO>] [--before <ISO>]` |
| Allowed tools | `Read`, `Write`, `Bash`, `Glob` |

### Variables

| Name | Description |
|------|-------------|
| `session_file` | Path to `.jsonl` session file, or `latest` for most recent |
| `output_path` | Optional output file path for the transcript |

### Parameters

| Flag | Type | Description |
|------|------|-------------|
| `--output` | path | Write transcript to this file instead of stdout |
| `--after` | ISO 8601 | Only include turns after this timestamp |
| `--before` | ISO 8601 | Only include turns before this timestamp |

### Data Paths

| Path | Contents |
|------|----------|
| `~/.clawft/sessions/*.jsonl` | Session log files (input) |

### Session Line Format

```json
{"ts": "ISO timestamp", "role": "user|assistant|tool_use|tool_result", "content": "...", "model": "..."}
```

### Output Format

Markdown transcript with:
- Header: file name, export timestamp, turn count.
- Per-turn: timestamp, role, content.
- Tool calls: indented blockquotes with summarized inputs/outputs (first 200
  characters).

### Edge Cases

- Empty session files: reports "No turns found".
- Corrupted lines: skips with warning count.
- Large sessions: processes in chunks via `Read` offset/limit.

---

## skill-vetting

Vet clawft skills for security and utility before installation. Evaluates
third-party skills from ClawHub or local sources against 57 automated audit
checks and a manual review checklist.

**Source**: [`skills/skill-vetting/SKILL.md`](../../skills/skill-vetting/SKILL.md)

### Metadata

| Field | Value |
|-------|-------|
| Version | `1.0.0` |
| Category | security |
| User-invocable | yes |
| Argument hint | `<skill-directory-or-archive>` |
| Allowed tools | `Read`, `Bash`, `Glob`, `Grep` |

### Variables

| Name | Description |
|------|-------------|
| `skill_path` | Path to skill directory or archive to vet |

### Vetting Pipeline

1. **Obtain** -- Download or locate the skill.
2. **Validate structure** -- Parse `SKILL.md` frontmatter, check required
   fields, verify semver, validate tool names.
3. **Automated scan** -- Run `weft security scan <path>` (57 checks across 10
   categories).
4. **Manual review** -- Checklist for prompt injection, data exfiltration,
   credential harvesting, scope creep, obfuscation, variable abuse.
5. **Decision matrix** -- APPROVE / REVIEW / REJECT based on findings.
6. **Report** -- Structured Markdown report with verdict and recommendations.

### Security Scan Categories

| Category | Checks |
|----------|--------|
| Prompt injection patterns | 8 |
| Data exfiltration URLs | 6 |
| Credential literals | 6 |
| Permission escalation | 6 |
| Unsafe shell commands | 6 |
| Supply chain risks | 6 |
| Denial of service patterns | 6 |
| Indirect prompt injection | 5 |
| Information disclosure | 4 |
| Cross-agent access | 4 |

### Decision Matrix

| Scan Result | Manual Review | Verdict |
|-------------|---------------|---------|
| 0 findings | Clear | APPROVE |
| Low only | Clear | APPROVE with notes |
| Medium+ | Clear | REVIEW |
| Any | Issues found | REJECT |
| Critical | Any | REJECT |

---

## social-auth

Manage OAuth2 authorization for social media platforms (Twitter/X, LinkedIn,
and any future OAuth2-based service). Shared auth layer used by platform-
specific skills.

**Source**: [`skills/social-auth/SKILL.md`](../../skills/social-auth/SKILL.md)

### Metadata

| Field | Value |
|-------|-------|
| Version | `1.0.0` |
| Category | auth |
| User-invocable | yes |
| Argument hint | `<platform> <action>` |
| Allowed tools | `Bash`, `Read`, `Write` |

### Variables

| Name | Description |
|------|-------------|
| `platform` | Provider name: `twitter`, `linkedin`, etc. |
| `action` | The operation to perform |

### Actions

| Action | Description | Tool Used |
|--------|-------------|-----------|
| `authorize` | Start OAuth2 authorization code flow | `oauth2_authorize` + `oauth2_callback` |
| `status` | Check token existence and expiry | Reads `~/.clawft/tokens/<platform>.json` |
| `refresh` | Refresh an expired access token | `oauth2_refresh` |
| `revoke` | Revoke tokens and delete local store | `rest_request` (POST to revocation endpoint) + file delete |

### Token Flow

```
~/.clawft/config.json
  oauth2.providers.<platform>:
    client_id, client_secret_ref, auth_url, token_url, scopes
        |
        v
/social-auth <platform> authorize
        |
        +-- oauth2_authorize -> browser URL
        +-- user authorizes -> callback
        +-- oauth2_callback -> token exchange
        |
        v
~/.clawft/tokens/<platform>.json
        |
        v
Platform skill calls rest_request
        +-- auto-injects Bearer token
        +-- if 401 -> oauth2_refresh -> retry
```

### Data Paths

| Path | Contents |
|------|----------|
| `~/.clawft/config.json` | OAuth2 provider configurations |
| `~/.clawft/tokens/<platform>.json` | Stored OAuth2 tokens |

### Provider Config Schema

```json
{
  "oauth2": {
    "providers": {
      "<platform>": {
        "preset": "custom",
        "client_id": "string",
        "client_secret_ref": { "env_var": "ENV_VAR_NAME" },
        "auth_url": "https://...",
        "token_url": "https://...",
        "scopes": ["scope1", "scope2"],
        "redirect_uri": "http://localhost:8085/callback"
      }
    }
  }
}
```

### Error Codes

| Condition | Skill Behavior |
|-----------|----------------|
| No provider config | Guide user through setup |
| No tokens stored | Suggest `authorize` |
| Token expired, no refresh | Suggest re-`authorize` |
| Refresh returns `invalid_grant` | Refresh token revoked, re-`authorize` |

### Safety

- Never displays raw access or refresh tokens.
- Client secrets referenced via env vars only (`client_secret_ref`).
- Tokens stored only in `~/.clawft/tokens/`.
- Revoke requires explicit confirmation.

---

## twitter

Interact with Twitter/X -- fetch bookmarks, categorize content, draft tweets,
post, and search. Uses the Twitter API v2.

**Source**: [`skills/twitter/SKILL.md`](../../skills/twitter/SKILL.md)

### Metadata

| Field | Value |
|-------|-------|
| Version | `1.0.0` |
| Category | social |
| User-invocable | yes |
| Argument hint | `<action> [options]` |
| Allowed tools | `Bash`, `Read`, `Write`, `Glob` |

### Variables

| Name | Description |
|------|-------------|
| `action` | The operation to perform |
| `topic` | Subject for content drafting |

### Actions

| Action | Description | API Endpoint |
|--------|-------------|--------------|
| `bookmarks` | Fetch authenticated user's bookmarks (paginated) | `GET /2/users/me/bookmarks` |
| `categorize` | Classify stored bookmarks by category via LLM | Local -- reads/writes workspace files |
| `draft` | Compose a tweet or thread | Local -- writes to drafts directory |
| `post` | Publish a draft or new tweet | `POST /2/tweets` |
| `search` | Search recent tweets | `GET /2/tweets/search/recent` |

### Data Paths

| Path | Contents |
|------|----------|
| `~/.clawft/tokens/twitter.json` | OAuth2 tokens (managed by `social-auth`) |
| `~/.clawft/workspace/social/twitter/bookmarks/<date>.json` | Raw bookmarks |
| `~/.clawft/workspace/social/twitter/bookmarks/categorized/<date>.json` | Categorized bookmarks |
| `~/.clawft/workspace/social/twitter/drafts/<slug>.json` | Tweet drafts |

### Bookmark Categorization Schema

Categories: `tech`, `news`, `personal`, `reference`, `career`,
`entertainment`, `other`.

```json
{
  "id": "tweet_id",
  "text_preview": "First 100 chars...",
  "category": "tech",
  "confidence": "high|medium|low",
  "tags": ["rust", "programming"],
  "author_id": "author_id",
  "created_at": "ISO timestamp"
}
```

### Draft Format

```json
{
  "topic": "original topic",
  "tweets": [
    { "text": "Tweet content #hashtag", "index": 1 }
  ],
  "hashtags": ["#hashtag1", "#hashtag2"],
  "created_at": "ISO timestamp",
  "status": "draft|posted"
}
```

### Rate Limits

| Endpoint | Limit |
|----------|-------|
| `GET /2/users/me/bookmarks` | 180 requests / 15 min |
| `POST /2/tweets` | 200 tweets / 15 min |
| `GET /2/tweets/search/recent` | 450 requests / 15 min |

### Dependencies

- Requires `social-auth` for OAuth2 token management.
- Provider config: `oauth2.providers.twitter` in `~/.clawft/config.json`.
- Required scopes: `tweet.read`, `tweet.write`, `users.read`, `bookmark.read`,
  `bookmark.write`, `offline.access`.

### Error Codes

| HTTP Status | Meaning | Skill Behavior |
|-------------|---------|----------------|
| 401 | Token expired | Auto-refresh via `oauth2_refresh`, retry once |
| 403 | Missing scope | Report required scope |
| 429 | Rate limited | Report reset time from `x-rate-limit-reset` header |
| 5xx | Twitter server error | Suggest retrying later |

### Safety

- Never posts without explicit user confirmation.
- Never stores raw tokens in workspace files.
- Sanitizes user input before API request bodies.

---

## Adding a New Skill to This Index

When creating a new skill, add an entry to this index following the template
below. This ensures consistent API-style documentation across all skills.

### Template

````markdown
## skill-name

One-line description of what the skill does.

**Source**: [`skills/skill-name/SKILL.md`](../../skills/skill-name/SKILL.md)

### Metadata

| Field | Value |
|-------|-------|
| Version | `x.y.z` |
| Category | category-name |
| User-invocable | yes/no |
| Argument hint | `<hint>` |
| Allowed tools | `Tool1`, `Tool2` |

### Variables

| Name | Description |
|------|-------------|
| `var_name` | What this variable controls |

### Actions

| Action | Description | Details |
|--------|-------------|---------|
| `action_name` | What it does | API endpoint, CLI command, or "Local" |

### Data Paths

| Path | Contents |
|------|----------|
| `~/.clawft/...` | What is stored here |

### Dependencies

List any required skills, plugins, provider configs, or scopes.

### Error Codes

| Condition | Skill Behavior |
|-----------|----------------|
| Error condition | What the skill does |

### Safety

- Safety constraint 1.
- Safety constraint 2.
````

### Checklist

1. Add the skill to the [Registry Summary](#registry-summary) table.
2. Add a full entry following the template above.
3. Assign a category (or create a new one and add it to the Categories list).
4. Update the skill count in the summary table if applicable.
5. Verify the `Source` link resolves correctly.
