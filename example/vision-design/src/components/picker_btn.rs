//! Dropdown trigger button for the properties panel (variable pickers,
//! Show/Hide targets, triggers, events). One shared `PickerMenu` in the
//! editor state keeps a single dropdown open at a time; the Popper itself
//! lives in `overlays::picker_menu`, anchored to this button's element.

use sequoia_ui::layout::LayoutCtx;
use sequoia_ui::prelude::*;
use std::rc::Rc;

use crate::state::{EditorState, PickerOption};
use crate::theme::colors;

#[derive(Clone)]
pub(crate) struct PickerBtn {
    pub(crate) state: EditorState,
    pub(crate) label: String,
    pub(crate) options: Vec<PickerOption>,
    pub(crate) selected: String,
    pub(crate) on_pick: Rc<dyn Fn(&str)>,
    pub(crate) width: f32,
    pub(crate) menu_width: f32,
}

impl Leaf for PickerBtn {
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
            .height(Length::px(24.0))
            .width(Length::px(self.width))
    }
    fn intrinsic_size(&self, _ctx: &mut LayoutCtx) -> Option<Size> {
        Some(Size::new(self.width, 24.0))
    }
    fn track(&self) {
        // Rebuild when the shared dropdown opens/closes (chevron state).
        let _ = self.state.menu_picker.open.get();
    }
    fn paint(&self, ctx: &mut PaintCtx) {
        let hot = ctx.state.hovered;
        let bg = if hot {
            colors::active().bg_lighter
        } else {
            colors::active().bg_darker
        };
        ctx.quad(ctx.rect, bg, 6.0);
        let m = ctx.text_metrics(12.0);
        let top = ctx.rect.top() + (ctx.rect.size.height - m.height) * 0.5;
        ctx.draw_text_tl(
            &self.label,
            (ctx.rect.left() + 8.0, top),
            12.0,
            Color::hex(0xE8E8E8),
        );
        // Chevron on the right.
        let chev_x = ctx.rect.left() + ctx.rect.size.width - 16.0;
        ctx.draw_text_tl("▾", (chev_x, top), 14.0, colors::active().icon_inactive);
    }
    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if let Event::Pointer(PointerEvent::Down {
            button: PointerButton::Left,
            ..
        }) = event
        {
            let on_pick = self.on_pick.clone();
            self.state.menu_picker.show(
                ctx.element(),
                self.options.clone(),
                &self.selected,
                self.menu_width,
                move |value| on_pick(value),
            );
            ctx.handled();
        }
    }
    fn has_visual_state(&self) -> bool {
        true
    }
    fn cursor(&self, _state: VisualState) -> CursorIcon {
        CursorIcon::Pointer
    }
}
impl_view_for_leaf!(PickerBtn);
