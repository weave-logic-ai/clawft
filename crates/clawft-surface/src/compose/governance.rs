//! Compose-time governance / GEPA policy contract (WEFT-277).
//!
//! ADR-006 rule 2 requires:
//!
//! ```text
//! affordances = raw_affordances ∩ governance.permit(caller, primitive, resource)
//! ```
//!
//! WEFT-430 landed the **session permit** half (manifest `influences` +
//! ADR-012 capture grants). This module is the **goal-aggregate /
//! GEPA** half (compositional-ui ADR-008):
//!
//! - `granted_affordances` / `denied_affordances` (deny always wins)
//! - optional `effect_ceiling` (5-D; child may only tighten)
//! - optional `surface_scope` (resource path prefix)
//! - GEPA mutate gate (`gepa_mutate_allowed`) for `surface.mutate` /
//!   `*.mutate` verbs when mutation axes are empty or frozen
//!
//! The contract is intentionally substrate-free: hosts build a
//! [`ComposeGovernance`] snapshot from their active goal (or the
//! chain-recorded `adhoc-scratch` default) and attach it to
//! [`crate::ComposePermits`]. Full Goal aggregate / chain events stay
//! on the kernel path; the composer only evaluates a frozen snapshot
//! so frame time stays pure and sync.

use std::collections::BTreeSet;

use crate::tree::{AffordanceDecl, SurfaceNode};

/// Normalize a WSP / fixture verb for permit + governance matching:
/// strip the optional `rpc.` dispatch prefix used by surface fixtures.
pub fn normalize_verb(verb: &str) -> String {
    verb.strip_prefix("rpc.")
        .unwrap_or(verb)
        .to_string()
}

/// 5-D effect ceiling (ADR-008 Constraints / ADR-034 effect algebra).
///
/// Values are 0.0–1.0. An affordance is denied when **any** dimension of
/// its estimated effect strictly exceeds the corresponding ceiling.
/// Missing ceiling (`None` on [`ComposeGovernance`]) means no
/// magnitude filter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectCeiling {
    pub risk: f64,
    pub fairness: f64,
    pub privacy: f64,
    pub novelty: f64,
    pub security: f64,
}

impl EffectCeiling {
    /// Fully open ceiling — every dimension at 1.0.
    pub const OPEN: Self = Self {
        risk: 1.0,
        fairness: 1.0,
        privacy: 1.0,
        novelty: 1.0,
        security: 1.0,
    };

    /// Fully closed ceiling — every dimension at 0.0 (only zero-effect
    /// verbs pass).
    pub const CLOSED: Self = Self {
        risk: 0.0,
        fairness: 0.0,
        privacy: 0.0,
        novelty: 0.0,
        security: 0.0,
    };

    /// True when `estimate` fits entirely under this ceiling.
    pub fn admits(&self, estimate: &EffectCeiling) -> bool {
        estimate.risk <= self.risk
            && estimate.fairness <= self.fairness
            && estimate.privacy <= self.privacy
            && estimate.novelty <= self.novelty
            && estimate.security <= self.security
    }
}

impl Default for EffectCeiling {
    fn default() -> Self {
        Self::OPEN
    }
}

/// Goal-aggregate / GEPA governance snapshot for ADR-006 rule 2.
///
/// Built once per compose frame (or session) from the active goal's
/// constraint fields. Default is the open `adhoc-scratch` policy:
/// no grant filter, empty deny set, no ceiling, mutate allowed,
/// no surface scope — so legacy hosts keep WEFT-430 behaviour until
/// they wire a real goal.
#[derive(Debug, Clone)]
pub struct ComposeGovernance {
    /// Active goal id (ADR-008 envelope). Informational at compose
    /// time; hosts should stamp the same id on dispatch envelopes.
    pub goal_id: String,
    /// When `Some`, only affordances whose **name** or normalised
    /// **verb** appear in the set are admitted (grant filter).
    /// `None` = open grants (default for `adhoc-scratch`).
    pub granted_affordances: Option<BTreeSet<String>>,
    /// Affordances denied by name or normalised verb. **Deny wins**
    /// over grants (ADR-008 invariant 1).
    pub denied_affordances: BTreeSet<String>,
    /// Optional effect ceiling. When set, estimated verb effects must
    /// fit under every dimension.
    pub effect_ceiling: Option<EffectCeiling>,
    /// When `false`, GEPA mutate verbs (`surface.mutate`, `*.mutate`,
    /// bare `mutate`) are hidden. Maps to empty / fully-frozen
    /// `mutation-axes` on the active goal surface.
    pub gepa_mutate_allowed: bool,
    /// Optional resource path prefix. When set, only nodes whose
    /// `path` equals or is under the prefix are allowed to expose
    /// write affordances (ADR-008 `surface_scope`).
    pub surface_scope: Option<String>,
}

