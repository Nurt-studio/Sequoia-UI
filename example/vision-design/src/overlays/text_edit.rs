use sequoia_ui::prelude::*;

use crate::state::EditorState;
use crate::theme::colors;

#[derive(Clone)]
struct TextEditOverlay {
    state: EditorState,
}

impl View for TextEditOverlay {
    fn body(&self) -> impl View + 'static {
        let open = self.state.editing_text.clone();
        if !open.get() {
            return AnyView::new(Unit);
        }
        let (ax, ay, aw, ah) = self.state.text_edit_anchor.get();
        let text_sig = self.state.sel_text.clone();
        let font = (self.state.sel_font_size.get() * self.state.zoom.get()).max(11.0);
        let st = self.state.clone();

        AnyView::new(
            Popper::new(open.clone())
                .anchor_rect(Rect::from_xywh(ax, ay, 0.0, 0.0))
                .placement(Placement::BottomStart)
                .offset(0.0, 0.0)
                .dismiss(Dismiss::OutsideClick)
                .child(
                    DismissableLayer::wrap(
                        Container::new()
                            .width(aw)
                            .height(ah)
                            .background(Color::hex(0x101012))
                            .border(1.0, colors::active().sel)
                            .radius(4.0)
                            .padding(Edges::xy(2.0, 2.0))
                            .child(
                                TextField::new(text_sig)
                                    .font_size(font)
                                    .text_color(Color::WHITE)
                                    .background(Color::TRANSPARENT)
                                    .border(0.0, Color::TRANSPARENT)
                                    .padding(Edges::xy(4.0, 2.0))
                                    .width(Length::Px(aw - 4.0))
                                    .height(Length::Px(ah - 4.0))
                                    .on_submit({
                                        let st = st.clone();
                                        move |text: String| {
                                            st.sel_text.set(text);
                                            st.end_text_edit(true);
                                        }
                                    })
                                    .on_edit_end({
                                        let st = st.clone();
                                        move |text: String| {
                                            st.sel_text.set(text);
                                            st.end_text_edit(true);
                                        }
                                    }),
                            ),
                    )
                    .dismiss_on_escape(true)
                    .bind(open.clone()),
                ),
        )
    }
}

// -------------------------- Command Palette --------------------------

pub(crate) fn text_edit_overlay(state: &EditorState) -> impl View + 'static {
    TextEditOverlay {
        state: state.clone(),
    }
}