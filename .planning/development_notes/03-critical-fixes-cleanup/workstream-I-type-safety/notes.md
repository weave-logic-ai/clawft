# Workstream I: Type Safety & Cleanup -- Notes

**Status**: Complete (8/8 items)
**Completed**: 2026-02-19
**Agent**: Agent-03B (ac21cac)

---

## Implementation Log

### I1: DelegationTarget serde consistency (P2) -- DONE
- Added `#[serde(rename_all = "snake_case")]` to `DelegationTarget` enum in `clawft-types/src/delegation.rs`
- Added `#[serde(alias = "...")]` for PascalCase backward compatibility
- Serializes as `"local"`, `"claude"`, `"flow"`, `"auto"`

### I2: String-typed policy modes to enums (P2) -- DONE
- `PolicyMode` enum defined in `clawft-types/src/config/policies.rs`
- Variants: `Allowlist`, `Denylist` with `#[serde(rename_all = "snake_case")]`
- `CommandPolicyConfig.mode` and `UrlPolicyConfig.mode` now use `PolicyMode` enum
- Old string values ("allowlist"/"denylist") still deserialize correctly

### I3: ChatMessage::content serialization (P2) -- DONE
- Added `#[serde(skip_serializing_if = "Option::is_none")]` to `content` field in `clawft-llm/src/types.rs`
- `None` now omits field from JSON instead of serializing as `null`

### I4: Job ID collision fix (P2) -- DONE
- Replaced `generate_job_id()` (seconds + PID) with `uuid::Uuid::new_v4()` in `clawft-cli/src/commands/cron.rs`
- `uuid` already in workspace deps, no new dependency

### I5: camelCase normalizer acronym handling (P2) -- DONE
- Updated `normalize_keys()` in `clawft-platform/src/config_loader.rs`
- Added consecutive-uppercase detection: `"HTMLParser"` -> `"html_parser"` (not `"h_t_m_l_parser"`)

### I6: Dead code removal (P2) -- DONE
- Removed `#[allow(dead_code)]` from `evict_if_needed` in `clawft-core/src/pipeline/rate_limiter.rs`
- Added `// TODO(E1)` tracking comments for Discord `ResumePayload`
- Added `// TODO(C5)` for interactive slash-command framework
- Removed no-op CLI flags or documented with tracking TODOs

### I7: Fix always-true test assertion (P2) -- DONE
- `clawft-core/src/pipeline/transport.rs` test now asserts specific expected outcome
- Replaced `assert!(result.is_err() || result.is_ok())` with actual error assertion

### I8: Share MockTransport (P2) -- DONE
- `MockTransport` exposed behind `test-utils` feature flag in `clawft-services`
- `clawft-services/Cargo.toml` defines `test-utils` feature
- Downstream crates can use `clawft-services = { features = ["test-utils"] }` in dev-dependencies
