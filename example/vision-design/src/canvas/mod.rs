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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::EditorState;
    use sequoia_ui::text::TextSystem;
    use sequoia_ui::view::Runtime;

    #[test]
    fn horizontal_wheel_pans_the_design_canvas() {
        let state = EditorState::demo();
        let rt = Runtime::new();
        rt.set_root_fill(true);
        rt.mount(canvas(&state));
        let mut text = TextSystem::new();
        rt.layout(Rect::from_xywh(0.0, 0.0, 1000.0, 700.0), &mut text);

        rt.wheel(
            Point::new(500.0, 350.0),
            Vec2::new(40.0, 0.0),
            Modifiers::NONE,
            &mut text,
        );

        assert_eq!(state.pan_x.get_untracked(), -40.0);
        assert_eq!(state.pan_y.get_untracked(), 0.0);
    }
}
