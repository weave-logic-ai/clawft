use serde_json::Value;

pub trait SubstrateViewer {
    /// Return a priority > 0 if this viewer can render `value`.
    /// Higher priority wins. The JSON fallback returns 1.
    fn matches(value: &serde_json::Value) -> u32;

    fn paint(ui: &mut egui::Ui, path: &str, value: &serde_json::Value);
}

// [[VIEWERS_MODULES_INSERT]]
pub mod audio_meter;
pub mod connection_badge;
pub mod depth_map;
// [[/VIEWERS_MODULES_INSERT]]

pub fn dispatch(ui: &mut egui::Ui, path: &str, value: &Value) {
    // [[VIEWERS_REGISTRATIONS_INSERT]]
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
    // [[/VIEWERS_REGISTRATIONS_INSERT]]
    // JsonFallbackViewer lives in the panel worker's branch and will be
    // spliced in here on merge after the registrations block above.
    let _ = (ui, path, value);
}
