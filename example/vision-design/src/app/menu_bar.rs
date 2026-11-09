use sequoia_ui::layout::LayoutCtx;
use sequoia_ui::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::components::menu::{menu_row as mrow, menu_row_env as mrow_env, menu_sep as msep};
use sequoia_ui::view::{Fill, GradientStops};

use crate::state::{EditorState, Menu, ProjectTab};
use crate::theme::colors;

const TITLE_H: f32 = 34.0;

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

/// Invisible background of the title bar: dragging any empty spot moves the
/// window, a double click maximizes or restores it. It sits below the
/// interactive row, so tabs/menu buttons win.
#[derive(Clone)]
struct CaptionDrag {
    /// Carried by the editor state so every window (and every test) keeps its
    /// own click history instead of sharing a thread-local.
    last_click: Rc<RefCell<Option<(Instant, Point)>>>,
}

impl Leaf for CaptionDrag {
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
            .width(Length::Fill(1.0))
            .height(Length::Fill(1.0))
    }

    fn paint(&self, _ctx: &mut PaintCtx) {}

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if let Event::Pointer(PointerEvent::Down {
            button: PointerButton::Left,
            pos,
            ..
        }) = event
            && let Some(windows) = ctx.environment.get::<Windows>()
        {
            let now = Instant::now();
            let mut last = self.last_click.borrow_mut();
            let double = (*last).is_some_and(|(then, at)| {
                now.duration_since(then) < Duration::from_millis(400)
                    && (at.x - pos.x).hypot(at.y - pos.y) < 6.0
            });
            *last = if double { None } else { Some((now, *pos)) };
            if double {
                windows.toggle_maximize(ctx.window);
            } else {
                windows.begin_drag(ctx.window);
            }
            ctx.handled();
        }
    }
}
impl_view_for_leaf!(CaptionDrag);

// ---------------------------------------------------------------- menu icon

#[derive(Clone)]
struct MbIconLeaf {
    menu: Menu,
    others: Vec<Signal<bool>>,
}

impl Leaf for MbIconLeaf {
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
            .width(Length::Px(24.0))
            .height(Length::Px(22.0))
    }

    fn intrinsic_size(&self, _ctx: &mut LayoutCtx) -> Option<Size> {
        Some(Size::new(24.0, 22.0))
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
        let x = ctx.rect.left() + 7.0;
        let y = ctx.rect.top() + 7.5;
        for i in 0..3 {
            ctx.quad(Rect::from_xywh(x, y + i as f32 * 4.0, 10.0, 1.5), fg, 0.75);
        }
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
impl_view_for_leaf!(MbIconLeaf);

fn menu_icon(state: &EditorState) -> MbIconLeaf {
    MbIconLeaf {
        menu: state.menus.file.clone(),
        others: state.menus.others(1),
    }
}

// ---------------------------------------------------------------- projects

#[derive(Clone)]
struct ProjectTabs {
    state: EditorState,
}

impl View for ProjectTabs {
    fn body(&self) -> impl View + 'static {
        let tabs = self.state.projects.get();
        let active = self.state.active_project.get();
        let children: Vec<AnyView> = tabs
            .iter()
            .map(|tab| project_tab(&self.state, tab, tab.id == active))
            .collect();
        Row::new()
            .gap(3.0)
            .align_items(Align::Center)
            .children(children)
    }
}

fn project_tab(state: &EditorState, tab: &ProjectTab, active: bool) -> AnyView {
    let switch = state.clone();
    let id = tab.id;
    let close_state = state.clone();
    // The close target is deliberately larger than the glyph (18 vs 10) so the
    // "×" stays clickable and readable at title-bar size, the same way native
    // window tabs do it.
    let close = Container::new()
        .width(18.0)
        .height(18.0)
        .center()
        .radius(4.0)
        .hover(colors::active().bg_darker)
        .on_press(move || close_state.close_project(id))
        .child(
            Icon::new("close")
                // The bundled glyph only fills ~52% of its 24-unit viewBox, so
                // 15 renders an "×" of roughly 8 px next to 11 px text.
                .size(15.0)
                .color(colors::active().icon_active),
        );

    let mut tab_box = Container::new()
        .height(26.0)
        // Without a minimum width the tab is sized by the label's advance, so
        // two names of slightly different length produce visibly different
        // cards. A shared floor makes the label/close pair sit at the same
        // place in every tab; longer names still grow past it.
        .min_width(120.0)
        .padding(Edges::xy(10.0, 0.0))
        // Container aligns children to the start by default, so the 18px
        // label/close row would hug the top edge of the 26px tab.
        .center()
        .radius(6.0)
        // Inactive tabs keep the same 1px frame as the active one (transparent
        // instead of omitted) so the label and close glyph do not shift by a
        // pixel when the selection moves between tabs.
        .border(1.0, Color::TRANSPARENT)
        .hover(colors::active().bg_lighter)
        .on_click(move || switch.switch_project(id))
        .child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(
                    Text::new(tab.name.clone())
                        .size(11.0)
                        .semibold()
                        .max_lines(1)
                        .color(if active {
                            Color::hex(0xEDEDED)
                        } else {
                            colors::active().icon_active
                        }),
                )
                .child(close),
        );
    tab_box = if active {
        tab_box
            .background(colors::active().bg_lighter)
            .border(1.0, colors::active().stroke_soft)
    } else {
        // A resting background gives the inactive tab the same visible card
        // bounds as the active one; without it the label/icon read as shifted
        // inside an invisible box.
        tab_box.background(colors::active().stroke_soft)
    };
    AnyView::new(tab_box)
}

