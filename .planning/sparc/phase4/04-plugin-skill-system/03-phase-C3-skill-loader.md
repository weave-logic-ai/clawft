# Phase C3: Skill Loader -- serde_yaml Migration & Discovery Enhancement

> **Element:** 04 -- Plugin & Skill System
> **Phase:** C3
> **Timeline:** Week 5-6
> **Priority:** P1
> **Crate:** `clawft-core/src/agent/`
> **Depends on:** C1 (Plugin Trait Crate), Element 03 B3 (skills_v2.rs file split)
> **Blocks:** C4 (Hot-Reload), C5 (Slash-Commands), C6 (MCP Exposure)
> **Status:** Planning

---

## 1. Overview

Phase C3 replaces the hand-rolled YAML frontmatter parser in `skills_v2.rs` with `serde_yaml` deserialization into a typed `SkillFrontmatter` struct. It also enhances skill discovery to support bundled skill paths (for plugin-shipped skills), WASM skill auto-registration, and OpenClaw SKILL.md compatibility. Additionally, C3 addresses the blocking I/O defect (D11) where `std::fs` calls block the Tokio executor.

**Key deliverables:**
- Replace `parse_yaml_frontmatter()` with `serde_yaml::from_str::<SkillFrontmatter>()`
- Add bundled skills path to discovery chain
- Add WASM skill auto-registration
- Replace `std::fs` with `tokio::fs` in `load_dir()`
- Preserve all existing security validations (SEC-SKILL-01 through SEC-SKILL-07)

---

## 2. Current Code

### 2.1 Hand-Rolled YAML Parser

**File:** `crates/clawft-core/src/agent/skills_v2.rs`

The `parse_yaml_frontmatter()` function (lines 207-273) is a minimal line-by-line parser that handles:
- Scalar values: `key: value`
- Boolean values: `key: true` / `key: false`
- Integer values: `count: 42`
- Quoted strings: `key: "value"` / `key: 'value'`
- Block sequences: lines starting with `  - item` under a key
- Comments: lines starting with `#`

**Missing capabilities (motivating the replacement):**
- Nested maps/objects
- Multi-line string values (literal `|` and folded `>` block scalars)
- Flow sequences `[a, b, c]`
- Flow mappings `{a: 1, b: 2}`
- Anchors and aliases (`&anchor` / `*alias`)
- Complex keys
- Tag directives
- Null values (`~`, `null`)

### 2.2 Entry Point: `parse_skill_md()`

**File:** `crates/clawft-core/src/agent/skills_v2.rs`, lines 59-172

The public `parse_skill_md()` function:
1. Validates content is non-empty
2. Calls `validate_file_size()` (SEC-SKILL-07)
3. Calls `extract_frontmatter()` to split YAML block from body
4. Calls `validate_yaml_depth()` (SEC-SKILL-01)
5. Calls `parse_yaml_frontmatter()` -- **this is what C3 replaces**
6. Extracts known fields (`name`, `description`, `version`, `variables`, `allowed-tools`, etc.)
7. Collects remaining fields as `metadata: HashMap<String, serde_json::Value>`
8. Calls `sanitize_skill_instructions()` on body (SEC-SKILL-06)
9. Returns `SkillDefinition`

### 2.3 Skill Discovery: `SkillRegistry::discover()`

**File:** `crates/clawft-core/src/agent/skills_v2.rs`, lines 347-421

Current three-level discovery:
1. Built-in skills (lowest priority) -- compiled into binary
2. User skills (`~/.clawft/skills/`) -- medium priority
3. Workspace skills (`.clawft/skills/`) -- highest priority, gated by `trust_workspace` flag (SEC-SKILL-05)

### 2.4 Directory Loader: `SkillRegistry::load_dir()`

**File:** `crates/clawft-core/src/agent/skills_v2.rs`, lines 434-536