impl Default for ComposeGovernance {
    fn default() -> Self {
        Self::open()
    }
}

impl ComposeGovernance {
    /// Open policy — `adhoc-scratch` default. No additional filtering
    /// beyond session permits (WEFT-430).
    pub fn open() -> Self {
        Self {
            goal_id: "adhoc-scratch".into(),
            granted_affordances: None,
            denied_affordances: BTreeSet::new(),
            effect_ceiling: None,
            gepa_mutate_allowed: true,
            surface_scope: None,
        }
    }

    /// Deny every affordance (empty grant set, no mutate).
    pub fn closed() -> Self {
        Self {
            goal_id: "adhoc-scratch".into(),
            granted_affordances: Some(BTreeSet::new()),
            denied_affordances: BTreeSet::new(),
            effect_ceiling: Some(EffectCeiling::CLOSED),
            gepa_mutate_allowed: false,
            surface_scope: None,
        }
    }

    /// Builder: attach a goal id.
    pub fn with_goal_id(mut self, goal_id: impl Into<String>) -> Self {
        self.goal_id = goal_id.into();
        self
    }

    /// Builder: explicit grant set (names and/or verbs).
    pub fn with_grants(mut self, items: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        self.granted_affordances = Some(normalize_set(items));
        self
    }

    /// Builder: explicit deny set (names and/or verbs). Deny wins.
    pub fn with_denies(mut self, items: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        self.denied_affordances = normalize_set(items);
        self
    }

    /// Builder: effect ceiling.
    pub fn with_effect_ceiling(mut self, ceiling: EffectCeiling) -> Self {
        self.effect_ceiling = Some(ceiling);
        self
    }

    /// Builder: freeze GEPA mutate verbs.
    pub fn with_gepa_mutate_allowed(mut self, allowed: bool) -> Self {
        self.gepa_mutate_allowed = allowed;
        self
    }

    /// Builder: surface path scope prefix.
    pub fn with_surface_scope(mut self, scope: impl Into<String>) -> Self {
        self.surface_scope = Some(scope.into());
        self
    }

    /// ADR-006 rule 2 / ADR-008 — whether this affordance is permitted
    /// under the current goal-aggregate snapshot for `node`.
    pub fn allows(&self, node: &SurfaceNode, aff: &AffordanceDecl) -> bool {
        let bare_verb = normalize_verb(&aff.verb);
        let name = aff.name.as_str();

        // 1. Deny wins (ADR-008 invariant 1).
        if set_matches(&self.denied_affordances, name, &bare_verb) {
            return false;
        }

        // 2. Grant filter (when present).
        if let Some(grants) = &self.granted_affordances
            && !set_matches(grants, name, &bare_verb)
        {
            return false;
        }

        // 3. Surface scope — write affordances only on in-scope nodes.
        if let Some(scope) = &self.surface_scope
            && !path_in_scope(&node.path, scope)
        {
            return false;
        }

        // 4. GEPA mutate gate.
        if !self.gepa_mutate_allowed && is_gepa_mutate_verb(&bare_verb) {
            return false;
        }

        // 5. Effect ceiling.
        if let Some(ceiling) = &self.effect_ceiling {
            let estimate = estimate_verb_effect(&bare_verb);
            if !ceiling.admits(&estimate) {
                return false;
            }
        }

        true
    }
}

