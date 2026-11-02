use sequoia_ui::prelude::*;

use crate::state::EditorState;
use crate::theme::colors;

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

        let actions: Vec<(&'static str, &'static str, u8)> = vec![
            ("Kod Svelte", "Ctrl+E", 13),
            ("Nowa ramka", "F", 1),
            ("Nowy prostokąt", "R", 2),
            ("Nowa elipsa", "O", 3),
            ("Auto layout zaznaczenia", "", 18),
            ("Dopasuj do okna", "Shift+1", 4),
            ("Zoom 100%", "Ctrl+0", 5),
            ("Przełącz siatkę", "", 6),
            ("Eksport SVG", "", 7),
            ("Eksportuj JSON do schowka", "", 14),
            ("Importuj JSON ze schowka", "", 15),
            ("Zapisz kod .svelte", "", 16),
            ("Zapisz dokument .json", "", 17),
            ("Cofnij", "Ctrl+Z", 8),
            ("Ponów", "Ctrl+Shift+Z", 9),
            ("Duplikuj", "Ctrl+D", 10),
            ("Usuń", "Del", 11),
            ("Skróty klawiszowe", "?", 12),
        ];

        let mut list_ui = Column::new().gap(2.0).fill_width();
        let mut has_results = false;

        for (label, hint, id) in actions {
            if query.is_empty() || label.to_lowercase().contains(&query) {
                has_results = true;
                let st_click = st.clone();
                let needs_env = matches!(id, 14 | 15);
                let row = Row::new()
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
                    );

                let container = if needs_env {
                    Container::new()
                        .fill_width()
                        .padding(Edges::xy(10.0, 8.0))
                        .radius(6.0)
                        .hover(colors::active().sel)
                        .on_click_env(move |env: &Environment| {
                            st_click.cmd_k_open.set(false);
                            match id {
                                14 => st_click.export_json_clipboard(env),
                                15 => st_click.import_json_clipboard(env),
                                _ => {}
                            }
                        })
                        .child(row)
                } else {
                    Container::new()
                        .fill_width()
                        .padding(Edges::xy(10.0, 8.0))
                        .radius(6.0)
                        .hover(colors::active().sel)
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
                                13 => {
                                    st_click.open_code_tab();
                                }
                                16 => {
                                    st_click.save_svelte_file();
                                }
                                17 => {
                                    st_click.save_json_file();
                                }
                                18 => {
                                    st_click.auto_layout_selection();
                                }
                                _ => {}
                            }
                        })
                        .child(row)
                };

                list_ui = list_ui.child(container);
            }
        }

        if !has_results {
            list_ui = list_ui.child(
                Container::new().padding(Edges::xy(10.0, 10.0)).child(
                    Text::new("Brak wyników")
                        .size(12.0)
                        .color(colors::active().icon_inactive),
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
                        .border(1.0, colors::active().stroke)
                        .radius(12.0)
                        .overflow(Overflow::Hidden)
                        .on_click(|| {})
                        .child(
                            Column::new()
                                .child(
                                    Container::new()
                                        .padding(Edges::xy(12.0, 12.0))
                                        .border_bottom(1.0, colors::active().stroke)
                                        .child(
                                            TextField::new(query_sig)
                                                .placeholder("Wpisz komendę…")
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

pub(crate) fn command_palette(state: &EditorState) -> impl View + 'static {
    CommandPalette {
        state: state.clone(),
    }
}