fn add_project(state: &EditorState) -> impl View + 'static {
    let st = state.clone();
    Container::new()
        .width(24.0)
        .height(24.0)
        .center()
        .radius(6.0)
        .hover(colors::active().bg_lighter)
        .on_press(move || st.new_project())
        .child(
            Text::new("+")
                .size(15.0)
                .color(colors::active().icon_inactive),
        )
}

// ---------------------------------------------------------------- window

fn window_button(
    icon: &'static str,
    hover: Color,
    action: impl Fn(&Windows) + 'static,
) -> impl View + 'static {
    Container::new()
        .width(46.0)
        .height(TITLE_H)
        .center()
        .hover(hover)
        .on_click_env(move |env| {
            if let Some(windows) = env.get::<Windows>() {
                action(&windows);
            }
        })
        .child(
            Icon::new(icon)
                .size(10.0)
                .color(colors::active().icon_active),
        )
}

fn window_controls() -> impl View + 'static {
    Row::new()
        .height(TITLE_H)
        .align_items(Align::Center)
        .child(window_button(
            "minimize",
            colors::active().bg_lighter,
            |w| {
                w.minimize(WindowId::PRIMARY);
            },
        ))
        .child(window_button(
            "maximize",
            colors::active().bg_lighter,
            |w| {
                w.toggle_maximize(WindowId::PRIMARY);
            },
        ))
        .child(window_button("close", Color::hex(0xE81123), |w| {
            w.close(WindowId::PRIMARY);
        }))
}

fn title_divider() -> impl View + 'static {
    Container::new()
        .width(1.0)
        .height(16.0)
        .background(colors::active().stroke_soft)
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

