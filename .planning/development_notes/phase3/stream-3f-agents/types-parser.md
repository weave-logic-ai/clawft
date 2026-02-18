# Stream 3F: Shared Types + SKILL.md Parser + SkillRegistry

## Summary

Implemented the unified skill definition type (`SkillDefinition`), a SKILL.md
frontmatter parser, and a three-level `SkillRegistry` for discovering and
loading skills from workspace, user, and built-in sources.

## Files Created

- `crates/clawft-types/src/skill.rs` -- `SkillDefinition` and `SkillFormat` types
- `crates/clawft-core/src/agent/skills_v2.rs` -- SKILL.md parser and SkillRegistry

## Files Modified

- `crates/clawft-types/src/lib.rs` -- added `pub mod skill;`
- `crates/clawft-core/src/agent/mod.rs` -- added `pub mod skills_v2;`

## Design Decisions

### No serde_yaml dependency

The YAML frontmatter parser is hand-rolled rather than depending on `serde_yaml`.
This keeps the dependency graph small and avoids pulling in `unsafe-libyaml`.
The parser supports:
- Scalar key-value pairs (`key: value`)
- Boolean detection (`true`/`false`/`yes`/`no`)
- Integer detection (no float -- `1.0` stays as a string to avoid version ambiguity)
- Sequences (`- item` syntax under a key with no inline value)
- Quoted strings (single and double)
- Comments (`# ...`)

### Unified SkillDefinition vs legacy Skill

The new `SkillDefinition` in `clawft-types` is the canonical type. The existing
`Skill` in `clawft-core::agent::skills` is untouched -- the legacy `SkillsLoader`
continues to work as before. The `SkillRegistry` in `skills_v2` loads both formats:
- SKILL.md (new) via `parse_skill_md`
- skill.json + prompt.md (legacy) via `load_legacy_skill`

When both exist in the same directory, SKILL.md takes precedence.

### Three-level priority

1. **Workspace** (`.clawft/skills/` in project root) -- highest
2. **User** (`~/.clawft/skills/`) -- medium
3. **Built-in** (compiled into binary) -- lowest

Higher-priority sources overwrite lower-priority ones with the same skill name.

### SkillRegistry uses synchronous I/O

The registry uses `std::fs` (not async) because:
- Skill discovery runs once at startup
- Directory scanning with async adds complexity without benefit
- Keeps the API simple (`discover()` returns `Result<Self>`)

## Test Coverage

### clawft-types::skill (4 tests)
- `skill_definition_new` -- constructor defaults
- `skill_format_default` -- Legacy is the default
- `skill_definition_serde_roundtrip` -- JSON serialize/deserialize, skip fields work
- `skill_definition_from_json_with_extras` -- flatten captures unknown fields as metadata

### clawft-core::agent::skills_v2 (25 tests)

Parser tests:
- Full frontmatter (all fields)
- Minimal frontmatter (name + description only)
- OpenClaw/ClawHub metadata section (extra fields preserved)
- Empty content returns error
- Missing frontmatter returns error
- Missing required name field returns error
- Invalid YAML returns error
- Boolean values parsed correctly
- Quoted string values handled

Registry tests:
- Empty registry when no sources
- Built-in skills loaded
- 3-level priority: workspace > user > builtin
- User overrides builtin
- Missing directories handled gracefully
- Legacy skill.json fallback works
- SKILL.md preferred over skill.json in same directory
- Multiple sources merged correctly
- Invalid skills skipped without aborting

Internal parser tests:
- `extract_frontmatter` basic, no opening, no closing
- `parse_yaml_frontmatter` scalars, lists, mixed, comments

## Verification

```
cargo test -p clawft-types --lib skill        # 4/4 pass
cargo test -p clawft-core --lib agent::skills_v2  # 25/25 pass
cargo clippy -p clawft-types -- -D warnings   # clean
cargo clippy -p clawft-core (skills_v2 only)  # clean (pre-existing warnings in workspace.rs)
```
