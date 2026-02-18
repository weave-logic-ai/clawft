# Expert Review: Phase 3F (Part 2) -- Agents, Skills, Commands

## Reviewer: Developer Experience Engineer (CLI + plugin specialist)
## Date: 2026-02-17
## Verdict: APPROVE_WITH_CHANGES

---

## Scores

| Dimension | Score | Notes |
|-----------|-------|-------|
| Completeness | 4/5 | Covers dual formats, discovery chain, slash commands, agents, helpers. Missing: skill installation workflow, skill validation command, and `context: fork` subagent implementation details. |
| Compatibility | 5/5 | Excellent alignment with Claude Code SKILL.md spec, OpenClaw metadata, and legacy skill.json. Serde rename strategy for kebab-case is correct. |
| Discovery Chain | 3/5 | 4-level chain is reasonable but diverges from 3G workspace plan and 02-technical-requirements.md. See critical issue #1. |
| Slash Commands | 4/5 | 12 new commands are well-chosen. Missing `/undo`, `/compact` (context management). `/skills` vs `/skill` distinction may confuse users. |
| Agent Definitions | 4/5 | 4 built-in agents are well-designed. AGENT.md format is practical. Missing: agent inheritance/composition and agent-level permission model. |
| Test Coverage | 4/5 | 70+ tests across 7 modules with explicit coverage targets. Missing: Windows path edge cases, concurrent discovery tests, and error message assertion tests. |
| Integration | 3/5 | ContextBuilder integration is well-planned but SkillRegistry wrapping SkillsLoader is underspecified. See major issue #1. |
| Security | 2/5 | Significant gaps in security considerations. See critical issue #2. |

**Overall: 3.6/5**

---

## Strengths

1. **Dual-format design is excellent.** Supporting both `skill.json` + `prompt.md` (legacy) and `SKILL.md` (Claude Code / OpenClaw) with clear precedence rules and automatic detection is the right approach. The detection precedence (SKILL.md wins when both exist) matches user expectations from Claude Code.

2. **Template rendering is thorough.** The `$ARGUMENTS`, `$ARGUMENTS[N]`, `$N`, `$SESSION_ID`, and `${CLAUDE_SESSION_ID}` substitution patterns cover all Claude Code compatibility requirements. The fallback behavior (appending `ARGUMENTS:` when no placeholder exists) is a smart ergonomic touch.

3. **Slash command architecture is well-abstracted.** The `SlashCommand` trait with `name()`, `description()`, `help()`, `execute()`, and `completions()` methods is clean. The `CommandContext` struct provides everything a command needs. Having user-invocable skills register as top-level slash commands (e.g., `/research args` instead of `/skill research args`) is excellent UX.

4. **TDD approach with explicit test cases.** Every module has concrete test functions written out in the plan, not just described. This significantly de-risks implementation. The test coverage targets (90% for parsers, 85% for registries, 80% for CLI) are appropriate.

5. **Implementation order is dependency-aware.** Starting with types, then parser, then template renderer, then registries, then slash commands, then CLI subcommands follows a clean bottom-up strategy that enables parallel workstreams after step 4.

6. **Agent definitions as separate files.** Keeping agents out of `config.json` and in dedicated `~/.clawft/agents/` directories is the right decision. This enables version control of agent definitions and sharing between projects.

7. **Helper module is well-scoped.** The `helpers.rs` module (fs, config, template, git helpers) provides utility functions that skills and commands need without polluting the tool namespace. The explicit note that helpers are NOT tools (not exposed to LLM) prevents a common design mistake.

8. **New dependency choices are conservative.** `serde_yaml` and `shell-words` are minimal, well-maintained crates. The note about potentially replacing `serde_yaml` with a 100-line minimal parser if binary size is a concern shows good engineering judgment.

---

## Issues Found

### Critical

**C1: Discovery chain conflicts with 3G workspace plan and 02-technical-requirements.md**

The 3F plan defines skill discovery as:

```
1. .clawft/skills/      (project-level)
2. ~/.clawft/skills/    (personal)
3. ~/.clawft/workspace/skills/   (legacy workspace)
4. ~/.nanobot/workspace/skills/  (nanobot compat)
```

But the 3G workspace plan (section 1.2.10, FR-W10) defines:

```
1. <workspace>/.clawft/skills/   (workspace-scoped)
2. Global skills: ~/.clawft/workspace/skills/
3. Legacy: ~/.nanobot/workspace/skills/
```

