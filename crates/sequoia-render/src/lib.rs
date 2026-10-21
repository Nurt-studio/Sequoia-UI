//! GPU renderer for the Sequoia GUI framework.
//!
//! Translates a [`sequoia_core::RenderList`] into wgpu draw calls: batching
//! paint items into instanced runs, uploading glyph and image data, and
//! compositing cached surfaces, backdrops, and filters.
//!
//! - [`gpu`] — pipelines, buffers, textures, and shaders.
//! - [`pass`] — backdrop, filter, and GUI render passes.
//! - [`renderer`] — frame orchestration and the top-level renderer.
//! - [`scene`] — instance conversion and composite planning.
//! - [`surface`] — embeddable renderer surfaces.
//! - [`types`] — GPU types, options, statistics, and errors.
//! - [`utils`] — shared helpers.

#![forbid(unsafe_code)]

pub mod gpu;
pub mod pass;
pub mod renderer;
pub mod scene;
pub mod surface;
pub mod types;
pub mod utils;

#[cfg(test)]
mod tests;

pub use renderer::GpuRenderer;
pub use surface::{PendingFrames, Revision, WgpuFrame, WgpuRenderer, WgpuSurface};
pub use types::{
    FrameSkip, GpuBackend, GpuOptions, PowerPreference, PresentMode, Region, RenderError,
    RenderStats,
};
