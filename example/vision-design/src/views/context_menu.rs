use sequoia_ui::prelude::*;

use crate::colors;
use crate::state::EditorState;

fn menu_row(
    label: &'static str,
    hint: Option<&'static str>,
    danger: bool,
    open: Signal<bool>,
    on_action: impl Fn() + 'static,
) -> AnyView {
    let hover_bg = if danger {
        Color::hexa(0xFA4D56E6)
    } else {
        colors::SEL
    };
    let mut row = Row::new()
        .fill_width()
        .height(27.0)
        .align_items(Align::Center)
        .padding_xy(8.0, 0.0)
        .child(Text::new(label).size(12.0).color(Color::hex(0xE6E6E6)));
    if let Some(hint) = hint {
        row = row.child(Spacer::flex(1.0)).child(
            Text::new(hint)
                .size(10.0)
                .color(Color::hex(0xE6E6E6).with_alpha(0.6)),
        );
    }
    AnyView::new(
        Container::new()
            .fill_width()
            .radius(5.0)
            .hover(hover_bg)
            .on_click(move || {
                open.set(false);
                on_action();
            })
            .child(row),
    )
}

fn menu_sep() -> impl View + 'static {
    Container::new()
        .fill_width()
        .padding(Edges::xy(4.0, 4.0))
        .child(
            Container::new()
                .height(1.0)
                .fill_width()
                .background(colors::STROKE),
        )
}

#[derive(Clone)]
struct ContextMenuView {
    state: EditorState,
}

impl View for ContextMenuView {
    fn body(&self) -> impl View + 'static {
        let open = self.state.ctx_menu_open.clone();
        let pos_x = self.state.ctx_menu_x.clone();
        let pos_y = self.state.ctx_menu_y.clone();
        let st = self.state.clone();

        let st_dup = st.clone();
        let st_cp = st.clone();
        let st_pst = st.clone();
        let st_del = st.clone();
        let st_ff = st.clone();
        let st_fr = st.clone();
        let st_bk = st.clone();
        let st_bb = st.clone();
        let st_lk = st.clone();
        let st_vis = st.clone();

        let px = pos_x.get();
        let py = pos_y.get();

        let rows: Vec<AnyView> = vec![
            menu_row(
                "Duplicate",
                Some("Ctrl+D"),
                false,
                open.clone(),
                move || st_dup.duplicate_selected(),
            ),
            menu_row("Copy", Some("Ctrl+C"), false, open.clone(), move || {
                st_cp.copy_selected()
            }),
            menu_row("Paste", Some("Ctrl+V"), false, open.clone(), move || {
                st_pst.paste()
            }),
            menu_row("Delete", Some("Del"), true, open.clone(), move || {
                st_del.delete_selected()
            }),
            AnyView::new(menu_sep()),
            menu_row(
                "Bring to Front",
                Some("]"),
                false,
                open.clone(),
                move || st_ff.bring_to_front(),
            ),
            menu_row("Bring Forward", None, false, open.clone(), move || {
                st_fr.bring_forward()
            }),
            menu_row("Send Backward", None, false, open.clone(), move || {
                st_bk.send_backward()
            }),
            menu_row("Send to Back", Some("["), false, open.clone(), move || {
                st_bb.send_to_back()
            }),
            AnyView::new(menu_sep()),
            menu_row("Lock / Unlock", None, false, open.clone(), move || {
                st_lk.toggle_lock()
            }),
            menu_row("Show / Hide", None, false, open.clone(), move || {
                st_vis.toggle_visibility()
            }),
        ];

        Popper::new(open.clone())
            .anchor_rect(Rect::from_xywh(px, py, 0.0, 0.0))
            .placement(Placement::BottomStart)
            .offset(2.0, 2.0)
            .dismiss(Dismiss::OutsideClick)
            .child(
                DismissableLayer::wrap(
                    Container::new()
                        .width(200.0)
                        .padding(4.0)
                        .radius(8.0)
                        .background(Color::hex(0x2A2A2C))
                        .border(1.0, colors::STROKE)
                        .child(Column::new().children(rows))
                        .shadow(ShadowSpec::new(Color::hexa(0x00000099), 40.0).offset(0.0, 16.0)),
                )
                .dismiss_on_escape(true)
                .bind(open.clone()),
            )
    }
}

pub(crate) fn context_menu(state: &EditorState) -> impl View + 'static {
    ContextMenuView {
        state: state.clone(),
    }
}
