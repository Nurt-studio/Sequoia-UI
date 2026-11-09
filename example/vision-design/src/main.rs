//! Application bootstrap: modules, window and startup only.

#![allow(dead_code)]
#![allow(unreachable_pub)]

mod app;
mod canvas;
mod codegen;
mod components;
mod core;
mod document;
mod model;
mod node_graph;
mod overlays;
mod panels;
mod state;
mod theme;

use sequoia_ui::prelude::*;

use crate::state::EditorState;

/// Value following `--flag` on the command line.
fn arg_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    let i = args.iter().position(|a| a == flag)?;
    args.get(i + 1).map(String::as_str)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let stress = arg_value(&args, "--stress").and_then(|v| v.parse().ok());
    let bench = arg_value(&args, "--bench").and_then(|v| v.parse().ok());

    // `--stress N` opens the editor with N rectangles on the canvas.
    let state = match stress {
        Some(n) => EditorState::with_seed(Some(document::demo::stress_tree(n)), None),
        None => EditorState::demo(),
    };

    let app = App::new()
        .title("Vision Design")
        .size(1440.0, 900.0)
        .decorations(false)
        .theme(app::editor_theme())
        .icons_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icons"))
        .default_font("IBM Plex Sans")
        .show_fps(true);

    // `--bench N` renders N frames in the window, prints the timings, then exits.
    let app = match bench {
        Some(frames) => app.benchmark(frames),
        None => app,
    };

    app::shortcuts::install(app, &state)
        .root(app::root(state))
        .backend(sequoia_ui::render::GpuBackend::Dx12)
        .run()
        .expect("failed to start");
}
