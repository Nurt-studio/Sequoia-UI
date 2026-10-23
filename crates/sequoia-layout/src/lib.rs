//! Layout engine for the Sequoia GUI framework.
//!
//! Resolves sizes and positions for the view tree: flex distribution, grid
//! tracks, docked panels, wrapping, and text measurement through
//! [`LayoutCtx`].
//!
//! - [`box_model`] — padding, margin, insets, and aspect-ratio helpers.
//! - [`container`] — axes, alignment, justification, and container styles.
//! - [`ctx`] — text measurement used during layout.
//! - [`dock`] — docking sides, thickness, and preview rectangles.
//! - [`flex`] — flex grow/shrink distribution and justification.
//! - [`grid`] — grid tracks, placement, and auto flow.
//! - [`masonry`] — waterfall columns with variable item heights.
//! - [`overflow`] — overflow and scrolling modes.
//! - [`resolve`] — length resolution and main-axis distribution.
//! - [`style`] — per-view layout styles and sanitizers.
//! - [`wrap`] — wrapping lines for flow containers.

#![forbid(unsafe_code)]

pub mod box_model;
pub mod container;
pub mod ctx;
pub mod dock;
pub mod flex;
pub mod grid;
pub mod masonry;
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
pub use masonry::{MasonryPlacement, MasonryStyle, masonry_assign, masonry_column_width};
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
