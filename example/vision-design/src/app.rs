use sequoia_ui::prelude::*;

use crate::colors;
use crate::state::{EditorState, ToastEntry, ToastKind};
use crate::views;

pub(crate) fn editor_theme() -> Theme {
    let mut t = Theme::dark();
    t.background = colors::BG_DARKER;
    t.surface = colors::SURFACE;
    t.text = colors::TEXT;
    t.border = colors::STROKE_SOFT;
    t.accent = colors::SEL;
    t.radius = 6.0;
    t
}

// -------------------------- toasts overlay --------------------------

fn toast_item(t: ToastEntry, state: &EditorState) -> impl View + 'static {
    let dot = match t.kind {
        ToastKind::Info => colors::OK,
        ToastKind::Warn => colors::WARN,
        ToastKind::Error => colors::DANGER,
    };
    let st = state.clone();
    let id = t.id;
    Container::new()
        .padding(Edges::xy(12.0, 7.0))
        .radius(7.0)
        .background(Color::hex(0x2C2C2E))
        .border(1.0, colors::STROKE)
        .on_click(move || st.dismiss_toast(id))
        .child(
            Row::new()
                .gap(8.0)
                .align_items(Align::Center)
                .child(
                    Container::new()
                        .width(7.0)
                        .height(7.0)
                        .radius(3.5)
                        .background(dot),
                )
                .child(Text::new(t.msg).size(11.0).color(Color::hex(0xF2F2F2))),
        )
        .shadow(ShadowSpec::new(Color::hexa(0x00000080), 30.0).offset(0.0, 10.0))
}

#[derive(Clone)]
struct ToastsView {
    state: EditorState,
}

impl View for ToastsView {
    fn body(&self) -> impl View + 'static {
        let toasts = self.state.toasts.get();
        let vw = self.state.viewport_w.get();
        let vh = self.state.viewport_h.get();
        let st = self.state.clone();
        let open: Signal<bool> = Signal::new(true);
        let items: Vec<AnyView> = toasts
            .into_iter()
            .map(|t| AnyView::new(toast_item(t, &st)))
            .collect();
        Popper::new(open)
            .anchor_rect(Rect::from_xywh(vw / 2.0, vh - 8.0, 0.0, 0.0))
            .placement(Placement::Top)
            .offset(0.0, 8.0)
            .child(
                Column::new()
                    .gap(6.0)
                    .align_items(Align::Center)
                    .children(items),
            )
    }
}

// -------------------------- inline text editor --------------------------

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
                            .border(1.0, colors::SEL)
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

#[derive(Clone)]
struct CommandPalette {
    state: EditorState,
}

impl View for CommandPalette {
    fn body(&self) -> impl View + 'static {
        let open = self.state.cmd_k_open.clone();
        if !open.get() {
            return AnyView::new(Unit);
        }

        let query_sig = self.state.cmdk_query.clone();
        let query = query_sig.get().to_lowercase();
        let st = self.state.clone();

        let actions = vec![
            ("New Frame", "F", 1),
            ("New Rectangle", "R", 2),
            ("New Ellipse", "O", 3),
            ("Zoom to Fit", "Shift+1", 4),
            ("Zoom 100%", "0", 5),
            ("Toggle Grid", "Ctrl+'", 6),
            ("Export to SVG", "", 7),
            ("Undo", "Ctrl+Z", 8),
            ("Redo", "Ctrl+Shift+Z", 9),
            ("Duplicate", "Ctrl+D", 10),
            ("Delete", "Del", 11),
            ("Keyboard Shortcuts", "?", 12),
        ];

        let mut list_ui = Column::new().gap(2.0).fill_width();
        let mut has_results = false;

        for (label, hint, id) in actions {
            if query.is_empty() || label.to_lowercase().contains(&query) {
                has_results = true;
                let st_click = st.clone();
                list_ui = list_ui.child(
                    Container::new()
                        .fill_width()
                        .padding(Edges::xy(10.0, 8.0))
                        .radius(6.0)
                        .hover(colors::SEL)
                        .on_click(move || {
                            st_click.cmd_k_open.set(false);
                            match id {
                                1 => {
                                    st_click.tool.set("F");
                                }
                                2 => {
                                    st_click.tool.set("R");
                                }
                                3 => {
                                    st_click.tool.set("O");
                                }
                                4 => {
                                    st_click.zoom_to_fit();
                                }
                                5 => {
                                    st_click.reset_view();
                                }
                                6 => {
                                    st_click.toggle_grid();
                                }
                                7 => {
                                    st_click.export_svg();
                                }
                                8 => {
                                    st_click.undo();
                                }
                                9 => {
                                    st_click.redo();
                                }
                                10 => {
                                    st_click.duplicate_selected();
                                }
                                11 => {
                                    st_click.delete_selected();
                                }
                                12 => {
                                    st_click.help_open.set(true);
                                }
                                _ => {}
                            }
                        })
                        .child(
                            Row::new()
                                .align_items(Align::Center)
                                .child(Text::new(label).size(12.0).color(Color::WHITE))
                                .child(Spacer::flex(1.0))
                                .child(
                                    Container::new()
                                        .padding(Edges::xy(4.0, 1.0))
                                        .radius(4.0)
                                        .background(Color::hex(0x2E2E30))
                                        .child(
                                            Text::new(hint)
                                                .size(9.5)
                                                .font("IBM Plex Mono")
                                                .color(Color::hex(0xAEAEAE)),
                                        ),
                                ),
                        ),
                );
            }
        }

