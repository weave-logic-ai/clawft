# Security Hardening - SEC-SKILL-01 through SEC-SKILL-08

## Summary

Implemented all 8 security controls for the agent/skill subsystem as specified
in the SPARC plan section 6.

## Controls Implemented

### SEC-SKILL-01: YAML Depth Limit
- **File**: `crates/clawft-core/src/security.rs` -- `validate_yaml_depth()`
- **Integration**: Called in `parse_skill_md()` before YAML parsing
- **Limit**: Max nesting depth of 10 levels (detected via indentation)
- **Test**: `yaml_depth_rejected_at_depth_11`, `yaml_depth_accepted_at_depth_10`

### SEC-SKILL-02: Directory Name Validation
- **File**: `crates/clawft-core/src/security.rs` -- `validate_directory_name()`
- **Integration**: Called in `SkillRegistry::load_dir()` and `AgentLoader::load_dir()`
- **Rejects**: `..`, `/`, `\`, absolute paths, empty names
- **Test**: `sec_skill_02_traversal_dir_rejected`, `dir_name_*` tests

### SEC-SKILL-03: Allowed Tools Intersection
- **File**: `crates/clawft-core/src/security.rs` -- `intersect_allowed_tools()`
- **Behavior**: When both skill and agent have `allowed_tools`, the effective
  set is their intersection. Empty lists are treated as "no restriction".
- **Test**: `tool_intersection_*` tests (5 cases)

### SEC-SKILL-04: Agent Model String Validation
- **File**: `crates/clawft-core/src/security.rs` -- `validate_model_string()`
- **Integration**: Called in `AgentLoader::load_file()` after parsing
- **Rejects**: Shell metacharacters (`;|&$` etc.), newlines, control chars
- **Test**: `sec_skill_04_shell_metachar_model_rejected`, `model_string_*` tests

### SEC-SKILL-05: `--trust-project-skills` Flag
- **Files**:
  - `crates/clawft-core/src/agent/skills_v2.rs` -- `discover_with_trust()`
  - `crates/clawft-cli/src/commands/agent.rs` -- `trust_project_skills` CLI arg
- **Behavior**: Without `--trust-project-skills`, workspace-level skills in
  `.clawft/skills/` are NOT loaded. Only user and built-in skills are available.
  The original `discover()` method defaults to `trust_workspace = true` for
  backward compatibility; callers opt in to the gate via `discover_with_trust()`.
- **Test**: `sec_skill_05_workspace_blocked_without_trust`,
  `sec_skill_05_workspace_allowed_with_trust`

### SEC-SKILL-06: Prompt Injection Guards
- **File**: `crates/clawft-core/src/security.rs` -- `sanitize_skill_instructions()`
- **Integration**: Called in `parse_skill_md()` on the body text
- **Strips**: `<system>`, `</system>`, `<|im_start|>`, `<|im_end|>`,
  `<|im_sep|>`, `<|endoftext|>`, `<|system|>`, `<|user|>`, `<|assistant|>`,
  `[INST]`, `[/INST]`, `<<SYS>>`, `<</SYS>>`
- **Warns**: On multiple system-like prompts remaining after stripping
- **Preserves**: Normal markdown (headings, bold, code, lists)
- **Test**: `sanitize_strips_system_tags`, `sanitize_preserves_normal_markdown`

### SEC-SKILL-07: File Size Limits
- **File**: `crates/clawft-core/src/security.rs` -- `validate_file_size()`,
  `MAX_SKILL_MD_SIZE` (50 KB), `MAX_AGENT_FILE_SIZE` (10 KB)
- **Integration**:
  - `parse_skill_md()` checks content length before parsing
  - `SkillRegistry::load_dir()` checks file metadata before reading
  - `AgentLoader::load_file()` checks file metadata before reading
- **Test**: `sec_skill_07_oversized_skill_md_rejected`,
  `sec_skill_07_oversized_agent_rejected`

### SEC-SKILL-08: MCP Tool Namespace Isolation
- **File**: `crates/clawft-core/src/security.rs` --
  `validate_mcp_tool_name()` (lenient) and `validate_mcp_tool_name_strict()`
- **Behavior**: MCP tools must use `{server}__{tool}` format (double underscore).
  The strict variant is for use during MCP tool registration. The lenient
  variant passes through local tools that happen to have single underscores.
- **Test**: `mcp_tool_with_double_underscore_valid`,
  `mcp_tool_without_double_underscore_rejected`

## Files Modified

| File | Changes |
|------|---------|
| `crates/clawft-core/src/security.rs` | Added 8 security functions + 35 tests |
| `crates/clawft-core/src/agent/skills_v2.rs` | Integrated SEC-01,02,05,06,07 + 8 tests |
| `crates/clawft-core/src/agent/agents.rs` | Integrated SEC-02,04,07 + 4 tests |
| `crates/clawft-cli/src/commands/agent.rs` | Added `--trust-project-skills` flag |

## Verification

```bash
cargo test -p clawft-core     # 461 passed, 0 failed
cargo test -p clawft-cli      # 285 passed, 0 failed (+ 29 integration)
cargo clippy -p clawft-core -- -D warnings  # Clean
```

## Design Decisions

1. **`discover()` backward compatibility**: The existing `discover()` method
   defaults to `trust_workspace = true` so existing call sites do not break.
   The new `discover_with_trust()` is the entry point for security-aware callers.

2. **YAML depth via indentation**: Since we use a minimal hand-rolled YAML
   parser (not `serde_yaml`), depth is detected by indentation level (2 spaces
   per level). This is consistent with the parser's own behavior.

3. **Lenient vs strict MCP tool validation**: `validate_mcp_tool_name()` is
   lenient for local tools (e.g., `read_file` is fine). The strict variant
   `validate_mcp_tool_name_strict()` is intended for MCP registration only.

4. **Prompt injection is strip-and-warn**: Rather than rejecting skills with
   injection tokens outright, we strip the tokens and log warnings. This allows
   skills with accidental token inclusion to still function while alerting
   operators.
