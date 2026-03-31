//! Permission engine -- K1 deferred, stub only in K0.
//!
//! ## K1 scope
//! - ACL-based permission checks with delegation shortcut
//! - EffectiveAclCache for fast repeated checks
//! - Integration with CapabilityChecker

use serde::{Deserialize, Serialize};

use crate::model::{Action, ResourceId, Role};

/// Permission decision.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    Allow,
    Deny,
    Delegate,
}

/// Check whether an agent with the given role may perform an action on a resource.
///
/// **K0 stub**: Always returns `Allow`. Real implementation in K1.
pub fn check(
    _agent_id: &str,
    _role: &Role,
    _action: &Action,
    _resource: &ResourceId,
) -> Decision {
    Decision::Allow
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k0_stub_always_allows() {
        let decision = check(
            "agent-001",
            &Role::Viewer,
            &Action::Admin,
            &ResourceId::new("/kernel"),
        );
        assert_eq!(decision, Decision::Allow);
    }

    #[test]
    fn decision_serde_roundtrip() {
        for decision in [Decision::Allow, Decision::Deny, Decision::Delegate] {
            let json = serde_json::to_string(&decision).unwrap();
            let back: Decision = serde_json::from_str(&json).unwrap();
            assert_eq!(back, decision);
        }
    }
}
