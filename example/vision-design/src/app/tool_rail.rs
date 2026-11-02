//! Toolbar 1:1 with CSS: .toolbar / .tool-group / .tool-sep / .tool-btn(+.active::after) / .spacer
use sequoia_ui::prelude::*;

use crate::state::EditorState;
use crate::theme::colors;

#[derive(Clone, Copy)]
pub(crate) struct ToolDef {
    pub(crate) key: &'static str,  // badge data-key + id
    pub(crate) icon: &'static str, // AssetRegistry name
    #[allow(dead_code)]
    pub(crate) label: &'static str, // tool label (statusbar, stage 6)
}

const GROUPS: &[&[ToolDef]] = &[
    &[
        ToolDef {
            key: "V",
            icon: "tool/select",
            label: "Zaznaczanie",
        },
        ToolDef {
            key: "H",
            icon: "tool/hand",
            label: "Łapka",
        },
    ],
    &[
        ToolDef {
            key: "F",
            icon: "tool/frame",
            label: "Ramka",
        },
        ToolDef {
            key: "R",
            icon: "tool/rect",
            label: "Prostokąt",
        },
        ToolDef {
            key: "O",
            icon: "tool/ellipse",
            label: "Elipsa",
        },
        ToolDef {
            key: "L",
            icon: "tool/line",
            label: "Linia",
        },
    ],
    &[
        ToolDef {
            key: "T",
            icon: "tool/text",
            label: "Tekst",
        },
        ToolDef {
            key: "C",
            icon: "tool/comment",
            label: "Komentarz",
        },
    ],
];

/// .tool-btn — 30x30, radius 7; hover = --bg-lighter; active = --bg-lighter
/// + inset ring 1px --stroke + badge data-key (mono 7px, opacity .55) in the bottom-right corner
#[derive(Clone)]
struct ToolBtn {
    def: ToolDef,
    current: Signal<&'static str>,
}

impl View for ToolBtn {
    fn body(&self) -> impl View + 'static {
        let on = self.current.get() == self.def.key; // tracked: rebuild on tool change
        let key = self.def.key;
        let current = self.current.clone();

        let icon = Icon::new(self.def.icon).size(16.0).color(if on {
            Color::WHITE
        } else {
            colors::active().icon_inactive
        });

        let mut stack = Stack::new().child(icon);
        if on {
            // .tool-btn.active::after — right:2px; bottom:1px; mono 7px; opacity .55
            stack = stack.child(
                Container::new()
                    .fill_size()
                    .justify_content(Justify::End)
                    .align_items(Align::End)
                    .padding(Edges::xy(2.0, 1.0))
                    .child(
                        Text::new(key)
                            .size(7.0)
                            .font("IBM Plex Mono")
                            .color(Color::WHITE)
                            .opacity(0.55),
                    ),
            );
        }

        Container::new()
            .width(30.0)
            .height(30.0)
            .center()
            .radius(7.0)
            .hover(colors::active().bg_lighter)
            .style(if on {
                // inset 0 0 0 1px var(--stroke) as a 1px border (box is fixed at 30x30)
                BoxStyle::new()
                    .background(colors::active().bg_lighter)
                    .border(1.0, colors::active().stroke)
            } else {
                BoxStyle::new()
            })
            .on_click(move || current.set(key))
            .child(stack)
    }
}

/// .tool-sep — 20x1, --stroke, margin 6px 0
fn tool_sep() -> impl View + 'static {
    Container::new()
        .width(20.0)
        .height(1.0)
        .margin(Edges::xy(0.0, 6.0))
        .background(colors::active().stroke)
}

/// .tool-group — column, gap 1, width 100%, center
fn tool_group(state: &EditorState, group: &[ToolDef]) -> impl View + 'static {
    let mut col = Column::new()
        .gap(1.0)
        .fill_width()
        .align_items(Align::Center);
    for def in group {
        col = col.child(ToolBtn {
            def: *def,
            current: state.tool.clone(),
        });
    }
    col
}

/// .toolbar — 44px, surface, border-right, padding 8px 0, gap 1, spacer at the bottom
pub(crate) fn tool_rail(state: &EditorState) -> impl View + 'static {
    let mut col = Column::new()
        .width(44.0)
        .fill_height()
        .background(colors::active().surface)
        .border_right(1.0, colors::active().stroke_soft)
        .padding(Edges::xy(0.0, 8.0))
        .gap(1.0)
        .align_items(Align::Center);

    for (i, group) in GROUPS.iter().enumerate() {
        if i > 0 {
            col = col.child(tool_sep());
        }
        col = col.child(tool_group(state, group));
    }
    col.child(Spacer::vertical(1.0)) // .toolbar .spacer{flex:1}
}