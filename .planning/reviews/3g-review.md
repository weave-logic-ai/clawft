# Expert Review: Phase 3G -- Projects/Workspaces

## Reviewer: Software Architect (workspace isolation + config specialist)
## Date: 2026-02-17
## Verdict: APPROVE_WITH_CHANGES

The plan is thorough, well-structured, and demonstrates a strong understanding of workspace
isolation patterns. The requirements traceability matrix is complete, the TDD plan is
well-ordered, and the architecture properly leverages the existing Platform abstraction.
However, there are two critical issues and several major issues that must be resolved
before implementation begins.

---

## Scores

| Dimension | Score | Notes |
|-----------|-------|-------|
| Requirements Coverage | 5/5 | All 11 FRs from 07-workspaces.md are traced and have acceptance criteria. Exemplary traceability matrix. |
| Config Deep Merge | 4/5 | Algorithm is correct for most cases. Missing: null-at-nested-level removal needs parent cleanup, and there is a cross-document conflict on array semantics. |
| Workspace Discovery | 5/5 | 4-step discovery correctly implemented with proper precedence, depth limits, and canonicalization. |
| CLAWFT.md Loading | 4/5 | Import resolution and hierarchical loading are well-designed. Minor: regex for imports is too broad and will match references inside code blocks. |
| Hook System | 5/5 | Both global and workspace hooks, JSON on stdin, 30s timeout, --no-hooks flag, fire-and-forget -- all correct. |
| Isolation | 5/5 | Sessions, memory, and skills are properly scoped with clear path injection via WorkspaceContext. |
| Backward Compatibility | 5/5 | Global fallback is thorough. No migration needed. Existing commands unchanged when no workspace detected. |
| Security | 4/5 | Path traversal and symlink handling are addressed. Missing: symlink-to-file attack vector on CLAWFT.md imports, and atomic write tmp file cleanup on crash. |

**Weighted Score: 4.6/5**

---

## Requirements Traceability

| Requirement | Covered? | Section | Notes |
|-------------|----------|---------|-------|
| FR-W01 | YES | 1.2.1 | All acceptance criteria present. Directory structure, registry, hooks, --git flag all specified. |
| FR-W02 | YES | 1.2.2 | Listing from registry + auto-discovery of cwd children. --all flag for missing workspaces. |
| FR-W03 | YES | 1.2.3 | Load by name or path. Registry update. Hook firing. Good note about CLI being non-persistent. |
| FR-W04 | YES | 1.2.4 | set/get/reset/edit/show all covered. Dot-path syntax. Schema validation. Hook on change. |
| FR-W05 | YES | 1.2.5 | Session count, memory size, skills count, CLAWFT.md presence, global mode indicator. |
| FR-W06 | YES | 1.2.6 | Confirmation required. Default removes .clawft/ + CLAWFT.md only. --keep-data. De-registers. |
| FR-W07 | YES | 1.2.7 | @imports, hierarchical loading, re-read per session, 10KB warn / 50KB truncate. Path traversal blocked. |
| FR-W08 | YES | 1.2.8 | Sessions scoped via WorkspaceContext.sessions_dir. --global and --all flags on sessions list. |
| FR-W09 | YES | 1.2.9 | Memory scoped via WorkspaceContext.memory_dir. --global flag on memory show. |
| FR-W10 | YES | 1.2.10 | SkillsLoader gains workspace dir with precedence over global. Provenance indicator in list. |
| FR-W11 | YES | 1.2.11 | Registry data model matches spec. Atomic writes. Idempotent registration. Version field. |

**Assessment: 11/11 requirements covered. No missing requirements.**

---

## Strengths

1. **Exemplary traceability**: Every FR has a precise module path, acceptance criteria list, and test plan. This is the gold standard for SPARC plans.

2. **Clean architecture**: The WorkspaceContext struct is a single point of truth passed to all subsystems. No global state, no environment variable hacks. The dependency flow (types -> core -> cli) is correct.

