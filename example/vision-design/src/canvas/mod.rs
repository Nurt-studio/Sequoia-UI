//! Canvas: view composition, painting and the interaction state machine.

use sequoia_ui::prelude::*;

mod canvas_view;
mod interaction;
mod paint;

pub(crate) use canvas_view::canvas;
pub(crate) use interaction::Interaction;

pub(crate) const GRID_COLOR: Color = Color::hexa(0xFFFFFF09);
pub(crate) const SEL_COLOR: Color = Color::hex(0x0D99FF);
pub(crate) const SEL_FILL: Color = Color::hexa(0x0D99FF29);
pub(crate) const GUIDE_COLOR: Color = Color::hex(0xFF3B30);
pub(crate) const ZOOM_MIN: f32 = 0.10;
pub(crate) const ZOOM_MAX: f32 = 8.0;
pub(crate) const ZOOM_STEP: f32 = 1.15;
pub(crate) const GUIDE_TOL_PX: f32 = 5.0;