Uses blocking `std::fs::read_dir()` and `std::fs::read_to_string()` calls. Detects format per subdirectory (SKILL.md preferred over skill.json). Applies security validations on directory names (SEC-SKILL-02) and file sizes (SEC-SKILL-07).

---

## 3. Implementation Tasks

### Task 3.1: Add `serde_yaml` Dependency

**File:** `crates/clawft-core/Cargo.toml`

```toml
[dependencies]
serde_yaml = "0.9"
```

Verify it does not conflict with existing `serde` / `serde_json` versions.

### Task 3.2: Define `SkillFrontmatter` Struct

**File:** `crates/clawft-core/src/agent/skills_v2.rs` (or new `skill_loader.rs` if B3 split is complete)

```rust
use serde::Deserialize;

/// Typed representation of SKILL.md YAML frontmatter.
///
/// Uses `serde_yaml` for deserialization, replacing the hand-rolled parser.
/// The `#[serde(flatten)]` on `metadata` captures any extra fields not
/// explicitly listed, preserving forwards-compatibility with OpenClaw
/// and custom skill metadata.
#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    variables: Option<Vec<String>>,
    #[serde(alias = "allowed-tools", default)]
    allowed_tools: Option<Vec<String>>,
    #[serde(alias = "user-invocable", default)]
    user_invocable: Option<bool>,
    #[serde(alias = "disable-model-invocation", default)]
    disable_model_invocation: Option<bool>,
    #[serde(alias = "argument-hint", default)]
    argument_hint: Option<String>,
    #[serde(flatten)]
    metadata: HashMap<String, serde_json::Value>,
}
```

**Alias handling:** serde's `alias` attribute handles both kebab-case (`allowed-tools`) and snake_case (`allowed_tools`) variants. The current code checks both manually; serde handles this natively.

**Metadata capture:** `#[serde(flatten)]` collects all fields not matched by named fields into the `metadata` HashMap, preserving the existing behavior of collecting unknown keys.

### Task 3.3: Replace `parse_yaml_frontmatter()` with serde_yaml

Modify `parse_skill_md()` to replace the call to `parse_yaml_frontmatter()` with:

```rust
let frontmatter: SkillFrontmatter = serde_yaml::from_str(yaml_block)
    .map_err(|e| ClawftError::PluginLoadFailed {
        plugin: format!("SKILL.md: invalid frontmatter: {e}"),
    })?;
```

Then construct `SkillDefinition` directly from the typed struct instead of the HashMap:

```rust
Ok(SkillDefinition {
    name: frontmatter.name,
    description: frontmatter.description.unwrap_or_default(),
    version: frontmatter.version.unwrap_or_default(),
    variables: frontmatter.variables.unwrap_or_default(),
    argument_hint: frontmatter.argument_hint,
    allowed_tools: frontmatter.allowed_tools.unwrap_or_default(),
    user_invocable: frontmatter.user_invocable.unwrap_or(false),
    disable_model_invocation: frontmatter.disable_model_invocation.unwrap_or(false),
    instructions: sanitized_body,
    format: SkillFormat::SkillMd,
    source_path: source_path.map(PathBuf::from),
    metadata: frontmatter.metadata,
})
```

**Functions to remove after migration:**
- `parse_yaml_frontmatter()` (lines 207-273)
- `parse_scalar()` (lines 280-303)
- `extract_string_list()` (lines 306-317)

**Functions to preserve:**
- `extract_frontmatter()` -- still needed to split YAML from body before passing to serde_yaml
- All security validation calls in `parse_skill_md()` remain unchanged

### Task 3.4: Security Validation Preservation

All existing security checks MUST remain in place and execute BEFORE `serde_yaml::from_str()`:

| Check | Code Reference | Position |
|-------|---------------|----------|
| SEC-SKILL-07: File size limit | `validate_file_size(content.len(), MAX_SKILL_MD_SIZE, "SKILL.md")` | Before frontmatter extraction |
| SEC-SKILL-01: YAML depth limit | `validate_yaml_depth(yaml_block)` | After extraction, before serde parse |
| SEC-SKILL-06: Instruction sanitization | `sanitize_skill_instructions(body)` | After serde parse, on body text |
| SEC-SKILL-02: Directory name validation | `validate_directory_name(&dir_name_str)` | In `load_dir()`, unchanged |
| SEC-SKILL-05: Workspace trust gate | `trust_workspace` flag check | In `discover_with_trust()`, unchanged |

**Critical:** `validate_yaml_depth()` must still run on the raw YAML string BEFORE serde_yaml parses it. This prevents pathological nesting from consuming excessive memory during deserialization.

### Task 3.5: Enhance Discovery -- Bundled Skills Path

Add a fourth discovery level between user and workspace:

**New priority order (highest first):**
1. Workspace skills (`.clawft/skills/`) -- highest, gated by trust
2. Managed/local skills (`~/.clawft/skills/`) -- user-installed
3. Bundled skills (plugin-shipped, from plugin manifest `"skills"` directories)
4. Built-in skills (compiled into binary) -- lowest

Update `SkillRegistry::discover_with_trust()` signature:

```rust
pub fn discover_with_trust(
    workspace_dir: Option<&Path>,
    user_dir: Option<&Path>,
    bundled_dirs: Vec<&Path>,  // NEW: plugin-shipped skill directories
    builtin_skills: Vec<SkillDefinition>,
    trust_workspace: bool,
) -> Result<Self>
```

Load bundled skills after built-ins but before user skills:

```rust
// 1. Built-in skills (lowest priority)
for skill in builtin_skills { ... }

// 2. Bundled skills (from plugin manifests)
for dir in bundled_dirs {
    match Self::load_dir(dir) {
        Ok(skills) => { for skill in skills { skills_map.insert(...); } }
        Err(e) => { debug!(...); }
    }
}

// 3. User skills (medium priority)
if let Some(dir) = user_dir { ... }

// 4. Workspace skills (highest priority, trust-gated)
if let Some(dir) = workspace_dir { ... }
```

### Task 3.6: WASM Skill Auto-Registration

When a skill directory contains a `.wasm` file alongside `SKILL.md`, mark the skill for WASM execution:

```rust
// In load_dir(), after parsing SKILL.md:
let wasm_path = path.join("module.wasm");
if wasm_path.exists() {
    skill.format = SkillFormat::Wasm;
    skill.metadata.insert(
        "wasm_module".to_string(),
        serde_json::Value::String(wasm_path.to_string_lossy().to_string()),
    );
}
```

This requires adding `SkillFormat::Wasm` variant to `clawft-types` (if not already present from C2).

### Task 3.7: OpenClaw SKILL.md Compatibility

OpenClaw skills use the same YAML frontmatter format but may include additional fields:
- `openclaw-category`
- `openclaw-license`
- `openclaw-version`
- `openclaw-registry`

These are already captured by `#[serde(flatten)] metadata` in the `SkillFrontmatter` struct. The existing test `parse_skill_md_with_openclaw_metadata()` (line 671) validates this.

No additional code changes needed -- serde_yaml with flatten handles this natively. Add a compatibility test that verifies OpenClaw-specific fields round-trip correctly.

### Task 3.8: Blocking I/O Fix (D11)

Replace blocking filesystem calls in `load_dir()` with async equivalents:

**Before (blocking):**
```rust
fn load_dir(dir: &Path) -> Result<Vec<SkillDefinition>> {
    let entries = std::fs::read_dir(dir).map_err(ClawftError::Io)?;
    // ...
    match std::fs::read_to_string(&skill_md_path) { ... }
}
```

