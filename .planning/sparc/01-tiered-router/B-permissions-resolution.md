# SPARC Implementation Plan: Phase B - UserPermissions & Permission Resolution

## Agent Instructions

### Context
This is Phase B of the Tiered Router sprint. The clawft project is a Rust AI assistant framework. The CLI binary is named `weft`.

### Dependencies
**Depends on Phase A (RoutingConfig types in clawft-types) completing first:**
- `RoutingConfig`, `ModelTierConfig`, `PermissionsConfig` structs in `clawft-types/src/routing.rs`
- `UserPermissions` struct definition (schema only, no resolution logic) in `clawft-types/src/routing.rs`
- `AuthContext` struct definition in `clawft-types/src/routing.rs`
- `PermissionLevelConfig` (all-Option fields for partial overrides) in `clawft-types/src/routing.rs`

**IMPORTANT (FIX-01):** All routing types are canonically owned by Phase A in `clawft_types::routing`.
Phase B MUST NOT redefine `UserPermissions`, `AuthContext`, or `PermissionLevelConfig`.
Phase B imports these types and adds `impl` methods in `clawft-core`.

### Planning Documents to Reference
- `08-tiered-router.md` -- Sections 2, 3: Permission model, granular dimensions
- `crates/clawft-types/src/routing.rs` -- Canonical type definitions (Phase A)
- `crates/clawft-core/src/pipeline/traits.rs` -- Pipeline types that permissions interact with

### File to Create
- `crates/clawft-core/src/pipeline/permissions.rs` (new)

### Branch
Work on branch: `weft/tiered-router`

---

## 1. Specification

### 1.1 UserPermissions Struct (Defined in Phase A -- DO NOT REDEFINE)

**IMPORTANT (FIX-01):** The `UserPermissions` struct is defined in `clawft_types::routing` (Phase A).
Phase B MUST import it: `use clawft_types::routing::UserPermissions;`
Phase B adds `impl` methods (named constructors, merge, level helpers) in `clawft-core`.

The `UserPermissions` struct represents the resolved capability matrix for a single request. It contains all 16+ dimensions from the design doc (Section 3.1 of `08-tiered-router.md`).

**Fields (all 16 dimensions) -- defined in Phase A:**

| Field | Type | Description |
|-------|------|-------------|
| `level` | `u8` | Permission level: 0 = zero_trust, 1 = user, 2 = admin |
| `max_tier` | `String` | Maximum model tier name (e.g. "free", "standard", "premium", "elite") |
| `model_access` | `Vec<String>` | Explicit model allowlist; supports globs like `"anthropic/*"`. Empty = all in tier. |
| `model_denylist` | `Vec<String>` | Explicit model denylist, checked after allowlist |
| `tool_access` | `Vec<String>` | Tool names allowed. `["*"]` = all tools. |
| `tool_denylist` | `Vec<String>` | Tool names denied even if `tool_access` allows |
| `max_context_tokens` | `usize` | Maximum input context tokens |
| `max_output_tokens` | `usize` | Maximum output tokens per response |
| `rate_limit` | `u32` | Requests per minute. 0 = unlimited. |
| `streaming_allowed` | `bool` | Whether SSE streaming is permitted |
| `escalation_allowed` | `bool` | Whether complexity-based tier escalation is permitted |
| `escalation_threshold` | `f32` | Complexity score (0.0-1.0) above which escalation triggers |
| `model_override` | `bool` | Whether user can manually specify a model |
| `cost_budget_daily_usd` | `f64` | Daily cost cap in USD. 0.0 = unlimited. |
| `cost_budget_monthly_usd` | `f64` | Monthly cost cap in USD. 0.0 = unlimited. |
| `custom_permissions` | `HashMap<String, serde_json::Value>` | Extensible custom dimensions for plugins |

### 1.2 AuthContext Struct (Defined in Phase A -- DO NOT REDEFINE)

**IMPORTANT (FIX-01):** The `AuthContext` struct is defined in `clawft_types::routing` (Phase A).
Phase B MUST import it: `use clawft_types::routing::AuthContext;`
Phase B MUST NOT provide its own `impl Default for AuthContext`.

The `AuthContext` struct threads authentication information from channels through the pipeline.

**Fields (defined in Phase A):**

| Field | Type | Description |
|-------|------|-------------|
| `sender_id` | `String` | Platform-specific sender ID (Telegram user ID, Slack user ID, `"local"` for CLI) |
| `channel` | `String` | Channel name the request came from (`"cli"`, `"telegram"`, `"discord"`, etc.) |
| `permissions` | `UserPermissions` | Resolved permissions for this sender/channel combination |

**Default behavior (FIX-01):** `AuthContext::default()` returns zero_trust permissions (NOT CLI admin).
This is a security-first design: absent context means minimal permissions.
For CLI admin behavior, Phase F's AgentLoop explicitly calls
`PermissionResolver::resolve_auth_context("local", "cli", false)`
which resolves to admin via the `determine_level()` logic, NOT via `Default`.

### 1.3 Permission Resolution Layering

Permissions are resolved by layering five sources, from lowest to highest priority:

```
Priority (lowest to highest):
  1. Built-in defaults for the resolved level (hardcoded in Rust)
  2. Global config:    routing.permissions.<level_name>  (e.g. routing.permissions.user)
  3. Workspace config: routing.permissions.<level_name>  (project-level .clawft/config.json override)
  4. Per-user:         routing.permissions.users.<sender_id>
  5. Per-channel:      routing.permissions.channels.<channel_name>
```

> **Design doc Section 3.2: per-channel (priority 5) > per-user (priority 4).
> Channel restrictions are enforceable even for named users.**
> This enables security patterns like "limit all users in #general to free tier."
> If an admin needs to bypass a channel restriction, the channel config itself
> should be modified rather than relying on per-user overrides to override channel rules.

**Resolution algorithm:**
1. Determine the user's permission level:
   - If `sender_id` is found in `routing.permissions.users`, use that entry's `level` field
   - Else if the `channel` is found in `routing.permissions.channels`, use that entry's `level` field
   - Else if the `sender_id` appears in the channel's `allow_from` list, assign level 1 (user)
   - Else assign level 0 (zero_trust)
2. Start with the built-in defaults for that level
3. Merge global config overrides for that level name (e.g. `routing.permissions.user` for level 1)
4. Merge workspace config overrides for that level name (if workspace config exists)
5. Merge per-user overrides (if `routing.permissions.users.<sender_id>` exists)
6. Merge per-channel overrides (if `routing.permissions.channels.<channel>` exists) -- highest priority

### 1.4 Three Built-in Permission Levels

#### Level 0: zero_trust

For anonymous or unidentified users. Maximally restrictive.

| Dimension | Default Value |
|-----------|---------------|
| `level` | 0 |
| `max_tier` | `"free"` |
| `model_access` | `[]` (empty = all models in allowed tiers) |
| `model_denylist` | `[]` |
| `tool_access` | `[]` (no tools) |
| `tool_denylist` | `[]` |
| `max_context_tokens` | 4096 |
| `max_output_tokens` | 1024 |
| `rate_limit` | 10 |
| `streaming_allowed` | false |
| `escalation_allowed` | false |
| `escalation_threshold` | 1.0 (effectively never) |
| `model_override` | false |
| `cost_budget_daily_usd` | 0.10 |
| `cost_budget_monthly_usd` | 2.00 |
| `custom_permissions` | `{}` |