And `02-technical-requirements.md` (section 10) defines:

```
1. .clawft/skills/       (project)
2. ~/.clawft/workspace/skills/
3. ~/.nanobot/workspace/skills/
```

The 3F plan introduces a level 2 (`~/.clawft/skills/`) that does not exist in either 3G or the technical requirements. Meanwhile, 3G uses `<workspace>/.clawft/skills/` which is dynamically resolved, not a fixed `.clawft/skills/` relative to cwd.

**Impact**: When 3G lands, the discovery chain will need reworking. The `SkillScope::Workspace` variant in 3F maps to the legacy path, but in 3G "workspace" means the dynamically-resolved workspace root.

**Recommendation**: Align the 3F discovery chain with 3G now:

```
1. <project>/.clawft/skills/       (project, via cwd walk-up for .clawft/)
2. ~/.clawft/skills/               (user-level personal)  -- NEW, 3F addition
3. ~/.clawft/workspace/skills/     (legacy global workspace)
4. ~/.nanobot/workspace/skills/    (nanobot compat)
```

Make `SkillScope` variants explicit: `Project`, `User`, `LegacyWorkspace`, `LegacyNanobot`. When 3G lands, the `Project` scope becomes workspace-aware via `WorkspaceContext.skills_dir`. The `User` scope at `~/.clawft/skills/` is a 3F-specific addition not present in other plans -- document this explicitly as a new feature and update 3G to acknowledge it.

---

**C2: No security model for loading arbitrary skill/agent files**

The plan describes loading SKILL.md and agent YAML from multiple filesystem locations, parsing YAML frontmatter, and executing skill prompts. There are no security considerations for:

1. **YAML deserialization attacks**: `serde_yaml` can deserialize arbitrary types. While `SkillDefinition` is a concrete type (no `Value` fields except `metadata`), the `metadata: Option<serde_json::Value>` field allows arbitrary nested data. A malicious SKILL.md could include extremely deeply nested YAML structures causing stack overflow or excessive memory allocation during parsing.

2. **Path traversal in skill directories**: The plan does not validate that skill directory names are safe. A skill directory named `../../../etc` could be created, and the registry would happily scan it. The `source_path` field stores whatever path was discovered.

3. **Prompt injection via skill content**: Skills inject prompts directly into the LLM context. A malicious skill installed from a future `weft skills install` (mentioned in NFR-3F-002) could contain prompt injection attacks. There is no content sanitization or sandboxing.

4. **Agent definition model override**: A user-defined agent can specify any model string. There is no validation that the model is a legitimate, configured provider. A project-level `agents/exploit.yaml` with `model: "attacker-controlled-endpoint/model"` could redirect LLM traffic.

5. **`allowed_tools` in skill definitions**: A skill can declare `allowed_tools` that "are temporarily added to the permission set." This means a skill can escalate tool access beyond what the user or agent normally has. There is no mention of this being intersected with (rather than unioned with) the current tool permissions.

**Impact**: Medium-to-high for items 1, 4, and 5. The plan mentions that `requires.bins` and `requires.env` are "validated (warn if requirements not met)" but does not address adversarial scenarios.

**Recommendation**: Add a security section to the plan:
- Limit YAML parsing depth (serde_yaml supports this)
- Validate skill directory names against a safe character set (`[a-zA-Z0-9_-]`)
- Validate agent model strings against the configured providers list
- Intersect (not union) `allowed_tools` with the current session's tool permissions
- Log a warning when project-level skills or agents override user-level or built-in ones
- Add a `--trust-project-skills` flag (default: prompt) for first-time project skill loading, similar to Claude Code's trust model

---

### Major

**M1: SkillRegistry wrapping SkillsLoader is underspecified**

Section 2.7 "Cross-Stream Integration" says:

> The existing `ContextBuilder` takes `Arc<SkillsLoader<P>>`. The new `SkillRegistry` wraps the legacy `SkillsLoader` functionality.

But the `SkillRegistry` has a completely different API surface:
- `discover()` vs `load_all()`
- `list()` returns `Vec<SkillSummary>` vs `list_skills()` returns `Vec<String>`
- `get()` returns `Option<SkillDefinition>` vs `get_skill()` returns `Option<Skill>`
- `match_context()` -- entirely new
- `render_prompt()` -- entirely new

