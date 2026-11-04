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

fn main() {
    let state = EditorState::demo();

    let app = App::new()
        .title("Vision Design")
        .size(1440.0, 900.0)
        .theme(app::editor_theme())
        .icons_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icons"))
        .default_font("IBM Plex Sans");

    app::shortcuts::install(app, &state)
        .root(app::root(state))
        .backend(sequoia_ui::render::GpuBackend::Dx12)
        .run()
        .expect("failed to start");
}