#### Level 1: user

For authenticated users identified via `allow_from` or channel auth.

| Dimension | Default Value |
|-----------|---------------|
| `level` | 1 |
| `max_tier` | `"standard"` |
| `model_access` | `[]` |
| `model_denylist` | `[]` |
| `tool_access` | `["read_file", "write_file", "edit_file", "list_dir", "web_search", "web_fetch", "message"]` |
| `tool_denylist` | `[]` |
| `max_context_tokens` | 16384 |
| `max_output_tokens` | 4096 |
| `rate_limit` | 60 |
| `streaming_allowed` | true |
| `escalation_allowed` | true |
| `escalation_threshold` | 0.6 |
| `model_override` | false |
| `cost_budget_daily_usd` | 5.00 |
| `cost_budget_monthly_usd` | 100.00 |
| `custom_permissions` | `{}` |

#### Level 2: admin

For operators, owners, and explicitly designated admins. Full access.

| Dimension | Default Value |
|-----------|---------------|
| `level` | 2 |
| `max_tier` | `"elite"` |
| `model_access` | `[]` |
| `model_denylist` | `[]` |
| `tool_access` | `["*"]` |
| `tool_denylist` | `[]` |
| `max_context_tokens` | 200000 |
| `max_output_tokens` | 16384 |
| `rate_limit` | 0 (unlimited) |
| `streaming_allowed` | true |
| `escalation_allowed` | true |
| `escalation_threshold` | 0.0 (escalate at any complexity) |
| `model_override` | true |
| `cost_budget_daily_usd` | 0.0 (unlimited) |
| `cost_budget_monthly_usd` | 0.0 (unlimited) |
| `custom_permissions` | `{}` |

### 1.5 Merge Semantics

When merging a higher-priority layer onto a lower-priority base:

- **Scalar fields** (`level`, `max_tier`, `max_context_tokens`, etc.): The higher-priority value replaces the lower-priority value entirely. Only fields explicitly present in the override layer are applied; absent fields retain the base value.
- **Vec fields** (`model_access`, `tool_access`, etc.): If the override provides a non-empty vec, it replaces the base vec entirely. An empty vec in the override means "no change" (keep base), NOT "clear the list".
- **`custom_permissions` map**: Keys from the override are merged into the base map. Override keys overwrite same-named base keys. Base keys not present in the override are preserved. This is a shallow merge, not deep.

### 1.6 CLI Default Behavior

When a request comes from the CLI channel:
- The `sender_id` is `"local"`
- The `channel` is `"cli"`
- `PermissionResolver::determine_level("local", "cli")` returns `admin` (level 2) because the local user owns the machine
- This can be overridden in config via `routing.permissions.channels.cli`

**IMPORTANT (FIX-01):** CLI admin behavior is achieved through `PermissionResolver::resolve()`,
NOT through `AuthContext::default()`. `AuthContext::default()` returns zero_trust.
Phase F's AgentLoop must explicitly call `resolver.resolve_auth_context("local", "cli", false)` for CLI.

### 1.7 Permission Level Name Mapping

| Level | Name |
|-------|------|
| 0 | `"zero_trust"` |
| 1 | `"user"` |
| 2 | `"admin"` |

The `level_name()` function converts a numeric level to its string name for config lookup.

---

## 2. Pseudocode

### 2.1 UserPermissions Struct (REFERENCE ONLY -- defined in Phase A)

**DO NOT REDEFINE.** Import from `clawft_types::routing::UserPermissions`.
The struct definition below is shown for reference only. Phase A owns it.

```rust
// REFERENCE ONLY -- this struct is defined in clawft_types::routing (Phase A).
// Phase B imports it and adds impl methods in clawft-core.
//
// use clawft_types::routing::UserPermissions;
//
// See Phase A plan for the canonical struct definition with all 16 fields.
```

### 2.2 AuthContext Struct (REFERENCE ONLY -- defined in Phase A)

**DO NOT REDEFINE.** Import from `clawft_types::routing::AuthContext`.
The struct definition and its `Default` impl are owned by Phase A.

```rust
// REFERENCE ONLY -- this struct is defined in clawft_types::routing (Phase A).
// Phase B imports it: use clawft_types::routing::AuthContext;
//
// IMPORTANT (FIX-01): AuthContext::default() returns zero_trust permissions,
// NOT CLI admin. This is defined in Phase A. Phase B must NOT override Default.
//
// Phase A's Default impl:
//   impl Default for AuthContext {
//       fn default() -> Self {
//           Self {
//               sender_id: "local".into(),
//               channel: "unknown".into(),
//               permissions: UserPermissions::zero_trust_defaults(),
//           }
//       }
//   }
//
// For CLI admin context, use PermissionResolver::resolve_auth_context("local", "cli", false).
```

### 2.3 Built-in Defaults (impl methods added by Phase B in clawft-core)

These `impl UserPermissions` methods are added in `clawft-core/src/pipeline/permissions.rs`.
The struct itself is defined in Phase A (`clawft_types::routing`).

**Named constructors (FIX-01):** Must use `_defaults` suffix:
`zero_trust_defaults()`, `user_defaults()`, `admin_defaults()`.

```rust
use clawft_types::routing::UserPermissions;

impl UserPermissions {
    /// Built-in defaults for Level 0 (zero_trust).
    pub fn zero_trust_defaults() -> Self {
        Self {
            level: 0,
            max_tier: "free".into(),
            model_access: vec![],
            model_denylist: vec![],
            tool_access: vec![],
            tool_denylist: vec![],
            max_context_tokens: 4096,
            max_output_tokens: 1024,
            rate_limit: 10,
            streaming_allowed: false,
            escalation_allowed: false,
            escalation_threshold: 1.0,
            model_override: false,
            cost_budget_daily_usd: 0.10,
            cost_budget_monthly_usd: 2.00,
            custom_permissions: HashMap::new(),
        }
    }

    /// Built-in defaults for Level 1 (user).
    pub fn user_defaults() -> Self {
        Self {
            level: 1,
            max_tier: "standard".into(),
            model_access: vec![],
            model_denylist: vec![],
            tool_access: vec![
                "read_file".into(),
                "write_file".into(),
                "edit_file".into(),
                "list_dir".into(),
                "web_search".into(),
                "web_fetch".into(),
                "message".into(),
            ],
            tool_denylist: vec![],
            max_context_tokens: 16384,
            max_output_tokens: 4096,
            rate_limit: 60,
            streaming_allowed: true,
            escalation_allowed: true,
            escalation_threshold: 0.6,
            model_override: false,
            cost_budget_daily_usd: 5.00,
            cost_budget_monthly_usd: 100.00,
            custom_permissions: HashMap::new(),
        }
    }

    /// Built-in defaults for Level 2 (admin).
    pub fn admin_defaults() -> Self {
        Self {
            level: 2,
            max_tier: "elite".into(),
            model_access: vec![],
            model_denylist: vec![],
            tool_access: vec!["*".into()],
            tool_denylist: vec![],
            max_context_tokens: 200_000,
            max_output_tokens: 16384,
            rate_limit: 0,
            streaming_allowed: true,
            escalation_allowed: true,
            escalation_threshold: 0.0,
            model_override: true,
            cost_budget_daily_usd: 0.0,
            cost_budget_monthly_usd: 0.0,
            custom_permissions: HashMap::new(),
        }
    }

    /// Return the built-in defaults for a given numeric level.
    /// Unknown levels fall back to zero_trust.
    pub fn defaults_for_level(level: u8) -> Self {
        match level {
            0 => Self::zero_trust_defaults(),
            1 => Self::user_defaults(),
            2 => Self::admin_defaults(),
            _ => Self::zero_trust_defaults(),
        }
    }

    /// Convert a numeric level to its string name for config lookup.
    pub fn level_name(level: u8) -> &'static str {
        match level {
            0 => "zero_trust",
            1 => "user",
            2 => "admin",
            _ => "zero_trust",
        }
    }

    /// Convert a string level name to its numeric level.
    /// Returns None for unknown names.
    pub fn level_from_name(name: &str) -> Option<u8> {
        match name {
            "zero_trust" => Some(0),
            "user" => Some(1),
            "admin" => Some(2),
            _ => None,
        }
    }
}
```