/// Custom window title bar, merged with the menu bar: app-menu icon, menus,
/// project tabs and the native-style window controls on the far right.
pub(crate) fn title_bar(state: &EditorState) -> impl View + 'static {
    let layers = state.layers.clone();
    Stack::new()
        .height(TITLE_H)
        .fill_width()
        .background(colors::active().surface)
        .border_bottom(1.0, colors::active().stroke_soft)
        .child(CaptionDrag {
            last_click: state.last_caption_click.clone(),
        })
        .child(
            Row::new()
                .fill_size()
                .gap(1.0)
                .padding(Edges::only(8.0, 0.0, 0.0, 0.0))
                .align_items(Align::Center)
                .hit_testable(false)
                .child(menu_icon(state))
                .child(Spacer::fixed(4.0))
                .child(logo())
                .child(Spacer::fixed(8.0))
                .child(mbtn(1, "File", state))
                .child(mbtn(2, "Edit", state))
                .child(mbtn(3, "View", state))
                .child(mbtn(4, "Object", state))
                .child(mbtn(5, "Window", state))
                .child(mbtn(6, "Help", state))
                .child(Spacer::fixed(8.0))
                .child(title_divider())
                .child(Spacer::fixed(8.0))
                .child(ProjectTabs {
                    state: state.clone(),
                })
                .child(add_project(state))
                .child(Spacer::flex(1.0))
                .child(
                    Text::with(move || format!("{} layers", layers.get().len()))
                        .size(10.0)
                        .font("IBM Plex Mono")
                        .color(colors::active().icon_inactive),
                )
                .child(Spacer::fixed(12.0))
                .child(window_controls()),
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
                let s_project = st.clone();
                let s_json_file = st.clone();
                let s_json_in = st.clone();
                let s_png = st.clone();
                vec![
                    mrow("New project", Some("Ctrl+N"), false, o.clone(), move || {
                        s_new.new_project()
                    }),
                    msep(),
                    mrow_env("Export JSON to clipboard", None, o.clone(), move |env| {
                        s_json_in.export_json_clipboard(env)
                    }),
                    mrow("Save JSON to file", None, false, o.clone(), move || {
                        s_json_file.save_json_file()
                    }),
                    mrow_env("Import JSON from clipboard", None, o.clone(), move |env| {
                        s_exp.import_json_clipboard(env)
                    }),
                    msep(),
                    mrow(
                        "Save Svelte code (.svelte)",
                        None,
                        false,
                        o.clone(),
                        move || s_svelte.save_svelte_file(),
                    ),
                    mrow(
                        "Export Svelte project…",
                        None,
                        false,
                        o.clone(),
                        move || s_project.save_svelte_project(),
                    ),
                    mrow("Export selection as SVG", None, false, o.clone(), {
                        let s = s_jpg.clone();
                        move || s.export_svg()
                    }),
                    mrow("Export PNG — coming soon", None, false, o.clone(), {
                        let s = s_png.clone();
                        move || s.toast_warn("Export PNG — coming soon")
                    }),
                ]
            }
            // ---------------- HELP ----------------
            6 => {
                let s_h = st.clone();
                let s_a = st.clone();
                vec![
                    mrow(
                        "Keyboard shortcuts",
                        Some("?"),
                        false,
                        o.clone(),
                        move || s_h.help_open.set(true),
                    ),
                    msep(),
                    mrow("About Vision Design", None, false, o.clone(), move || {
                        s_a.toast("Vision Design — a Rust/Sequoia UI editor, code: Svelte")
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
                    mrow("Lock / unlock", None, false, o.clone(), {
                        let s = s_lk.clone();
                        move || s.toggle_lock()
                    }),
                    mrow("Show / hide", None, false, o.clone(), {
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
                    mrow("Svelte code", Some("Ctrl+E"), false, o.clone(), move || {
                        s_code.open_code_tab()
                    }),
                    msep(),
                    mrow("Zoom in", Some("Ctrl++"), false, o.clone(), move || {
                        let z = s_zi.zoom.get_untracked() * 1.25;
                        s_zi.zoom.set(z.clamp(0.1, 8.0));
                    }),
                    mrow("Zoom out", Some("Ctrl+-"), false, o.clone(), move || {
                        let z = s_zo.zoom.get_untracked() / 1.25;
                        s_zo.zoom.set(z.clamp(0.1, 8.0));
                    }),
                    mrow("Zoom 100%", Some("Ctrl+0"), false, o.clone(), move || {
                        s_100.reset_view()
                    }),
                    msep(),
                    mrow(
                        "Fit to window",
                        Some("Shift+1"),
                        false,
                        o.clone(),
                        move || s_fit.zoom_to_fit(),
                    ),
                    mrow("Toggle grid", None, false, o.clone(), move || {
                        s_gr.toggle_grid()
                    }),
                    mrow(
                        "Rulers — coming soon",
                        None,
                        false,
                        o.clone(),
                        move || s_gr2.toast_warn("Rulers — coming soon"),
                    ),
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
                    mrow("Bring to front", Some("]"), false, o.clone(), move || {
                        s_ff.bring_to_front()
                    }),
                    mrow("Bring forward", None, false, o.clone(), move || {
                        s_fr.bring_forward()
                    }),
                    mrow("Send backward", None, false, o.clone(), move || {
                        s_bk.send_backward()
                    }),
                    mrow("Send to back", Some("["), false, o.clone(), move || {
                        s_bb.send_to_back()
                    }),
                    msep(),
                    mrow(
                        "Auto layout (H → V → G)",
                        Some("Ctrl+L"),
                        false,
                        o.clone(),
                        {
                            let s = s_lay.clone();
                            move || s.auto_layout_selection()
                        },
                    ),
                ]
            }
            // ---------------- WINDOW ----------------
            5 => {
                let s_v = st.clone();
                let s_code = st.clone();
                vec![
                    mrow("Variable library", None, false, o.clone(), move || {
                        s_v.toast_warn("Variable library — open Node Graph from the top bar")
                    }),
                    msep(),
                    mrow("Svelte code", Some("Ctrl+E"), false, o.clone(), move || {
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
                        "Keyboard shortcuts",
                        Some("?"),
                        false,
                        o.clone(),
                        move || s_h.help_open.set(true),
                    ),
                    msep(),
                    mrow("About Vision Design", None, false, o.clone(), move || {
                        s_a.toast("Vision Design — a Rust/Sequoia UI editor, code: Svelte")
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
