//! `LatticeApi` — primary external surface (7 methods; WEFT-527).
//!
//! ServiceApi registration and process wiring land with
//! `clawft-worldmodel-service` (WEFT-524). This module only defines the
//! method surface and a null implementation for compile/tests.

use alloc::vec::Vec;

use crate::error::WorldModelResult;
use crate::latent::{zero_latent, Latent};
use crate::planner::{ActionPlan, PlanStep, PlannerKind};
use crate::types::{Action, NodeId, ObservationFrame, RecallHit, SubscriptionId};

/// Canonical LatticeApi method names (ADR-052 / research stream).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum LatticeMethod {
    /// Ingest a sensor/observation frame into the local latent state.
    Observe = 1,
    /// Ingest a frame attributed to a specific mesh node.
    ObserveNode = 2,
    /// One-step latent prediction under an action.
    Predict = 3,
    /// Multi-step latent plan (CEM / MPPI / gradient).
    Plan = 4,
    /// Nearest-neighbour recall in latent memory.
    Recall = 5,
    /// Subscribe to VoE surprise events.
    SubscribeSurprise = 6,
    /// Subscribe to SIGReg / temporal drift events (local-derivable).
    SubscribeDrift = 7,
}

impl LatticeMethod {
    /// Snake_case wire / ServiceApi name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::ObserveNode => "observe_node",
            Self::Predict => "predict",
            Self::Plan => "plan",
            Self::Recall => "recall",
            Self::SubscribeSurprise => "subscribe_surprise",
            Self::SubscribeDrift => "subscribe_drift",
        }
    }
}

/// All seven LatticeApi methods in stable order.
pub const LATTICE_METHODS: &[LatticeMethod] = &[
    LatticeMethod::Observe,
    LatticeMethod::ObserveNode,
    LatticeMethod::Predict,
    LatticeMethod::Plan,
    LatticeMethod::Recall,
    LatticeMethod::SubscribeSurprise,
    LatticeMethod::SubscribeDrift,
];

/// Count of LatticeApi methods (must stay 7 until a major API bump).
pub const LATTICE_METHOD_COUNT: usize = 7;

/// World-model lattice query / control surface.
///
/// Implementations live behind the optional WorldModelService. Callers
/// **must** treat `Unavailable` as a soft failure and continue on local ECC
/// (ADR-090 decoupling invariant).
pub trait LatticeApi {
    /// Ingest observation; return updated latent.
    fn observe(&mut self, frame: ObservationFrame<'_>) -> WorldModelResult<Latent>;

    /// Ingest observation for a remote/local node id.
    fn observe_node(
        &mut self,
        node: NodeId,
        frame: ObservationFrame<'_>,
    ) -> WorldModelResult<Latent>;

    /// Predict ẑ_{t+1} from z_t and action.
    fn predict(&self, z_t: &Latent, action: &Action) -> WorldModelResult<Latent>;

    /// Plan `horizon` steps from z_t.
    fn plan(&self, z_t: &Latent, horizon: usize) -> WorldModelResult<ActionPlan>;

    /// Recall up to `k` neighbours near `query`.
    fn recall(&self, query: &Latent, k: usize) -> WorldModelResult<Vec<RecallHit>>;

    /// Register interest in surprise stream (handle assigned by caller/service).
    fn subscribe_surprise(&mut self, handle: SubscriptionId) -> WorldModelResult<()>;

    /// Register interest in drift stream (local-derivable without cluster WM).
    fn subscribe_drift(&mut self, handle: SubscriptionId) -> WorldModelResult<()>;
}

/// Null lattice: zero latents, empty recall, accepts subscriptions.
#[derive(Debug, Clone)]
pub struct NullLattice {
    /// Last observed latent (always zero for null).
    pub last: Latent,
    /// Subscription handles accepted.
    pub surprise_subs: Vec<SubscriptionId>,
    /// Drift subscription handles accepted.
    pub drift_subs: Vec<SubscriptionId>,
}

impl Default for NullLattice {
    fn default() -> Self {
        Self {
            last: zero_latent(),
            surprise_subs: Vec::new(),
            drift_subs: Vec::new(),
        }
    }
}

impl LatticeApi for NullLattice {
    fn observe(&mut self, frame: ObservationFrame<'_>) -> WorldModelResult<Latent> {
        frame.check_dim()?;
        self.last = zero_latent();
        Ok(self.last)
    }

    fn observe_node(
        &mut self,
        _node: NodeId,
        frame: ObservationFrame<'_>,
    ) -> WorldModelResult<Latent> {
        self.observe(frame)
    }

    fn predict(&self, z_t: &Latent, _action: &Action) -> WorldModelResult<Latent> {
        Ok(*z_t)
    }

    fn plan(&self, z_t: &Latent, horizon: usize) -> WorldModelResult<ActionPlan> {
        let mut steps = Vec::with_capacity(horizon);
        for _ in 0..horizon {
            steps.push(PlanStep {
                action: Action::null(),
                z_hat: *z_t,
            });
        }
        Ok(ActionPlan {
            steps,
            kind: PlannerKind::Cem,
        })
    }

    fn recall(&self, _query: &Latent, _k: usize) -> WorldModelResult<Vec<RecallHit>> {
        Ok(Vec::new())
    }

    fn subscribe_surprise(&mut self, handle: SubscriptionId) -> WorldModelResult<()> {
        self.surprise_subs.push(handle);
        Ok(())
    }

    fn subscribe_drift(&mut self, handle: SubscriptionId) -> WorldModelResult<()> {
        self.drift_subs.push(handle);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_names_stable() {
        let names: alloc::vec::Vec<_> = LATTICE_METHODS.iter().map(|m| m.as_str()).collect();
        assert_eq!(
            names,
            [
                "observe",
                "observe_node",
                "predict",
                "plan",
                "recall",
                "subscribe_surprise",
                "subscribe_drift",
            ]
        );
    }
}
