use sequoia_ui::layout::LayoutCtx;
use sequoia_ui::prelude::*;

use crate::components::menu::{menu_row as mrow, menu_row_env as mrow_env, menu_sep as msep};
use sequoia_ui::view::{Fill, GradientStops};

use crate::state::{EditorState, Menu};
use crate::theme::colors;

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

fn mdrop_content(open: Signal<bool>, rows: Vec<AnyView>) -> impl View + 'static {
    DismissableLayer::wrap(
        Container::new()
            .width(232.0)
            .padding(4.0)
            .radius(8.0)
            .background(Color::hex(0x2A2A2C))
            .border(1.0, colors::active().stroke)
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
            colors::active().bg_lighter
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
        .background(colors::active().surface)
        .border_bottom(1.0, colors::active().stroke_soft)
        .child(logo())
        .child(Spacer::fixed(5.0))
        .child(mbtn(1, "Plik", state))
        .child(mbtn(2, "Edycja", state))
        .child(mbtn(3, "Widok", state))
        .child(mbtn(4, "Obiekt", state))
        .child(mbtn(5, "Okno", state))
        .child(mbtn(6, "Pomoc", state))
        .child(Spacer::flex(1.0))
        .child(
            Text::with({
                let layers = state.layers.clone();
                move || format!("{} warstw", layers.get().len())
            })
            .size(10.0)
            .font("IBM Plex Mono")
            .color(colors::active().icon_inactive),
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
                let s_svelte = st.clone();
                let s_json_file = st.clone();
                let s_json_in = st.clone();
                let s_png = st.clone();
                vec![
                    mrow("Nowy dokument", Some("Ctrl+N"), false, o.clone(), move || {
                        s_new.new_document()
                    }),
                    msep(),
                    mrow_env(
                        "Eksportuj JSON do schowka",
                        None,
                        o.clone(),
                        move |env| s_json_in.export_json_clipboard(env),
                    ),
                    mrow("Zapisz JSON do pliku", None, false, o.clone(), move || {
                        s_json_file.save_json_file()
                    }),
                    mrow_env(
                        "Importuj JSON ze schowka",
                        None,
                        o.clone(),
                        move |env| s_exp.import_json_clipboard(env),
                    ),
                    msep(),
                    mrow("Zapisz kod Svelte (.svelte)", None, false, o.clone(), move || {
                        s_svelte.save_svelte_file()
                    }),
                    mrow("Eksportuj zaznaczenie jako SVG", None, false, o.clone(), {
                        let s = s_jpg.clone();
                        move || s.export_svg()
                    }),
                    mrow("Eksport PNG — wkrótce", None, false, o.clone(), {
                        let s = s_png.clone();
                        move || s.toast_warn("Eksport PNG — wkrótce")
                    }),
                ]
            }
            // ---------------- HELP ----------------
            6 => {
                let s_h = st.clone();
                let s_a = st.clone();
                vec![
                    mrow(
                        "Skróty klawiszowe",
                        Some("?"),
                        false,
                        o.clone(),
                        move || s_h.help_open.set(true),
                    ),
                    msep(),
                    mrow("O edytorze", None, false, o.clone(), move || {
                        s_a.toast("Vision Design — edytor Rust/Sequoia UI, kod: Svelte")
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
                    mrow("Cofnij", Some("Ctrl+Z"), false, o.clone(), move || {
                        s_undo.undo()
                    }),
                    mrow("Ponów", Some("Ctrl+Shift+Z"), false, o.clone(), move || {
                        s_redo.redo()
                    }),
                    msep(),
                    mrow("Wytnij", Some("Ctrl+X"), false, o.clone(), {
                        let s = s_cp.clone();
                        move || {
                            s.copy_selected();
                            s.delete_selected();
                        }
                    }),
                    mrow("Kopiuj", Some("Ctrl+C"), false, o.clone(), {
                        let s = s_cp.clone();
                        move || s.copy_selected()
                    }),
                    mrow("Wklej", Some("Ctrl+V"), false, o.clone(), {
                        let s = s_pst.clone();
                        move || s.paste()
                    }),
                    msep(),
                    mrow("Duplikuj", Some("Ctrl+D"), false, o.clone(), {
                        let s = s_dup.clone();
                        move || s.duplicate_selected()
                    }),
                    mrow("Usuń", Some("Del"), true, o.clone(), {
                        let s = s_del.clone();
                        move || s.delete_selected()
                    }),
                    msep(),
                    mrow("Zablokuj / odblokuj", None, false, o.clone(), {
                        let s = s_lk.clone();
                        move || s.toggle_lock()
                    }),
                    mrow("Pokaż / ukryj", None, false, o.clone(), {
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
                let s_code = st.clone();
                vec![
                    mrow("Kod Svelte", Some("Ctrl+E"), false, o.clone(), move || {
                        s_code.open_code_tab()
                    }),
                    msep(),
                    mrow("Przybliż", Some("Ctrl++"), false, o.clone(), move || {
                        let z = s_zi.zoom.get_untracked() * 1.25;
                        s_zi.zoom.set(z.clamp(0.1, 8.0));
                    }),
                    mrow("Oddal", Some("Ctrl+-"), false, o.clone(), move || {
                        let z = s_zo.zoom.get_untracked() / 1.25;
                        s_zo.zoom.set(z.clamp(0.1, 8.0));
                    }),
                    mrow("Zoom 100%", Some("Ctrl+0"), false, o.clone(), move || {
                        s_100.reset_view()
                    }),
                    msep(),
                    mrow(
                        "Dopasuj do okna",
                        Some("Shift+1"),
                        false,
                        o.clone(),
                        move || s_fit.zoom_to_fit(),
                    ),
                    mrow("Przełącz siatkę", None, false, o.clone(), move || {
                        s_gr.toggle_grid()
                    }),
                    mrow("Linijki — wkrótce", None, false, o.clone(), move || {
                        s_gr2.toast_warn("Linijki — wkrótce")
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
                    mrow("Na wierzch", Some("]"), false, o.clone(), move || {
                        s_ff.bring_to_front()
                    }),
                    mrow("Wyżej", None, false, o.clone(), move || {
                        s_fr.bring_forward()
                    }),
                    mrow("Niżej", None, false, o.clone(), move || {
                        s_bk.send_backward()
                    }),
                    mrow("Na spód", Some("["), false, o.clone(), move || {
                        s_bb.send_to_back()
                    }),
                    msep(),
                    mrow("Auto layout (H → V → G)", None, false, o.clone(), {
                        let s = s_lay.clone();
                        move || s.auto_layout_selection()
                    }),
                ]
            }
            // ---------------- WINDOW ----------------
            5 => {
                let s_v = st.clone();
                let s_code = st.clone();
                vec![
                    mrow("Biblioteka zmiennych", None, false, o.clone(), move || {
                        s_v.toast_warn("Biblioteka zmiennych — otwórz Node Graph z paska górnego")
                    }),
                    msep(),
                    mrow("Kod Svelte", Some("Ctrl+E"), false, o.clone(), move || {
                        s_code.open_code_tab()
                    }),
                ]
            }
            // ---------------- HELP (fallback) ----------------
            _ => {
                let s_h = st.clone();
                let s_a = st.clone();
                vec![
                    mrow(
                        "Skróty klawiszowe",
                        Some("?"),
                        false,
                        o.clone(),
                        move || s_h.help_open.set(true),
                    ),
                    msep(),
                    mrow("O edytorze", None, false, o.clone(), move || {
                        s_a.toast("Vision Design — edytor Rust/Sequoia UI, kod: Svelte")
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