### 2.4 Merge Logic

**IMPORTANT (FIX-01):** Phase B does NOT define a `PermissionOverrides` struct.
Instead, it uses Phase A's `PermissionLevelConfig` (all-Option fields) from `clawft_types::routing`.
The `PermissionLevelConfig` struct is the canonical partial-override type for all config layers
(global level defaults, workspace overrides, per-channel overrides, per-user overrides).

```rust
use clawft_types::routing::PermissionLevelConfig;

impl UserPermissions {
    /// Merge overrides into self. Only fields present in `overrides`
    /// (i.e. Some(...)) replace the corresponding field in self.
    ///
    /// For Vec fields: a non-empty override replaces the base entirely.
    /// An empty override vec means "no change" (keep base).
    ///
    /// For custom_permissions: shallow key-level merge. Override keys
    /// overwrite base keys; base keys absent from override are kept.
    pub fn merge(&mut self, overrides: &PermissionLevelConfig) {
        if let Some(level) = overrides.level {
            self.level = level;
        }
        if let Some(ref max_tier) = overrides.max_tier {
            self.max_tier = max_tier.clone();
        }
        if let Some(ref model_access) = overrides.model_access {
            if !model_access.is_empty() {
                self.model_access = model_access.clone();
            }
        }
        if let Some(ref model_denylist) = overrides.model_denylist {
            if !model_denylist.is_empty() {
                self.model_denylist = model_denylist.clone();
            }
        }
        if let Some(ref tool_access) = overrides.tool_access {
            if !tool_access.is_empty() {
                self.tool_access = tool_access.clone();
            }
        }
        if let Some(ref tool_denylist) = overrides.tool_denylist {
            if !tool_denylist.is_empty() {
                self.tool_denylist = tool_denylist.clone();
            }
        }
        if let Some(max_context_tokens) = overrides.max_context_tokens {
            self.max_context_tokens = max_context_tokens;
        }
        if let Some(max_output_tokens) = overrides.max_output_tokens {
            self.max_output_tokens = max_output_tokens;
        }
        if let Some(rate_limit) = overrides.rate_limit {
            self.rate_limit = rate_limit;
        }
        if let Some(streaming_allowed) = overrides.streaming_allowed {
            self.streaming_allowed = streaming_allowed;
        }
        if let Some(escalation_allowed) = overrides.escalation_allowed {
            self.escalation_allowed = escalation_allowed;
        }
        if let Some(escalation_threshold) = overrides.escalation_threshold {
            self.escalation_threshold = escalation_threshold;
        }
        if let Some(model_override) = overrides.model_override {
            self.model_override = model_override;
        }
        if let Some(cost_budget_daily_usd) = overrides.cost_budget_daily_usd {
            self.cost_budget_daily_usd = cost_budget_daily_usd;
        }
        if let Some(cost_budget_monthly_usd) = overrides.cost_budget_monthly_usd {
            self.cost_budget_monthly_usd = cost_budget_monthly_usd;
        }
        if let Some(ref custom) = overrides.custom_permissions {
            for (key, value) in custom {
                self.custom_permissions.insert(key.clone(), value.clone());
            }
        }
    }
}
```

### 2.5 PermissionResolver

**IMPORTANT (FIX-02):** The constructor takes `&RoutingConfig` (not 6 separate HashMap params).
`resolve()` takes 3 params: `(sender_id, channel, allow_from_match)`.
This is the ONLY permission resolution implementation -- Phase C and F must NOT re-implement.

**IMPORTANT (FIX-04):** The resolver accepts both global and workspace configs separately.
Workspace configs are subject to ceiling enforcement: workspace permissions cannot expand
beyond global config for security-sensitive fields.