3. **Proper platform abstraction**: All filesystem access goes through the Platform trait. The plan explicitly calls this out in Phase C acceptance criteria ("No direct `std::fs` usage"). WASM considerations are addressed with `#[cfg]` guards.

4. **Comprehensive TDD plan**: 70+ test cases across 5 implementation phases with clear Red-Green-Refactor structure. Coverage targets are appropriate (100% for deep_merge and discovery, 80%+ for manager operations).

5. **Thorough error handling table**: Section 3.8 covers 12 error cases with specific behaviors. The principle of "hooks never block" is correctly applied.

6. **Implementation notes section**: Section 5.6 has 13 actionable notes for the coder agent, including the critical detail about `MemoryStore::with_paths` being `#[cfg(test)]` and needing to be made public. This prevents discovery-during-implementation delays.

7. **Backward compatibility is paramount**: The plan explicitly states this as principle #8 and validates it with a dedicated section in the pre-merge checklist.

---

## Issues Found

### Critical

**ISSUE-C1: Array merge semantics conflict between 07-workspaces.md and 02-technical-requirements.md**

The authoritative workspace requirements document (07-workspaces.md, section 3.3) states:
> "Arrays: workspace replaces global (no array merging -- too ambiguous)"

The technical requirements document (02-technical-requirements.md, section 10 "Config Hierarchy") states:
> "Arrays (e.g., `mcp_servers`): concatenate (project entries appended after global)"

The 3G SPARC plan follows the 07-workspaces.md semantics (replacement, not concatenation). This is a deliberate design choice that must be formally reconciled, because `mcp_servers` is a HashMap in the current `Config` struct (not an array), so the 02-technical-requirements.md example is arguably about a different data structure. However, if any config field uses `Vec<T>` (like `CommandPolicyConfig.allowlist`), the replacement semantics could be surprising: a workspace that sets `allowlist: ["git"]` would completely replace a global `allowlist: ["git", "cargo", "rustc", "npm"]`.

**Resolution required**: Add a note in the SPARC plan acknowledging this conflict and document the rationale for replacement semantics. Consider adding a convention for "append mode" arrays using a `+` prefix (e.g., `"+allowlist": ["extra-cmd"]` means append) as a future enhancement. For now, replacement is the safer default, but document this prominently in the CLAWFT.md template and workspace config guide.

**ISSUE-C2: `config.rs` has no `serde_json::Value` intermediate for deep merge**

The current `Config` struct in `clawft-types/src/config.rs` uses concrete typed fields (not `serde_json::Value`). The deep merge algorithm operates on `serde_json::Value`, which means the implementation must:

1. Serialize the global `Config` to `serde_json::Value`
2. Parse the workspace `config.json` as raw `serde_json::Value`
3. Deep merge the two Value trees
4. Deserialize the merged Value back to `Config`

This roundtrip (Config -> Value -> merge -> Value -> Config) works, and the plan mentions it in section 2.4 (`load` function). However, the plan does not address a subtle issue: the current config loading in `config_loader.rs` calls `normalize_keys()` which converts camelCase to snake_case. The workspace config file will also need this normalization applied BEFORE the deep merge, or the merge will fail to match keys (e.g., `"maxTokens"` in workspace vs `"max_tokens"` in the already-normalized global config).

**Resolution required**: Add a step in the `load` function pseudocode: `normalize_keys(ws_json)` before calling `deep_merge(global_json, ws_json)`. Add a test case: workspace config with camelCase keys merges correctly with snake_case global config.

### Major

**ISSUE-M1: Import regex `r"@([\w./-]+)"` is too permissive and misses edge cases**

The regex `@([\w./-]+)` will match:
- `@path/to/file` (correct)
- `@path.md` (correct)
- Email addresses like `user@domain.com` (incorrect -- false positive)
- `@mention` in markdown (incorrect -- false positive)
- `@` inside code blocks (incorrect -- should be ignored)

The regex also does not handle:
- Paths with spaces (would need quoting)
- The common pattern `@import "path/to/file"` that other tools use

