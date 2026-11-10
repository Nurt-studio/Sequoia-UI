//! Side panels: layers and properties.

mod layers;
mod properties;

pub(crate) use layers::{drag_overlay as layer_drag_overlay, panel_left};
pub(crate) use properties::panel_right;
