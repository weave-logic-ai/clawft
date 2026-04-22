//! `ui://select` — closed-choice picker primitive (ADR-001 row 5).
//!
//! Session-5 §5 maps this to `egui::ComboBox` for ≤N and `RadioButton`
//! for small N; the first-pass shipping here is the ComboBox form
//! against a static `&[&'static str]` option slice (matches the task
//! spec). `TableBuilder`-based large-set form is a future extension
//! kept inside `ui://table`.

use std::borrow::Cow;

use eframe::egui;

use super::response::CanonResponse;
use super::types::{Affordance, Confidence, IdentityUri, MutationAxis, Tooltip, VariantId};
use super::CanonWidget;

const IDENTITY: &str = "ui://select";

static AFFORDANCES_ACTIVE: &[Affordance] = &[Affordance {
    name: Cow::Borrowed("select"),
    verb: Cow::Borrowed("wsp.set"),
    actors: &[],
    args_schema: None,
    reorderable: false,
}];
static AFFORDANCES_DISABLED: &[Affordance] = &[];

static MUTATION_AXES: &[MutationAxis] = &[
    MutationAxis::new("options-order"),
    MutationAxis::new("default-choice"),
];

/// Closed-choice picker over a static option slice. The caller owns
/// the `&mut usize` index binding.
pub struct Select<'b> {
    id_source: egui::Id,
    label: Cow<'static, str>,
    options: &'static [&'static str],
    selected: &'b mut usize,
    enabled: bool,
    tooltip: Option<Tooltip>,
    variant: VariantId,
}

impl<'b> Select<'b> {
    pub fn new(
        id_source: impl std::hash::Hash,
        label: impl Into<Cow<'static, str>>,
        options: &'static [&'static str],
        selected: &'b mut usize,
    ) -> Self {
        Self {
            id_source: egui::Id::new(("canon.select", id_source)),
            label: label.into(),
            options,
            selected,
            enabled: true,
            tooltip: None,
            variant: 0,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
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

impl CanonWidget for Select<'_> {
    fn id(&self) -> egui::Id {
        self.id_source
    }

    fn identity_uri(&self) -> IdentityUri {
        Cow::Borrowed(IDENTITY)
    }

    fn affordances(&self) -> &[Affordance] {
        if self.enabled {
            AFFORDANCES_ACTIVE
        } else {
            AFFORDANCES_DISABLED
        }
    }

    fn confidence(&self) -> Confidence {
        Confidence::input()
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
        let enabled = self.enabled;
        let tooltip = self.tooltip.clone();
        let options = self.options;
        let selected = self.selected;
        let label = self.label;

        let current_text: &str = options
            .get(*selected)
            .copied()
            .unwrap_or_else(|| if options.is_empty() { "" } else { options[0] });

        let mut changed = false;
        let mut resp = ui
            .scope(|ui| {
                if !enabled {
                    ui.disable();
                }
                let combo_resp = egui::ComboBox::from_id_salt(id)
                    .selected_text(current_text)
                    .show_ui(ui, |ui| {
                        for (i, opt) in options.iter().enumerate() {
                            if ui.selectable_label(*selected == i, *opt).clicked()
                                && *selected != i
                            {
                                *selected = i;
                                changed = true;
                            }
                        }
                    });
                // Attach the static label next to the combo so the
                // primitive has a human-readable anchor.
                if !label.is_empty() {
                    ui.label(label.as_ref());
                }
                combo_resp.response
            })
            .inner;

        if let Some(tt) = &tooltip {
            resp = resp.on_hover_text(tt.as_ref());
        }

        let chosen: Option<&'static str> =
            if enabled && changed { Some("select") } else { None };

        CanonResponse::from_egui(resp, Cow::Borrowed(IDENTITY), variant, chosen).with_id_hint(id)
    }
}