The plan says `SkillRegistry` "wraps" `SkillsLoader` but never shows how. The existing `SkillsLoader` only handles `skill.json` format. The new registry handles both formats. There is no adapter or delegation pattern described.

**Recommendation**: Choose one of:
- (a) `SkillRegistry` internally uses `SkillsLoader` for legacy format directories only, implementing SKILL.md parsing itself. Show the delegation in pseudocode.
- (b) `SkillRegistry` replaces `SkillsLoader` entirely, incorporating its logic. The existing `SkillsLoader` public API is preserved as a thin wrapper for backward compat.
- (c) The "wraps" statement is aspirational. Clarify that `SkillsLoader` is deprecated and `SkillRegistry` is the new implementation.

Option (b) seems most practical. The existing `SkillsLoader` has 13 tests that should be preserved. Add a note that `SkillsLoader` becomes a facade delegating to `SkillRegistry` for the discovery of `skill.json` format skills.

---

**M2: `context: fork` subagent implementation is hand-waved**

FR-3F-022 says:

> If `context == Fork`, spawn a subagent with the skill prompt

But there is no SubagentManager in the codebase (GAP-30 in 3i-gap-analysis.md confirms this). The plan does not explain:
- How a forked skill spawns a subagent
- What happens to the subagent's tool permissions
- How the subagent result is returned to the parent session
- Whether the forked skill runs with the same or different model

The `SkillContext::Fork` variant is defined but its execution path is missing from the pseudocode in section 3.3 (slash command dispatch). The dispatch code handles `Fork` with a comment but no implementation detail.

**Recommendation**: Either:
- (a) Defer `context: fork` to a future phase and document it as not-yet-supported (with `SkillContext::Fork` still parseable but generating a warning). This is honest and avoids shipping broken functionality.
- (b) Define the fork execution model: create a new `AgentLoop` instance with the skill's `agent` field (or `default`), filtered tools, and the rendered prompt as the sole user message. Return the final text response to the parent. This requires `SubagentManager` from GAP-30.

Option (a) is recommended for this phase.

---

**M3: Slash command dispatch returns rendered prompt but mechanism is unclear**

In section 3.3, the slash command dispatch for skill invocation:

```rust
let rendered = SkillRegistry::<NativePlatform>::render_prompt(&skill, &args_vec);
println!("[Invoking skill: {cmd_name}]");
// Return rendered prompt to caller for LLM processing
// (The caller in agent.rs sends it as a user message)
return Ok(true); // signal: handled as skill invocation
```

The rendered prompt is created but never actually returned to the caller. The function returns `Result<bool>`, not `Result<Option<String>>`. The comment says "return rendered prompt to caller" but the code does not do this.

**Recommendation**: Change the dispatch signature or add an output channel. Options:
- (a) `dispatch()` returns `Result<DispatchResult>` where `DispatchResult` is `NotACommand`, `Handled`, or `SkillInvocation(String)`. The caller checks for `SkillInvocation` and sends the rendered prompt to the LLM.
- (b) Add a `pending_prompt: Option<String>` field to `CommandContext` that dispatch writes to and the caller reads from.

Option (a) is cleaner.

---

**M4: Agent tool filtering mutates config in place**

Section 3.4 `apply_agent_definition`:

```rust
if !agent_def.tools.is_empty() {
    let allowed: std::collections::HashSet<&str> =
        agent_def.tools.iter().map(String::as_str).collect();
    ctx.tools_mut().retain(|name| allowed.contains(name));
}
```

This permanently removes tools from the registry. When the user switches agents (via `/agent default`), the filtered tools are gone. There is no mechanism to restore the original tool set.

**Recommendation**: Store the original `ToolRegistry` state before applying agent definitions, and restore it when switching agents. Alternatively, use a view/filter layer over the tool registry rather than mutating it. The `AgentLoop` should keep a reference to the full tool set and apply per-agent filters at invocation time.

---

### Minor

**m1: `$N` substitution regex collision**

The template renderer replaces `$0`, `$1`, etc. But `$0` also matches inside `$0.50` (a price), `$100` (a dollar amount), or `$1000000`. The replacement:

```rust
result = result.replace(&format!("${i}"), arg);
```

Will turn `"Budget is $100"` with args `["quantum"]` into `"Budget is quantum00"` because `$1` matches and `$0` matches.

