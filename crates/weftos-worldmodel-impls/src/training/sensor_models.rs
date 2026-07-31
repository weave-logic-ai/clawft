//! Per-sensor-class RVF-hosted small models with tick-aligned hot-swap
//! and AND-gate rollback (WEFT-532).

extern crate alloc;

use alloc::vec::Vec;

use weftos_worldmodel_core::{
    GateVerdict, HotSwapOutcome, ModelCheckpoint, ModelHotSwap, SensorClass, SmallModelKind,
    WorldModelError, WorldModelResult,
};

/// Default version tag for an empty / boot slot.
pub const DEFAULT_BOOT_VERSION: u64 = 0;

/// State of one (sensor class × model kind) slot.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotState {
    /// Sensor class.
    pub sensor_class: SensorClass,
    /// Model kind (transmit-gate / aggregate / encode).
    pub kind: SmallModelKind,
    /// Currently serving checkpoint.
    pub active: Option<ModelCheckpoint>,
    /// Staged for commit at `target_tick`.
    pub pending: Option<ModelCheckpoint>,
    /// Last successfully serving checkpoint (rollback target).
    pub last_good: Option<ModelCheckpoint>,
    /// Successful commits.
    pub commit_count: u64,
    /// Rollbacks performed.
    pub rollback_count: u64,
}

impl SlotState {
    fn new(sensor_class: SensorClass, kind: SmallModelKind) -> Self {
        Self {
            sensor_class,
            kind,
            active: None,
            pending: None,
            last_good: None,
            commit_count: 0,
            rollback_count: 0,
        }
    }
}

/// Registry of three small models × sensor classes with tick hot-swap.
#[derive(Debug, Clone)]
pub struct SensorModelRegistry {
    slots: Vec<SlotState>,
    /// Current mesh / DEMOCRITUS tick (host updates via commit).
    pub current_tick: u64,
    /// Total stage operations.
    pub stage_count: u64,
    /// Total successful commits.
    pub commit_count: u64,
    /// Total rollbacks.
    pub rollback_count: u64,
}

impl Default for SensorModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SensorModelRegistry {
    /// Allocate slots for every [`SensorClass`] × [`SmallModelKind`].
    pub fn new() -> Self {
        let mut slots = Vec::with_capacity(SensorClass::ALL.len() * SmallModelKind::ALL.len());
        for &class in &SensorClass::ALL {
            for &kind in &SmallModelKind::ALL {
                slots.push(SlotState::new(class, kind));
            }
        }
        Self {
            slots,
            current_tick: 0,
            stage_count: 0,
            commit_count: 0,
            rollback_count: 0,
        }
    }

    fn index(class: SensorClass, kind: SmallModelKind) -> Option<usize> {
        let ci = SensorClass::ALL.iter().position(|&c| c == class)?;
        let ki = SmallModelKind::ALL.iter().position(|&k| k == kind)?;
        Some(ci * SmallModelKind::ALL.len() + ki)
    }

    fn slot_mut(&mut self, class: SensorClass, kind: SmallModelKind) -> Option<&mut SlotState> {
        let i = Self::index(class, kind)?;
        self.slots.get_mut(i)
    }

    fn slot(&self, class: SensorClass, kind: SmallModelKind) -> Option<&SlotState> {
        let i = Self::index(class, kind)?;
        self.slots.get(i)
    }

    /// Iterate all slots.
    pub fn slots(&self) -> &[SlotState] {
        &self.slots
    }

    /// Stage from raw RVF wire bytes (decode + stage).
    pub fn stage_wire(
        &mut self,
        wire: &[u8],
        surface: weftos_worldmodel_core::TrainingSurfaceKind,
        target_tick: Option<u64>,
    ) -> WorldModelResult<()> {
        let seg = super::rvf_segment::model_segment_from_wire(wire)?;
        let mut cp = super::rvf_segment::decode_model_segment(&seg, surface, target_tick)?;
        if cp.target_tick.is_none() {
            // Commit on the next tick by default.
            cp.target_tick = Some(self.current_tick.saturating_add(1));
        }
        self.stage(cp)
    }

    /// Apply AND-gate verdict: if `!promote`, rollback all slots that have
    /// pending or recently committed online checkpoints for `class`.
    ///
    /// Hosts call this after streaming-merge train steps or offline swap
    /// when the four-condition gate vetoes.
    pub fn apply_gate_verdict(
        &mut self,
        class: SensorClass,
        kind: SmallModelKind,
        gate: GateVerdict,
    ) -> WorldModelResult<HotSwapOutcome> {
        if gate.promote {
            Ok(HotSwapOutcome::Idle)
        } else {
            self.rollback(class, kind)
        }
    }