```rust
use clawft_types::routing::{
    AuthContext, PermissionLevelConfig, RoutingConfig, UserPermissions,
};

/// The single, authoritative permission resolution implementation.
///
/// Phase C and Phase F MUST use this resolver. They MUST NOT re-implement
/// resolution or merge logic.
///
/// This struct holds pre-extracted config layers from both global and
/// workspace RoutingConfig sources for the 5-layer resolution algorithm.
///
/// SECURITY (FIX-04): When a workspace config is provided, ceiling
/// enforcement ensures the workspace cannot expand permissions beyond
/// the global config for security-sensitive fields.
pub struct PermissionResolver {
    /// Global-level permission overrides keyed by level name
    /// (e.g. "zero_trust", "user", "admin").
    global_level_overrides: HashMap<String, PermissionLevelConfig>,

    /// Workspace-level permission overrides keyed by level name.
    /// Applied after global, before channel/user.
    /// Subject to ceiling enforcement against global config.
    workspace_level_overrides: HashMap<String, PermissionLevelConfig>,

    /// Per-user permission overrides keyed by sender_id.
    user_overrides: HashMap<String, PermissionLevelConfig>,

    /// Per-channel permission overrides keyed by channel name.
    channel_overrides: HashMap<String, PermissionLevelConfig>,

    /// Channel allow_from lists, keyed by channel name.
    /// Pre-extracted at construction time for O(1) lookup during resolve().
    channel_allow_from: HashMap<String, Vec<String>>,

    /// Default permission level for the CLI channel.
    /// Defaults to 2 (admin) if not configured.
    cli_default_level: u8,
}

impl PermissionResolver {
    /// Create a new resolver from a global RoutingConfig.
    ///
    /// IMPORTANT (FIX-02): Constructor takes &RoutingConfig, not 6 HashMaps.
    /// Extracts all needed config layers internally.
    ///
    /// IMPORTANT (FIX-04): workspace_config is accepted separately.
    /// Ceiling enforcement is applied: workspace cannot expand permissions
    /// beyond global config for security-sensitive fields.
    pub fn new(
        global_config: &RoutingConfig,
        workspace_config: Option<&RoutingConfig>,
    ) -> Self {
        // Extract global level overrides from global_config.permissions
        let mut global_level_overrides = HashMap::new();
        if let Some(ref zt) = global_config.permissions.zero_trust {
            global_level_overrides.insert("zero_trust".to_string(), zt.clone());
        }
        if let Some(ref user) = global_config.permissions.user {
            global_level_overrides.insert("user".to_string(), user.clone());
        }
        if let Some(ref admin) = global_config.permissions.admin {
            global_level_overrides.insert("admin".to_string(), admin.clone());
        }

        // Extract workspace level overrides (subject to ceiling enforcement)
        let mut workspace_level_overrides = HashMap::new();
        if let Some(ws) = workspace_config {
            if let Some(ref zt) = ws.permissions.zero_trust {
                workspace_level_overrides.insert("zero_trust".to_string(), zt.clone());
            }
            if let Some(ref user) = ws.permissions.user {
                workspace_level_overrides.insert("user".to_string(), user.clone());
            }
            if let Some(ref admin) = ws.permissions.admin {
                workspace_level_overrides.insert("admin".to_string(), admin.clone());
            }
        }

        // Extract per-user overrides from global config
        let user_overrides = global_config.permissions.users.clone().unwrap_or_default();

        // Extract per-channel overrides from global config
        let channel_overrides = global_config.permissions.channels.clone().unwrap_or_default();

        // Build channel allow_from map from global config's channel definitions.
        // The allow_from lists are extracted at construction time for O(1) lookup.
        let channel_allow_from = global_config
            .permissions
            .channel_allow_from
            .clone()
            .unwrap_or_default();

        Self {
            global_level_overrides,
            workspace_level_overrides,
            user_overrides,
            channel_overrides,
            channel_allow_from,
            cli_default_level: 2,
        }
    }

    /// Create a minimal resolver with no config overrides.
    /// CLI defaults to admin; everything else is zero_trust.
    pub fn default_resolver() -> Self {
        Self {
            global_level_overrides: HashMap::new(),
            workspace_level_overrides: HashMap::new(),
            user_overrides: HashMap::new(),
            channel_overrides: HashMap::new(),
            channel_allow_from: HashMap::new(),
            cli_default_level: 2,
        }
    }

    /// Resolve the effective permissions for a sender on a channel.
    ///
    /// IMPORTANT (FIX-02): Takes 3 params: (sender_id, channel, allow_from_match).
    /// The `allow_from_match` parameter indicates whether the channel plugin already
    /// confirmed this sender_id is in the channel's allow_from list. This decouples
    /// allow_from checking from the resolver (the channel plugin does the check).
    ///
    /// Implements the 5-layer resolution:
    ///   1. Built-in defaults for the resolved level
    ///   2. Global config overrides for that level
    ///   3. Workspace config overrides for that level (with ceiling enforcement)
    ///   4. Per-user overrides
    ///   5. Per-channel overrides (highest priority -- design doc Section 3.2)
    ///   6. Workspace ceiling enforcement (FIX-04)
    pub fn resolve(
        &self,
        sender_id: &str,
        channel: &str,
        allow_from_match: bool,
    ) -> UserPermissions {
        // Step 1: Determine the permission level
        let level = self.determine_level(sender_id, channel, allow_from_match);

        // Step 2: Start with built-in defaults for that level
        let mut permissions = UserPermissions::defaults_for_level(level);

        // Step 3: Merge global config overrides for this level name
        let level_name = UserPermissions::level_name(level);
        if let Some(global_overrides) = self.global_level_overrides.get(level_name) {
            permissions.merge(global_overrides);
        }

        // Step 4: Merge workspace config overrides for this level name
        if let Some(workspace_overrides) = self.workspace_level_overrides.get(level_name) {
            permissions.merge(workspace_overrides);
        }

        // Step 5: Merge per-user overrides
        if let Some(user_overrides) = self.user_overrides.get(sender_id) {
            permissions.merge(user_overrides);
        }

        // Step 6: Merge per-channel overrides (highest priority)
        // Design doc Section 3.2: per-channel > per-user. Channel restrictions
        // are enforceable even for named users.
        if let Some(channel_overrides) = self.channel_overrides.get(channel) {
            permissions.merge(channel_overrides);
        }

        // Step 7 (FIX-04): Apply workspace ceiling enforcement
        // If a workspace config was provided, enforce that security-sensitive
        // fields do not exceed the global config's resolved values.
        if !self.workspace_level_overrides.is_empty() {
            let global_resolved = self.resolve_global_only(sender_id, channel, allow_from_match);
            Self::enforce_workspace_ceiling(&mut permissions, &global_resolved);
        }

        permissions
    }

    /// Resolve permissions using ONLY global config (no workspace overrides).
    /// Used internally for ceiling enforcement comparison.
    fn resolve_global_only(
        &self,
        sender_id: &str,
        channel: &str,
        allow_from_match: bool,
    ) -> UserPermissions {
        let level = self.determine_level(sender_id, channel, allow_from_match);
        let mut permissions = UserPermissions::defaults_for_level(level);
        let level_name = UserPermissions::level_name(level);
        if let Some(global_overrides) = self.global_level_overrides.get(level_name) {
            permissions.merge(global_overrides);
        }
        // Per-user before per-channel (channel has highest priority)
        if let Some(user_overrides) = self.user_overrides.get(sender_id) {
            permissions.merge(user_overrides);
        }
        if let Some(channel_overrides) = self.channel_overrides.get(channel) {
            permissions.merge(channel_overrides);
        }
        permissions
    }

    /// Enforce workspace ceiling: workspace cannot expand permissions beyond
    /// global config for security-sensitive fields.
    ///
    /// SECURITY (FIX-04): The following fields are ceiling-enforced:
    /// - level: cannot exceed global level
    /// - escalation_allowed: cannot enable if global disables
    /// - tool_access: cannot add tools not present in global tool_access
    /// - rate_limit: cannot increase (0 = unlimited is highest, non-zero values:
    ///   workspace cannot set higher than global)
    /// - cost_budget_daily_usd: cannot increase (0.0 = unlimited is highest)
    /// - cost_budget_monthly_usd: cannot increase (0.0 = unlimited is highest)
    /// - max_tier: cannot upgrade beyond global max_tier
    fn enforce_workspace_ceiling(
        permissions: &mut UserPermissions,
        global_ceiling: &UserPermissions,
    ) {
        // Level: cannot exceed global
        if permissions.level > global_ceiling.level {
            permissions.level = global_ceiling.level;
        }

        // Escalation: cannot enable if global disables
        if !global_ceiling.escalation_allowed {
            permissions.escalation_allowed = false;
        }

        // Tool access: workspace cannot add tools not in global.
        // If global is ["*"], any workspace tools are allowed.
        // Otherwise, filter workspace tools to only those in global.
        if !global_ceiling.tool_access.contains(&"*".to_string()) {
            permissions.tool_access.retain(|tool| {
                tool == "*" || global_ceiling.tool_access.contains(tool)
            });
            // If workspace had ["*"] but global did not, replace with global's list
            if permissions.tool_access.contains(&"*".to_string()) {
                permissions.tool_access = global_ceiling.tool_access.clone();
            }
        }

        // Rate limit: cannot increase beyond global.
        // 0 = unlimited (most permissive). Non-zero: lower is more restrictive.
        // If global is non-zero and workspace is 0 (unlimited), clamp to global.
        // If global is non-zero and workspace is higher, clamp to global.
        if global_ceiling.rate_limit > 0 {
            if permissions.rate_limit == 0 || permissions.rate_limit > global_ceiling.rate_limit {
                permissions.rate_limit = global_ceiling.rate_limit;
            }
        }

        // Cost budgets: cannot increase beyond global.
        // 0.0 = unlimited (most permissive).
        // If global is non-zero and workspace exceeds it, clamp.
        if global_ceiling.cost_budget_daily_usd > 0.0 {
            if permissions.cost_budget_daily_usd == 0.0
                || permissions.cost_budget_daily_usd > global_ceiling.cost_budget_daily_usd
            {
                permissions.cost_budget_daily_usd = global_ceiling.cost_budget_daily_usd;
            }
        }
        if global_ceiling.cost_budget_monthly_usd > 0.0 {
            if permissions.cost_budget_monthly_usd == 0.0
                || permissions.cost_budget_monthly_usd > global_ceiling.cost_budget_monthly_usd
            {
                permissions.cost_budget_monthly_usd = global_ceiling.cost_budget_monthly_usd;
            }
        }

        // max_tier: cannot upgrade beyond global.
        // Tier ordering is determined by the tier index in the routing config.
        // For now, we compare by the known tier names. A more robust approach
        // would use the tier ordering from RoutingConfig, but that requires
        // passing additional context. The implementation should use a tier_rank()
        // helper that maps tier names to numeric ranks.
        // Known tier order: free(0) < standard(1) < premium(2) < elite(3)
        let tier_rank = |t: &str| -> u8 {
            match t {
                "free" => 0,
                "standard" => 1,
                "premium" => 2,
                "elite" => 3,
                _ => 0, // unknown tiers treated as lowest
            }
        };
        if tier_rank(&permissions.max_tier) > tier_rank(&global_ceiling.max_tier) {
            permissions.max_tier = global_ceiling.max_tier.clone();
        }
    }

    /// Determine the permission level for a sender on a channel.
    ///
    /// IMPORTANT (FIX-02): Takes allow_from_match as a parameter rather than
    /// looking up allow_from lists internally. The channel plugin provides this.
    ///
    /// Priority:
    ///   1. Explicit per-user level (routing.permissions.users.<sender_id>.level)
    ///   2. Explicit per-channel level (routing.permissions.channels.<channel>.level)
    ///   3. If allow_from_match is true -> level 1 (user)
    ///   4. CLI channel -> cli_default_level (default: admin)
    ///   5. Otherwise -> level 0 (zero_trust)
    fn determine_level(
        &self,
        sender_id: &str,
        channel: &str,
        allow_from_match: bool,
    ) -> u8 {
        // Check per-user override first for level determination
        // (Note: level determination priority differs from merge priority.
        //  Level determination: user > channel. Merge priority: channel > user.)
        if let Some(user_overrides) = self.user_overrides.get(sender_id) {
            if let Some(level) = user_overrides.level {
                return level;
            }
        }

        // Check per-channel override
        if let Some(channel_overrides) = self.channel_overrides.get(channel) {
            if let Some(level) = channel_overrides.level {
                return level;
            }
        }

        // Check if sender is confirmed in channel's allow_from
        if allow_from_match {
            return 1; // user level
        }

        // CLI channel defaults to admin
        if channel == "cli" {
            return self.cli_default_level;
        }

        // Default: zero_trust
        0
    }

    /// Resolve permissions and wrap in an AuthContext.
    ///
    /// IMPORTANT (FIX-02): Takes 3 params matching resolve() signature.
    pub fn resolve_auth_context(
        &self,
        sender_id: &str,
        channel: &str,
        allow_from_match: bool,
    ) -> AuthContext {
        let permissions = self.resolve(sender_id, channel, allow_from_match);
        AuthContext {
            sender_id: sender_id.to_string(),
            channel: channel.to_string(),
            permissions,
        }
    }

    /// Validate that a workspace config does not attempt to expand permissions
    /// beyond the global config ceiling.
    ///
    /// SECURITY (FIX-04): Returns a list of warnings/violations found.
    /// This is a static validation that can be run at config load time
    /// (Phase H validation) to warn operators about problematic workspace configs.
    ///
    /// Security-sensitive fields checked:
    /// - level: workspace cannot grant higher level than global
    /// - escalation_allowed: workspace cannot enable if global disables
    /// - tool_access: workspace cannot add tools not in global
    /// - rate_limit: workspace cannot increase beyond global
    /// - cost_budget_daily_usd: workspace cannot increase beyond global
    /// - cost_budget_monthly_usd: workspace cannot increase beyond global
    /// - max_tier: workspace cannot upgrade beyond global
    pub fn validate_workspace_ceiling(
        global_config: &RoutingConfig,
        workspace_config: &RoutingConfig,
    ) -> Vec<String> {
        let mut violations = Vec::new();

        // Check each level name for ceiling violations
        for level_name in &["zero_trust", "user", "admin"] {
            let global_overrides = match *level_name {
                "zero_trust" => global_config.permissions.zero_trust.as_ref(),
                "user" => global_config.permissions.user.as_ref(),
                "admin" => global_config.permissions.admin.as_ref(),
                _ => None,
            };
            let workspace_overrides = match *level_name {
                "zero_trust" => workspace_config.permissions.zero_trust.as_ref(),
                "user" => workspace_config.permissions.user.as_ref(),
                "admin" => workspace_config.permissions.admin.as_ref(),
                _ => None,
            };

            if let Some(ws) = workspace_overrides {
                // Check level ceiling
                if let Some(ws_level) = ws.level {
                    let global_level = global_overrides
                        .and_then(|g| g.level)
                        .unwrap_or(UserPermissions::level_from_name(level_name).unwrap_or(0));
                    if ws_level > global_level {
                        violations.push(format!(
                            "Workspace {}: level {} exceeds global ceiling {}",
                            level_name, ws_level, global_level
                        ));
                    }
                }

                // Check escalation_allowed ceiling
                if let Some(true) = ws.escalation_allowed {
                    if let Some(g) = global_overrides {
                        if let Some(false) = g.escalation_allowed {
                            violations.push(format!(
                                "Workspace {}: escalation_allowed=true exceeds global ceiling (false)",
                                level_name
                            ));
                        }
                    }
                }

                // Additional field checks follow the same pattern.
                // The full implementation checks all security-sensitive fields listed above.
            }
        }

        violations
    }
}
```