**Recommendation**: Use a regex-based replacer that requires word boundaries or specific delimiters. Match `$N` only when followed by a non-digit or end-of-string. Alternatively, require `${N}` syntax (with braces) for indexed args, which is unambiguous. The `$ARGUMENTS[N]` syntax already works; `$N` is a shorthand that introduces ambiguity.

---

**m2: Missing `weft skills install` subcommand**

NFR-3F-002 mentions "Skills from ClawHub (installed via future `weft skills install`) should work without modification." But there is no `install` subcommand defined in FR-3F-050. The plan defines `list`, `show`, and `run` but not `install`.

This is fine if `install` is explicitly deferred, but the plan should say so rather than referencing it as if it will exist.

---

**m3: `serde_yaml` in `clawft-core` vs YAML in agent definitions**

The plan adds `serde_yaml` to `clawft-core/Cargo.toml` for SKILL.md parsing. But agent definitions also use YAML (`agent.yaml`, `AGENT.md` with YAML frontmatter). The plan says `AgentRegistry` lives in `clawft-core/src/agent/agents.rs`, which would also need `serde_yaml`. This is fine but should be explicitly noted as a shared dependency within the crate.

---

**m4: No offline/cache invalidation strategy**

The `SkillRegistry` uses `Arc<RwLock<HashMap>>` for caching. The plan says skills are discovered via `discover()` which populates the cache. But there is no cache invalidation: if a user adds a new skill to `~/.clawft/skills/` while a `weft agent` session is running, the new skill will not appear until the next `discover()` call.

**Recommendation**: Add a note about when `discover()` is called (session start only? on `/skills` command? periodic?). Consider adding a `/skills refresh` command for manual invalidation.

---

**m5: `planner` agent has "no tools, high temperature" -- questionable defaults**

The built-in `planner` agent is described as having "No tools, high temperature." A planning agent with no tool access cannot read files, check project structure, or verify anything. A high temperature encourages creative but potentially inaccurate planning.

**Recommendation**: Consider giving `planner` read-only tools (`read_file`, `list_directory`) and a moderate temperature (0.5-0.7). A planner that cannot inspect the codebase is significantly less useful. Alternatively, rename it to `brainstorm` to set accurate expectations.

---

**m6: Missing `--json` output flag for `weft skills list` and `weft agents list`**

The plan mentions `--format <table|json>` for `weft skills list` and `weft agents list`, which is good. But the default format is not specified. For CI/scripting scenarios, `--json` is commonly expected as a shorthand. Consider adding it as an alias.

---

## Cross-Phase Conflicts

### 3F vs 3G: Skill directory naming

3F uses `SkillScope::Project` for `.clawft/skills/` relative to cwd.
3G uses `WorkspaceContext.skills_dir` which is `<workspace>/.clawft/skills/`.

These are the same directory when cwd is the workspace root, but differ when cwd is a subdirectory. 3F's project detection (FR-3F-060) walks up looking for `.clawft/` or `.git/`, which is the same walk 3G does. But 3F hard-codes `SkillSearchPath` at registry construction time, while 3G dynamically resolves the workspace root and passes `skills_dir` as a parameter.

**Resolution**: 3F should accept `project_dir: Option<PathBuf>` (already in the API) and let the caller determine the project root. When 3G lands, it passes `WorkspaceContext.root` as the project_dir. Until then, 3F uses its own walk-up heuristic. This is already partially addressed in section 2.7 but needs to be more explicit about the `SkillScope::Project` path being derived from the `project_dir` parameter, not hard-coded.

### 3F vs 3H: Skill `allowed_tools` interacting with MCP tools

3H adds MCP tools to the `ToolRegistry` with `{server}__{tool}` namespacing. If a skill specifies `allowed_tools: [read_file, claude-flow__memory_store]`, the MCP-namespaced tool name must be supported. The plan does not address MCP tool names in the `allowed_tools` list.

**Resolution**: Document that `allowed_tools` uses the tool names as they appear in the `ToolRegistry`, including MCP-namespaced names. No code change needed, but add a note.

### 3F vs 3G: Agent discovery chain missing workspace-level agents

3F defines agent discovery as:
```
1. .clawft/agents/   (project)
2. ~/.clawft/agents/  (personal)
3. Built-in
```

3G's workspace layout (section 3.6) includes `<workspace>/.clawft/agents/` but does not define agent discovery behavior. The 3F agent discovery should be consistent with the skill discovery and include workspace-level agents when 3G is active.

