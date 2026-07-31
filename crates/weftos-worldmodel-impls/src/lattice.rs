//! Composed [`LatticeApi`] stub for service scaffolding (WEFT-524/527 later).

use alloc::vec::Vec;

use weftos_worldmodel_core::{
    Action, ActionPlan, Encoder, Latent, LatentPlanner, LatticeApi, NodeId, ObservationFrame,
    Predictor, RecallHit, SubscriptionId, WorldModelResult,
};

use crate::encoder::NullEncoder;
use crate::planner::CemPlanner;
use crate::predictor::LinearPredPhi;

/// Weight-free lattice that composes null encoder / linear `pred_φ` /
/// CEM planner (WEFT-529 defaults).
///
/// Holds last observed latent and subscription lists so service wiring can
/// be tested without candle or a mesh backend.
#[derive(Debug, Clone)]
pub struct StubLattice {
    /// Encoder used by [`LatticeApi::observe`].
    pub encoder: NullEncoder,
    /// Predictor used by [`LatticeApi::predict`] (`pred_φ`).
    pub predictor: LinearPredPhi,
    /// Planner used by [`LatticeApi::plan`] (CEM default).
    pub planner: CemPlanner,
    /// Last observed latent.
    pub last: Latent,
    /// Surprise subscription handles.
    pub surprise_subs: Vec<SubscriptionId>,
    /// Drift subscription handles.
    pub drift_subs: Vec<SubscriptionId>,
}

impl Default for StubLattice {
    fn default() -> Self {
        Self {
            encoder: NullEncoder,
            predictor: LinearPredPhi::default(),
            planner: CemPlanner::default(),
            last: weftos_worldmodel_core::zero_latent(),
            surprise_subs: Vec::new(),
            drift_subs: Vec::new(),
        }
    }
}

impl LatticeApi for StubLattice {
    fn observe(&mut self, frame: ObservationFrame<'_>) -> WorldModelResult<Latent> {
        frame.check_dim()?;
        self.last = self.encoder.encode(frame.bytes)?;
        Ok(self.last)
    }

    fn observe_node(
        &mut self,
        _node: NodeId,
        frame: ObservationFrame<'_>,
    ) -> WorldModelResult<Latent> {
        self.observe(frame)
    }

    fn predict(&self, z_t: &Latent, action: &Action) -> WorldModelResult<Latent> {
        self.predictor.predict(z_t, action)
    }

    fn plan(&self, z_t: &Latent, horizon: usize) -> WorldModelResult<ActionPlan> {
        self.planner.plan(z_t, horizon)
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