---

## 3. Architecture

### 3.1 File Location

**New file:** `crates/clawft-core/src/pipeline/permissions.rs`

This file contains:
- `impl UserPermissions` methods: named constructors (`zero_trust_defaults()`, `user_defaults()`, `admin_defaults()`), `defaults_for_level()`, `level_name()`, `level_from_name()`, `merge()`
- `PermissionResolver` struct and resolution algorithm (the ONLY resolution implementation)
- `validate_workspace_ceiling()` static validation method

**This file does NOT define (FIX-01):**
- `UserPermissions` struct (defined in `clawft_types::routing`, Phase A)
- `AuthContext` struct (defined in `clawft_types::routing`, Phase A)
- `PermissionLevelConfig` struct (defined in `clawft_types::routing`, Phase A)

All types are imported: `use clawft_types::routing::{AuthContext, PermissionLevelConfig, RoutingConfig, UserPermissions};`

### 3.2 Module Wiring

The `pipeline/mod.rs` must re-export the new module:

```rust
// In crates/clawft-core/src/pipeline/mod.rs
pub mod permissions;
pub use permissions::PermissionResolver;

// NOTE (FIX-01): UserPermissions, AuthContext, and PermissionLevelConfig are
// NOT re-exported from here. They are imported directly from clawft_types::routing.
// Downstream code uses: use clawft_types::routing::{UserPermissions, AuthContext, ...};
```

