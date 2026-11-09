//! Canvas: view composition, painting and the interaction state machine.

use sequoia_ui::prelude::*;

mod canvas_view;
mod interaction;
mod paint;
pub(crate) mod text_input;

pub(crate) use canvas_view::canvas;
pub(crate) use interaction::Interaction;

// The old 0x09 alpha (~3.5%) was invisible on the dark canvas — the grid
// toggle "did nothing". 0x2B (~17%) reads clearly but stays subtle.
pub(crate) const GRID_COLOR: Color = Color::hexa(0xFFFFFF2B);
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
    use crate::model::svelte_ast::{SvelteNode, new_text};
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

    #[test]
    fn typing_on_canvas_edits_text_inline() {
        let state = EditorState::with_seed(Some(new_text(1, 0.0, 0.0, "Hi")), Some(1));
        let rt = Runtime::new();
        rt.set_root_fill(true);
        rt.mount(canvas(&state));
        let mut text = TextSystem::new();
        rt.layout(Rect::from_xywh(0.0, 0.0, 1000.0, 700.0), &mut text);
        rt.tick();

        // A click on the canvas focuses it; keys go straight into the text.
        rt.pointer_down(
            Point::new(10.0, 10.0),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );
        rt.pointer_up(
            Point::new(10.0, 10.0),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );

        state.start_text_edit(false);
        assert!(state.editing_text.get_untracked());

        rt.key(Key::from_char('!'), Modifiers::NONE, &mut text);
        rt.key(Key::BACKSPACE, Modifiers::NONE, &mut text);
        rt.key(Key::from_char('!'), Modifiers::NONE, &mut text);
        rt.tick();

        assert_eq!(state.sel_text.get_untracked(), "Hi!");
        let content = state.root.with_untracked(|r| {
            r.as_ref()
                .and_then(|r| crate::model::tree_ops::find(r, 1))
                .and_then(|n| match n {
                    SvelteNode::Text(t) => Some(t.content.clone()),
                    _ => None,
                })
        });
        assert_eq!(content.as_deref(), Some("Hi!"));

        rt.key(Key::ESCAPE, Modifiers::NONE, &mut text);
        assert!(!state.editing_text.get_untracked());
        assert_eq!(state.sel_text.get_untracked(), "Hi!");
    }

    #[test]
    fn dragging_a_child_of_an_auto_layout_reorders_it() {
        use crate::model::svelte_ast::{Dimension, Display, FlexDir, Position, Style, Tag};

        let box_ = |id: u64| {
            SvelteNode::Element(crate::model::svelte_ast::Element {
                id,
                name: format!("Box {id}"),
                tag: Tag::Div,
                style: Style {
                    display: Display::None,
                    position: Position::Relative,
                    width: Dimension::Fixed(100.0),
                    height: Dimension::Fixed(40.0),
                    ..Default::default()
                },
                children: vec![],
                visible: true,
                locked: false,
                component: false,
                instance: false,
                interface: None,
                overrides: Vec::new(),
                slot_content: Vec::new(),
                behavior: None,
                binding: None,
                hover: None,
            })
        };
        let frame = SvelteNode::Element(crate::model::svelte_ast::Element {
            id: 1,
            name: "Row".into(),
            tag: Tag::Div,
            style: Style {
                display: Display::Flex,
                flex_dir: FlexDir::Row,
                gap: 10.0,
                width: Dimension::Fixed(400.0),
                height: Dimension::Fixed(100.0),
                ..Default::default()
            },
            children: vec![box_(2), box_(3), box_(4)],
            visible: true,
            locked: false,
            component: false,
            instance: false,
            interface: None,
            overrides: Vec::new(),
            slot_content: Vec::new(),
            behavior: None,
            binding: None,
            hover: None,
        });

        let state = EditorState::with_seed(Some(frame), None);
        let rt = Runtime::new();
        rt.set_root_fill(true);
        rt.mount(canvas(&state));
        let mut text = TextSystem::new();
        rt.layout(Rect::from_xywh(0.0, 0.0, 1000.0, 700.0), &mut text);
        rt.tick();

        // Drag the first child past the last one.
        rt.pointer_down(
            Point::new(50.0, 20.0),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );
        rt.pointer_move(Point::new(350.0, 20.0), &mut text);
        rt.pointer_up(
            Point::new(350.0, 20.0),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );
        rt.tick();

        let order = state.root.with_untracked(|r| {
            r.as_ref()
                .map(|n| n.children().iter().map(|c| c.id()).collect::<Vec<_>>())
                .unwrap_or_default()
        });
        assert_eq!(order, vec![3, 4, 2], "child order after the drag");

        // Reordered children are laid out by the parent, not moved freely.
        let xs: Vec<f32> = state
            .layers
            .get_untracked()
            .iter()
            .filter(|l| l.id != 1)
            .map(|l| l.x)
            .collect();
        assert_eq!(xs, vec![0.0, 110.0, 220.0], "children stay in the flow");
    }

    #[test]
    fn dragging_text_inside_auto_layout_moves_it() {
        use crate::model::svelte_ast::{Dimension, Display, Element, FlexDir, Style, Tag};

        let frame = SvelteNode::Element(Element {
            id: 1,
            name: "Row".into(),
            tag: Tag::Div,
            style: Style {
                display: Display::Flex,
                flex_dir: FlexDir::Row,
                gap: 10.0,
                width: Dimension::Fixed(400.0),
                height: Dimension::Fixed(100.0),
                ..Default::default()
            },
            children: vec![new_text(2, 0.0, 0.0, "Hi")],
            visible: true,
            locked: false,
            component: false,
            instance: false,
            interface: None,
            overrides: Vec::new(),
            slot_content: Vec::new(),
            behavior: None,
            binding: None,
            hover: None,
        });
        let state = EditorState::with_seed(Some(frame), Some(2));
        let rt = Runtime::new();
        rt.set_root_fill(true);
        rt.mount(canvas(&state));
        let mut text = TextSystem::new();
        rt.layout(Rect::from_xywh(0.0, 0.0, 1000.0, 700.0), &mut text);
        rt.tick();

        rt.pointer_down(
            Point::new(10.0, 10.0),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );
        // Alt turns snapping off, so the drag offset is exact.
        rt.pointer_move_with_mods(Point::new(110.0, 40.0), Modifiers::ALT, &mut text);
        rt.pointer_up(
            Point::new(110.0, 40.0),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );
        rt.tick();

        let layer = state
            .layers
            .get_untracked()
            .into_iter()
            .find(|l| l.id == 2)
            .expect("text layer");
        assert!((layer.x - 100.0).abs() < 0.5, "text x = {}", layer.x);
        assert!((layer.y - 30.0).abs() < 0.5, "text y = {}", layer.y);
        assert_eq!(layer.position_mode, "ABSOLUTE", "text left the flow");
    }

    #[test]
    fn dragging_moves_the_text_layer() {
        let state = EditorState::with_seed(Some(new_text(1, 0.0, 0.0, "Hi")), Some(1));
        let rt = Runtime::new();
        rt.set_root_fill(true);
        rt.mount(canvas(&state));
        let mut text = TextSystem::new();
        rt.layout(Rect::from_xywh(0.0, 0.0, 1000.0, 700.0), &mut text);
        rt.tick();

        rt.pointer_down(
            Point::new(10.0, 10.0),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );
        // Alt turns snapping off, so the drag offset is exact.
        rt.pointer_move_with_mods(Point::new(110.0, 40.0), Modifiers::ALT, &mut text);
        rt.pointer_up(
            Point::new(110.0, 40.0),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );
        rt.tick();

        let (x, y) = state
            .layers
            .get_untracked()
            .iter()
            .find(|l| l.id == 1)
            .map(|l| (l.x, l.y))
            .expect("text layer");
        assert!((x - 100.0).abs() < 0.5, "text x = {x}");
        assert!((y - 30.0).abs() < 0.5, "text y = {y}");
    }
}
