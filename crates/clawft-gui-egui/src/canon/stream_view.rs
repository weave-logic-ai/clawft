//! `ui://stream-view` — live-tailing scrollable view. ADR-001 row 10.
//!
//! Follows the ring-buffer pattern from `blocks/terminal.rs` — uses
//! `egui::ScrollArea::vertical().stick_to_bottom(true)` to keep the
//! newest line visible while the caller owns the line buffer.

use std::borrow::Cow;

use eframe::egui;

use super::response::CanonResponse;
use super::types::{Affordance, Confidence, IdentityUri, MutationAxis, Tooltip, VariantId};
use super::CanonWidget;

const IDENTITY: &str = "ui://stream-view";

static AFFORDANCES: &[Affordance] = &[
    Affordance {
        name: Cow::Borrowed("scroll-to-bottom"),
        verb: Cow::Borrowed("wsp.update"),
        actors: &[],
        args_schema: None,
        reorderable: false,
    },
    Affordance {
        name: Cow::Borrowed("subscribe"),
        verb: Cow::Borrowed("wsp.subscribe"),
        actors: &[],
        args_schema: None,
        reorderable: false,
    },
    Affordance {
        name: Cow::Borrowed("unsubscribe"),
        verb: Cow::Borrowed("wsp.unsubscribe"),
        actors: &[],
        args_schema: None,
        reorderable: false,
    },
];

static MUTATION_AXES: &[MutationAxis] = &[
    MutationAxis::new("max-lines"),
    MutationAxis::new("follow-tail"),
    MutationAxis::new("wrap"),
    MutationAxis::new("timestamp-format"),
];

/// Live-tailing scrollable view. Generic over the element type so any
/// `AsRef<str>` buffer (strings, `Cow`, monomorphic line structs with a
/// display projection) can be rendered without conversion.
pub struct StreamView<'a, T: AsRef<str>> {
    id_source: egui::Id,
    lines: &'a [T],
    min_height: f32,
    max_height: Option<f32>,
    stick_to_bottom: bool,
    monospace: bool,
    tooltip: Option<Tooltip>,
    variant: VariantId,
}

impl<'a, T: AsRef<str>> StreamView<'a, T> {
    pub fn new(id_source: impl std::hash::Hash) -> Self {
        Self {
            id_source: egui::Id::new(("canon.stream_view", id_source)),
            lines: &[],
            min_height: 120.0,
            max_height: None,
            stick_to_bottom: true,
            monospace: true,
            tooltip: None,
            variant: 0,
        }
    }

    pub fn lines(mut self, lines: &'a [T]) -> Self {
        self.lines = lines;
        self
    }

    pub fn min_height(mut self, h: f32) -> Self {
        self.min_height = h;
        self
    }

    pub fn max_height(mut self, h: f32) -> Self {
        self.max_height = Some(h);
        self
    }

    pub fn stick_to_bottom(mut self, stick: bool) -> Self {
        self.stick_to_bottom = stick;
        self
    }

    pub fn monospace(mut self, monospace: bool) -> Self {
        self.monospace = monospace;
        self
    }

    pub fn tooltip(mut self, text: impl Into<Tooltip>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    pub fn variant(mut self, variant: VariantId) -> Self {
        self.variant = variant;
        self
    }
}

impl<'a, T: AsRef<str>> CanonWidget for StreamView<'a, T> {
    fn id(&self) -> egui::Id {
        self.id_source
    }

    fn identity_uri(&self) -> IdentityUri {
        Cow::Borrowed(IDENTITY)
    }

    fn affordances(&self) -> &[Affordance] {
        AFFORDANCES
    }

    fn confidence(&self) -> Confidence {
        Confidence::deterministic()
    }

    fn variant_id(&self) -> VariantId {
        self.variant
    }

    fn mutation_axes(&self) -> &[MutationAxis] {
        MUTATION_AXES
    }

    fn tooltip(&self) -> Option<&Tooltip> {
        self.tooltip.as_ref()
    }

    fn show(self, ui: &mut egui::Ui) -> CanonResponse {
        let id = self.id_source;
        let variant = self.variant;
        let tooltip = self.tooltip.clone();
        let min_height = self.min_height;
        let max_height = self.max_height;
        let stick = self.stick_to_bottom;
        let monospace = self.monospace;
        let lines = self.lines;

        let frame = egui::Frame::none()
            .fill(egui::Color32::from_rgb(8, 10, 14))
            .rounding(4.0)
            .inner_margin(egui::Margin::symmetric(10.0, 8.0));

        let inner = frame.show(ui, |ui| {
            ui.set_min_height(min_height);
            let mut area = egui::ScrollArea::vertical().stick_to_bottom(stick);
            if let Some(h) = max_height {
                area = area.max_height(h);
            }
            area.show(ui, |ui| {
                for line in lines {
                    let rich = if monospace {
                        egui::RichText::new(line.as_ref()).monospace()
                    } else {
                        egui::RichText::new(line.as_ref())
                    };
                    ui.label(rich);
                }
            });
        });

        let mut resp = inner.response;
        if let Some(tt) = &tooltip {
            resp = resp.on_hover_text(tt.as_ref());
        }

        // We cannot observe the scroll-to-bottom click from a plain
        // ScrollArea; mark no chosen affordance this frame. A future
        // revision that surfaces scroll state through egui::Memory can
        // flip this to "scroll-to-bottom" when tail is re-locked.
        let chosen: Option<&'static str> = None;

        CanonResponse::from_egui(resp, Cow::Borrowed(IDENTITY), variant, chosen).with_id_hint(id)
    }
}
