//! Substrate value viewers — a registry of shape-specific renderers
//! for the Explorer's right-hand detail pane.

pub trait SubstrateViewer {
    /// Return a priority > 0 if this viewer can render `value`.
    /// Higher priority wins. The JSON fallback returns 1.
    fn matches(value: &serde_json::Value) -> u32;

    fn paint(ui: &mut egui::Ui, path: &str, value: &serde_json::Value);
}

pub mod json_fallback;
// [[VIEWERS_MODULES_INSERT]]
pub mod audio_meter;
pub mod chain_tail;
pub mod connection_badge;
pub mod depth_map;
pub mod mesh_nodes;
pub mod time_series;
pub mod waveform;

/// Dispatch rendering of `value` at `path` to the highest-priority
/// matching viewer. Falls through to [`json_fallback::JsonFallbackViewer`]
/// which always matches.
///
/// `needless_return` is allowed below so the JSON-fallback branch keeps
/// the same `paint … ; return;` shape that every subsequent viewer
/// branch uses. That uniformity is what lets another worker drop their
/// viewer's `if matches { paint; return; }` block at the registration
/// marker without having to special-case the last arm.
#[allow(clippy::needless_return)]
pub fn dispatch(ui: &mut egui::Ui, path: &str, value: &serde_json::Value) {
    // [[VIEWERS_REGISTRATIONS_INSERT]]
    if waveform::WaveformViewer::matches(value) > 0 {
        waveform::WaveformViewer::paint(ui, path, value);
        return;
    }
    if mesh_nodes::MeshNodesViewer::matches(value) > 0 {
        mesh_nodes::MeshNodesViewer::paint(ui, path, value);
        return;
    }
    if chain_tail::ChainTailViewer::matches(value) > 0 {
        chain_tail::ChainTailViewer::paint(ui, path, value);
        return;
    }
    if audio_meter::AudioMeterViewer::matches(value) > 0 {
        audio_meter::AudioMeterViewer::paint(ui, path, value);
        return;
    }
    if connection_badge::ConnectionBadgeViewer::matches(value) > 0 {
        connection_badge::ConnectionBadgeViewer::paint(ui, path, value);
        return;
    }
    if depth_map::DepthMapViewer::matches(value) > 0 {
        depth_map::DepthMapViewer::paint(ui, path, value);
        return;
    }
    if time_series::TimeSeriesViewer::matches(value) > 0 {
        time_series::TimeSeriesViewer::paint(ui, path, value);
        return;
    }
    if json_fallback::JsonFallbackViewer::matches(value) > 0 {
        json_fallback::JsonFallbackViewer::paint(ui, path, value);
        return;
    }
}
