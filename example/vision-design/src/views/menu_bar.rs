use sequoia_ui::layout::LayoutCtx;
use sequoia_ui::prelude::*;
use sequoia_ui::view::{Fill, GradientStops};

use crate::colors;
use crate::state::{EditorState, Menu};

// ---------------------------------------------------------------- logo

fn logo() -> impl View + 'static {
    Canvas::new()
        .width(16.0)
        .height(16.0)
        .focusable(false)
        .on_draw(|g| {
            g.rounded(
                g.rect(),
                4.0,
                Fill::horizontal(GradientStops::two(
                    Color::hex(0x0D99FF),
                    Color::hex(0x0F62FE),
                )),
            );
            g.text_center(g.rect(), "VD", 8.0, Color::WHITE);
        })
}

fn mrow(
    label: &'static str,
    hint: Option<&'static str>,
    danger: bool,
    open: Signal<bool>,
    action: impl Fn() + 'static,
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
                action();
            })
            .child(row),
    )
}

fn msep() -> AnyView {
    AnyView::new(
        Container::new()
            .fill_width()
            .padding(Edges::xy(6.0, 4.0))
            .child(
                Container::new()
                    .height(1.0)
                    .fill_width()
                    .background(colors::STROKE),
            ),
    )
}

fn mdrop_content(open: Signal<bool>, rows: Vec<AnyView>) -> impl View + 'static {
    DismissableLayer::wrap(
        Container::new()
            .width(232.0)
            .padding(4.0)
            .radius(8.0)
            .background(Color::hex(0x2A2A2C))
            .border(1.0, colors::STROKE)
            .child(Column::new().children(rows))
            .shadow(ShadowSpec::new(Color::hexa(0x00000099), 40.0).offset(0.0, 16.0)),
    )
    .dismiss_on_escape(true)
    .bind(open.clone())
}

#[derive(Clone)]
struct MbBtnLeaf {
    label: &'static str,
    menu: Menu,
    others: Vec<Signal<bool>>,
}

impl Leaf for MbBtnLeaf {
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
            .height(Length::px(22.0))
            .width(Length::Fit)
    }
    fn intrinsic_size(&self, ctx: &mut LayoutCtx) -> Option<Size> {
        Some(Size::new(ctx.text_width(self.label, 12.0) + 16.0, 22.0))
    }
    fn track(&self) {
        let _ = self.menu.open.get();
    }
    fn paint(&self, ctx: &mut PaintCtx) {
        let on = self.menu.open.get_untracked();
        let hot = on || ctx.state.hovered;
        let bg = if hot {
            colors::BG_LIGHTER
        } else {
            Color::TRANSPARENT
        };
        ctx.quad(ctx.rect, bg, 5.0);
        let fg = if hot {
            Color::WHITE
        } else {
            Color::hex(0xB9B9BD)
        };
        let m = ctx.text_metrics(12.0);
        let top = ctx.rect.top() + (ctx.rect.size.height - m.height) * 0.5;
        ctx.draw_text_tl(self.label, (ctx.rect.left() + 8.0, top), 12.0, fg);
    }
    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if let Event::Pointer(PointerEvent::Down {
            button: PointerButton::Left,
            ..
        }) = event
        {
            self.menu.anchor.set(Some(ctx.element()));
            for o in &self.others {
                o.set(false);
            }
            self.menu.open.update(|b| *b = !*b);
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
impl_view_for_leaf!(MbBtnLeaf);

fn mbtn(id: u8, label: &'static str, state: &EditorState) -> MbBtnLeaf {
    MbBtnLeaf {
        label,
        menu: state.menus.by_id(id).clone(),
        others: state.menus.others(id),
    }
}

pub(crate) fn menu_bar(state: &EditorState) -> impl View + 'static {
    Row::new()
        .height(28.0)
        .fill_width()
        .gap(1.0)
        .padding(Edges::xy(8.0, 0.0))
        .align_items(Align::Center)
        .background(colors::SURFACE)
        .border_bottom(1.0, colors::STROKE_SOFT)
        .child(logo())
        .child(Spacer::fixed(5.0))
        .child(mbtn(1, "File", state))
        .child(mbtn(2, "Edit", state))
        .child(mbtn(3, "View", state))
        .child(mbtn(4, "Object", state))
        .child(mbtn(5, "Window", state))
        .child(mbtn(6, "Help", state))
        .child(Spacer::flex(1.0))
        .child(
            Text::with({
                let layers = state.layers.clone();
                move || format!("{} layers", layers.get().len())
            })
            .size(10.0)
            .font("IBM Plex Mono")
            .color(colors::ICON_INACTIVE),
        )
}

