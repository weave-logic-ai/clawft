//! WeftOS substrate state tree + `OntologyAdapter` contract.
//!
//! Implements [ADR-017](../../.planning/symposiums/compositional-ui/adrs/adr-017-ontology-adapter-contract.md).
//!
//! An **ontology adapter** is a stream *producer* that owns a data source
//! (the kernel daemon, `git`, GitHub, an LSP, …) and publishes its state
//! onto declared **topics** as a sequence of structured
//! [`StateDelta`]s. The [`Substrate`] state tree aggregates deltas from
//! every subscribed adapter into a flat `BTreeMap<path, Value>` that
//! surface composers read via [`OntologySnapshot`].
//!
//! M1.5 ships the `kernel` reference adapter (see [`kernel`]). Additional
//! adapters (`git`, `gh`, `workspace`, `fs`, `lsp`, `deployment`) are
//! scheduled for M1.6–M1.9 per Session 10 §7.
//!
//! # What this crate does NOT do (yet)
//! - No governance gating of [`OntologyAdapter::open`]. ADR-017 §3 calls
//!   for install-time permission intersection; M1.5 treats `permissions()`
//!   as advisory and expects the app-manifest layer (M1.5-A, TODO) to
//!   enforce it before calling `open`.
//! - No dynamic-lib adapter registration (ADR-017 §3 path 2 — deferred).
//! - No adapter-health topic (`substrate/meta/adapter/<id>/health`,
//!   ADR-017 §7) — stub in kernel adapter, surfaces as TODO.
//! - [`PermissionReq`] is a re-export of
//!   [`clawft_app::manifest::Permission`] (unified in M1.5-D).

#![deny(rust_2018_idioms)]
#![warn(missing_docs)]

pub mod adapter;
pub mod delta;
pub mod projection;
pub mod snapshot;

// Kernel adapter requires a real daemon RPC client and tokio's full
// runtime (time + net). WASM targets proxy RPC through postMessage
// and wire their substrate bindings through the webview host, so the
// kernel module is native-only.
#[cfg(not(target_arch = "wasm32"))]
pub mod kernel;

/// Host-local WiFi / ethernet / battery adapter. Reads `/sys/class/*`
/// directly — no daemon round-trip, no NetworkManager dependency.
/// Native-only; the wasm path is covered by the legacy-Snapshot
/// fallback in `clawft_gui_egui::live` (M1.6+ migrates this to a real
/// substrate-over-postMessage bridge).
#[cfg(not(target_arch = "wasm32"))]
pub mod network;

/// Host-local bluetooth adapter. Reads `/sys/class/bluetooth` +
/// `/sys/class/rfkill` directly — no bluez / bluetoothctl dependency.
/// Native-only for the same reason as [`network`].
#[cfg(not(target_arch = "wasm32"))]
pub mod bluetooth;

pub use adapter::{
    AdapterError, BufferPolicy, OntologyAdapter, PermissionReq, RefreshHint, Sensitivity, SubId,
    Subscription, TopicDecl,
};
pub use delta::StateDelta;
pub use snapshot::{OntologySnapshot, Substrate};