### 3.3 Integration with ChatRequest

The `ChatRequest` type (in `crates/clawft-core/src/pipeline/traits.rs`) gains an optional `auth_context` field:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<LlmMessage>,
    #[serde(default)]
    pub tools: Vec<serde_json::Value>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub max_tokens: Option<i32>,
    #[serde(default)]
    pub temperature: Option<f64>,

    /// Authentication context for permission-aware routing.
    /// When None, the router falls back to default behavior
    /// (admin for CLI, zero_trust for channels).
    #[serde(default)]
    pub auth_context: Option<AuthContext>,
}
```

### 3.4 Integration with Pipeline Flow

The `TieredRouter` (Phase C) uses the resolved permissions.

**IMPORTANT (FIX-02):** Phase C reads `request.auth_context.permissions` directly.
It does NOT re-implement resolution or merge logic. Phase F's AgentLoop calls
`resolver.resolve_auth_context(sender_id, channel, allow_from_match)` and attaches
the result to ChatRequest before passing to the pipeline.

```
Channel Plugin
  |  identifies sender, creates InboundMessage with sender_id + allow_from_match metadata
  v
AgentLoop (Phase F)
  |  extracts sender_id, channel, allow_from_match from InboundMessage
  |  calls PermissionResolver::resolve_auth_context(sender_id, channel, allow_from_match)
  |  attaches AuthContext to ChatRequest
  v
PipelineRegistry::complete(request)
  |
  v
TieredRouter::route(request, profile)
  |  reads request.auth_context.permissions
  |  filters tiers by max_tier
  |  checks escalation_allowed
  |  applies rate_limit, cost_budget
  v
RoutingDecision
```

### 3.5 How Channels Provide Auth Context

Each channel plugin is responsible for identifying the sender, checking allow_from membership, and setting metadata on the `InboundMessage`. The `AgentLoop` then resolves permissions.

| Channel | sender_id source | allow_from_match source | How identified |
|---------|------------------|------------------------|----------------|
| CLI | `"local"` (constant) | `false` (N/A for CLI) | Always local user |
| Telegram | `update.message.from.id` (numeric, as string) | Channel plugin checks allow_from list | Telegram API |
| Discord | `message.author.id` (snowflake, as string) | Channel plugin checks allow_from list | Discord API |
| Slack | `event.user` (Slack user ID) | Channel plugin checks allow_from list | Slack Events API |
| WhatsApp | Phone number from bridge | Channel plugin checks allow_from list | WhatsApp bridge |

The `AgentLoop` calls `PermissionResolver::resolve_auth_context(sender_id, channel, allow_from_match)` (FIX-02: 3 params) which performs the full 5-layer resolution with workspace ceiling enforcement. The resulting `AuthContext` is attached to the `ChatRequest` before passing it to the pipeline.

### 3.6 CLI Default Behavior

For CLI mode:
- `sender_id = "local"`
- `channel = "cli"`
- `allow_from_match = false` (CLI has no allow_from concept)
- `PermissionResolver::determine_level("local", "cli", false)` checks:
  1. Is `"local"` in `user_overrides`? (typically no)
  2. Is `"cli"` in `channel_overrides`? (check config)
  3. Is `allow_from_match` true? No.
  4. Is `channel == "cli"`? **Yes** -> return `cli_default_level` (2 = admin)
- Result: full admin permissions

**NOTE (FIX-01):** This admin resolution happens through the resolver, NOT through
`AuthContext::default()`. `AuthContext::default()` returns zero_trust.

### 3.7 Dependency Graph

```
clawft-types/src/routing.rs (Phase A)
    |  RoutingConfig, PermissionsConfig, UserPermissions, AuthContext,
    |  PermissionLevelConfig types (canonical definitions)
    v
clawft-core/src/pipeline/permissions.rs (Phase B -- this file)
    |  impl UserPermissions methods, PermissionResolver (ONLY resolver implementation)
    |  validate_workspace_ceiling() (FIX-04)
    v
clawft-core/src/pipeline/traits.rs
    |  ChatRequest gains auth_context field
    v
clawft-core/src/pipeline/tiered_router.rs (Phase C)
    |  TieredRouter reads permissions from auth_context
    |  Does NOT re-implement resolution (FIX-02)
    v
clawft-core/src/agent/loop.rs (Phase F)
    |  AgentLoop calls resolver.resolve_auth_context(sender_id, channel, allow_from_match)
    |  Does NOT re-implement resolution (FIX-02)