#[derive(Clone)]
struct AnchoredDrop {
    menu: Menu,
    rows: Vec<AnyView>,
}

impl View for AnchoredDrop {
    fn body(&self) -> impl View + 'static {
        match self.menu.anchor.get() {
            Some(id) => AnyView::new(
                Popper::new(self.menu.open.clone())
                    .anchor_view(id)
                    .placement(Placement::BottomStart)
                    .offset(0.0, 4.0)
                    .dismiss(Dismiss::OutsideClick)
                    .child(mdrop_content(self.menu.open.clone(), self.rows.clone())),
            ),
            None => AnyView::new(Unit),
        }
    }
}

// ---------------------------------------------------------------- drops

pub(crate) fn menu_drops(state: &EditorState) -> impl View + 'static {
    let rows = |id: u8, st: &EditorState| -> Vec<AnyView> {
        let o = st.menus.by_id(id).open.clone();
        match id {
            // ---------------- FILE ----------------
            1 => {
                let s_new = st.clone();
                let s_exp = st.clone();
                let s_jpg = st.clone();
                vec![
                    mrow("New", Some("Ctrl+N"), false, o.clone(), move || {
                        s_new.new_document()
                    }),
                    msep(),
                    mrow("Export JSON to clipboard", None, false, o.clone(), {
                        let s = s_exp.clone();
                        move || s.toast_warn("JSON export — not implemented yet")
                    }),
                    mrow("Export selection as SVG", None, false, o.clone(), {
                        let s = s_jpg.clone();
                        move || s.export_svg()
                    }),
                    mrow("Export selection as PNG", None, false, o.clone(), {
                        let s = s_exp.clone();
                        move || s.toast_warn("PNG export — coming soon")
                    }),
                ]
            }
            // ---------------- HELP ----------------
            6 => {
                let s_h = st.clone();
                let s_a = st.clone();
                vec![
                    mrow(
                        "Keyboard Shortcuts",
                        Some("?"),
                        false,
                        o.clone(),
                        move || s_h.help_open.set(true),
                    ),
                    msep(),
                    mrow("About Editor", None, false, o.clone(), move || {
                        s_a.toast("Vision Design — Rust/Sequoia UI editor")
                    }),
                ]
            }
            // ---------------- EDIT ----------------
            2 => {
                let s_undo = st.clone();
                let s_redo = st.clone();
                let s_dup = st.clone();
                let s_del = st.clone();
                let s_cp = st.clone();
                let s_pst = st.clone();
                let s_lk = st.clone();
                let s_vis = st.clone();
                vec![
                    mrow("Undo", Some("Ctrl+Z"), false, o.clone(), move || {
                        s_undo.undo()
                    }),
                    mrow("Redo", Some("Ctrl+Shift+Z"), false, o.clone(), move || {
                        s_redo.redo()
                    }),
                    msep(),
                    mrow("Cut", Some("Ctrl+X"), false, o.clone(), {
                        let s = s_cp.clone();
                        move || {
                            s.copy_selected();
                            s.delete_selected();
                        }
                    }),
                    mrow("Copy", Some("Ctrl+C"), false, o.clone(), {
                        let s = s_cp.clone();
                        move || s.copy_selected()
                    }),
                    mrow("Paste", Some("Ctrl+V"), false, o.clone(), {
                        let s = s_pst.clone();
                        move || s.paste()
                    }),
                    msep(),
                    mrow("Duplicate", Some("Ctrl+D"), false, o.clone(), {
                        let s = s_dup.clone();
                        move || s.duplicate_selected()
                    }),
                    mrow("Delete", Some("Del"), true, o.clone(), {
                        let s = s_del.clone();
                        move || s.delete_selected()
                    }),
                    msep(),
                    mrow("Lock / Unlock", None, false, o.clone(), {
                        let s = s_lk.clone();
                        move || s.toggle_lock()
                    }),
                    mrow("Show / Hide", None, false, o.clone(), {
                        let s = s_vis.clone();
                        move || s.toggle_visibility()
                    }),
                ]
            }
            // ---------------- VIEW ----------------
            3 => {
                let s_zi = st.clone();
                let s_zo = st.clone();
                let s_100 = st.clone();
                let s_fit = st.clone();
                let s_gr = st.clone();
                let s_gr2 = st.clone();
                vec![
                    mrow("Zoom In", Some("Ctrl++"), false, o.clone(), move || {
                        let z = s_zi.zoom.get_untracked() * 1.25;
                        s_zi.zoom.set(z.clamp(0.1, 8.0));
                    }),
                    mrow("Zoom Out", Some("Ctrl+-"), false, o.clone(), move || {
                        let z = s_zo.zoom.get_untracked() / 1.25;
                        s_zo.zoom.set(z.clamp(0.1, 8.0));
                    }),
                    mrow("Zoom 100%", Some("Ctrl+0"), false, o.clone(), move || {
                        s_100.reset_view()
                    }),
                    msep(),
                    mrow(
                        "Zoom to fit",
                        Some("Shift+1"),
                        false,
                        o.clone(),
                        move || s_fit.zoom_to_fit(),
                    ),
                    mrow("Toggle Grid", None, false, o.clone(), move || {
                        s_gr.toggle_grid()
                    }),
                    mrow("Rulers", None, false, o.clone(), move || {
                        s_gr2.toast_warn("Rulers — coming soon")
                    }),
                ]
            }
            // ---------------- OBJECT ----------------
            4 => {
                let s_fr = st.clone();
                let s_bk = st.clone();
                let s_ff = st.clone();
                let s_bb = st.clone();
                let s_lay = st.clone();
                vec![
                    mrow("Bring to Front", Some("]"), false, o.clone(), move || {
                        s_ff.bring_to_front()
                    }),
                    mrow("Bring Forward", None, false, o.clone(), move || {
                        s_fr.bring_forward()
                    }),
                    mrow("Send Backward", None, false, o.clone(), move || {
                        s_bk.send_backward()
                    }),
                    mrow("Send to Back", Some("["), false, o.clone(), move || {
                        s_bb.send_to_back()
                    }),
                    msep(),
                    mrow("Auto layout", None, false, o.clone(), {
                        let s = s_lay.clone();
                        move || s.toast_warn("Auto layout — coming soon")
                    }),
                ]
            }
            // ---------------- WINDOW ----------------
            5 => {
                let s_v = st.clone();
                vec![mrow(
                    "Variables Library",
                    None,
                    false,
                    o.clone(),
                    move || s_v.toast_warn("Variables — coming soon"),
                )]
            }
            // ---------------- HELP ----------------
            _ => {
                let s_h = st.clone();
                let s_a = st.clone();
                vec![
                    mrow(
                        "Keyboard Shortcuts",
                        Some("?"),
                        false,
                        o.clone(),
                        move || s_h.toast_warn("Help — coming soon"),
                    ),
                    msep(),
                    mrow("About Editor", None, false, o.clone(), move || {
                        s_a.toast("Vision Design — Rust/Sequoia UI editor")
                    }),
                ]
            }
        }
    };

    let st = state.clone();
    Container::new()
        .width(0.0)
        .height(0.0)
        .child(Column::new().children(vec![
            AnyView::new(AnchoredDrop {
                menu: state.menus.file.clone(),
                rows: rows(1, &st),
            }),
            AnyView::new(AnchoredDrop {
                menu: state.menus.edit.clone(),
                rows: rows(2, &st),
            }),
            AnyView::new(AnchoredDrop {
                menu: state.menus.view.clone(),
                rows: rows(3, &st),
            }),
            AnyView::new(AnchoredDrop {
                menu: state.menus.object.clone(),
                rows: rows(4, &st),
            }),
            AnyView::new(AnchoredDrop {
                menu: state.menus.window.clone(),
                rows: rows(5, &st),
            }),
            AnyView::new(AnchoredDrop {
                menu: state.menus.help.clone(),
                rows: rows(6, &st),
            }),
        ]))
}
