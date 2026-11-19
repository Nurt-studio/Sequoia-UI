//! IDE-style docking built on `DockModel`, `DockView`, and `DockFloatLayer`.
//!
//! The default layout is:
//!
//!   ┌───────────┬────────────────────────────┬────────────┐
//!   │           │   Editor / Preview tabs    │            │
//!   │ Explorer  ├────────────────────────────┤ Inspector  │
//!   │           │          Console           │            │
//!   └───────────┴────────────────────────────┴────────────┘
//!
//! Drag a tab onto another panel to dock it, near an edge to split, or hold
//! Alt while dropping to float it. The toolbar can reset the layout, float the
//! inspector, and save or load it through `DockLayoutStore`.
//!
//! Run with:
//!     cargo run --example dock -p sequoia-ui

use sequoia_ui::prelude::*;

const EXPLORER: PanelId = PanelId::new(1);
const EDITOR: PanelId = PanelId::new(2);
const PREVIEW: PanelId = PanelId::new(3);
const INSPECTOR: PanelId = PanelId::new(4);
const CONSOLE: PanelId = PanelId::new(5);

const APP_BG: Color = Color::hex(0x14161A);
const BAR_BG: Color = Color::hex(0x1E2127);
const PANEL_BG: Color = Color::hex(0x1A1D22);
const STAGE_BG: Color = Color::hex(0x101215);
const TEXT: Color = Color::hex(0xD7DCE5);
const MUTED: Color = Color::hex(0x8A93A6);
const ACCENT: Color = Color::hex(0x7FD1FF);
const CONSOLE_GREEN: Color = Color::hex(0x8FD18A);

fn default_layout() -> DockModel {
    let mut model = DockModel::single(EDITOR);
    let _ = model.apply(DockOp::Insert {
        panel: EXPLORER,
        target: EDITOR,
        drop: DockDrop::Left,
        frac: 0.22,
        moving: false,
    });
    let _ = model.apply(DockOp::Insert {
        panel: INSPECTOR,
        target: EDITOR,
        drop: DockDrop::Right,
        frac: 0.76,
        moving: false,
    });
    let _ = model.apply(DockOp::Insert {
        panel: CONSOLE,
        target: EDITOR,
        drop: DockDrop::Bottom,
        frac: 0.68,
        moving: false,
    });
    let _ = model.apply(DockOp::Insert {
        panel: PREVIEW,
        target: EDITOR,
        drop: DockDrop::Tab,
        frac: 0.5,
        moving: false,
    });
    model
}

fn panel_title(panel: PanelId) -> String {
    match panel {
        EXPLORER => String::from("Explorer"),
        EDITOR => String::from("Editor"),
        PREVIEW => String::from("Preview"),
        INSPECTOR => String::from("Inspector"),
        CONSOLE => String::from("Console"),
        other => format!("Panel {}", other.get()),
    }
}

fn panel_view(panel: PanelId) -> AnyView {
    match panel {
        EXPLORER => explorer(),
        EDITOR => editor(),
        PREVIEW => preview(),
        INSPECTOR => inspector(),
        CONSOLE => console(),
        other => AnyView::new(
            Text::new(format!("No content for panel {}", other.get()))
                .size(13.0)
                .color(MUTED),
        ),
    }
}

fn tree_row(icon: &'static str, label: &'static str) -> impl View {
    Row::new()
        .gap(8.0)
        .padding_xy(4.0, 3.0)
        .align_items(Align::Center)
        .child(Icon::new(icon).size(14.0).color(ACCENT))
        .child(Text::new(label).size(13.0).color(TEXT))
}

