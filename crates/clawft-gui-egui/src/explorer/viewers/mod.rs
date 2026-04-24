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
    if json_fallback::JsonFallbackViewer::matches(value) > 0 {
        json_fallback::JsonFallbackViewer::paint(ui, path, value);
        return;
    }
}