**After (async):**
```rust
async fn load_dir(dir: &Path) -> Result<Vec<SkillDefinition>> {
    if !tokio::fs::try_exists(dir).await.unwrap_or(false) {
        return Ok(Vec::new());
    }
    let mut entries = tokio::fs::read_dir(dir).await.map_err(ClawftError::Io)?;
    while let Some(entry) = entries.next_entry().await.map_err(ClawftError::Io)? {
        // ...
        let content = tokio::fs::read_to_string(&skill_md_path).await.map_err(ClawftError::Io)?;
        // ...
    }
}
```

This requires `discover()` and `discover_with_trust()` to become `async fn` as well. Callers must be updated. Check the call sites:
- `crates/clawft-core/src/agent/loop_core.rs` (agent startup)
- `crates/clawft-cli/src/commands/` (CLI commands)

Also update `load_legacy_skill()` to use `tokio::fs`.

---

## 4. Concurrency Plan

### 4.1 Parallel Skill Loading

Skill directories are independent. Load them concurrently within each priority level:

```rust
use futures::future::join_all;

async fn load_dir(dir: &Path) -> Result<Vec<SkillDefinition>> {
    let mut entries = tokio::fs::read_dir(dir).await?;
    let mut tasks = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_dir() {
            tasks.push(tokio::spawn(async move {
                load_single_skill(&path).await
            }));
        }
    }

    let results = join_all(tasks).await;
    // Collect successful results, log failures
}
```

### 4.2 Sequential Priority Merge

Priority levels must be applied sequentially (lower inserts first, higher overwrites):
1. Built-in (sync, in-memory)
2. Bundled dirs (async, parallel across dirs, sequential within priority)
3. User dir (async)
4. Workspace dir (async, trust-gated)

### 4.3 Thread Safety

`SkillRegistry` is currently not `Send + Sync`. If hot-reload (C4) requires shared access, wrap in `Arc<RwLock<SkillRegistry>>`. This is deferred to C4 -- C3 keeps the current ownership model.

---

## 5. Dependencies

### Crate Dependencies

| Dependency | Version | Purpose | New? |
|-----------|---------|---------|------|
| `serde_yaml` | 0.9 | YAML frontmatter deserialization | Yes |
| `serde` | existing | Derive `Deserialize` | No (already present) |
| `serde_json` | existing | Metadata HashMap values | No (already present) |
| `tokio` | existing | Async filesystem operations | No (already present) |

### Internal Dependencies

| Dependency | Status | Notes |
|-----------|--------|-------|
| C1 (Plugin Trait Crate) | Required | `Skill` trait definition used by registry |
| Element 03 B3 (file split) | Required | `skills_v2.rs` must be split before modifying |
| `clawft-types::SkillDefinition` | Existing | May need `SkillFormat::Wasm` variant |
| `clawft-core::security` | Existing | All SEC-SKILL validations preserved |

---

## 6. Tests Required

### 6.1 Regression Tests (Must Continue to Pass)

All existing tests in `skills_v2.rs` lines 598-1158:

| Test | Lines | Purpose |
|------|-------|---------|
| `parse_skill_md_full_frontmatter` | 614-653 | Full YAML frontmatter with all fields |
| `parse_skill_md_minimal_frontmatter` | 656-668 | Minimal frontmatter (name + description only) |
| `parse_skill_md_with_openclaw_metadata` | 671-700 | Extra metadata fields captured correctly |
| `parse_skill_md_empty_returns_error` | 703-708 | Empty input rejected |
| `parse_skill_md_no_frontmatter_returns_error` | 711-716 | Missing `---` delimiters rejected |
| `parse_skill_md_missing_name_returns_error` | 719-725 | Missing required `name` field |
| `parse_skill_md_invalid_yaml_returns_error` | 728-733 | Malformed YAML rejected |
| `parse_skill_md_boolean_values` | 736-749 | Boolean field parsing |
| `parse_skill_md_quoted_values` | 752-757 | Quoted string values |
| `registry_empty_when_no_sources` | 762-768 | Empty registry |
| `registry_loads_builtin_skills` | 771-783 | Built-in skill loading |
| `registry_priority_workspace_over_user_over_builtin` | 786-812 | Priority ordering |
| `registry_user_overrides_builtin` | 815-827 | User > built-in |
| `registry_handles_missing_directories` | 830-841 | Missing dirs handled gracefully |
| `registry_loads_legacy_skill_json` | 844-857 | Legacy format still works |
| `registry_skill_md_preferred_over_skill_json` | 860-888 | SKILL.md > skill.json in same dir |
| `registry_merges_skills_from_multiple_sources` | 891-910 | Multi-source merge |
| `registry_skips_invalid_skills` | 913-934 | Invalid skills skipped, valid loaded |
| `sec_skill_01_deep_yaml_rejected` | 1030-1046 | Deep nesting rejected |
| `sec_skill_01_depth_10_accepted` | 1049-1054 | Normal depth accepted |
| `sec_skill_02_traversal_dir_rejected` | 1059-1080 | Path traversal directory rejected |
| `sec_skill_05_workspace_blocked_without_trust` | 1085-1105 | Trust gate blocks workspace skills |
| `sec_skill_05_workspace_allowed_with_trust` | 1108-1119 | Trust gate allows workspace skills |
| `sec_skill_06_system_tags_stripped` | 1124-1130 | Prompt injection tags stripped |
| `sec_skill_06_normal_markdown_preserved` | 1133-1138 | Normal markdown preserved |
| `sec_skill_07_oversized_skill_md_rejected` | 1143-1150 | Oversized files rejected |
| `sec_skill_07_normal_size_accepted` | 1153-1157 | Normal size accepted |

### 6.2 New Tests for serde_yaml Capabilities

```rust
#[test]
fn test_serde_yaml_nested_structures() {
    // Verify serde_yaml handles nested maps in metadata
    let content = r#"---
name: nested
description: Nested structures
config:
  timeout: 30
  retry:
    max_attempts: 3
    backoff_ms: 1000
---

Instructions.
"#;
    let skill = parse_skill_md(content, None).unwrap();
    assert_eq!(skill.name, "nested");
    let config = skill.metadata.get("config").unwrap();
    assert_eq!(config["timeout"], 30);
    assert_eq!(config["retry"]["max_attempts"], 3);
}

#[test]
fn test_serde_yaml_multiline_values() {
    // Verify serde_yaml handles literal block scalars
    let content = "---\nname: multiline\ndescription: |\n  Line one\n  Line two\n  Line three\n---\n\nBody.";
    let skill = parse_skill_md(content, None).unwrap();
    assert!(skill.description.contains("Line one"));
    assert!(skill.description.contains("Line two"));
}

#[test]
fn test_serde_yaml_flow_sequences() {
    // Verify serde_yaml handles [a, b, c] syntax
    let content = "---\nname: flow\ndescription: Flow sequences\nvariables: [topic, depth, format]\nallowed-tools: [WebSearch, Read]\n---\n\nBody.";
    let skill = parse_skill_md(content, None).unwrap();
    assert_eq!(skill.variables, vec!["topic", "depth", "format"]);
    assert_eq!(skill.allowed_tools, vec!["WebSearch", "Read"]);
}

#[test]
fn test_serde_yaml_null_values() {
    // Verify serde_yaml handles null/~ values
    let content = "---\nname: nulls\ndescription: ~\nversion: null\n---\n\nBody.";
    let skill = parse_skill_md(content, None).unwrap();
    assert_eq!(skill.name, "nulls");
    // description defaults to empty when null
    assert!(skill.description.is_empty());
}
```

### 6.3 New Tests for Enhanced Discovery