```

---

## 4. Refinement

### 4.1 Edge Cases

#### Unknown sender_id
When a `sender_id` is not found in any `user_overrides` or `allow_from` list, and the channel has no explicit override, the user gets level 0 (zero_trust). This is intentional: unknown users should be maximally restricted.

#### Missing channel config
If a channel name is not present in `channel_overrides`, the channel-level merge step is simply skipped. The user gets only the built-in defaults + global overrides for their level. This is safe because all dimensions have sensible defaults.

#### Conflicting overrides
Conflicts are resolved by layer priority. Per-channel overrides take final precedence over per-user overrides (design doc Section 3.2: per-channel > per-user). If a per-user override sets `max_tier: "standard"` but a per-channel override sets `max_tier: "free"`, the per-channel restriction wins because channel overrides are applied after user overrides. This ensures channel restrictions are enforceable even for named users.

Example resolution chain:
```
Built-in admin defaults:        max_tier = "elite"
User override (alice):          max_tier = "standard"  -> max_tier = "standard"
Channel override (discord):     max_tier = "free"      -> max_tier = "free"
Final result:                   max_tier = "free"
```

#### Unknown permission level numbers
Levels outside 0-2 fall back to zero_trust defaults. The `defaults_for_level` function handles this via the `_ => Self::zero_trust_defaults()` match arm. This prevents privilege escalation via crafted config values.

#### allow_from_match parameter (FIX-02)
The `allow_from_match` parameter is a boolean passed by the channel plugin / AgentLoop.
The channel plugin is responsible for checking whether the sender_id is in the channel's
allow_from list. The resolver does not look up allow_from lists itself -- it trusts
the caller's determination.

- `allow_from_match = true`: sender is confirmed in allow_from, gets level 1 (user)
- `allow_from_match = false`: falls through to CLI default or zero_trust

**Important distinction:** `allow_from` gates channel *access* (binary: can the user talk to the bot?). Permission level gates *capabilities* (granular: what can the user do?). The channel plugin rejects users not in `allow_from` before the message ever reaches the permission resolver.

### 4.2 Security Considerations

#### Zero-trust must be truly restrictive
- No tool access by default (empty `tool_access`)
- Low token limits (4096 context, 1024 output)
- No escalation allowed
- No model override
- Tight budget ($0.10/day, $2.00/month)
- No streaming (reduces server resource usage)
- Escalation threshold set to 1.0 (unreachable), ensuring zero_trust users cannot accidentally escalate

#### No privilege escalation by default
- Unknown levels map to zero_trust (not user or admin)
- The `level` field in `PermissionLevelConfig` is an `Option<u8>`, so it can only be set explicitly
- The merge function does not allow escalation beyond what the override provides
- CLI defaults to admin, but this is appropriate because the local user owns the machine

#### Custom permissions map safety
- The `custom_permissions` merge is shallow: override keys replace base keys
- Unknown keys are preserved during merge (forward compatibility for plugins)
- Plugins should validate values they consume; the permission system treats them as opaque

#### Config injection resistance
- All config values go through serde deserialization with explicit types
- Numeric fields have proper types (u8, u32, usize, f32, f64) preventing overflow
- String fields are just strings; any glob matching happens in the router, not the resolver
- The resolver does not execute any code based on config values

#### Workspace ceiling enforcement (FIX-04)
- Workspace configs cannot expand permissions beyond global config for security-sensitive fields
- Security-sensitive fields with ceiling enforcement:
  - `level`: workspace cannot grant a higher level than global
  - `escalation_allowed`: workspace cannot enable escalation if global disables it
  - `tool_access`: workspace cannot add tools not present in global tool_access
  - `rate_limit`: workspace cannot set a higher rate limit than global (0=unlimited is highest)
  - `cost_budget_daily_usd`: workspace cannot set a higher daily budget (0.0=unlimited is highest)
  - `cost_budget_monthly_usd`: workspace cannot set a higher monthly budget (0.0=unlimited is highest)
  - `max_tier`: workspace cannot upgrade the tier beyond global's max_tier
- `enforce_workspace_ceiling()` is called after all merge layers, clamping to global ceiling
- `validate_workspace_ceiling()` provides static validation at config load time (Phase H)
- This prevents a compromised or misconfigured workspace from escalating privileges

### 4.3 Custom Permissions Map Handling

The `custom_permissions` field supports arbitrary key-value pairs for plugins:

- **During merge:** Override keys are inserted into the base map. Base keys not in the override are preserved. This is a shallow merge (values are not recursively merged).
- **During resolution:** All five layers contribute custom_permissions. A plugin setting a key at the global level can be overridden at the workspace, channel, or user level.
- **Unknown keys are preserved:** The resolver never drops custom_permissions keys, even if it does not understand them. This allows plugins to define their own permission dimensions.
- **Type safety:** Plugins must validate the `serde_json::Value` they read from custom_permissions. The resolver treats all values as opaque JSON.

Example:
```json
{
  "custom_permissions": {
    "max_file_size_bytes": 10485760,
    "allowed_mcp_servers": ["filesystem", "web-search"],
    "vision_enabled": true
  }
}
```

Plugin code:
```rust
let max_size = permissions.custom_permissions
    .get("max_file_size_bytes")
    .and_then(|v| v.as_u64())
    .unwrap_or(1_048_576); // 1 MB default if not configured
