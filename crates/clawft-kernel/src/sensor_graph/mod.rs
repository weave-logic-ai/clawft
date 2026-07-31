//! Spatio-temporal GNN for distributed sonobuoy / sensor arrays (KG-013 / WEFT-354).
//!
//! K-STEMIT-style dual-branch architecture adapted from ice-penetrating radar
//! (arXiv:2604.09922) to underwater acoustic sensing:
//!
//! - **Spatial**: GraphSAGE on a buoy-position graph with inverse-distance
//!   (and optional propagation-delay) edge weights — learned beamforming
//!   over irregular arrays.
//! - **Temporal**: Gated linear unit (GLU) convolution over per-buoy acoustic
//!   time series — learned matched filtering for transients.
//! - **Physics prior**: ocean acoustic node features (SSP, thermocline, sea
//!   state, current, bottom type) projected into the same hidden space.
//! - **Adaptive fusion**: learnable `α ∈ [0,1]` balances spatial vs temporal;
//!   physics residual is added with a fixed scale (or learnable γ).
//! - **Classification head**: linear → logits for detection / species-style labels.
//!
//! Pure Rust, no GPU / no ndarray. Behind the `sensor` feature flag.
//!
//! Design note: [`docs/plans/weft-354-k-stemit.md`](../../../../docs/plans/weft-354-k-stemit.md).

mod fixture;
mod graph;
mod layers;
mod model;
mod train;

pub use fixture::{
    SyntheticConfig, SyntheticDataset, SyntheticSample, class_name, generate_synthetic_array,
};
pub use graph::{SensorEdge, SensorGraph, SensorNode};
pub use layers::{GraphSageLayer, Linear, TemporalGlu};
pub use model::{ForwardOutput, KstemitConfig, KstemitModel};
pub use train::{EvalMetrics, TrainConfig, TrainResult, evaluate, train_sgd};