```rust
#[tokio::test]
async fn test_bundled_skills_load_between_builtin_and_user() {
    // Verify bundled skills override built-ins but not user skills
}

#[tokio::test]
async fn test_wasm_skill_registration() {
    // Verify skill with module.wasm gets SkillFormat::Wasm
}

#[tokio::test]
async fn test_openclaw_skill_compat() {
    // Verify OpenClaw-specific fields preserved in metadata
    // Verify openclaw-category, openclaw-license, etc.
}

#[tokio::test]
async fn test_async_load_dir_concurrent() {
    // Verify multiple skill directories load concurrently
    // Measure that N skills load faster than N * single-skill-time
}
```

### 6.4 serde_yaml vs Hand-Rolled Behavioral Differences

These behavioral differences must be tested and documented:

| Scenario | Hand-rolled | serde_yaml | Resolution |
|----------|------------|------------|------------|
| `version: 1.0.0` | Parsed as string | May parse as float `1.0` then fail on `.0` | Use `Option<String>` -- serde_yaml can deserialize `1.0.0` as string |
| `yes` / `no` scalars | Parsed as bool | serde_yaml 0.9 also parses as bool | Compatible |
| Flow sequences `[a, b]` | **FAILS** -- not supported | Supported | Improvement |
| Nested maps | **FAILS** -- not supported | Supported via `#[serde(flatten)]` | Improvement |
| Duplicate keys | Last wins | serde_yaml: last wins | Compatible |

---

## 7. Acceptance Criteria

- [ ] `serde_yaml` dependency added to `clawft-core/Cargo.toml`
- [ ] `SkillFrontmatter` struct defined with proper `Deserialize` derive and field aliases
- [ ] `parse_yaml_frontmatter()`, `parse_scalar()`, and `extract_string_list()` removed
- [ ] `parse_skill_md()` uses `serde_yaml::from_str::<SkillFrontmatter>()` for deserialization
- [ ] All 27 existing tests pass without modification (regression)
- [ ] New tests for nested structures, multi-line values, flow sequences pass
- [ ] Security validations SEC-SKILL-01 through SEC-SKILL-07 all preserved and verified
- [ ] `validate_yaml_depth()` still runs BEFORE serde_yaml parse
- [ ] Skill discovery enhanced: workspace > managed > bundled > built-in
- [ ] WASM skill auto-registration: `.wasm` file detected, `SkillFormat::Wasm` set
- [ ] OpenClaw metadata fields preserved via `#[serde(flatten)]`
- [ ] Blocking `std::fs` replaced with `tokio::fs` in `load_dir()` and `load_legacy_skill()`
- [ ] `discover()` and `discover_with_trust()` are `async fn`
- [ ] All call sites updated for async discover

---

## 8. Risk Notes

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| serde_yaml behavioral differences break existing tests | Medium | Medium | Run full regression suite early. Test `version: 1.0.0` parsing specifically -- serde_yaml may interpret this differently than the hand-rolled parser. Add `#[serde(deserialize_with = ...)]` if needed. |
| `validate_yaml_depth()` bypass via serde_yaml pre-parse | Low | High | Enforce that `validate_yaml_depth()` runs on raw YAML string BEFORE `serde_yaml::from_str()`. Never skip the depth check. |
| Async migration breaks synchronous test helpers | Medium | Low | Keep synchronous `discover()` wrapper that calls `tokio::runtime::Handle::current().block_on()` for tests, or convert tests to `#[tokio::test]`. |
| `#[serde(flatten)]` performance with large metadata | Low | Low | SKILL.md size is already capped at `MAX_SKILL_MD_SIZE` (50KB). Metadata maps will be small. |
| serde_yaml 0.9 `unsafe-serde-yaml` deprecation | Low | Medium | serde_yaml 0.9 is the latest stable release. If deprecated, `serde_yml` is the successor. Pin version in Cargo.toml. |
| Bundled skills path changes `discover()` signature | Medium | Low | This is a breaking change to the internal API. All callers (loop_core.rs, CLI commands) must be updated. Coordinate with C1 trait definitions. |