```

### 4.4 Performance Considerations

- Permission resolution is O(1) for each layer (HashMap lookups)
- The merge function is O(n) where n = number of fields (16 scalar + 2 maps)
- Full 5-layer resolution is O(5 * 16) = O(80) operations, which is negligible
- The resolver is not async; it can run synchronously on the hot path
- No allocations beyond cloning overridden string/vec fields

---

## 5. Completion

### 5.1 Exit Criteria

- [ ] `UserPermissions`, `AuthContext`, `PermissionLevelConfig` are imported from `clawft_types::routing` -- NOT redefined (FIX-01)
- [ ] No `PermissionOverrides` struct exists -- `PermissionLevelConfig` is used instead (FIX-01)
- [ ] Named constructors use `_defaults` suffix: `zero_trust_defaults()`, `user_defaults()`, `admin_defaults()` (FIX-01)
- [ ] Built-in defaults for levels 0, 1, 2 match the tables in `08-tiered-router.md` Section 2 exactly
- [ ] `defaults_for_level()` maps unknown levels to zero_trust (most restrictive) (FIX-01)
- [ ] `UserPermissions::merge()` correctly applies `PermissionLevelConfig` overrides with the specified semantics
- [ ] `PermissionResolver::new()` takes `&RoutingConfig` (not 6 separate HashMaps) (FIX-02)
- [ ] `PermissionResolver::resolve()` takes 3 params: `(sender_id, channel, allow_from_match)` (FIX-02)
- [ ] This is the ONLY permission resolution implementation -- no resolution in Phase C or F (FIX-02)
- [ ] `PermissionResolver::determine_level()` correctly prioritizes user > channel > allow_from_match > cli > zero_trust (note: determine_level picks the *level*; the *merge* ordering is channel > user per design doc Section 3.2)
- [ ] CLI channel defaults to admin (level 2) via resolver, NOT via AuthContext::default()
- [ ] `enforce_workspace_ceiling()` clamps workspace permissions to global ceiling (FIX-04)
- [ ] `validate_workspace_ceiling()` provides static validation for config load time (FIX-04)
- [ ] Unknown levels fall back to zero_trust
- [ ] All unit tests pass
- [ ] Code compiles without warnings (`cargo clippy`)
- [ ] File is under 500 lines

### 5.2 Test Plan

Minimum 22 unit tests covering the following scenarios:

#### Built-in Defaults Tests (3 tests)
1. **`test_zero_trust_defaults`**: Verify all 16 fields of `UserPermissions::zero_trust_defaults()` match the design doc Level 0 table.
2. **`test_user_defaults`**: Verify all 16 fields of `UserPermissions::user_defaults()` match the design doc Level 1 table.
3. **`test_admin_defaults`**: Verify all 16 fields of `UserPermissions::admin_defaults()` match the design doc Level 2 table.

#### Level Resolution Tests (5 tests)
4. **`test_determine_level_per_user_override`**: A sender_id present in `user_overrides` with explicit level gets that level, regardless of channel.
5. **`test_determine_level_per_channel_override`**: A channel with explicit level in `channel_overrides` assigns that level to unknown senders.
6. **`test_determine_level_allow_from_match`**: When `allow_from_match = true`, sender gets level 1 (user). When `false`, falls through to next rule.
7. **`test_determine_level_cli_default`**: Sender `"local"` on channel `"cli"` with `allow_from_match = false` gets admin (level 2).
8. **`test_determine_level_unknown_sender`**: An unknown sender on a non-CLI channel with `allow_from_match = false` and no overrides gets level 0 (zero_trust).

#### Merge Tests (4 tests)
9. **`test_merge_scalar_fields`**: Override `max_tier` and `rate_limit` via `PermissionLevelConfig`, verify both are updated, other fields unchanged.
10. **`test_merge_vec_fields_non_empty`**: Override `tool_access` with a non-empty vec, verify it replaces the base.
11. **`test_merge_vec_fields_empty_no_change`**: Override `tool_access` with an empty vec, verify the base is preserved.
12. **`test_merge_custom_permissions`**: Override with new custom keys, verify base keys are preserved and override keys are applied.

#### Full Resolution Tests (4 tests)
13. **`test_resolve_full_stack`**: Set up all 5 layers with overrides. Call `resolve(sender_id, channel, allow_from_match)` with 3 params. Verify the final result reflects correct precedence (channel > user > workspace > global > built-in, per design doc Section 3.2).
14. **`test_resolve_cli_always_admin`**: With no config overrides, `resolve("local", "cli", false)` resolves to full admin permissions.
15. **`test_resolve_zero_trust_enforcement`**: A zero_trust user cannot escalate, has no tools, has tight budgets, even when global config raises defaults.
16. **`test_resolve_per_user_budget_override`**: A user-level user with a per-user `cost_budget_daily_usd` override has the custom budget, not the level default.

#### Constructor Tests (1 test)
17. **`test_resolver_constructed_from_routing_config`**: Verify `PermissionResolver::new(&routing_config, None)` correctly extracts all config layers from `&RoutingConfig`. No 6-HashMap constructor.

#### Workspace Ceiling Tests (FIX-04) (3 tests)
18. **`test_workspace_ceiling_level_clamped`**: Workspace sets level higher than global; verify ceiling enforcement clamps it.
19. **`test_workspace_ceiling_tool_access_filtered`**: Workspace adds tools not in global tool_access; verify they are removed by ceiling enforcement.
20. **`test_workspace_ceiling_budget_clamped`**: Workspace sets higher cost_budget than global; verify it is clamped to global ceiling.

#### Edge Case Tests (2 tests)
21. **`test_unknown_level_falls_back_to_zero_trust`**: `defaults_for_level(99)` returns zero_trust defaults.
22. **`test_auth_context_default_is_zero_trust`**: `AuthContext::default()` has zero_trust permissions (NOT admin). Verify sender_id, channel, and permissions.level == 0.

### 5.3 Development Notes

**Design decisions to document:**
- (FIX-01) All types (`UserPermissions`, `AuthContext`, `PermissionLevelConfig`) are imported from `clawft_types::routing`. Phase B only adds `impl` methods and the `PermissionResolver` struct. No type redefinitions.
- (FIX-01) `AuthContext::default()` returns zero_trust, not CLI admin. This is a security-first design. CLI admin behavior is achieved through the resolver, not through Default.
- (FIX-01) The old `PermissionOverrides` struct is eliminated. `PermissionLevelConfig` from Phase A serves the same purpose (all-Option fields for partial overrides).
- (FIX-02) `PermissionResolver::new()` takes `&RoutingConfig` (and optional workspace config), not 6 separate HashMaps. This ensures a single source of truth and simplifies the API.
- (FIX-02) `resolve()` takes `allow_from_match: bool` as a third parameter. The channel plugin is responsible for checking allow_from membership; the resolver trusts this determination.
- (FIX-02) This is the ONLY permission resolution implementation. Phase C and Phase F must NOT re-implement resolution or merge logic.
- (FIX-04) Workspace ceiling enforcement ensures a workspace config cannot escalate privileges beyond the global config. This is enforced after all merge layers, before returning resolved permissions.
- `PermissionLevelConfig` uses `Option<T>` for all fields rather than a sentinel value. This cleanly separates "not configured" from "configured to a specific value" (e.g., `rate_limit: Some(0)` means "explicitly unlimited", while `rate_limit: None` means "use default").
- Empty vec in override means "no change." This was chosen over "clear the list" semantics because it is more common for users to omit a field than to explicitly empty it. Users who need to clear a list can set it to `["__none__"]` or similar sentinel, which is documented but not special-cased in the resolver.
- The resolver is synchronous (no async). Permission resolution is pure computation over in-memory config data. Making it async would add unnecessary complexity.
- The `custom_permissions` merge is shallow (key-level), not deep (recursive). Deep merge is avoided because plugin values may have complex schemas that the permission system cannot understand. Plugins are responsible for their own value semantics.
- The `level` field appears in both `UserPermissions` (resolved) and `PermissionLevelConfig` (config). In `PermissionLevelConfig`, it determines which built-in defaults to start with. In `UserPermissions`, it records the final resolved level for downstream code to check.

---

## Remediation Applied

**Date**: 2026-02-18
**Source**: remediation-plan.md

The following fixes from the remediation plan have been applied to this Phase B document:

### FIX-01: Canonical Type Ownership (CRITICAL)

Changes applied:
- Removed `PermissionOverrides` struct definition. All references replaced with Phase A's `PermissionLevelConfig` (all-Option fields) from `clawft_types::routing`.
- Named constructors confirmed with `_defaults` suffix: `zero_trust_defaults()`, `user_defaults()`, `admin_defaults()`.
- `AuthContext::default()` now documented as returning zero_trust (not CLI admin). Phase B does NOT redefine the `AuthContext` struct or its `Default` impl -- both are imported from `clawft_types::routing`.
- `UserPermissions` and `AuthContext` struct definitions removed from pseudocode (marked as reference-only). Phase B imports these from `clawft_types::routing` and adds only `impl` methods.
- `defaults_for_level()` confirmed: unknown levels map to zero_trust (most restrictive).
- All import paths updated from `clawft-types/src/config.rs` to `clawft-types/src/routing.rs`.
- Module wiring updated: `pipeline/mod.rs` only re-exports `PermissionResolver`, not Phase A types.

### FIX-02: Single Permission Resolution (CRITICAL)

Changes applied:
- `PermissionResolver::new()` constructor now takes `&RoutingConfig` (and optional `&RoutingConfig` for workspace), not 6 separate HashMap parameters. Config layer extraction happens internally.
- `resolve()` signature updated to 3 params: `(sender_id: &str, channel: &str, allow_from_match: bool) -> UserPermissions`. The `allow_from_match` parameter decouples allow_from checking from the resolver.
- `resolve_auth_context()` signature updated to match: 3 params.
- `determine_level()` updated to accept `allow_from_match: bool` instead of looking up `channel_allow_from` internally.
- Documentation added throughout emphasizing this is the ONLY permission resolution implementation. Phase C and Phase F must NOT re-implement.
- Pipeline flow diagram updated to show Phase F calling `resolve_auth_context(sender_id, channel, allow_from_match)`.

### FIX-04: Workspace Config Ceiling Enforcement (CRITICAL SECURITY)

Changes applied:
- Documentation added that `PermissionResolver` accepts both global and workspace configs separately via the `new(global_config, workspace_config)` constructor.
- `enforce_workspace_ceiling()` private method added: clamps resolved permissions to global ceiling for security-sensitive fields after all merge layers.
- Security-sensitive fields with ceiling enforcement documented: `level`, `escalation_allowed`, `tool_access`, `rate_limit`, `cost_budget_daily_usd`, `cost_budget_monthly_usd`, `max_tier`.
- `validate_workspace_ceiling()` public static method added for config-load-time validation (used by Phase H).
- `resolve_global_only()` private helper added for computing the global ceiling.
- Security considerations section updated with workspace ceiling documentation.
- 3 new unit tests added to test plan for workspace ceiling enforcement.
- Exit criteria updated to include ceiling enforcement verification.

### Additional Changes

- Test 19 (`test_auth_context_default_is_cli_admin`) renamed to `test_auth_context_default_is_zero_trust` and updated to verify zero_trust permissions.
- Test 6 updated to reflect `allow_from_match` parameter instead of allow_from list lookup.
- Test 17 added for constructor verification (`test_resolver_constructed_from_routing_config`).
- Total test count increased from 19 to 22 (3 new workspace ceiling tests).
- `test_level_name_roundtrip` test removed (covered by basic defaults tests; level_name/level_from_name are trivial).
