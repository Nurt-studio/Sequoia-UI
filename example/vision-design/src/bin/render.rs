//! Headless renderer for the Vision Design preview used by the README.
//!
//! Run from the workspace root with:
//!     cargo run -p vision-design --bin render
//!
//! An optional path argument overrides `docs/src/assets/vision-design.png`.

#![allow(dead_code)]
#![allow(unreachable_pub)]

#[path = "../app/mod.rs"]
mod app;
#[path = "../canvas/mod.rs"]
mod canvas;
#[path = "../codegen/mod.rs"]
mod codegen;
#[path = "../components/mod.rs"]
mod components;
#[path = "../core/mod.rs"]
mod core;
#[path = "../document/mod.rs"]
mod document;
#[path = "../model/mod.rs"]
mod model;
#[path = "../node_graph/mod.rs"]
mod node_graph;
#[path = "../overlays/mod.rs"]
mod overlays;
#[path = "../panels/mod.rs"]
mod panels;
#[path = "../state/mod.rs"]
mod state;
#[path = "../theme/mod.rs"]
mod theme;

use std::path::PathBuf;

use sequoia_ui::assets::AssetRegistry;
use sequoia_ui::export::{ExportOptions, export_png_with_assets};

use crate::state::EditorState;

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest
        .parent()
        .and_then(|path| path.parent())
        .expect("vision-design lives under example/");
    let out = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.join("docs/src/assets/vision-design.png"));

    let assets = AssetRegistry::with_builtin_icons();
    assets.set_default_font("IBM Plex Sans");
    let report = assets
        .register_icon_dir(manifest.join("assets/icons"))
        .expect("read vision-design icons");
    if !report.failed.is_empty() {
        eprintln!("{} icons failed to load", report.failed.len());
    }

    let theme = app::editor_theme();
    let opts = ExportOptions::new(1440.0, 900.0)
        .scale(1.0)
        .theme(theme)
        .background(theme.background);
    let path = export_png_with_assets(app::root(EditorState::demo()), &opts, assets, &out)
        .expect("render vision-design preview");
    println!("wrote {}", path.display());
}