**Recommendation**: Use a more specific pattern that requires the import to be at the start of a line:
```
r"^@([\w./-]+\.(?:md|txt|toml|json|yaml|yml))$"
```
This anchors to line start, requires a file extension, and avoids false positives on email addresses and @mentions. Add test cases for false positive scenarios.

**ISSUE-M2: Hierarchical CLAWFT.md loading walks up from workspace root indefinitely**

The pseudocode in section 2.3 walks up from `workspace_root.parent()` with a depth limit of 10, collecting CLAWFT.md files from parent directories. However, this means running `weft` inside `/home/user/projects/my-project/` could load CLAWFT.md files from:
- `/home/user/projects/CLAWFT.md`
- `/home/user/CLAWFT.md`
- `/home/CLAWFT.md`
- `/CLAWFT.md`

This is a potential information leakage issue: a CLAWFT.md in `/home/user/` might contain instructions from a completely unrelated project. It also raises security concerns if a shared system has a `/CLAWFT.md`.

**Recommendation**: The hierarchical walk should stop at the first `.git/` boundary or at the home directory, whichever comes first. Add a stop condition:
```
if dir == home_dir || dir.join(".git").is_dir():
    break
```
This matches Claude Code's behavior where CLAUDE.md hierarchy is bounded by the git repository root.

**ISSUE-M3: `WorkspaceContext` does not carry `--no-hooks` flag**

The `--no-hooks` global CLI flag is parsed in `main.rs` but the `WorkspaceContext` struct (section 3.2) has no `hooks_enabled: bool` field. The hook firing functions (`fire_hook`) need to know whether hooks are suppressed. Currently, the plan does not show how `--no-hooks` propagates to `fire_hook()`.

**Recommendation**: Either add `hooks_enabled: bool` to `WorkspaceContext`, or pass it as a parameter to `WorkspaceManager::new()`. The latter is cleaner since hooks are a lifecycle concern, not a context concern.

**ISSUE-M4: Missing test for deep merge with `normalize_keys` interaction**

The deep merge test suite (section 4.2, Phase A) has 12 test cases but none of them test the interaction between `normalize_keys()` and `deep_merge()`. Since workspace config files may use camelCase (to match the existing config.json convention), this interaction is critical.

**Recommendation**: Add test `deep_merge_camelcase_workspace_over_snake_global()` that verifies:
```json
// global (already normalized): {"agents": {"defaults": {"model": "gpt-4"}}}
// workspace (raw camelCase): {"agents": {"defaults": {"maxTokens": 4096}}}
// after normalize + merge: {"agents": {"defaults": {"model": "gpt-4", "max_tokens": 4096}}}
```

**ISSUE-M5: `weft workspace create` does not create MEMORY.md and HISTORY.md files**

The directory structure in section 1.2.1 shows `memory/MEMORY.md (empty)` and `memory/HISTORY.md (empty)`, but the pseudocode in section 2.4 (`create` function) only creates:
```
for subdir in [sessions, memory, skills, agents, hooks]:
    platform.fs.create_dir_all(dot_clawft.join(subdir))
platform.fs.write(dot_clawft.join("config.json"), "{}")
platform.fs.write(root.join("CLAWFT.md"), template(name))
```

The MEMORY.md and HISTORY.md files inside `memory/` are not created. While the gap analysis (3i-gap-analysis.md, GAP-26) notes that `MemoryStore` should create parent directories on first write, the scaffold should still create the empty files to match the documented structure.

**Recommendation**: Add two lines to the `create` pseudocode:
```
platform.fs.write(dot_clawft.join("memory/MEMORY.md"), "")
platform.fs.write(dot_clawft.join("memory/HISTORY.md"), "")
```

### Minor

**ISSUE-m1: Atomic write tmp file naming**

Section 1.2.11 specifies "write to `.tmp`, then rename" for atomic writes. The tmp filename should include a random suffix or PID to prevent collisions when multiple processes write simultaneously:
```
workspaces.json.tmp.{pid}  ->  rename  ->  workspaces.json
```

