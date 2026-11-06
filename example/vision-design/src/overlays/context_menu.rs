use sequoia_ui::prelude::*;

use crate::components::menu::{menu_row as mrow, menu_sep as msep};

use crate::state::EditorState;
use crate::theme::colors;

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
        let st_al = st.clone();

        let px = pos_x.get();
        let py = pos_y.get();

        let rows: Vec<AnyView> = vec![
            mrow(
                "Duplicate",
                Some("Ctrl+D"),
                false,
                open.clone(),
                move || st_dup.duplicate_selected(),
            ),
            mrow("Copy", Some("Ctrl+C"), false, open.clone(), move || {
                st_cp.copy_selected()
            }),
            mrow("Paste", Some("Ctrl+V"), false, open.clone(), move || {
                st_pst.paste()
            }),
            mrow("Delete", Some("Del"), true, open.clone(), move || {
                st_del.delete_selected()
            }),
            AnyView::new(msep()),
            mrow(
                "Bring to front",
                Some("]"),
                false,
                open.clone(),
                move || st_ff.bring_to_front(),
            ),
            mrow("Bring forward", None, false, open.clone(), move || {
                st_fr.bring_forward()
            }),
            mrow("Send backward", None, false, open.clone(), move || {
                st_bk.send_backward()
            }),
            mrow("Send to back", Some("["), false, open.clone(), move || {
                st_bb.send_to_back()
            }),
            AnyView::new(msep()),
            mrow("Auto layout", None, false, open.clone(), move || {
                st_al.auto_layout_selection()
            }),
            AnyView::new(msep()),
            mrow("Lock / unlock", None, false, open.clone(), move || {
                st_lk.toggle_lock()
            }),
            mrow("Show / hide", None, false, open.clone(), move || {
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
                        .border(1.0, colors::active().stroke)
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
