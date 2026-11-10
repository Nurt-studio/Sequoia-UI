//! Paint / effect / prototype edits that do not fit the per-field props
//! pipeline: they replace whole lists (gradient stops, shadows) or store an
//! optional connection on the selected element.

use sequoia_ui::prelude::Color;

use crate::model::svelte_ast::{
    GradientDef, GradientKind, GradientStopDef, MAX_GRADIENT_STOPS, ProtoAction, PrototypeLink,
    ShadowEffect, SvelteNode,
};
use crate::model::tree_ops;

use super::{EditorState, PickerOption};

impl EditorState {
    // ── gradient fill ───────────────────────────────────────────

    /// Switches the fill between solid and a gradient. Turning a gradient on
    /// keeps the current solid color as its first stop.
    pub(crate) fn set_fill_kind(&self, kind: &str) {
        let current = self.sel_gradient.get_untracked();
        match kind {
            "LINEAR" | "RADIAL" => {
                let mut g = current.unwrap_or_else(|| {
                    GradientDef::from_solid(
                        self.sel_fill.get_untracked(),
                        &self.sel_fill_hex.get_untracked(),
                    )
                });
                g.kind = if kind == "RADIAL" {
                    GradientKind::Radial
                } else {
                    GradientKind::Linear
                };
                self.sel_gradient.set(Some(g));
            }
            _ => self.sel_gradient.set(None),
        }
    }

    pub(crate) fn set_gradient_angle(&self, angle: f32) {
        let Some(mut g) = self.sel_gradient.get_untracked() else {
            return;
        };
        g.angle = angle.clamp(0.0, 360.0);
        self.sel_gradient.set(Some(g));
    }

    pub(crate) fn add_gradient_stop(&self) {
        let Some(mut g) = self.sel_gradient.get_untracked() else {
            return;
        };
        if g.stops.len() >= MAX_GRADIENT_STOPS {
            return;
        }
        g.stops
            .push(GradientStopDef::new(Color::hex(0xFFFFFF), "#FFFFFF", 1.0));
        self.sel_gradient.set(Some(g));
    }

    pub(crate) fn remove_gradient_stop(&self, index: usize) {
        let Some(mut g) = self.sel_gradient.get_untracked() else {
            return;
        };
        if g.stops.len() > 2 && index < g.stops.len() {
            g.stops.remove(index);
            self.sel_gradient.set(Some(g));
        }
    }

    pub(crate) fn set_gradient_stop_color(&self, index: usize, hex: &str) {
        let Some(color) = Color::parse_hex(hex.trim()) else {
            return;
        };
        let Some(mut g) = self.sel_gradient.get_untracked() else {
            return;
        };
        let Some(stop) = g.stops.get_mut(index) else {
            return;
        };
        stop.color = color;
        stop.hex = hex.trim().to_uppercase();
        self.sel_gradient.set(Some(g));
    }

    /// `percent` is the stop position in percent (0..100).
    pub(crate) fn set_gradient_stop_position(&self, index: usize, percent: f32) {
        let Some(mut g) = self.sel_gradient.get_untracked() else {
            return;
        };
        let Some(stop) = g.stops.get_mut(index) else {
            return;
        };
        stop.at = (percent / 100.0).clamp(0.0, 1.0);
        self.sel_gradient.set(Some(g));
    }

    // ── effects ─────────────────────────────────────────────────

    pub(crate) fn add_shadow_effect(&self) {
        self.sel_effects.update(|effects| {
            effects.push(ShadowEffect::new());
        });
    }

    pub(crate) fn remove_shadow_effect(&self, index: usize) {
        self.sel_effects.update(|effects| {
            if index < effects.len() {
                effects.remove(index);
            }
        });
    }

    pub(crate) fn toggle_shadow_effect(&self, index: usize) {
        self.sel_effects.update(|effects| {
            if let Some(e) = effects.get_mut(index) {
                e.enabled = !e.enabled;
            }
        });
    }

    pub(crate) fn set_shadow_number(&self, index: usize, field: &str, value: f32) {
        self.sel_effects.update(|effects| {
            let Some(e) = effects.get_mut(index) else {
                return;
            };
            match field {
                "x" => e.x = value,
                "y" => e.y = value,
                "blur" => e.blur = value.max(0.0),
                "spread" => e.spread = value,
                _ => {}
            }
        });
    }

    pub(crate) fn set_shadow_color(&self, index: usize, hex: &str) {
        let Some(color) = Color::parse_hex(hex.trim()) else {
            return;
        };
        self.sel_effects.update(|effects| {
            if let Some(e) = effects.get_mut(index) {
                e.color = color;
                e.hex = hex.trim().to_uppercase();
            }
        });
    }

    // ── prototype connections ───────────────────────────────────

    pub(crate) fn selected_prototype(&self) -> Option<PrototypeLink> {
        let id = self.selected_layer.get_untracked()?;
        match self
            .root
            .get_untracked()
            .as_ref()
            .and_then(|r| tree_ops::find(r, id))?
        {
            SvelteNode::Element(e) => e.prototype.clone(),
            SvelteNode::Text(_) => None,
        }
    }

    /// Frames offered as prototype targets: every element with children, in
    /// document order, labelled with its id.
    pub(crate) fn prototype_target_options(&self) -> Vec<PickerOption> {
        fn walk(node: &SvelteNode, out: &mut Vec<PickerOption>) {
            if let SvelteNode::Element(e) = node
                && !e.children.is_empty()
            {
                out.push(PickerOption {
                    label: format!("{} (#{})", e.name, e.id),
                    value: e.id.to_string(),
                });
            }
            for c in node.children() {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        if let Some(root) = self.root.get_untracked().as_ref() {
            walk(root, &mut out);
        }
        out
    }

    /// Default target: the first frame that is not the selected element.
    fn default_prototype_target(&self) -> Option<u64> {
        let selected = self.selected_layer.get_untracked();
        self.prototype_target_options()
            .iter()
            .filter_map(|o| o.value.parse::<u64>().ok())
            .find(|id| Some(*id) != selected)
    }

    pub(crate) fn add_prototype_link(&self) {
        let link = PrototypeLink::new(self.default_prototype_target());
        self.set_prototype(Some(link));
    }

    pub(crate) fn remove_prototype_link(&self) {
        self.set_prototype(None);
    }

    pub(crate) fn update_prototype(&self, f: impl FnOnce(&mut PrototypeLink)) {
        let mut link = self
            .selected_prototype()
            .unwrap_or_else(|| PrototypeLink::new(self.default_prototype_target()));
        f(&mut link);
        self.set_prototype(Some(link));
    }

    fn set_prototype(&self, link: Option<PrototypeLink>) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        self.push_history();
        self.root.update(|r| {
            if let Some(SvelteNode::Element(e)) =
                r.as_mut().and_then(|rr| tree_ops::find_mut(rr, id))
            {
                e.prototype = link;
            }
        });
    }
}

/// Whether a prototype action points somewhere (`Back` does not).
pub(crate) fn proto_needs_target(action: ProtoAction) -> bool {
    matches!(action, ProtoAction::Navigate | ProtoAction::OpenOverlay)
}
