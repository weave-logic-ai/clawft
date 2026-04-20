//! `ui://field` — typed input binding primitive (ADR-001 row 3).
//!
//! Session-5 §3 lists five candidate egui surfaces (`TextEdit`,
//! `DragValue`, `DatePickerButton`, `ComboBox`, `CodeEditor`). This
//! first pass ships three: `Text`, `Number`, `Choice`. `Date`
//! (DatePickerButton) and `Code` (CodeEditor) are explicit TODOs —
//! they require separate state shapes (naïve date / multi-line code
//! buffer) that the current `FieldValue` enum doesn't yet model.

use std::borrow::Cow;

use eframe::egui;

use super::response::CanonResponse;
use super::types::{Affordance, Confidence, IdentityUri, MutationAxis, Tooltip, VariantId};
use super::CanonWidget;

const IDENTITY: &str = "ui://field";

static AFFORDANCES_ACTIVE: &[Affordance] = &[
    Affordance {
        name: Cow::Borrowed("edit"),
        verb: Cow::Borrowed("wsp.set"),
        actors: &[],
        args_schema: None,
        reorderable: false,
    },
    Affordance {
        name: Cow::Borrowed("commit"),
        verb: Cow::Borrowed("wsp.commit"),
        actors: &[],
        args_schema: None,
        reorderable: false,
    },
];
static AFFORDANCES_DISABLED: &[Affordance] = &[];

static MUTATION_AXES: &[MutationAxis] = &[
    MutationAxis::new("placeholder"),
    MutationAxis::new("mask"),
    MutationAxis::new("validation-hint"),
];

/// Which egui surface the field should render with. `Date` and `Code`
/// are deliberately left out of this first pass; see file header.
#[derive(Clone, Debug)]
pub enum FieldKind {
    Text {
        placeholder: Cow<'static, str>,
        multiline: bool,
        password: bool,
    },
    Number {
        min: f64,
        max: f64,
        step: f64,
    },
    Choice {
        options: &'static [&'static str],
    },
    // TODO: Date  — `egui_extras::DatePickerButton` + chrono::NaiveDate state.
    // TODO: Code  — `egui::TextEdit::multiline` + `egui_extras::syntax_highlighting`.
}

impl FieldKind {
    pub fn text(placeholder: impl Into<Cow<'static, str>>) -> Self {
        Self::Text {
            placeholder: placeholder.into(),
            multiline: false,
            password: false,
        }
    }

    pub fn multiline(placeholder: impl Into<Cow<'static, str>>) -> Self {
        Self::Text {
            placeholder: placeholder.into(),
            multiline: true,
            password: false,
        }
    }

    pub fn password(placeholder: impl Into<Cow<'static, str>>) -> Self {
        Self::Text {
            placeholder: placeholder.into(),
            multiline: false,
            password: true,
        }
    }

    pub fn number(min: f64, max: f64, step: f64) -> Self {
        Self::Number { min, max, step }
    }

    pub fn choice(options: &'static [&'static str]) -> Self {
        Self::Choice { options }
    }
}

/// Bound value that the caller threads through as a `&mut`. Enum
/// variants must line up with the `FieldKind` passed — mismatches
/// render a warning label rather than panicking.
#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue {
    Text(String),
    Number(f64),
    Choice(usize),
}

impl FieldValue {
    pub const fn as_kind_tag(&self) -> &'static str {
        match self {
            Self::Text(_) => "Text",
            Self::Number(_) => "Number",
            Self::Choice(_) => "Choice",
        }
    }
}

/// Typed input binding. Caller passes a `FieldKind` describing how to
/// render and a `&mut FieldValue` holding the current bound value.
pub struct Field<'b> {
    id_source: egui::Id,
    kind: FieldKind,
    value: &'b mut FieldValue,
    enabled: bool,
    tooltip: Option<Tooltip>,
    variant: VariantId,
}

impl<'b> Field<'b> {
    pub fn new(
        id_source: impl std::hash::Hash,
        kind: FieldKind,
        value: &'b mut FieldValue,
    ) -> Self {
        Self {
            id_source: egui::Id::new(("canon.field", id_source)),
            kind,
            value,
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

impl CanonWidget for Field<'_> {
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
        let kind = self.kind;
        let value = self.value;

        let (mut resp, chosen_affordance) = ui
            .scope(|ui| {
                if !enabled {
                    ui.disable();
                }
                match (&kind, &mut *value) {
                    (
                        FieldKind::Text {
                            placeholder,
                            multiline,
                            password,
                        },
                        FieldValue::Text(s),
                    ) => {
                        let mut edit = if *multiline {
                            egui::TextEdit::multiline(s)
                        } else {
                            egui::TextEdit::singleline(s)
                        };
                        edit = edit.hint_text(placeholder.as_ref()).password(*password);
                        let r = ui.add(edit);
                        let chosen: Option<&'static str> = if !enabled {
                            None
                        } else if r.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            Some("commit")
                        } else if r.changed() {
                            Some("edit")
                        } else {
                            None
                        };
                        (r, chosen)
                    }
                    (FieldKind::Number { min, max, step }, FieldValue::Number(n)) => {
                        let r = ui.add(
                            egui::DragValue::new(n)
                                .range(*min..=*max)
                                .speed(*step),
                        );
                        let chosen: Option<&'static str> = if enabled && r.changed() {
                            Some("edit")
                        } else {
                            None
                        };
                        (r, chosen)
                    }
                    (FieldKind::Choice { options }, FieldValue::Choice(idx)) => {
                        let current = options.get(*idx).copied().unwrap_or("");
                        let mut changed = false;
                        let combo = egui::ComboBox::from_id_salt(id)
                            .selected_text(current)
                            .show_ui(ui, |ui| {
                                for (i, opt) in options.iter().enumerate() {
                                    if ui
                                        .selectable_label(*idx == i, *opt)
                                        .clicked()
                                        && *idx != i
                                    {
                                        *idx = i;
                                        changed = true;
                                    }
                                }
                            });
                        let chosen: Option<&'static str> = if enabled && changed {
                            Some("edit")
                        } else {
                            None
                        };
                        (combo.response, chosen)
                    }
                    _ => {
                        let r = ui.label(format!(
                            "[canon::field] kind/value mismatch — value is {}",
                            value.as_kind_tag()
                        ));
                        (r, None)
                    }
                }
            })
            .inner;

        if let Some(tt) = &tooltip {
            resp = resp.on_hover_text(tt.as_ref());
        }

        CanonResponse::from_egui(resp, Cow::Borrowed(IDENTITY), variant, chosen_affordance)
            .with_id_hint(id)
    }
}