    /// Full offline edge cycle: train → RVF wire → stage → commit at tick.
    ///
    /// Convenience for integration tests / service scaffolding.
    pub fn deliver_offline_checkpoint(
        &mut self,
        cp: ModelCheckpoint,
        commit_tick: u64,
    ) -> WorldModelResult<HotSwapOutcome> {
        let mut cp = cp;
        cp.target_tick = Some(commit_tick);
        self.stage(cp)?;
        let outcomes = self.commit_at_tick(commit_tick)?;
        for (_, _, o) in outcomes {
            if matches!(o, HotSwapOutcome::Committed { .. }) {
                return Ok(o);
            }
        }
        Ok(HotSwapOutcome::Idle)
    }
}

impl ModelHotSwap for SensorModelRegistry {
    fn stage(&mut self, checkpoint: ModelCheckpoint) -> WorldModelResult<()> {
        let class = checkpoint.sensor_class;
        let kind = checkpoint.kind;
        let slot = self
            .slot_mut(class, kind)
            .ok_or(WorldModelError::HotSwapFailed)?;
        slot.pending = Some(checkpoint);
        self.stage_count = self.stage_count.saturating_add(1);
        Ok(())
    }

    fn commit_at_tick(
        &mut self,
        tick: u64,
    ) -> WorldModelResult<Vec<(SensorClass, SmallModelKind, HotSwapOutcome)>> {
        self.current_tick = tick;
        let mut out = Vec::new();
        for slot in &mut self.slots {
            let Some(pending) = slot.pending.as_ref() else {
                continue;
            };
            let target = pending.target_tick.unwrap_or(tick);
            if tick < target {
                out.push((
                    slot.sensor_class,
                    slot.kind,
                    HotSwapOutcome::Deferred {
                        target_tick: target,
                    },
                ));
                continue;
            }
            // Commit: active → last_good, pending → active.
            if let Some(active) = slot.active.take() {
                slot.last_good = Some(active);
            }
            let committed = slot.pending.take().expect("pending checked");
            let version_tag = committed.version_tag;
            slot.active = Some(committed);
            slot.commit_count = slot.commit_count.saturating_add(1);
            self.commit_count = self.commit_count.saturating_add(1);
            out.push((
                slot.sensor_class,
                slot.kind,
                HotSwapOutcome::Committed { version_tag },
            ));
        }
        Ok(out)
    }

    fn rollback(
        &mut self,
        class: SensorClass,
        kind: SmallModelKind,
    ) -> WorldModelResult<HotSwapOutcome> {
        let slot = self
            .slot_mut(class, kind)
            .ok_or(WorldModelError::HotSwapFailed)?;
        // Drop pending (failed candidate).
        slot.pending = None;
        let Some(good) = slot.last_good.clone() else {
            // No last-good: clear active if present and report idle-style rollback.
            if slot.active.is_some() {
                slot.active = None;
                slot.rollback_count = slot.rollback_count.saturating_add(1);
                self.rollback_count = self.rollback_count.saturating_add(1);
                return Ok(HotSwapOutcome::RolledBack {
                    version_tag: DEFAULT_BOOT_VERSION,
                });
            }
            return Ok(HotSwapOutcome::Idle);
        };
        let version_tag = good.version_tag;
        slot.active = Some(good);
        slot.rollback_count = slot.rollback_count.saturating_add(1);
        self.rollback_count = self.rollback_count.saturating_add(1);
        Ok(HotSwapOutcome::RolledBack { version_tag })
    }

    fn active(&self, class: SensorClass, kind: SmallModelKind) -> Option<&ModelCheckpoint> {
        self.slot(class, kind).and_then(|s| s.active.as_ref())
    }

