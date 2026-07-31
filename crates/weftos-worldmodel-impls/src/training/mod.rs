//! LeWM training surfaces + RVF-hosted small models (WEFT-531 / WEFT-532).
//!
//! # Surfaces
//!
//! | Surface | Type | Role |
//! |---------|------|------|
//! | Offline edge | [`OfflineEdgePipeline`] | Per-sensor-class residual train → RVF segment → tick hot-swap |
//! | Online streaming-merge | [`StreamingMergePipeline`] | Importance-weighted replay + AND-gated promotion |
//!
//! # Small models (WEFT-532)
//!
//! [`SensorModelRegistry`] holds transmit-gate / aggregate / encode slots per
//! [`SensorClass`], stages RVF-delivered checkpoints, commits at tick
//! boundaries, and rolls back to last-good on AND-gate veto.
//!
//! Full neural optimisers are **stubbed**: residual mean updates over the
//! 192-d latent. Segment I/O, importance sampling, gate hooks, and swap
//! lifecycle are production-shaped and unit-tested.

mod offline_edge;
mod replay;
mod rvf_segment;
mod sensor_models;
mod streaming_merge;

pub use offline_edge::OfflineEdgePipeline;
pub use replay::{ImportanceReplayBuffer, DEFAULT_REPLAY_CAPACITY};
pub use rvf_segment::{
    decode_model_segment, encode_model_segment, model_segment_from_wire, model_segment_to_wire,
    LewmModelSegment, SEGMENT_HEADER_LEN,
};
pub use sensor_models::{SensorModelRegistry, SlotState, DEFAULT_BOOT_VERSION};
pub use streaming_merge::{require_promoted, StreamingMergePipeline};