        if !has_results {
            list_ui = list_ui.child(
                Container::new().padding(Edges::xy(10.0, 10.0)).child(
                    Text::new("No results")
                        .size(12.0)
                        .color(colors::ICON_INACTIVE),
                ),
            );
        }

        AnyView::new(
            Container::new()
                .fill_size()
                .background(Color::hexa(0x00000099))
                .center()
                .on_click({
                    let st = st.clone();
                    move || st.cmd_k_open.set(false)
                })
                .child(
                    Container::new()
                        .width(520.0)
                        .background(Color::hex(0x232324))
                        .border(1.0, colors::STROKE)
                        .radius(12.0)
                        .overflow(Overflow::Hidden)
                        .on_click(|| {})
                        .child(
                            Column::new()
                                .child(
                                    Container::new()
                                        .padding(Edges::xy(12.0, 12.0))
                                        .border_bottom(1.0, colors::STROKE)
                                        .child(
                                            TextField::new(query_sig)
                                                .placeholder("Type a command…")
                                                .font_size(13.0)
                                                .text_color(Color::WHITE)
                                                .background(Color::TRANSPARENT)
                                                .border(0.0, Color::TRANSPARENT),
                                        ),
                                )
                                .child(
                                    ScrollView::new(
                                        Container::new().padding(Edges::all(6.0)).child(list_ui),
                                    )
                                    .vertical(),
                                ),
                        ),
                ),
        )
    }
}

// -------------------------- Help Overlay --------------------------

#[derive(Clone)]
struct HelpOverlay {
    state: EditorState,
}

impl View for HelpOverlay {
    fn body(&self) -> impl View + 'static {
        let open = self.state.help_open.clone();
        if !open.get() {
            return AnyView::new(Unit);
        }
        let st = self.state.clone();

        let shortcuts = vec![
            ("Select / Hand", "V · H"),
            ("Frame / Rect", "F · R"),
            ("Ellipse / Text", "O · T"),
            ("Duplicate / Delete", "Ctrl+D / Del"),
            ("Undo / Redo", "Ctrl+Z / Ctrl+Shift+Z"),
            ("Zoom / Fit", "Ctrl+Wheel / Shift+1"),
            ("Pan Canvas", "Space + Drag / Middle Click"),
            ("Command Palette", "Ctrl+K"),
            ("Inline Text Edit", "Double click text"),
        ];

        let mut grid = Column::new().gap(4.0).fill_width();
        for (k, v) in shortcuts {
            grid = grid.child(
                Row::new()
                    .fill_width()
                    .padding(Edges::xy(0.0, 4.0))
                    .border_bottom(1.0, colors::STROKE_SOFT)
                    .child(Text::new(k).size(11.0).color(Color::hex(0xC9C9C9)))
                    .child(Spacer::flex(1.0))
                    .child(
                        Container::new()
                            .padding(Edges::xy(5.0, 2.0))
                            .background(Color::hex(0x2E2E30))
                            .border(1.0, colors::STROKE)
                            .radius(4.0)
                            .child(
                                Text::new(v)
                                    .size(9.5)
                                    .font("IBM Plex Mono")
                                    .color(Color::hex(0xAEAEAE)),
                            ),
                    ),
            );
        }

        AnyView::new(
            Container::new()
                .fill_size()
                .background(Color::hexa(0x00000099))
                .center()
                .on_click({
                    let st = st.clone();
                    move || st.help_open.set(false)
                })
                .child(
                    Container::new()
                        .width(540.0)
                        .background(Color::hex(0x232324))
                        .border(1.0, colors::STROKE)
                        .radius(12.0)
                        .padding(Edges::all(16.0))
                        .on_click(|| {})
                        .child(
                            Column::new()
                                .gap(10.0)
                                .child(
                                    Text::new("Keyboard Shortcuts")
                                        .size(14.0)
                                        .semibold()
                                        .color(Color::WHITE),
                                )
                                .child(
                                    Text::new("Press Esc to close")
                                        .size(11.0)
                                        .color(colors::ICON_INACTIVE),
                                )
                                .child(grid),
                        ),
                ),
        )
    }
}

// -------------------------- root --------------------------

pub(crate) fn root(state: EditorState) -> impl View + 'static {
    Stack::new()
        .fill_size()
        .child(
            Column::new()
                .fill_size()
                .child(views::menu_bar(&state))
                .child(
                    Row::new().fill_size().child(views::toolbar(&state)).child(
                        Column::new()
                            .fill_size()
                            .child(views::topbar(&state))
                            .child(
                                Row::new()
                                    .fill_size()
                                    .child(views::panel_left(&state))
                                    .child(views::canvas(&state))
                                    .child(views::panel_right(&state)),
                            ),
                    ),
                ),
        )
        .child(views::menu_drops(&state))
        .child(views::context_menu(&state))
        .child(TextEditOverlay {
            state: state.clone(),
        })
        .child(CommandPalette {
            state: state.clone(),
        })
        .child(HelpOverlay {
            state: state.clone(),
        })
        .child(views::color_picker(&state))
        .child(ToastsView {
            state: state.clone(),
        })
}
