//! The shared dropdown overlay: renders the `PickerMenu` request as a Popper
//! anchored to the trigger button — the same mechanics as the menu bar drops.

use sequoia_ui::prelude::*;

use crate::state::EditorState;
use crate::theme::colors;

#[derive(Clone)]
struct PickerOverlay {
    state: EditorState,
}

impl View for PickerOverlay {
    fn body(&self) -> impl View + 'static {
        // Tracked reads: the view rebuilds on open/close and re-anchor.
        let open = self.state.menu_picker.open.get();
        let anchor = self.state.menu_picker.anchor.get();
        if !open {
            return AnyView::new(Unit);
        }
        let Some(anchor_id) = anchor else {
            return AnyView::new(Unit);
        };
        let Some(req) = self.state.menu_picker.request.borrow().clone() else {
            return AnyView::new(Unit);
        };

        let mut rows: Vec<AnyView> = Vec::new();
        for opt in &req.options {
            let st = self.state.clone();
            let on_pick = req.on_pick.clone();
            let value = opt.value.clone();
            let selected = opt.value == req.selected;
            rows.push(AnyView::new(
                Container::new()
                    .fill_width()
                    .height(26.0)
                    .padding_xy(8.0, 0.0)
                    .radius(5.0)
                    .align_items(Align::Center)
                    .hover(colors::active().sel)
                    .on_click(move || {
                        on_pick(&value);
                        st.menu_picker.close();
                    })
                    .child(
                        Row::new()
                            .fill_width()
                            .align_items(Align::Center)
                            .child(Text::new(opt.label.clone()).size(12.0).color(if selected {
                                Color::WHITE
                            } else {
                                Color::hex(0xE6E6E6)
                            }))
                            .child(Spacer::flex(1.0))
                            .child(
                                Text::new(if selected { "✓" } else { "" })
                                    .size(14.0)
                                    .color(Color::hex(0x4ADE80)),
                            ),
                    ),
            ));
        }
        if rows.is_empty() {
            rows.push(AnyView::new(
                Container::new().fill_width().padding_xy(8.0, 4.0).child(
                    Text::new("(brak opcji)")
                        .size(11.0)
                        .color(colors::active().icon_inactive),
                ),
            ));
        }

        AnyView::new(
            Popper::new(self.state.menu_picker.open.clone())
                .anchor_view(anchor_id)
                .placement(Placement::BottomStart)
                .offset(0.0, 4.0)
                .dismiss(Dismiss::OutsideClick)
                .child(
                    DismissableLayer::wrap(
                        Container::new()
                            .width(req.menu_width)
                            .padding(4.0)
                            .radius(8.0)
                            .background(Color::hex(0x2A2A2C))
                            .border(1.0, colors::active().stroke)
                            .child(Column::new().children(rows))
                            .shadow(
                                ShadowSpec::new(Color::hexa(0x00000099), 40.0).offset(0.0, 16.0),
                            ),
                    )
                    .dismiss_on_escape(true)
                    .bind(self.state.menu_picker.open.clone()),
                ),
        )
    }
}

pub(crate) fn picker_menu(state: &EditorState) -> impl View + 'static {
    PickerOverlay {
        state: state.clone(),
    }
}