fn explorer() -> AnyView {
    let tree = Column::new()
        .fill_width()
        .padding(12.0)
        .gap(2.0)
        .child(Text::new("PROJECT").size(11.0).bold().color(MUTED))
        .child(tree_row("folder", "src"))
        .child(tree_row("file", "main.rs"))
        .child(tree_row("file", "shell.rs"))
        .child(tree_row("folder", "examples"))
        .child(tree_row("file", "Cargo.toml"))
        .child(tree_row("file", "README.md"))
        .child(Text::new("DOCK PANELS").size(11.0).bold().color(MUTED))
        .child(tree_row("file", "Editor"))
        .child(tree_row("eye", "Preview"))
        .child(tree_row("sliders", "Inspector"))
        .child(tree_row("play", "Console"));

    AnyView::new(
        Container::fill()
            .background(PANEL_BG)
            .child(ScrollView::new(tree).vertical().fill_size()),
    )
}

fn editor() -> AnyView {
    let code = [
        "fn main() {",
        "    let model = Signal::new(default_layout());",
        "",
        "    App::new()",
        "        .title(\"Dock example\")",
        "        .root(shell(model))",
        "        .run()",
        "        .expect(\"event loop failed\");",
        "}",
    ];

    let mut lines = Column::new().fill_width().padding(16.0).gap(3.0);
    for (index, line) in code.iter().enumerate() {
        lines = lines.child(
            Row::new()
                .gap(12.0)
                .child(
                    Text::new(format!("{:>2}", index + 1))
                        .size(12.0)
                        .color(MUTED),
                )
                .child(Text::new(*line).size(13.0).font("monospace").color(TEXT)),
        );
    }

    AnyView::new(
        Container::fill()
            .background(PANEL_BG)
            .child(ScrollView::new(lines).vertical().fill_size()),
    )
}

fn preview() -> AnyView {
    let stage = Column::new()
        .center()
        .gap(8.0)
        .fill_size()
        .child(Icon::new("layers").size(40.0).color(ACCENT))
        .child(Text::new("Canvas area").size(13.0).color(MUTED))
        .child(
            Text::new("Float or dock the Inspector to inspect it.")
                .size(12.0)
                .color(MUTED),
        );

    AnyView::new(
        Container::fill().background(STAGE_BG).padding(16.0).child(
            Container::fill()
                .background(Color::hex(0x181B20))
                .radius(8.0)
                .child(stage),
        ),
    )
}

fn inspector() -> AnyView {
    let rows = [
        ("Fill", "#7FD1FF"),
        ("Stroke", "#8A93A6"),
        ("Radius", "8 px"),
        ("Opacity", "100%"),
        ("Blend", "Normal"),
    ];

    let mut body = Column::new()
        .fill_width()
        .padding(14.0)
        .gap(9.0)
        .child(Text::new("APPEARANCE").size(11.0).bold().color(MUTED));
    for (name, value) in rows {
        body = body.child(
            Row::new()
                .fill_width()
                .gap(8.0)
                .child_flex(Text::new(name).size(13.0).color(TEXT), 1.0)
                .child(Text::new(value).size(13.0).color(MUTED)),
        );
    }
    body = body
        .child(Divider::horizontal())
        .child(Text::new("LAYOUT").size(11.0).bold().color(MUTED))
        .child(
            Row::new()
                .gap(8.0)
                .child(Icon::new("frame").size(14.0).color(ACCENT))
                .child(Text::new("Docked right").size(13.0).color(TEXT)),
        );

    AnyView::new(
        Container::fill()
            .background(PANEL_BG)
            .child(ScrollView::new(body).vertical().fill_size()),
    )
}

fn console() -> AnyView {
    let lines = [
        "$ cargo run --example dock -p sequoia-ui",
        "   Compiling sequoia-ui v0.1.6",
        "   Finished dev profile",
        "",
        "Drag a tab to dock, split, or float a panel.",
        "Changes stay in the DockModel signal.",
    ];

    let mut body = Column::new().fill_width().padding(14.0).gap(3.0);
    for line in lines {
        let color = if line.starts_with('$') {
            CONSOLE_GREEN
        } else {
            TEXT
        };
        body = body.child(Text::new(line).size(12.0).font("monospace").color(color));
    }

    AnyView::new(
        Container::fill()
            .background(STAGE_BG)
            .child(ScrollView::new(body).vertical().fill_size()),
    )
}