**ISSUE-m2: WorkspaceContext missing `name` field**

The `WorkspaceContext` struct (section 3.2) has `root: Option<PathBuf>` but no `name: String` field. The workspace name is needed for:
- `workspace.loaded` hook payload
- `weft workspace status` output
- Session key prefixing (FR-W08: "Session keys prefixed with workspace name")

The name could be derived from `root.file_name()`, but it would be cleaner to store it explicitly.

**ISSUE-m3: `config edit` opens `$EDITOR` but has no fallback**

Section 1.2.4 specifies `weft workspace config edit` opens `$EDITOR`. No fallback is specified when `$EDITOR` is unset. Standard practice: fall back to `$VISUAL`, then `vi`.

**ISSUE-m4: `--path` flag for `weft workspace create` not specified**

The CLI command hierarchy (section 3.5) shows `create <name> [--git] [--template] [--path]`, but `--path` is not explained in the FR-W01 specification. Presumably it overrides the parent directory (default: cwd). Add this to the acceptance criteria.

**ISSUE-m5: `weft workspace config show --merged` mentioned in risk table but not in CLI spec**

Section 5.5 (Risk Mitigations) references `weft workspace config show --merged` for debugging, but section 1.2.4 does not list a `--merged` flag. The bare `weft workspace config` already shows the merged config by default. Clarify or remove the reference.

---

## Cross-Phase Conflicts

### Conflict 1: Skill discovery chain between 3F and 3G

Phase 3F (section 2.1, FR-3F-002) defines a 4-level skill discovery chain:
```
1. .clawft/skills/ (project-level)
2. ~/.clawft/skills/ (personal)
3. ~/.clawft/workspace/skills/ (legacy workspace)
4. ~/.nanobot/workspace/skills/ (nanobot compat)
```

Phase 3G (section 1.2.10) defines:
```
- workspace skills loaded from <workspace>/.clawft/skills/
- global skills from global dir
```

The 3G plan correctly identifies the integration point: "SkillsLoader gains a second directory (workspace skills dir) that is checked before the global skills dir." However, it does not account for the full 4-level chain from 3F. When 3G integrates with 3F's `SkillRegistry`, the workspace skills dir should map to priority level 1 (project-level) in the 3F chain.

**Resolution**: Section 1.2.10 should explicitly reference the 3F skill discovery chain and state that `WorkspaceContext.skills_dir` maps to the "Project" scope in `SkillScope::Project`. The `SkillRegistry::new(platform, project_dir)` parameter `project_dir` should receive `ctx.skills_dir`.

### Conflict 2: Array merge vs 02-technical-requirements.md

Already documented as ISSUE-C1 above.

### Conflict 3: 3H MCP config scoping

Phase 3H (tool delegation) adds workspace-scoped MCP server configs. The 02-technical-requirements.md section 10 mentions `.clawft/mcp/servers.json` as a project-scoped MCP config file. Phase 3G does not mention this file in the workspace directory layout (section 3.6). If MCP server configs should be workspace-scoped, the 3G workspace scaffold should create an `mcp/` subdirectory or the 3H plan should use `.clawft/config.json` for MCP server overrides (which would be merged via deep_merge).

**Resolution**: Not blocking for 3G, but add a note in section 3.6 that `mcp/` is reserved for Phase 3H.

---

## Missing Requirements

1. **Environment variable overlay (from 02-technical-requirements.md section 10)**: The tech requirements spec lists `$CLAWFT_*` environment variables as the highest priority config overlay (priority 5). The 3G plan does not address env var overlay in the config hierarchy. The current plan has 3 layers (defaults, global, workspace). The tech requirements add 2 more layers (legacy fallback, env vars). The legacy fallback is handled by the existing `config_loader.rs`, but env var overlay is not.

   **Assessment**: This is GAP-27 from the gap analysis. Not a 3G responsibility -- it should remain a separate gap item. The 3G plan's 3-layer hierarchy is correct for workspace config; env var overlay is orthogonal and can be applied after the merge.