fn normalize_set(items: impl IntoIterator<Item = impl AsRef<str>>) -> BTreeSet<String> {
    items
        .into_iter()
        .map(|s| {
            let s = s.as_ref();
            // Names stay as-is; verbs may carry rpc. — normalise both
            // forms so fixtures can list either.
            let stripped = normalize_verb(s);
            if stripped == s {
                s.to_string()
            } else {
                stripped
            }
        })
        .collect()
}

fn set_matches(set: &BTreeSet<String>, name: &str, bare_verb: &str) -> bool {
    set.contains(name) || set.contains(bare_verb)
}

fn path_in_scope(path: &str, scope: &str) -> bool {
    if path == scope {
        return true;
    }
    let scope = scope.trim_end_matches('/');
    path.starts_with(scope)
        && path
            .as_bytes()
            .get(scope.len())
            .copied()
            .is_some_and(|b| b == b'/')
}

/// GEPA experimental mutation verb (ADR-005 `mutate` / `surface.mutate`).
pub fn is_gepa_mutate_verb(bare_verb: &str) -> bool {
    let v = bare_verb.to_ascii_lowercase();
    v == "mutate" || v == "surface.mutate" || v.ends_with(".mutate")
}

/// Lightweight effect estimate for compose-time ceiling checks.
///
/// Deliberately independent of `clawft-core::agent::effects` so the
/// surface crate stays free of agent-loop deps. Heuristics mirror the
/// tool table enough for honesty gates; kernel double-gate still
/// re-evaluates with the full EffectVector at dispatch.
pub fn estimate_verb_effect(bare_verb: &str) -> EffectCeiling {
    let v = bare_verb.to_ascii_lowercase();

    // Capture / sensor — privacy + security.
    if v.contains("mic") || v.contains("camera") || v.contains("screen.capture") {
        return EffectCeiling {
            privacy: 0.8,
            security: 0.4,
            risk: 0.2,
            fairness: 0.0,
            novelty: 0.0,
        };
    }

    // Destructive kernel admin.
    if v.contains("kill") || v.contains("shutdown") || v.contains("wipe") {
        return EffectCeiling {
            risk: 0.7,
            security: 0.6,
            privacy: 0.0,
            fairness: 0.0,
            novelty: 0.1,
        };
    }

    // Service restart / mutate config — moderate security.
    if v.contains("restart") || v.contains("reload") || v.contains("write") {
        return EffectCeiling {
            risk: 0.3,
            security: 0.4,
            privacy: 0.0,
            fairness: 0.0,
            novelty: 0.1,
        };
    }

    // GEPA mutate — novelty + moderate risk.
    if is_gepa_mutate_verb(&v) {
        return EffectCeiling {
            novelty: 0.7,
            risk: 0.3,
            security: 0.2,
            privacy: 0.0,
            fairness: 0.0,
        };
    }

    // Dump / spy / core — privacy.
    if v.contains("dump") || v.contains("spy") || v.contains("exfil") {
        return EffectCeiling {
            privacy: 0.7,
            security: 0.5,
            risk: 0.2,
            fairness: 0.0,
            novelty: 0.0,
        };
    }

    // Default: near-zero (reads, UI chrome, soft actions).
    EffectCeiling {
        risk: 0.0,
        fairness: 0.0,
        privacy: 0.0,
        novelty: 0.0,
        security: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{AffordanceDecl, IdentityIri, SurfaceNode};

    fn aff(name: &str, verb: &str) -> AffordanceDecl {
        AffordanceDecl {
            name: name.to_string(),
            verb: verb.to_string(),
            invocations: vec![],
            args_schema: None,
        }
    }

    fn node_at(path: &str) -> SurfaceNode {
        SurfaceNode::new(IdentityIri::Pressable, path)
    }

    #[test]
    fn open_admits_everything() {
        let g = ComposeGovernance::open();
        let n = node_at("/root/x");
        assert!(g.allows(&n, &aff("kill", "kernel.kill-process")));
        assert!(g.allows(&n, &aff("mutate", "surface.mutate")));
    }

    #[test]
    fn closed_denies_everything() {
        let g = ComposeGovernance::closed();
        let n = node_at("/root/x");
        assert!(!g.allows(&n, &aff("kill", "kernel.kill-process")));
        assert!(!g.allows(&n, &aff("noop", "ui.noop")));
    }

    #[test]
    fn deny_wins_over_grant() {
        let g = ComposeGovernance::open()
            .with_grants(["kill", "restart", "kernel.kill-process", "kernel.restart-service"])
            .with_denies(["kill", "kernel.kill-process"]);
        let n = node_at("/root");
        assert!(!g.allows(&n, &aff("kill", "rpc.kernel.kill-process")));
        assert!(g.allows(&n, &aff("restart", "kernel.restart-service")));
    }

    #[test]
    fn grant_filter_by_name_or_verb() {
        let g = ComposeGovernance::open().with_grants(["kernel.restart-service"]);
        let n = node_at("/root");
        assert!(g.allows(&n, &aff("restart", "rpc.kernel.restart-service")));
        assert!(!g.allows(&n, &aff("kill", "kernel.kill-process")));
    }

    #[test]
    fn surface_scope_hides_out_of_tree() {
        let g = ComposeGovernance::open().with_surface_scope("/root/admin");
        let in_scope = node_at("/root/admin/procs");
        let out = node_at("/root/other");
        let a = aff("kill", "kernel.kill-process");
        assert!(g.allows(&in_scope, &a));
        assert!(!g.allows(&out, &a));
        // Exact match on scope root.
        assert!(g.allows(&node_at("/root/admin"), &a));
        // Sibling prefix must not match ("/root/admin-extra").
        assert!(!g.allows(&node_at("/root/admin-extra"), &a));
    }

    #[test]
    fn gepa_mutate_gate() {
        let g = ComposeGovernance::open().with_gepa_mutate_allowed(false);
        let n = node_at("/root");
        assert!(!g.allows(&n, &aff("evolve", "surface.mutate")));
        assert!(!g.allows(&n, &aff("evolve", "ui.layout.mutate")));
        assert!(g.allows(&n, &aff("kill", "kernel.kill-process")));
    }

    #[test]
    fn effect_ceiling_blocks_high_risk() {
        // Low risk ceiling: kill estimate (0.7) exceeds.
        let g = ComposeGovernance::open().with_effect_ceiling(EffectCeiling {
            risk: 0.5,
            fairness: 1.0,
            privacy: 1.0,
            novelty: 1.0,
            security: 1.0,
        });
        let n = node_at("/root");
        assert!(!g.allows(&n, &aff("kill", "kernel.kill-process")));
        // Soft / zero-effect verb still passes.
        assert!(g.allows(&n, &aff("refresh", "ui.refresh")));
    }

    #[test]
    fn effect_ceiling_open_admits_kill() {
        let g = ComposeGovernance::open().with_effect_ceiling(EffectCeiling::OPEN);
        let n = node_at("/root");
        assert!(g.allows(&n, &aff("kill", "kernel.kill-process")));
    }

    #[test]
    fn path_scope_helpers() {
        assert!(path_in_scope("/a/b", "/a"));
        assert!(path_in_scope("/a", "/a"));
        assert!(path_in_scope("/a/b", "/a/"));
        assert!(!path_in_scope("/ab", "/a"));
        assert!(!path_in_scope("/a", "/a/b"));
    }

    #[test]
    fn is_mutate_verb_detects_forms() {
        assert!(is_gepa_mutate_verb("mutate"));
        assert!(is_gepa_mutate_verb("surface.mutate"));
        assert!(is_gepa_mutate_verb("ui.layout.mutate"));
        assert!(!is_gepa_mutate_verb("kernel.kill-process"));
        assert!(!is_gepa_mutate_verb("mutation-report"));
    }
}