**Resolution**: Add a note that 3G will inject the workspace agents directory as the highest-priority search path, following the same pattern as skills.

---

## Missing Requirements

1. **FR-W10 workspace-scoped skills (07-workspaces.md)**: The 3F plan acknowledges workspace integration in section 2.7 but does not implement the `weft skills list` provenance indicator (`workspace/global/project`) that FR-W10 requires. The `SkillSummary.scope` field is defined but `SkillScope` does not include a `Workspace` variant that maps to the dynamic workspace concept from 3G.

2. **Skill version conflict detection**: When a project-level skill shadows a user-level skill, the plan says the higher-priority one wins. But there is no warning or diagnostic when this happens. The 3G plan (section 1.2.10) explicitly requires: "Debug log when workspace skill shadows global skill." Add this to 3F as well.

3. **Skill prompt size limits**: CLAWFT.md has a 50KB truncation limit (3G plan). Skill prompts have no size limit. A malicious or accidentally large SKILL.md could consume the entire context window. Add a configurable `max_skill_prompt_bytes` with a reasonable default (e.g., 32KB).

4. **Agent definition validation**: No validation is specified for agent YAML files. What happens if `temperature` is 5.0 (out of range)? What if `max_tool_iterations` is 0? What if `tools` references a nonexistent tool name? Add validation rules or at minimum document expected behavior.

5. **Skill deactivation**: There is no way to deactivate a skill without deleting it. A `disabled: true` field in SKILL.md frontmatter or a `.disabled` marker file would allow users to temporarily turn off skills without losing configuration.

---

## Recommendations

1. **Resolve the discovery chain conflict (Critical C1) before implementation begins.** This is the highest-priority item. A misaligned discovery chain will cause painful rework when 3G lands. Spend 30 minutes aligning 3F, 3G, and 02-technical-requirements.md on a single canonical chain.

2. **Add a security section (Critical C2).** Even a brief section acknowledging the attack surface and documenting mitigations will significantly improve the plan. At minimum: YAML depth limits, directory name validation, `allowed_tools` intersection semantics.

3. **Change dispatch return type (Major M3).** The current pseudocode has a logical gap where the rendered prompt is created but not communicated to the caller. This will be caught during implementation, but fixing it now saves a design iteration.

4. **Defer `context: fork` (Major M2).** Parse it, log a warning that it is not yet supported, and track it as a follow-up. Shipping a `Fork` variant that silently does nothing (or crashes) is worse than a clear "not yet supported" message.

5. **Fix template `$N` collision (Minor m1).** This is a subtle bug that will produce confusing results in real-world use. Require `${N}` syntax or use regex word boundaries.

6. **Add a cache refresh mechanism (Minor m4).** At minimum, re-discover on `/skills` command. Users will add skills during a session and expect them to appear.

7. **Consider the developer onboarding experience.** The plan covers the technical system well but does not describe what a new user sees when they first type `/skills` with no skills installed. Add a "getting started" help topic that explains how to create a first skill (including a minimal SKILL.md template).

---

## Timeline Assessment

**Estimated LOE: 3-4 weeks** -- This is reasonable given the scope. The implementation order is well-structured for parallelism (types + parser can proceed independently from CLI commands after step 4).

**Risk factors**:
- The SkillRegistry/SkillsLoader integration (M1) is the riskiest technical item. Budget an extra day for this if the "wrapping" approach proves awkward.
- The YAML frontmatter parser may surface edge cases (Windows line endings, BOM markers, Unicode in field names). Budget time for parser hardening.
- Slash command integration with the existing `agent.rs` REPL will require understanding the current input processing flow. The plan references `commands/agent.rs` but the actual REPL loop code structure is not shown -- this could be complex.

**Suggested timeline adjustment**:
- Week 1: Types, parser, template renderer, skill registry (steps 1-4)
- Week 2: Agent registry, helpers, slash command framework (steps 5-7)
- Week 3: Built-in commands, CLI subcommands, main.rs integration (steps 8-10)
- Week 4: ContextBuilder integration, help topics, security hardening, testing (steps 11-12 + fixes from this review)

The plan is solid engineering work. With the discovery chain alignment, security considerations, and the few design fixes called out above, it will deliver a high-quality skill and agent system that feels natural to users coming from Claude Code or similar tools.