2. **`weft init` vs `weft workspace create`**: The 02-technical-requirements.md section 10 specifies a `weft init` command that scaffolds a workspace in the current directory (in-place). The 3G plan has `weft workspace create <name>` which creates a new subdirectory. These are different workflows. `weft init` should be a convenience alias for creating a workspace at cwd.

   **Assessment**: Add `weft workspace init` as an alias that runs `weft workspace create . --in-place` (creates `.clawft/` in cwd without creating a parent directory). This is a minor addition but matches user expectations from `git init`.

3. **No mention of `.gitignore` template content**: FR-W01 mentions `--git` creates a `.gitignore` but does not specify what goes in it. The 07-workspaces.md section 8 says `.clawft/config.json` should be gitignored (it may contain secrets) but `CLAWFT.md` should be committed.

   **Assessment**: Add the `.gitignore` template to the plan:
   ```
   .clawft/config.json
   .clawft/sessions/
   .clawft/memory/
   ```

---

## Recommendations

1. **Resolve the array merge conflict (ISSUE-C1) before implementation begins.** Add a section "Array Merge Semantics Decision" that acknowledges the conflict with 02-technical-requirements.md and documents the chosen behavior. Replacement is the correct choice for 07-workspaces.md, but it should be explicit.

2. **Add `normalize_keys` to the merge pipeline (ISSUE-C2).** This is a one-line fix in the pseudocode but prevents a class of subtle bugs.

3. **Bound the hierarchical CLAWFT.md walk (ISSUE-M2).** Stop at `.git/` boundary or home directory. This is a security improvement and matches Claude Code behavior.

4. **Tighten the import regex (ISSUE-M1).** The current regex will produce false positives in real-world markdown files.

5. **Add `weft workspace init` as an in-place creation alias.** This matches `git init` ergonomics and the `weft init` command from 02-technical-requirements.md.

6. **Consider adding a `workspace_name()` helper method to `WorkspaceContext`** that derives the name from `root.file_name()` to avoid repeated `.file_name()` calls throughout the codebase.

7. **Add a performance test for the full config resolution pipeline** (discovery + load global + load workspace + normalize + merge + deserialize) to validate the < 50ms NFR-W01 target end-to-end.

---

## Timeline Assessment

The estimated timeline of 2-3 sessions (~6-8 hours) is **realistic but tight**. The breakdown:

| Phase | Estimate | Assessment |
|-------|----------|------------|
| A: Types + Deep Merge | 1.5h | Accurate. 12 test cases + types is well-scoped. |
| B: Workspace Discovery | 1h | Accurate. 11 tests, straightforward logic. |
| C: Workspace Manager | 2h | **Tight**. 30+ tests including CLAWFT.md loading, hooks, config operations. More likely 2.5-3h. |
| D: CLI Commands | 2h | Accurate. 18 parsing tests + 8 handler functions. Clap boilerplate is well-understood. |
| E: Integration | 1.5h | **Tight**. Cross-crate wiring, ContextBuilder changes, and 4 integration tests. More likely 2h. |

**Revised estimate: 8-10 hours** to account for Phase C complexity (CLAWFT.md loading + hooks + config operations are three distinct subsystems) and Phase E integration friction.

The parallel agent strategy (Coder 1: types + core, Coder 2: CLI) is sound and could bring the wall-clock time back to the original estimate if both agents can work simultaneously after Phase A types are defined.

---

## Summary

This is a strong plan with excellent requirements coverage and a well-structured TDD approach. The two critical issues (array merge conflict and normalize_keys interaction) are straightforward to resolve. The major issues (import regex, CLAWFT.md walk boundary, --no-hooks propagation) are design improvements that should be incorporated before implementation. None of the issues require fundamental rearchitecting.

**Verdict: APPROVE_WITH_CHANGES** -- resolve ISSUE-C1 and ISSUE-C2 before starting implementation; incorporate ISSUE-M1 through ISSUE-M5 during implementation.