fn toolbar(
    status: Signal<String>,
    reset: impl Fn() + 'static,
    save: impl Fn() + 'static,
    load: impl Fn() + 'static,
    float_inspector: impl Fn() + 'static,
) -> impl View {
    Row::new()
        .fill_width()
        .height(46.0)
        .padding_xy(14.0, 7.0)
        .gap(8.0)
        .align_items(Align::Center)
        .background(BAR_BG)
        .child(Icon::new("layers").size(18.0).color(ACCENT))
        .child(Text::new("Dock").size(15.0).bold().color(TEXT))
        .child(Text::with(move || status.get()).size(12.0).color(MUTED))
        .child_flex(Unit, 1.0)
        .child(
            Button::labeled("Reset")
                .style(ButtonStyle::SECONDARY)
                .on_click(reset),
        )
        .child(
            Button::labeled("Save")
                .style(ButtonStyle::SECONDARY)
                .on_click(save),
        )
        .child(
            Button::labeled("Load")
                .style(ButtonStyle::SECONDARY)
                .on_click(load),
        )
        .child(
            Button::labeled("Float / dock inspector")
                .style(ButtonStyle::PRIMARY)
                .on_click(float_inspector),
        )
}

fn main() {
    let store = DockLayoutStore::new(Settings::for_app("com", "NurtStudio", "SequoiaDockExample"))
        .file_name("dock-layout.txt");

    let model = Signal::new(store.load().unwrap_or_else(default_layout));
    let status = Signal::new(String::from("Drag a tab to rearrange the layout"));

    let reset_model = model.clone();
    let reset_status = status.clone();
    let reset = move || {
        reset_model.set(default_layout());
        reset_status.set(String::from("Reset to the default layout"));
    };

    let save_store = store.clone();
    let save_model = model.clone();
    let save_status = status.clone();
    let save = move || {
        let result = save_store.save(&save_model.get_untracked());
        save_status.set(match result {
            Ok(()) => String::from("Saved dock layout"),
            Err(error) => format!("Save failed: {error}"),
        });
    };

    let load_store = store.clone();
    let load_model = model.clone();
    let load_status = status.clone();
    let load = move || match load_store.load() {
        Some(saved) => {
            load_model.set(saved);
            load_status.set(String::from("Loaded saved layout"));
        }
        None => load_status.set(String::from("No saved layout yet")),
    };

    let float_model = model.clone();
    let float_status = status.clone();
    let float_inspector = move || {
        let floating = float_model.get_untracked().is_floating(INSPECTOR);
        float_model.update(|m| {
            if floating {
                let _ = m.apply(DockOp::DockFloating {
                    panel: INSPECTOR,
                    target: EDITOR,
                    drop: DockDrop::Tab,
                    frac: 0.5,
                });
            } else {
                let _ = m.apply(DockOp::Float {
                    panel: INSPECTOR,
                    x: 560.0,
                    y: 110.0,
                    w: 320.0,
                    h: 300.0,
                });
            }
        });
        let now_floating = float_model.get_untracked().is_floating(INSPECTOR);
        float_status.set(String::from(if now_floating {
            "Inspector is floating"
        } else {
            "Inspector is docked"
        }));
    };

    let dock = DockView::new(model.clone(), panel_view)
        .title_provider(panel_title)
        .fill_size();
    let floats = DockFloatLayer::new(model.clone(), panel_view)
        .title_provider(panel_title)
        .card_background(PANEL_BG)
        .header_background(BAR_BG)
        .fill_size();

    let workspace = Container::fill()
        .background(APP_BG)
        .child(Stack::new().fill_size().child(dock).child(floats));

    let root = Column::new()
        .fill_size()
        .background(APP_BG)
        .child(toolbar(status, reset, save, load, float_inspector))
        .child_flex(workspace, 1.0);

    App::new()
        .title("Dock example")
        .size(1180.0, 760.0)
        .theme(Theme::dark())
        .root(root)
        .run()
        .expect("event loop failed");
}