    fn last_good(&self, class: SensorClass, kind: SmallModelKind) -> Option<&ModelCheckpoint> {
        self.slot(class, kind).and_then(|s| s.last_good.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use weftos_worldmodel_core::{
        zero_latent, GateVerdict, ModelHotSwap, OfflineEdgeTrainer, RollbackGateMetrics,
        TrainingSample, TrainingSurfaceKind,
    };

    use crate::training::{
        encode_model_segment, model_segment_to_wire, OfflineEdgePipeline, StreamingMergePipeline,
    };
    use weftos_worldmodel_core::StreamingMergeTrainer;

    fn mk_cp(class: SensorClass, kind: SmallModelKind, ver: u64, tick: u64) -> ModelCheckpoint {
        ModelCheckpoint::new(
            class,
            kind,
            ver,
            TrainingSurfaceKind::OfflineEdge,
            alloc::vec![ver as u8, 1, 2, 3],
            Some(tick),
        )
    }

    #[test]
    fn tick_aligned_hot_swap_commit() {
        let mut reg = SensorModelRegistry::new();
        reg.stage(mk_cp(
            SensorClass::Imu,
            SmallModelKind::TransmitGate,
            1,
            5,
        ))
        .unwrap();
        // Too early.
        let early = reg.commit_at_tick(3).unwrap();
        assert!(early.iter().any(|(_, _, o)| matches!(
            o,
            HotSwapOutcome::Deferred { target_tick: 5 }
        )));
        assert!(reg
            .active(SensorClass::Imu, SmallModelKind::TransmitGate)
            .is_none());

        let committed = reg.commit_at_tick(5).unwrap();
        assert!(committed.iter().any(|(_, _, o)| matches!(
            o,
            HotSwapOutcome::Committed { version_tag: 1 }
        )));
        assert_eq!(
            reg.active(SensorClass::Imu, SmallModelKind::TransmitGate)
                .unwrap()
                .version_tag,
            1
        );
    }

    #[test]
    fn swap_and_rollback_cycle() {
        let mut reg = SensorModelRegistry::new();
        // Install v1.
        reg.stage(mk_cp(SensorClass::Audio, SmallModelKind::Encode, 1, 1))
            .unwrap();
        reg.commit_at_tick(1).unwrap();
        // Stage v2 and commit.
        reg.stage(mk_cp(SensorClass::Audio, SmallModelKind::Encode, 2, 2))
            .unwrap();
        reg.commit_at_tick(2).unwrap();
        assert_eq!(
            reg.active(SensorClass::Audio, SmallModelKind::Encode)
                .unwrap()
                .version_tag,
            2
        );
        assert_eq!(
            reg.last_good(SensorClass::Audio, SmallModelKind::Encode)
                .unwrap()
                .version_tag,
            1
        );

        // AND-gate veto → rollback to v1.
        let veto = GateVerdict::evaluate(RollbackGateMetrics {
            sigreg_health: 0.1,
            held_out_probe_accuracy: 1.0,
            voe_surprise_diff: 1.0,
            temporal_straightening: 1.0,
        });
        assert!(!veto.promote);
        let outcome = reg
            .apply_gate_verdict(SensorClass::Audio, SmallModelKind::Encode, veto)
            .unwrap();
        assert_eq!(
            outcome,
            HotSwapOutcome::RolledBack { version_tag: 1 }
        );
        assert_eq!(
            reg.active(SensorClass::Audio, SmallModelKind::Encode)
                .unwrap()
                .version_tag,
            1
        );
        assert!(reg.rollback_count >= 1);
    }

    #[test]
    fn offline_edge_rvf_delivery_to_hot_swap() {
        let mut edge = OfflineEdgePipeline::new();
        for t in 0..4 {
            let mut z0 = zero_latent();
            let mut z1 = zero_latent();
            z0[0] = t as f32;
            z1[0] = t as f32 + 0.5;
            edge.push_sample(&TrainingSample::basic(
                SensorClass::Lidar,
                z0,
                z1,
                t * 100,
            ))
            .unwrap();
        }
        let cp = edge
            .train_cycle(SensorClass::Lidar, SmallModelKind::Aggregate, 7)
            .expect("train");
        let wire = model_segment_to_wire(&encode_model_segment(&cp));
        assert_eq!(wire[0], SmallModelKind::Aggregate as u8);

        let mut reg = SensorModelRegistry::new();
        reg.stage_wire(&wire, TrainingSurfaceKind::OfflineEdge, Some(7))
            .unwrap();
        let outs = reg.commit_at_tick(7).unwrap();
        assert!(outs.iter().any(|(_, k, o)| {
            *k == SmallModelKind::Aggregate
                && matches!(o, HotSwapOutcome::Committed { .. })
        }));
        assert!(reg
            .active(SensorClass::Lidar, SmallModelKind::Aggregate)
            .is_some());
    }

    #[test]
    fn streaming_merge_promote_then_stage() {
        let mut stream = StreamingMergePipeline::new();
        stream.gate.min_samples = 2;
        for t in 0..8u64 {
            let mut z0 = zero_latent();
            let mut z1 = zero_latent();
            z0[0] = t as f32;
            z1[0] = (t + 1) as f32;
            let mut s = TrainingSample::basic(SensorClass::RgbD, z0, z1, t);
            s.importance = 1.0;
            stream.push_sample(&s).unwrap();
        }
        stream.gate.set_sigreg_health(0.99);
        let result = stream
            .train_step(SensorClass::RgbD, SmallModelKind::Encode)
            .unwrap();
        assert!(result.promoted, "gate={:?}", result.gate);

        let mut reg = SensorModelRegistry::new();
        // Online checkpoint: stage for next tick.
        let mut cp = result.checkpoint;
        cp.target_tick = Some(3);
        reg.stage(cp).unwrap();
        reg.commit_at_tick(3).unwrap();
        assert_eq!(
            reg.active(SensorClass::RgbD, SmallModelKind::Encode)
                .unwrap()
                .surface,
            TrainingSurfaceKind::OnlineStreamingMerge
        );
    }

    #[test]
    fn three_model_kinds_independent_slots() {
        let mut reg = SensorModelRegistry::new();
        for (i, kind) in SmallModelKind::ALL.iter().enumerate() {
            reg.stage(mk_cp(SensorClass::Proprio, *kind, (i as u64) + 1, 1))
                .unwrap();
        }
        reg.commit_at_tick(1).unwrap();
        for (i, kind) in SmallModelKind::ALL.iter().enumerate() {
            assert_eq!(
                reg.active(SensorClass::Proprio, *kind)
                    .unwrap()
                    .version_tag,
                (i as u64) + 1
            );
        }
    }
}
