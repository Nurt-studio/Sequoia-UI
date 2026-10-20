#![forbid(unsafe_code)]

pub mod box_model;
pub mod container;
pub mod ctx;
pub mod dock;
pub mod flex;
pub mod grid;
pub mod overflow;
pub mod resolve;
pub mod style;
pub mod wrap;

pub use box_model::{
    ShrinkAxis, apply_aspect_ratio, inset_rect, shrink_by_margin, shrink_by_padding,
};
pub use container::{Align, Axis, ContainerStyle, Justify};
pub use ctx::LayoutCtx;
pub use dock::{
    Dock, DockStyle, dock_layout_order, dock_preview_rect, dock_thickness, effective_dock,
    resolve_dock, sanitize_frac, sanitize_gap, sanitize_preview_frac, split_axis_for_dock,
};
pub use flex::{FlexItem, distribute_flex, justify_layout};
pub use grid::{
    GridFlow, GridPlacement, GridStyle, Track, auto_cells, resolve_tracks, resolve_tracks_clamped,
    track_offsets,
};
pub use overflow::Overflow;
pub use resolve::{
    clamp_to, distribute_main, distribute_main_clamped, resolve_cross, resolve_flex_basis,
    resolve_main,
};
pub use style::{
    Indents, Inset, LayoutStyle, Position, sanitize_flex, sanitize_indent, sanitize_ratio,
    sanitize_shrink,
};
pub use wrap::{WrapStyle, wrap_lines};
