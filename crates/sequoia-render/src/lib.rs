//! GPU renderer for the Sequoia GUI framework.
//!
//! Translates a [`sequoia_core::RenderList`] into wgpu draw calls: batching
//! paint items into instanced runs, uploading glyph and image data, and
//! compositing cached surfaces, backdrops, and filters.
//!
//! - [`gpu`] — pipelines, buffers, textures, and shaders.
//! - [`color`] — color-space helpers for surface and texture formats.
//! - [`pass`] — backdrop, filter, and GUI render passes.
//! - [`renderer`] — frame orchestration and the top-level renderer.
//! - [`scene`] — instance conversion and composite planning.
//! - [`shared`] — a device and image cache shared by several windows.
//! - [`surface`] — embeddable renderer surfaces.
//! - [`types`] — GPU types, options, statistics, and errors.
//! - [`utils`] — shared helpers.

#![forbid(unsafe_code)]

pub mod color;
pub mod gpu;
pub mod pass;
pub mod renderer;
pub mod scene;
pub mod shared;
pub mod surface;
pub mod types;
pub mod utils;

#[cfg(test)]
mod tests;

pub use color::{ColorSpace, linear_to_srgb, srgb_to_linear};
pub use gpu::shaders::{compose_user_shader, passthrough_wgsl, validate_user_shader};
pub use renderer::{GpuRenderer, PreinitGpu, preinit_gpu};
pub use shared::SharedGpu;
pub use surface::{PendingFrames, Revision, WgpuFrame, WgpuRenderer, WgpuSurface};
pub use types::{
    FrameSkip, GpuBackend, GpuOptions, MemoryBudget, MemoryUsage, PowerPreference, PresentMode,
    QualityLevel, Region, RenderError, RenderStats,
};

pub use wgpu;
